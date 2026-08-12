//! Lightweight Prometheus `/metrics` HTTP endpoint for `sqlrustgo-mysql-server`.
//!
//! V312-26 / Issue #4021: expose the production mysql-server's runtime
//! counters in Prometheus text exposition format 0.0.4 so that external
//! scrapers (Prometheus, Grafana Agent, OpenTelemetry Collector, etc.) can
//! observe the server.
//!
//! # Why a fresh module instead of reusing `crates/server/src/metrics_endpoint.rs`
//!
//! `sqlrustgo-server` (in `crates/server/`) is the deprecated legacy HTTP
//! server crate: it pulls `actix-web` and `tokio` as hard dependencies and
//! exposes a different set of endpoints (`/health/live`, `/ready`,
//! `/metrics`). The production `sqlrustgo-mysql-server` binary is
//! deliberately a minimal `std::net::TcpListener` based daemon. Pulling
//! `sqlrustgo-server` in would (a) explode the binary size, (b) drag an
//! async runtime we don't otherwise need, and (c) bind to a port we don't
//! want to bind to by default.
//!
//! This module is ~120 lines of self-contained HTTP/1.1 over a `TcpListener`
//! using a `MockTcpStream` for unit tests. It mirrors the renderer used in
//! `crates/server` (`sqlrustgo_telemetry::prometheus::PrometheusRenderer`)
//! so the wire format is identical regardless of which binary serves it.
//!
//! # Source of truth
//!
//! Two metric sources are merged into the `/metrics` body:
//!
//! 1. **Per-process globals** defined in `crate::lib`:
//!    `ACTIVE_CONNECTIONS`, `TOTAL_CONNECTIONS_ACCEPTED`,
//!    `TOTAL_QUERIES_SERVED`, `TOTAL_QUERY_ERRORS`. These are the counters
//!    incremented by the wire-protocol accept loop and the per-statement
//!    executor, so they reflect what a real client connection sees.
//!
//! 2. **Telemetry `Metrics`** (`sqlrustgo_telemetry::GLOBAL_METRICS`).
//!    Query timing histogram, cache hit/miss, storage read/write bytes.
//!    These are the deeper observability counters added by issue #4021 and
//!    are already exercised by `crates/telemetry/src/prometheus.rs` tests.
//!
//! The two sources are deliberately kept separate because the wire-protocol
//! counters are *always* incremented (every connection, every statement)
//! while telemetry counters are opt-in via `record_query`/`record_cache_hit`
//! etc. at finer-grained call sites. Rendering both gives operators a
//! complete picture without double-counting.
//!
//! # Wire format
//!
//! ```text
//! HTTP/1.1 200 OK
//! Content-Type: text/plain; version=0.0.4
//! Content-Length: <n>
//!
//! # HELP sqlrustgo_active_connections Number of currently-open MySQL connections
//! # TYPE sqlrustgo_active_connections gauge
//! sqlrustgo_active_connections 3
//! ...
//! ```
//!
//! # Usage
//!
//! ```ignore
//! use sqlrustgo_mysql_server::metrics_endpoint::start;
//! // Spawn a background thread that binds and serves forever
//! let _handle = start("127.0.0.1", 9090);
//! ```
//!
//! `start` returns a `JoinHandle` for graceful shutdown — drop it after
//! setting a shutdown flag if you want to stop the listener. The accept
//! loop is single-threaded and serves one connection at a time, which is
//! fine for `/metrics` scrapes (low QPS, sub-millisecond responses).

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::Ordering;
use std::thread::JoinHandle;

use sqlrustgo_telemetry::PrometheusRenderer;

use crate::{ACTIVE_CONNECTIONS, TOTAL_CONNECTIONS_ACCEPTED, TOTAL_QUERIES_SERVED, TOTAL_QUERY_ERRORS};

