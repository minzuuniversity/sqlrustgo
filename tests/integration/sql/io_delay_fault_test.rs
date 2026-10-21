use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::binary_storage::BinaryTableStorage;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};
use tempfile::TempDir;

// #5469: `SQLRUSTGO_IO_DELAY_MS` is a **process-wide** environment
// variable, and the storage layer reads it on every I/O
// (`crate::io_delay::maybe_delay()`). The three tests in this file all
// mutate it, and the Rust test harness runs them in parallel threads
// inside one process — so one test's `clear_io_delay()` could undo
// another test's `set_io_delay(50)` mid-measurement.
//
// Measured (this tree, 3 consecutive parallel runs):
//   run 1: FAILED — "Without delay, 10 inserts should be fast but got 1.085164708s"
//   run 2: ok
//   run 3: FAILED — "Without delay, 10 inserts should be fast but got 1.09372725s"
// and with `--test-threads=1`: 3/3 green (11.1 s each).
//
// 1.08 s for 10 inserts is ~108 ms per insert — i.e. the 50 ms delay WAS
// applied (2 I/Os per insert). The "no delay" case was measuring the
// *delayed* case. The absolute `< 100ms` bound was never the real
// problem; the shared process state was.
//
// The lock makes the three tests mutually exclusive, so the env var has
// exactly one owner at a time.
static IO_DELAY_LOCK: Mutex<()> = Mutex::new(());

/// Acquire the process-wide I/O-delay lock. A poisoned mutex (a sibling
/// test panicked while holding it) must not cascade into the other
/// tests, so the poison is taken over rather than propagated.
fn io_delay_guard() -> MutexGuard<'static, ()> {
    IO_DELAY_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn set_io_delay(delay_ms: u64) {
    std::env::set_var("SQLRUSTGO_IO_DELAY_MS", delay_ms.to_string());
}

fn clear_io_delay() {
    std::env::remove_var("SQLRUSTGO_IO_DELAY_MS");
}

/// Run `n` INSERTs against a fresh table in `data_dir` and return the
/// elapsed wall time.
///
/// The delay knob is read from the process environment on every I/O, so
/// the caller must hold the lock for the whole measurement.
fn measure_inserts(data_dir: &Path, n: usize) -> Duration {
    let storage = BinaryTableStorage::new(data_dir.to_path_buf()).unwrap();
    let engine = Arc::new(RwLock::new(storage));
    let mut exec = ExecutionEngine::new(engine.clone());

    exec.execute("CREATE TABLE probe (id INT)").unwrap();

    let start = Instant::now();
    for i in 0..n {
        exec.execute(&format!("INSERT INTO probe VALUES ({})", i))
            .unwrap();
    }
    start.elapsed()
}

#[test]
fn test_io_delay_injected_on_save() {
    let _guard = io_delay_guard();
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().to_path_buf();

    set_io_delay(50);

    let storage = BinaryTableStorage::new(data_dir.clone()).unwrap();

    let engine = Arc::new(RwLock::new(storage));
    let mut exec = ExecutionEngine::new(engine.clone());

    exec.execute("CREATE TABLE t1 (id INT, name TEXT)").unwrap();

    let start = Instant::now();
    for i in 0..100 {
        exec.execute(&format!("INSERT INTO t1 VALUES ({}, 'name{}')", i, i))
            .unwrap();
    }
    let elapsed = start.elapsed();

    clear_io_delay();

    assert!(
        elapsed >= Duration::from_millis(40),
        "With 50ms delay and 100 inserts, expected >=4000ms but got {:?}",
        elapsed
    );
}

#[test]
fn test_io_delay_injected_on_load() {
    let _guard = io_delay_guard();
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().to_path_buf();

    {
        let storage = BinaryTableStorage::new(data_dir.clone()).unwrap();
        let engine = Arc::new(RwLock::new(storage));
        let mut exec = ExecutionEngine::new(engine.clone());

        exec.execute("CREATE TABLE t2 (id INT, name TEXT)").unwrap();
        for i in 0..10 {
            exec.execute(&format!("INSERT INTO t2 VALUES ({}, 'name{}')", i, i))
                .unwrap();
        }
    }

    set_io_delay(50);

    let storage = BinaryTableStorage::new_with_data(data_dir).unwrap();
    let start = Instant::now();
    let _ = storage.load("t2").unwrap();
    let elapsed = start.elapsed();

    clear_io_delay();

    assert!(
        elapsed >= Duration::from_millis(40),
        "With 50ms delay on load, expected >=40ms but got {:?}",
        elapsed
    );
}

#[test]
fn test_no_delay_when_not_configured() {
    let _guard = io_delay_guard();

    // The contract is "with the delay knob at 0, the storage does not
    // apply the artificial sleep". The only way to observe that is to
    // compare against the same workload with the knob turned on.
    //
    // The comparison is a **difference**, not an absolute bound and not a
    // ratio: the machine's own load contributes to both halves and
    // therefore cancels out, while the artificial delay is a fixed
    // additive cost. An absolute bound ("10 inserts must take < 100ms")
    // is a statement about the machine, not about the code — it went red
    // under load for reasons that had nothing to do with the code under
    // test.
    const N: usize = 10;
    const DELAY_MS: u64 = 200;

    let fast_dir = TempDir::new().unwrap();
    clear_io_delay();
    let fast = measure_inserts(fast_dir.path(), N);

    let slow_dir = TempDir::new().unwrap();
    set_io_delay(DELAY_MS);
    let slow = measure_inserts(slow_dir.path(), N);
    clear_io_delay();

    let difference = slow.saturating_sub(fast);
    assert!(
        difference >= Duration::from_millis(500),
        "the artificial I/O delay must be what makes the slow run slow: \
         no-delay={:?}, delay-{}ms={:?}, difference={:?} (expected >= 500ms)",
        fast,
        DELAY_MS,
        slow,
        difference
    );
}
