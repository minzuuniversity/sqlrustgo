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
//! # What this test measures, and how
//!
//! The property under test is **"two concurrent SELECTs do not serialize"**.
//! Two independent measurements are used, because neither one alone can
//! decide that property on an arbitrary host:
//!
//! **M1 — throughput ratio (host-capacity-normalized).**
//! Thread A runs the heavy 3-way CROSS JOIN in a loop against the server
//! under test. A **second** thread runs the *same* heavy query, either
//!
//! * against the **same** server (the *contended* arm — the two queries
//!   share the server's lock), or
//! * against a **second, independent server** carrying the same data (the
//!   *baseline* arm — identical CPU load, but **no shared lock**).
//!
//! Comparing the two arms cancels the host's spare-CPU capacity: the
//! baseline arm measures what this host can do with two heavy queries when
//! the database does **not** serialize them, and the contended arm measures
//! the same thing when it does. The ratio `contended / baseline` is
//! therefore ~1.0 when reads run in parallel and collapses towards 0.5 when
//! they serialize — **independently of how loaded the host is**.
//!
//! **M2 — probe latency (directly measures blocking).**
//! A `SELECT 1` probe is timed while the heavy query is running. If the
//! heavy query holds an **exclusive** lock, the probe has to wait for it
//! (hundreds of milliseconds); with a shared/read lock the probe returns
//! immediately. This measurement does not depend on the host's CPU
//! capacity at all — it observes *blocking*, which is the defect itself.
//!
//! # History of this file's measurement (all of it was a premise about the
//! # host, not about the code under test)
//!
//! * The window was a fixed 2 seconds and the test asserted "the heavy
//!   query must complete more than once inside it". On a loaded host one
//!   iteration can take longer than the whole window ⇒ the guard against a
//!   meaningless measurement *was* the failure, and the measurement never
//!   happened (#5549). Fixed by deriving the window from one **measured**
//!   iteration.
//! * The throughput ratio compared the contended arm against a **truly
//!   alone** arm. The competing query was itself a heavy 8M-row cross join,
//!   so on a host without a spare core the ratio dropped to 0.667 — a
//!   statement about the host, not about the database. Measured on a host
//!   with load 7–11 on 10 cores: `contended/alone = 0.667` on every run,
//!   while a pure-CPU competitor (no database at all) gave 0.83–1.00 and a
//!   cheap `SELECT 1` competitor gave 0.83–1.00. ⇒ the read path does not
//!   serialize; the old metric was simply measuring CPU sharing.
//! * With only ~6 samples per arm the ratio is quantised to multiples of
//!   1/6, so a system whose true ratio is exactly 0.7 could be reported as
//!   0.667 and fail. The window now guarantees at least
//!   `MIN_WINDOW_ITERATIONS` samples.

#[path = "../../common/mod.rs"]
mod common;
use common::MySqlTestClient;
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Heavy 3-way self-join. On 500 rows this is ~8M row comparisons,
/// taking hundreds of ms each. The bug-starvation effect is observable:
/// when the lock is contended, this query is starved.
const HEAVY_SELECT: &str = "SELECT COUNT(*) FROM content c1, content c2 \
                            WHERE c1.hash < c2.hash";

/// The **probe** query — deliberately cheap.
///
/// The scenario this test is about is "a **fast** probe with frequent short
/// lock acquisitions starves a heavy query" (see the header). The probe must
/// therefore be cheap: it has to reach the lock path constantly without
/// competing for CPU, otherwise the measurement becomes a statement about
/// the host's spare cores instead of about the database's locking.
const PROBE_SELECT: &str = "SELECT 1";

const ROW_COUNT: i64 = 500;

// #5549 — the fixed 2-second windows that used to live here
// (`ALONE_DURATION` / `CONTENDED_DURATION`) are replaced by a window
// derived from a **measured** iteration; see `MIN_WINDOW` /
// `MAX_WINDOW` / `WINDOW_ITERATIONS` below.