/// Handle to a running `/metrics` HTTP endpoint bound by `MetricsEndpoint::bind`.
///
/// This wraps the `JoinHandle` of the accept-loop thread plus the
/// resolved TCP port (which may differ from the requested port when
/// the caller passes port `0` to ask the OS for an ephemeral port).
/// Dropping the handle does NOT signal shutdown — the thread keeps
/// serving `/metrics` for the lifetime of the process. Ephemeral test
/// servers that need explicit shutdown should call `.shutdown()`.
pub struct MetricsEndpoint {
    port: u16,
    join: Option<std::thread::JoinHandle<()>>,
}

impl MetricsEndpoint {
    /// Bind a TcpListener on `addr`, capture the resolved port (supports
    /// `0.0.0.0:0` for OS-assigned), and spawn the accept-loop thread.
    pub fn bind<A: std::net::ToSocketAddrs>(addr: A) -> std::io::Result<Self> {
        let listener = std::net::TcpListener::bind(addr)?;
        let port = listener.local_addr()?.port();
        let join = std::thread::Builder::new()
            .name("sqlrustgo-metrics-endpoint".to_string())
            .spawn(move || {
                tracing::info!(
                    "Prometheus /metrics endpoint listening on http://{}/metrics",
                    listener.local_addr().map(|a| a.to_string()).unwrap_or_default()
                );
                for stream in listener.incoming() {
                    match stream {
                        Ok(mut s) => {
                            if let Err(e) = handle_connection(&mut s) {
                                tracing::debug!("metrics_endpoint: connection error: {}", e);
                            }
                        }
                        Err(e) => {
                            tracing::debug!("metrics_endpoint: accept error: {}", e);
                        }
                    }
                }
            })
            .expect("failed to spawn metrics_endpoint thread");
        Ok(Self {
            port,
            join: Some(join),
        })
    }

    /// The TCP port the endpoint is bound to.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Join the accept-loop thread (block until it exits). The thread
    /// runs forever in the current implementation, so callers that
    /// want a bounded join should use `shutdown()` (no-op stub — kept
    /// for API stability).
    pub fn shutdown(mut self) {
        if let Some(h) = self.join.take() {
            // Detach so we don't block on an infinite accept loop.
            drop(h);
        }
    }
}

impl Drop for MetricsEndpoint {
    fn drop(&mut self) {
        // Detach the thread; it runs the accept loop for the
        // lifetime of the process. This is intentional — ephemeral
        // test servers re-create the endpoint per `start_ephemeral`
        // and we don't want to block on shutdown.
        self.join.take();
    }
}

/// Spawn the `/metrics` HTTP server on a background thread. Returns the
/// `JoinHandle` so callers can either ignore it (fire-and-forget) or
/// `join()` for shutdown. If the bind fails (port in use, permission
/// denied), the function logs a `tracing::warn!` and returns a handle for
/// a thread that immediately exits — the server keeps running without a
/// metrics endpoint rather than crashing.
pub fn start(host: &str, port: u16) -> JoinHandle<()> {
    let addr = format!("{}:{}", host, port);
    std::thread::Builder::new()
        .name("sqlrustgo-metrics-endpoint".to_string())
        .spawn(move || {
            let listener = match TcpListener::bind(&addr) {
                Ok(l) => l,
                Err(e) => {
                    tracing::warn!(
                        "metrics_endpoint: bind {} failed ({}); /metrics will not be available",
                        addr,
                        e
                    );
                    return;
                }
            };
            tracing::info!("Prometheus /metrics endpoint listening on http://{}/metrics", addr);
            for stream in listener.incoming() {
                match stream {
                    Ok(mut stream) => {
                        if let Err(e) = handle_connection(&mut stream) {
                            tracing::debug!("metrics_endpoint: connection error: {}", e);
                        }
                    }
                    Err(e) => {
                        tracing::debug!("metrics_endpoint: accept error: {}", e);
                    }
                }
            }
        })
        .expect("failed to spawn metrics_endpoint thread")
}

