//! L3 acceptance test for the canonical `sqlrustgo-mysql-server`
//! binary.
//!
//! Unlike the in-process `embedded_harness_*` and
//! `wire_protocol_smoke` tests (which drive the server through
//! `start_ephemeral`), this test spawns the **actual compiled
//! `sqlrustgo-mysql-server` binary** as a subprocess, then
//! connects to it over the wire. That exercises:
//!
//! - the canonical binary's `serve` subcommand and clap parsing
//! - the production `run_server` entry point
//! - the full MySQL wire protocol (handshake, auth, COM_QUERY)
//! - shutdown when the subprocess receives SIGTERM
//!
//! This is the gate the openspec change calls "L3 acceptance":
//! prove the canonical binary works, not just the library.
//!
//! Refs: `openspec/changes/mysql-server-canonical-entry/specs/mysql-server-canonical-entry/spec.md`

mod common;

use common::MySqlTestClient;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

/// Locate the workspace root from `CARGO_MANIFEST_DIR` (which for
/// integration tests is the root package).
fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn canonical_binary() -> PathBuf {
    let mut p = workspace_root();
    p.push("target");
    p.push("debug");
    p.push("sqlrustgo-mysql-server");
    p
}

struct SubprocessHandle {
    child: Child,
    port: u16,
}

impl Drop for SubprocessHandle {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn spawn_canonical_server() -> SubprocessHandle {
    let bin = canonical_binary();
    assert!(
        bin.exists(),
        "canonical binary not found at {bin:?}; \
         run `cargo build -p sqlrustgo-mysql-server` first"
    );

    // Bind the listener on an OS-assigned port. We can't ask the
    // server to do `port = 0` via the CLI, so we pre-pick a free
    // port and pass it in. To avoid races we let the OS pick via
    // TcpListener::bind((host, 0)) below, then close the
    // listener; small window for reuse but acceptable for an
    // acceptance test.
    let probe = std::net::TcpListener::bind("127.0.0.1:0").expect("bind probe port");
    let port = probe.local_addr().expect("probe local_addr").port();
    drop(probe);

    let child = Command::new(&bin)
        .arg("serve")
        .args(["--host", "127.0.0.1"])
        .args(["--port", &port.to_string()])
        .args(["--log-level", "warn"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn sqlrustgo-mysql-server serve");

    // Wait for the listener to be ready by polling TCP connect.
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    loop {
        if std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(100)).is_ok() {
            break;
        }
        if std::time::Instant::now() >= deadline {
            panic!("canonical server did not become reachable on 127.0.0.1:{port} within 5s");
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    SubprocessHandle { child, port }
}

#[ignore = "L3 acceptance — implementation pending (audit 2026-06-04)"]
#[test]
fn l3_canonical_binary_serve_handshake_auth_and_query() {
    let server = spawn_canonical_server();
    // The production `serve` subcommand ships the server's
    // built-in users only: `root` (no password) and
    // `mysql/mysql`. We authenticate as `root` with an empty
    // password here.
    let client = MySqlTestClient::connect_at(("127.0.0.1", server.port), "root", "")
        .expect("MySqlTestClient should connect to the canonical subprocess server");

    // Run a small DDL + DML + DQL round trip.
    let mut client = client;
    client
        .exec("CREATE TABLE l3 (id INT PRIMARY KEY, v TEXT)")
        .expect("CREATE TABLE");
    client
        .exec("INSERT INTO l3 VALUES (1, 'l3-ok')")
        .expect("INSERT");
    let count = client
        .query_one_i64("SELECT COUNT(*) FROM l3")
        .expect("SELECT COUNT");
    assert_eq!(count, 1, "one row should be visible");
    let rows = client
        .query_rows("SELECT id, v FROM l3 ORDER BY id")
        .expect("SELECT");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], "1");
    assert_eq!(rows[0][1], "l3-ok");
}