/// #5549 — the measurement window must scale with the machine.
///
/// The original premise was "the heavy query must complete **more than
/// once** inside a fixed 2-second window". That premise is a statement
/// about the *host*, not about the code under test: the heavy query is a
/// 3-way self-join over 500 rows (~8M row comparisons). On an idle
/// development machine it takes tens of milliseconds and the window holds
/// dozens of iterations; on a loaded CI container (or a slow laptop) a
/// single iteration can take more than a second, the window then holds
/// **one** iteration, and the test fails with
///
///     baseline (no contention): thread A only ran 1 queries — the heavy
///     query is too slow for the test to be meaningful
///
/// i.e. the failure is reported by the very assertion that is supposed to
/// *guard* the measurement's meaningfulness — the measurement was never
/// made.
///
/// The fix keeps the contract and removes the host dependence: the window
/// is derived from one **measured** iteration, so it always spans several
/// iterations no matter how slow the host is. Both arms use the same
/// window, so the ratio stays meaningful.
const MIN_WINDOW: Duration = Duration::from_secs(2);
const MAX_WINDOW: Duration = Duration::from_secs(60);
const WINDOW_ITERATIONS: u32 = 3;

/// The window must also give the **ratio** enough resolution.
///
/// With `N` samples the smallest ratio the measurement can *report* is
/// `floor(threshold · N) / N`. At `N = 6` that is `floor(0.7 · 6) / 6 =
/// 0.667` — **below** the 0.7 threshold. In other words, at 6 samples a
/// system whose true ratio is exactly 0.7 can be reported as 0.667 and fail
/// the assertion: the verdict would then depend on which side of a single
/// iteration the run happened to land on, i.e. on noise.
///
/// `MIN_WINDOW_ITERATIONS = 10` makes the threshold exactly reachable
/// (`floor(0.7 · 10) / 10 = 0.7`), so the comparison is decided by the
/// measured throughput rather than by the sample count.
const MIN_WINDOW_ITERATIONS: u32 = 10;

/// #5549 — the throughput criterion itself is unchanged: the contended arm
/// must reach at least 70% of the baseline arm.
const THROUGHPUT_MIN_RATIO: f64 = 0.7;

/// M2 — a probe that has to wait for the heavy query must be visibly
/// slower.
///
/// The probe's own latency on an idle server is well under a millisecond;
/// the heavy query holds the lock for hundreds of milliseconds. The bound
/// is deliberately generous (10× the probe's own baseline plus 100 ms) so
/// that it only fires when the probe is genuinely **blocked** — not when
/// the host is merely busy.
const PROBE_LATENCY_FACTOR: u32 = 10;
const PROBE_LATENCY_SLACK: Duration = Duration::from_millis(100);

/// How many probes M2 times. Ten is enough: M2 looks for the **worst**
/// latency, and a single blocked probe (hundreds of ms, vs. a sub-ms idle
/// baseline) already decides the verdict. Ten also bounds the phase's
/// runtime at ~10 × the heavy query's duration in the pathological case.
const PROBE_SAMPLES: usize = 10;

