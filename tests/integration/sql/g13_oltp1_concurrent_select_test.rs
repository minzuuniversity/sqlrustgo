//! G13-OLTP-1 read-lock-for-SELECT regression test.
//!
//! Verifies that two concurrent SELECTs on the same ephemeral MySQL server
//! do NOT serialize against each other.
//!
//! Background: PR #3642 (commit 82e1c55814) added poisoning recovery to
//! the engine write lock but did NOT change the lock granularity. Every
//! query (including SELECT) acquired `engine.write()`. With multiple
//! concurrent clients running heavy SELECTs (TPC-H Q7/Q8/Q9/Q21 multi-
//! table joins), every client serialized behind every other client.
//!
//! On Z6G4 this manifested as 4+ ESTABLISHED connections never returning,
//! 5+ CLOSE_WAIT on the server side, and a "complete server deadlock"
//! after 2-3 minutes. macmini (M4) was unaffected because the workload
//! stayed under 1.7 cores and contention rarely surfaced.
//!
//! Fix (this test's pre-condition): dispatch SELECT / SHOW / DESCRIBE to
//! `engine.read()` and DDL / DML to `engine.write()`. Two concurrent
//! SELECTs can now run in parallel.
//!
//! Test plan (the metric is throughput, not latency, because the bug
//! starves heavy queries while a fast probe sees low latency — both
//! threads take the same write lock, but the heavy one is starved by
//! the fast one's frequent short acquisitions):
//!
//! 1. Start an ephemeral server with `server_threads=4` and a fresh data
//!    dir; populate the catalog `content` table with 200 rows.
//! 2. Run the heavy 3-way CROSS JOIN in a loop on thread A for 2 seconds
//!    ALONE (no contention). Count queries completed.
//! 3. Run the heavy 3-way CROSS JOIN in a loop on thread A for 2 seconds
//!    WHILE thread B hammers the server with `SELECT 1`.
//! 4. Assert: with the bug (everything takes the write lock), A's
//!    throughput in step 3 is much lower than in step 2. With the
//!    fix (SELECTs take the read lock), A's throughput is unchanged.

#[path = "../../common/mod.rs"]
mod common;
use common::MySqlTestClient;
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Heavy 3-way self-join. On 200 rows, this is ~8M row comparisons,
/// taking tens of ms each. The bug-starvation effect is observable:
/// when the lock is contended, this query is starved.
const HEAVY_SELECT: &str = "SELECT COUNT(*) FROM content c1, content c2 \
                            WHERE c1.hash < c2.hash";

const ROW_COUNT: i64 = 500;
const ALONE_DURATION: Duration = Duration::from_secs(2);
const CONTENDED_DURATION: Duration = Duration::from_secs(2);