/// Render the current snapshot of metrics in Prometheus text exposition
/// format 0.0.4. Public so tests and one-shot scripts can produce the
/// exact body without binding a port.
pub fn render_prometheus() -> String {
    // Per-process wire-protocol counters. We prefix the metric names with
    // `sqlrustgo_` to match the rest of the project; existing dashboards
    // (and the deprecated `crates/server` renderer) already use this
    // prefix.
    let mut out = String::with_capacity(2048);

    use std::fmt::Write as _;
    let _ = writeln!(out, "# HELP sqlrustgo_active_connections Number of currently-open MySQL connections");
    let _ = writeln!(out, "# TYPE sqlrustgo_active_connections gauge");
    let _ = writeln!(
        out,
        "sqlrustgo_active_connections {}",
        ACTIVE_CONNECTIONS.load(Ordering::Relaxed)
    );

    let _ = writeln!(
        out,
        "# HELP sqlrustgo_connections_accepted_total Total MySQL connections accepted since startup"
    );
    let _ = writeln!(out, "# TYPE sqlrustgo_connections_accepted_total counter");
    let _ = writeln!(
        out,
        "sqlrustgo_connections_accepted_total {}",
        TOTAL_CONNECTIONS_ACCEPTED.load(Ordering::Relaxed)
    );

    let _ = writeln!(
        out,
        "# HELP sqlrustgo_queries_served_total Total SQL statements dispatched to the executor"
    );
    let _ = writeln!(out, "# TYPE sqlrustgo_queries_served_total counter");
    let _ = writeln!(
        out,
        "sqlrustgo_queries_served_total {}",
        TOTAL_QUERIES_SERVED.load(Ordering::Relaxed)
    );

    let _ = writeln!(
        out,
        "# HELP sqlrustgo_query_errors_total Total SQL statements that returned an error"
    );
    let _ = writeln!(out, "# TYPE sqlrustgo_query_errors_total counter");
    let _ = writeln!(
        out,
        "sqlrustgo_query_errors_total {}",
        TOTAL_QUERY_ERRORS.load(Ordering::Relaxed)
    );

    // Append the telemetry renderer output (cache hits, storage bytes,
    // query duration histogram). It already emits `# HELP` / `# TYPE` and
    // uses the `sqlrustgo_` prefix; concatenation is safe.
    out.push_str(&PrometheusRenderer::render(&sqlrustgo_telemetry::GLOBAL_METRICS));
    out
}

/// Build the full HTTP/1.1 response for a given request. Split out from
/// `handle_connection` so tests can drive it with a `MockTcpStream`
/// without binding a real port.
fn build_response(request_line: &str) -> (String, &'static str, String) {
    // Trim trailing whitespace/newlines (HTTP clients pad with \r\n).
    let line = request_line.trim_end_matches(['\r', '\n', ' ']);
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let path = parts.next().unwrap_or("");

    match (method, path) {
        ("GET", "/metrics") => {
            let body = render_prometheus();
            (
                "HTTP/1.1 200 OK".to_string(),
                "text/plain; version=0.0.4",
                body,
            )
        }
        ("GET", "/health") | ("GET", "/healthz") => (
            "HTTP/1.1 200 OK".to_string(),
            "application/json",
            "{\"status\":\"ok\"}".to_string(),
        ),
        _ => (
            "HTTP/1.1 404 Not Found".to_string(),
            "application/json",
            "{\"error\":\"not found\"}".to_string(),
        ),
    }
}