#[test]
fn g13_oltp1_concurrent_selects_run_in_parallel() {
    eprintln!("[test] start");

    // 1) Two ephemeral servers:
    //    * `handle_a` — the server under test;
    //    * `handle_b` — the CPU-load control. It carries the SAME data but
    //      is a different server, so the heavy query it runs shares no lock
    //      with the one under test. That is what makes the two throughput
    //      arms comparable on any host.
    let tmp_a = tempfile::TempDir::new().expect("TempDir A");
    let tmp_b = tempfile::TempDir::new().expect("TempDir B");
    let handle_a = start_ephemeral(server_config(tmp_a.path())).expect("server A");
    let handle_b = start_ephemeral(server_config(tmp_b.path())).expect("server B");
    let port_a = handle_a.port;
    let port_b = handle_b.port;
    let host: &'static str = "127.0.0.1";
    eprintln!("[test] ephemeral servers started: A={port_a} (under test), B={port_b} (control)");
    std::thread::sleep(Duration::from_millis(200));

    // 2) Populate `content` on BOTH servers with the same rows.
    for (tag, port) in [("A", port_a), ("B", port_b)] {
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
        eprintln!("[test] server {tag} populated with {count} rows");
        let _ = admin.quit();
    }

    // 3) Measure ONE iteration of the heavy query, then derive the
    //    measurement window from it (#5549). The window must span enough
    //    iterations on any host, otherwise the "throughput" comparison
    //    compares a single iteration against a single iteration.
    let one_iteration = {
        let mut probe =
            MySqlTestClient::connect_at((host, port_a), "tester", "tester").expect("probe connect");
        let _ = probe.set_timeouts(Duration::from_secs(120), Duration::from_secs(120));
        let t0 = Instant::now();
        probe
            .query_rows(HEAVY_SELECT)
            .expect("probe heavy query succeeds");
        t0.elapsed()
    };
    eprintln!("[test] one heavy query takes {one_iteration:?}");
    let window = std::cmp::min(
        std::cmp::max(
            MIN_WINDOW,
            one_iteration * std::cmp::max(WINDOW_ITERATIONS, MIN_WINDOW_ITERATIONS),
        ),
        MAX_WINDOW,
    );
    eprintln!("[test] measurement window = {window:?}");

    // 4) Arm ALONE: the heavy query with no competitor at all. This is the
    //    "is the measurement meaningful" reference (the original #5549
    //    assertion) — it must complete more than one iteration.
    let alone_q = run_heavy_for(host, port_a, None, window);
    eprintln!("ALONE: thread A completed {alone_q} heavy queries in {window:?}");
    assert!(
        alone_q > 1,
        "baseline (no contention): thread A only ran {alone_q} queries in {window:?} — \
         the heavy query is too slow for the test to be meaningful. Lower \
         ROW_COUNT or the join depth."
    );

    // 5) Arm BASELINE: the same two heavy queries, but the competitor runs
    //    against the CONTROL server. Identical CPU load, **no shared lock**.
    //    This is the host's capacity with two heavy queries — the premise
    //    the contended arm is compared against.
    let baseline_q = run_heavy_for(host, port_a, Some(port_b), window);
    eprintln!(
        "BASELINE: thread A completed {baseline_q} heavy queries in {window:?} \
         (competitor on the control server {port_b})"
    );
    assert!(
        baseline_q > 1,
        "control arm: thread A only ran {baseline_q} queries in {window:?} — \
         the heavy query is too slow for the test to be meaningful."
    );

    // 6) Arm CONTENDED: the competitor runs the same heavy query against
    //    the SAME server, so the two queries share the server's lock.
    let contended_q = run_heavy_for(host, port_a, Some(port_a), window);
    eprintln!(
        "CONTESTED: thread A completed {contended_q} heavy queries in {window:?} \
         (competitor on the server under test {port_a})"
    );

    // 7) G13-OLTP-1 regression assertion (M1). With the bug (everything
    //    takes the write lock) the contended arm collapses towards half
    //    the baseline: the two heavy queries serialize. With the fix
    //    (SELECTs take the read lock) both arms measure the same host
    //    capacity and the ratio is ~1.0.
    //
    //    Threshold: contended >= baseline * 0.7 (unchanged — #5549).
    let ratio = contended_q as f64 / baseline_q as f64;
    eprintln!("THROUGHPUT RATIO: contended/baseline = {ratio:.2}");
    assert!(
        ratio >= THROUGHPUT_MIN_RATIO,
        "G13-OLTP-1 regression: thread A's heavy-query throughput drops \
         to {ratio:.2}× of the no-shared-lock baseline ({contended_q} vs \
         {baseline_q}). The fix should let SELECTs use a read lock so the \
         heavy query and the competing query run in parallel. Without the \
         fix, every query (including the competitor) takes the write lock \
         and starves the heavy query."
    );

    // 8) M2 — the probe must not be BLOCKED by the concurrent heavy query.
    //
    //    This is the measurement that does not depend on the host's CPU
    //    capacity: it observes blocking, which is the defect itself.
    let probe_base = measure_probe_latency(host, port_a, false, window);
    let probe_contended = measure_probe_latency(host, port_a, true, window);
    eprintln!("PROBE LATENCY: idle={probe_base:?} under-contention={probe_contended:?}");
    let probe_bound = probe_base * PROBE_LATENCY_FACTOR + PROBE_LATENCY_SLACK;
    assert!(
        probe_contended <= probe_bound,
        "G13-OLTP-1 regression: a concurrent SELECT is BLOCKED by the heavy \
         SELECT — the probe took {probe_contended:?} while the heavy query \
         ran (idle baseline {probe_base:?}, bound {probe_bound:?}). The fix \
         should let SELECTs use a read lock so the two run in parallel."
    );

    drop(handle_a);
    drop(handle_b);
    drop(tmp_a);
    drop(tmp_b);
    eprintln!("[test] done");
}

fn server_config(data_dir: &std::path::Path) -> EphemeralConfig {
    EphemeralConfig {
        host: "127.0.0.1".to_string(),
        bootstrap_tables: true,
        bootstrap_users: true,
        data_dir: Some(data_dir.to_path_buf()),
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        bulk_insert_rows_per_flush: 10_000,
        server_threads: 16,
        storage: None,
        metrics_port: None,
        ..Default::default()
    }
}