#[test]
fn g13_oltp1_concurrent_selects_run_in_parallel() {
    eprintln!("[test] start");

    // 1) Start ephemeral server with server_threads=4. Fresh data dir.
    let tmp = tempfile::TempDir::new().expect("TempDir");
    let config = EphemeralConfig {
        host: "127.0.0.1".to_string(),
        bootstrap_tables: true,
        bootstrap_users: true,
        data_dir: Some(tmp.path().to_path_buf()),
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 16,
        storage: None,
        metrics_port: None,

        ..Default::default()
    };
    let handle = start_ephemeral(config).expect("start_ephemeral");
    let port = handle.port;
    let host: &'static str = "127.0.0.1";
    eprintln!("[test] ephemeral server started, port={port}");
    std::thread::sleep(Duration::from_millis(200));

    // 2) Populate `content` with 200 rows.
    {
        let mut admin =
            MySqlTestClient::connect_at((host, port), "tester", "tester").expect("admin connect");
        let _ = admin.set_timeouts(Duration::from_secs(60), Duration::from_secs(60));
        admin.query_rows("SELECT 1").expect("admin baseline");
        for i in 0..ROW_COUNT {
            let sql = format!(
                "INSERT INTO content (hash, doc, created_at) VALUES ('k{i:04}', 'd', '2026-06-29')"
            );
            let _ = admin.exec(&sql);
        }
        let count = admin
            .query_one_i64("SELECT COUNT(*) FROM content")
            .expect("count content");
        assert_eq!(count, ROW_COUNT);
        eprintln!("[test] populated content with {count} rows");
        let _ = admin.quit();
    }

    // 3) Run the heavy query ALONE for ALONE_DURATION. Record throughput.
    let alone_q = run_heavy_for(host, port, ALONE_DURATION, None);
    eprintln!(
        "ALONE: thread A completed {alone_q} heavy queries in {}s",
        ALONE_DURATION.as_secs()
    );
    assert!(
        alone_q > 1,
        "baseline (no contention): thread A only ran {alone_q} queries — the \
         heavy query is too slow for the test to be meaningful. Lower \
         ROW_COUNT or the join depth."
    );

    // 4) Run the heavy query WHILE thread B hammers with `SELECT 1`.
    //    The bug-present code makes both A and B contend for the same
    //    write lock. With server_threads=4, A still gets to run between
    //    B's probes, but B's frequent lock acquisitions (every ~1ms)
    //    starve A. With the fix, B's SELECT 1 takes the read lock and
    //    A's heavy query runs concurrently.
    let contended_q = run_heavy_for(host, port, CONTENDED_DURATION, Some(HEAVY_SELECT));
    eprintln!(
        "CONTESTED: thread A completed {contended_q} heavy queries in {}s \
         (with thread B hammering SELECT 1)",
        CONTENDED_DURATION.as_secs()
    );

    // 5) G13-OLTP-1 regression assertion. With the bug, the
    //    contended throughput is dramatically lower than the alone
    //    throughput (typically 50% lower; the SELECT 1 probe steals
    //    ~half the lock-acquisition opportunities). With the fix, the
    //    two runs are within 30% of each other (small noise from
    //    connection setup, but no serialization).
    //
    //    Threshold: contended >= alone * 0.7.
    let ratio = contended_q as f64 / alone_q as f64;
    eprintln!("THROUGHPUT RATIO: contended/alone = {ratio:.2}");
    assert!(
        ratio >= 0.7,
        "G13-OLTP-1 regression: thread A's heavy-query throughput drops \
         to {ratio:.2}× of the uncontended baseline ({contended_q} vs {alone_q}). \
         The fix should let SELECTs use a read lock so the heavy query \
         and the probe query run in parallel. Without the fix, every \
         query (including the probe) takes the write lock and starves \
         the heavy query."
    );

    drop(handle);
    drop(tmp);
    eprintln!("[test] done");
}

/// Run the heavy query in a tight loop on a worker thread for `duration`.
/// If `also_run` is Some, also run that query on a second worker thread
/// for the same duration (to simulate a competing client).
/// Returns the number of heavy queries completed.
fn run_heavy_for(host: &'static str, port: u16, duration: Duration, also_run: Option<&str>) -> u64 {
    let stop = Arc::new(AtomicBool::new(false));
    let count = Arc::new(AtomicU64::new(0));
    let stop_clone = Arc::clone(&stop);
    let count_clone = Arc::clone(&count);

    // Thread A: heavy query loop.
    let heavy_thread = std::thread::spawn(move || {
        let mut client =
            MySqlTestClient::connect_at((host, port), "tester", "tester").expect("heavy connect");
        let _ = client.set_timeouts(Duration::from_secs(30), Duration::from_secs(30));
        while !stop_clone.load(Ordering::Relaxed) {
            if client.query_rows(HEAVY_SELECT).is_ok() {
                count_clone.fetch_add(1, Ordering::Relaxed);
            }
        }
        let _ = client.quit();
    });

    // Thread B (optional): competing query loop.
    let other_thread = also_run.map(|q_sql| {
        let stop_b = Arc::clone(&stop);
        let q_sql = q_sql.to_string();
        std::thread::spawn(move || {
            let mut client = MySqlTestClient::connect_at((host, port), "tester", "tester")
                .expect("competing connect");
            let _ = client.set_timeouts(Duration::from_secs(5), Duration::from_secs(5));
            while !stop_b.load(Ordering::Relaxed) {
                let _ = client.query_rows(&q_sql);
            }
            let _ = client.quit();
        })
    });

    std::thread::sleep(duration);
    stop.store(true, Ordering::SeqCst);
    let _ = heavy_thread.join();
    if let Some(t) = other_thread {
        let _ = t.join();
    }
    count.load(Ordering::Relaxed)
}
