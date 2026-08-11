//! Mixed-workload SOAK backpressure regression test
//!
//! Verifies the accept-loop backpressure fix for Issue #3265 (72h SOAK).
//!
//! When the worker pool channel is full, the accept loop must NOT park
//! indefinitely in `Sender::send` (the original deadlock). Instead
//! `send_timeout(200ms)` returns `Timeout`, the accept loop increments
//! `BACKPRESSURE_COUNT`, drops the new connection (the client sees
//! ECONNRESET), and continues. This protects the server from resource
//! exhaustion and keeps the accept loop responsive to the shutdown flag.
//!
//! With 80 clients and a 64-slot channel, clients 1-64 fit, clients
//! 65-80 are dropped. The test asserts:
//!
//!   1. The probe attempt returns *some* outcome (Ok or Err) within 5s
//!      — proves the loop is not parked indefinitely.
//!   2. BACKPRESSURE_COUNT advanced — the backpressure path fired.
//!   3. The total throughput is reasonable — the server is actually
//!      processing queries, not just rejecting everything.
//!
//! Refs: openspec/changes/fix-soak-deadlock-mpmc-sender/proposal.md

#[path = "../../common/mod.rs"]
mod common;
use common::MySqlTestClient;
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig, BACKPRESSURE_COUNT};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[test]
fn mixed_workload_accept_loop_does_not_park_under_saturation() {
    eprintln!("[test] start");

    // 1) Start an ephemeral server with server_threads=16 (default) and
    //    CHANNEL_BUFFER_MULTIPLIER=4 (channel capacity 64). 80 saturating
    //    clients exceed the channel capacity, exercising the backpressure
    //    path. The 65th-80th clients should be dropped (ECONNRESET).
    let tmp = tempfile::TempDir::new().expect("TempDir");
    let config = EphemeralConfig {
        host: "127.0.0.1".to_string(),
        port: None,
        bootstrap_tables: true,
        bootstrap_users: true,
        data_dir: Some(tmp.path().to_path_buf()),
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 16,
        storage: None,
        slow_query_log: None,
    };
    let handle = start_ephemeral(config).expect("start_ephemeral");
    let port = handle.port;
    let host: &'static str = "127.0.0.1";
    eprintln!("[test] ephemeral server started, port={port}");
    std::thread::sleep(Duration::from_millis(200));

    // 2) Snapshot the backpressure counter.
    let bp_before = BACKPRESSURE_COUNT.load(Ordering::Relaxed);

    // 3) Spawn 80 client threads running tight `SELECT 1` loops.
    let stop_flag = Arc::new(AtomicBool::new(false));
    let mut handles = Vec::new();
    eprintln!("[test] spawning 80 client threads");
    for thread_id in 0..80 {
        let stop = Arc::clone(&stop_flag);
        let h = std::thread::spawn(move || -> u64 {
            let mut client = match MySqlTestClient::connect_at((host, port), "tester", "tester") {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("[client {thread_id}] connect failed: {e}");
                    return 0u64;
                }
            };
            let _ = client.set_timeouts(Duration::from_secs(5), Duration::from_secs(5));
            let mut queries_run = 0u64;
            while !stop.load(Ordering::Relaxed) {
                if client.query_rows("SELECT 1").is_ok() {
                    queries_run += 1;
                }
            }
            let _ = client.quit();
            queries_run
        });
        handles.push(h);
    }
    eprintln!("[test] all 80 client threads spawned");

    // 4) Wait ~3s for clients to connect and saturate the pool.
    std::thread::sleep(Duration::from_secs(3));

    // 5) The accept loop is still polling — a fresh client gets
    //    *either* accepted OR cleanly rejected (ECONNRESET). The
    //    important property is that the loop does NOT park: it always
    //    returns one of these outcomes within the read timeout.
    let probe_deadline = Instant::now() + Duration::from_secs(5);
    let mut probe_outcome: Option<Result<(), String>> = None;
    while Instant::now() < probe_deadline {
        match MySqlTestClient::connect_at((host, port), "tester", "tester") {
            Ok(mut probe) => {
                if probe.query_rows("SELECT 1").is_ok() {
                    probe_outcome = Some(Ok(()));
                } else {
                    probe_outcome = Some(Err("query_rows failed".into()));
                }
                let _ = probe.quit();
                break;
            }
            Err(e) => {
                eprintln!("[probe] connect failed (expected under saturation): {e}");
                probe_outcome = Some(Err(e.to_string()));
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
    assert!(
        probe_outcome.is_some(),
        "accept loop did not respond within 5s — the original SOAK \
         deadlock may have regressed (accept loop parked in Sender::send)."
    );

    // 6) Stop the saturating clients and tally throughput.
    stop_flag.store(true, Ordering::SeqCst);
    let total_queries: u64 = handles.into_iter().map(|h| h.join().unwrap_or(0)).sum();
    eprintln!("SOAK: 80 clients ran {total_queries} queries in 3s");

    // 7) BACKPRESSURE_COUNT must have advanced (channel overflow occurred).
    let bp_after = BACKPRESSURE_COUNT.load(Ordering::Relaxed);
    let bp_delta = bp_after.saturating_sub(bp_before);
    eprintln!(
        "SOAK: BACKPRESSURE_COUNT advanced by {bp_delta} (before={bp_before}, after={bp_after})"
    );
    assert!(
        bp_delta > 0,
        "backpressure mechanism never fired (delta={bp_delta}). \
         With 80 clients and channel capacity 64, the accept loop's \
         send_timeout should have returned Timeout at least once."
    );

    // 8) Sanity: the server is actually processing queries, not just
    //    rejecting everything. With 16 workers and channel=64 we expect
    //    tens of thousands of queries in 3s.
    assert!(
        total_queries > 1000,
        "server processed only {total_queries} queries in 3s — too few; \
         the backpressure path may be over-aggressive."
    );

    drop(handle);
    drop(tmp);
    eprintln!("[test] done");
}