/// Run the heavy query in a tight loop on a worker thread for `duration`.
///
/// `competitor_port`:
/// * `None` — no competitor (the ALONE arm);
/// * `Some(p)` — a second worker thread runs the same heavy query against
///   the server at `p`. Pointing `p` at the *same* port gives the CONTENDED
///   arm (shared lock); pointing it at the control server gives the
///   BASELINE arm (same CPU load, no shared lock).
///
/// Returns the number of heavy queries the FIRST worker completed.
fn run_heavy_for(
    host: &'static str,
    port: u16,
    competitor_port: Option<u16>,
    duration: Duration,
) -> u64 {
    let stop = Arc::new(AtomicBool::new(false));
    let count = Arc::new(AtomicU64::new(0));
    let stop_clone = Arc::clone(&stop);
    let count_clone = Arc::clone(&count);

    // Thread A: heavy query loop.
    let heavy_thread = std::thread::spawn(move || {
        let mut client =
            MySqlTestClient::connect_at((host, port), "tester", "tester").expect("heavy connect");
        let _ = client.set_timeouts(Duration::from_secs(120), Duration::from_secs(120));
        while !stop_clone.load(Ordering::Relaxed) {
            if client.query_rows(HEAVY_SELECT).is_ok() {
                count_clone.fetch_add(1, Ordering::Relaxed);
            }
        }
        let _ = client.quit();
    });

    // Thread B (optional): competing heavy query loop.
    let other_thread = competitor_port.map(|cp| {
        let stop_b = Arc::clone(&stop);
        std::thread::spawn(move || {
            let mut client = MySqlTestClient::connect_at((host, cp), "tester", "tester")
                .expect("competing connect");
            let _ = client.set_timeouts(Duration::from_secs(120), Duration::from_secs(120));
            while !stop_b.load(Ordering::Relaxed) {
                let _ = client.query_rows(HEAVY_SELECT);
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

/// M2 — the slowest `SELECT 1` probe latency observed.
///
/// With `concurrent_heavy = true` a background thread runs the heavy query
/// against the same server, and the probe is only sampled **after that
/// background query has completed at least once** — otherwise the probe
/// would finish its samples before the heavy query even started, and the
/// measurement would overlap nothing (which is exactly how an earlier
/// version of this function silently measured nothing: 30 probes at ~0.3ms
/// each finished in 9ms, long before the first heavy query completed).
fn measure_probe_latency(
    host: &'static str,
    port: u16,
    concurrent_heavy: bool,
    max_wait: Duration,
) -> Duration {
    let stop = Arc::new(AtomicBool::new(false));
    let background_done = Arc::new(AtomicU64::new(0));
    let background = if concurrent_heavy {
        let stop_b = Arc::clone(&stop);
        let done = Arc::clone(&background_done);
        Some(std::thread::spawn(move || {
            let mut client = MySqlTestClient::connect_at((host, port), "tester", "tester")
                .expect("background connect");
            let _ = client.set_timeouts(Duration::from_secs(120), Duration::from_secs(120));
            while !stop_b.load(Ordering::Relaxed) {
                if client.query_rows(HEAVY_SELECT).is_ok() {
                    done.fetch_add(1, Ordering::Relaxed);
                }
            }
            let _ = client.quit();
        }))
    } else {
        None
    };

    // Wait until the background heavy query has actually run once, so the
    // probes below overlap a live heavy query.
    if concurrent_heavy {
        let give_up = Instant::now() + max_wait;
        while background_done.load(Ordering::Relaxed) == 0 && Instant::now() < give_up {
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    let mut client =
        MySqlTestClient::connect_at((host, port), "tester", "tester").expect("probe connect");
    let _ = client.set_timeouts(Duration::from_secs(120), Duration::from_secs(120));

    let mut worst = Duration::ZERO;
    let mut samples = 0usize;
    while samples < PROBE_SAMPLES {
        let t0 = Instant::now();
        let r = client.query_rows(PROBE_SELECT);
        let dt = t0.elapsed();
        if dt > worst {
            worst = dt;
        }
        if let Err(e) = &r {
            eprintln!("[test] probe error: {e}");
        }
        samples += 1;
    }
    let _ = client.quit();

    stop.store(true, Ordering::SeqCst);
    if let Some(t) = background {
        let _ = t.join();
    }
    worst
}