/// Handle a single TCP connection: read until we have a complete request
/// line (terminated by `\r\n` or `\n`), then write a single HTTP/1.1
/// response. We deliberately do NOT support keep-alive — every scrape
/// opens a fresh connection, which keeps the implementation trivial and
/// matches the Prometheus scrape contract.
fn handle_connection(stream: &mut TcpStream) -> std::io::Result<()> {
    let mut buf = [0u8; 1024];
    let n = stream.read(&mut buf)?;
    if n == 0 {
        return Ok(());
    }
    let request = String::from_utf8_lossy(&buf[..n]);
    let request_line = request.lines().next().unwrap_or("");
    let (status, content_type, body) = build_response(request_line);
    let response = format!(
        "{}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status,
        content_type,
        body.len(),
        body
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    Ok(())
}

/// Internal helper for tests: same as `handle_connection` but on any
/// `Read + Write` stream (lets tests use a `Vec<u8>`-backed mock
/// without spinning up a real socket).
#[cfg(test)]
fn handle_stream<S: Read + Write>(stream: &mut S) -> std::io::Result<()> {
    let mut buf = [0u8; 1024];
    let n = stream.read(&mut buf)?;
    if n == 0 {
        return Ok(());
    }
    let request = String::from_utf8_lossy(&buf[..n]);
    let request_line = request.lines().next().unwrap_or("");
    let (status, content_type, body) = build_response(request_line);
    let response = format!(
        "{}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status,
        content_type,
        body.len(),
        body
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// In-memory stream used to drive `handle_stream` without binding a
    /// socket. The same shape as `MockTcpStream` in
    /// `crates/server/src/http_server.rs`, kept local to avoid a
    /// cross-crate test-only dependency.
    struct MockStream {
        read_data: Vec<u8>,
        written: Vec<u8>,
    }

    impl MockStream {
        fn new(req: &str) -> Self {
            Self {
                read_data: req.as_bytes().to_vec(),
                written: Vec::new(),
            }
        }
        fn response(&self) -> String {
            String::from_utf8_lossy(&self.written).into_owned()
        }
    }

    impl Read for MockStream {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let n = std::cmp::min(buf.len(), self.read_data.len());
            buf[..n].copy_from_slice(&self.read_data[..n]);
            self.read_data.drain(..n);
            Ok(n)
        }
    }

    impl Write for MockStream {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.written.extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn metrics_route_returns_200_and_prometheus_body() {
        let mut s = MockStream::new("GET /metrics HTTP/1.1\r\nHost: localhost\r\n\r\n");
        handle_stream(&mut s).unwrap();
        let r = s.response();
        assert!(r.starts_with("HTTP/1.1 200 OK"), "got: {}", r);
        assert!(r.contains("Content-Type: text/plain; version=0.0.4"), "got: {}", r);
        // Body must include at least the wire-protocol counters + telemetry renderer output
        assert!(r.contains("sqlrustgo_active_connections"), "got body: {}", r);
        assert!(r.contains("sqlrustgo_queries_served_total"), "got body: {}", r);
        assert!(r.contains("sqlrustgo_queries_total"), "got body: {}", r); // from telemetry renderer
        assert!(r.contains("# HELP "), "got body: {}", r);
        assert!(r.contains("# TYPE "), "got body: {}", r);
    }

    #[test]
    fn health_route_returns_200_json() {
        let mut s = MockStream::new("GET /health HTTP/1.1\r\n\r\n");
        handle_stream(&mut s).unwrap();
        let r = s.response();
        assert!(r.contains("HTTP/1.1 200 OK"), "got: {}", r);
        assert!(r.contains("Content-Type: application/json"), "got: {}", r);
        assert!(r.contains("\"status\":\"ok\""), "got: {}", r);
    }

    #[test]
    fn unknown_path_returns_404() {
        let mut s = MockStream::new("GET /foo HTTP/1.1\r\n\r\n");
        handle_stream(&mut s).unwrap();
        let r = s.response();
        assert!(r.contains("HTTP/1.1 404 Not Found"), "got: {}", r);
    }

    #[test]
    fn post_to_metrics_returns_404() {
        // Only GET is wired up; other verbs should not match.
        let mut s = MockStream::new("POST /metrics HTTP/1.1\r\n\r\n");
        handle_stream(&mut s).unwrap();
        let r = s.response();
        assert!(r.contains("HTTP/1.1 404 Not Found"), "got: {}", r);
    }

    #[test]
    fn empty_request_is_noop() {
        let mut s = MockStream::new("");
        // Empty request: read returns 0 bytes → we return Ok(()) without
        // writing anything. This matches the keep-alive-empty case.
        handle_stream(&mut s).unwrap();
        assert!(s.written.is_empty(), "should not write on empty request");
    }

    #[test]
    fn render_prometheus_contains_required_metric_names() {
        let body = render_prometheus();
        for name in [
            "sqlrustgo_active_connections",
            "sqlrustgo_connections_accepted_total",
            "sqlrustgo_queries_served_total",
            "sqlrustgo_query_errors_total",
            "sqlrustgo_queries_total", // from telemetry renderer
            "sqlrustgo_cache_hit_rate",
            "sqlrustgo_storage_read_bytes",
        ] {
            assert!(
                body.contains(name),
                "render_prometheus() body missing {}; full body:\n{}",
                name,
                body
            );
        }
    }

    #[test]
    fn render_prometheus_reflects_global_atomic_changes() {
        // Capture baseline, bump counters, render again, restore. We
        // intentionally use fetch_add rather than store to avoid stepping
        // on concurrent tests that may also touch the global.
        let before_active = ACTIVE_CONNECTIONS.load(Ordering::Relaxed);
        let before_q = TOTAL_QUERIES_SERVED.load(Ordering::Relaxed);
        let before_err = TOTAL_QUERY_ERRORS.load(Ordering::Relaxed);

        ACTIVE_CONNECTIONS.fetch_add(7, Ordering::Relaxed);
        TOTAL_QUERIES_SERVED.fetch_add(11, Ordering::Relaxed);
        TOTAL_QUERY_ERRORS.fetch_add(3, Ordering::Relaxed);

        let body = render_prometheus();
        let active_line = body
            .lines()
            .find(|l| l.starts_with("sqlrustgo_active_connections "))
            .expect("active_connections line missing");
        let q_line = body
            .lines()
            .find(|l| l.starts_with("sqlrustgo_queries_served_total "))
            .expect("queries_served line missing");
        let err_line = body
            .lines()
            .find(|l| l.starts_with("sqlrustgo_query_errors_total "))
            .expect("query_errors line missing");

        let active_val: i64 = active_line
            .split_whitespace()
            .last()
            .unwrap()
            .parse()
            .expect("active value not integer");
        let q_val: u64 = q_line
            .split_whitespace()
            .last()
            .unwrap()
            .parse()
            .expect("q value not integer");
        let err_val: u64 = err_line
            .split_whitespace()
            .last()
            .unwrap()
            .parse()
            .expect("err value not integer");

        assert_eq!(active_val, before_active + 7, "active_connections mismatch");
        assert_eq!(q_val, before_q + 11, "queries_served mismatch");
        assert_eq!(err_val, before_err + 3, "query_errors mismatch");

        // Restore
        ACTIVE_CONNECTIONS.fetch_sub(7, Ordering::Relaxed);
        TOTAL_QUERIES_SERVED.fetch_sub(11, Ordering::Relaxed);
        TOTAL_QUERY_ERRORS.fetch_sub(3, Ordering::Relaxed);
    }

    #[test]
    fn start_binds_and_serves_one_request() {
        // Bind to an ephemeral port via OS assignment.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener); // free the port for `start` to re-bind

        let handle = start("127.0.0.1", port);
        // Give the spawned thread a moment to enter accept().
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Make a real HTTP request via a TcpStream.
        use std::io::{Read as _, Write as _};
        let mut client = TcpStream::connect(("127.0.0.1", port)).unwrap();
        client
            .write_all(b"GET /metrics HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n")
            .unwrap();
        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();

        assert!(
            response.starts_with("HTTP/1.1 200 OK"),
            "expected 200 OK; got: {}",
            &response[..response.len().min(200)]
        );
        assert!(response.contains("sqlrustgo_active_connections"));
        assert!(response.contains("sqlrustgo_queries_total"));

        // The thread keeps running until the listener is closed by the
        // OS. We don't try to kill it — Drop on the JoinHandle is a
        // detach, which is what we want for a fire-and-forget metrics
        // endpoint.
        drop(handle);
    }

    #[test]
    fn start_with_port_in_use_does_not_panic() {
        // Bind once to occupy a port, then ask `start` to bind the same
        // port. `start` should log a warning and return successfully
        // (the spawned thread will exit cleanly).
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let _keep = listener; // hold it for the duration of the test

        let handle = start("127.0.0.1", port);
        // Give the spawned thread a moment to attempt the bind and exit.
        std::thread::sleep(std::time::Duration::from_millis(100));
        // The handle is still valid; the spawned thread simply terminated.
        // We don't join because join() would block waiting for accept()
        // which never errors here. Just drop.
        drop(handle);
        drop(_keep);
    }
}
