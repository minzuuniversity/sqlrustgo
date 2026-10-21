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

// =========================================================================
// The judgement itself (#5476)
// =========================================================================
//
// #5476 pointed out that the two delay assertions used a floor of
// `elapsed >= 40ms` while their own failure message announced
// "expected >=4000ms" — a 100x gap between the stated expectation and the
// enforced bound. The real problem is deeper than the number: the bound was
// on the **total** wall time.
//
// A bound on total wall time is a statement about the *machine*, not about
// the code under test. On a slow or loaded machine, the un-delayed workload
// alone can exceed it — the assertion then passes **without the delay ever
// having been applied**. That is a false green, and it is the failure mode
// #5476 is really about.
//
// The bound below is on the **net effect** of the injection: the same
// workload is measured twice, once without the delay and once with it, and
// only the *difference* is judged. The machine's own contribution appears in
// both halves and cancels out; the artificial delay is a fixed additive
// cost, so what remains is evidence that the delay was actually applied.
//
// The floor is one injected delay (not one per INSERT): the delay knob is
// applied per I/O, and the number of I/Os an INSERT performs is an
// implementation detail that may legitimately change (e.g. if writes are
// batched into a single fsync). Binding the judgement to that count would
// make a legitimate implementation change look like a regression; a
// dedicated test covers the batching path instead.

/// The minimum net effect that proves the injected delay was applied.
///
/// One injected delay: the contract asserted here is "the delay reached at
/// least one I/O", which is exactly what "the delay is effective on this
/// path" means.
pub fn expected_delay_effect(delay_ms: u64) -> Duration {
    Duration::from_millis(delay_ms)
}

/// Whether the measured net effect proves the delay was applied.
///
/// `net_effect` is `slow - fast` (saturating). A negative net effect means
/// the "slow" run was *faster* — the delay was certainly not applied, and
/// the judgement must say so rather than pass on a saturated zero.
pub fn delay_effect_holds(net_effect: Duration, delay_ms: u64) -> bool {
    net_effect >= expected_delay_effect(delay_ms)
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

/// Measure the wall time of one `load` of table `t2` from `data_dir`.
///
/// The delay knob is read on every I/O, so the caller must hold the lock.
fn measure_load(data_dir: &Path) -> Duration {
    let storage = BinaryTableStorage::new_with_data(data_dir.to_path_buf()).unwrap();
    let start = Instant::now();
    let _ = storage.load("t2").unwrap();
    start.elapsed()
}

#[test]
fn test_io_delay_injected_on_save() {
    let _guard = io_delay_guard();

    // #5476: the bound is now on the **net effect** of the injection, not on
    // the total wall time. The same workload is measured twice and only the
    // difference is judged, so the machine's own speed cancels out and a
    // slow machine can no longer make the assertion pass by itself.
    //
    // The floor is one injected delay: "the delay reached at least one I/O
    // on the save path". How many I/Os an INSERT performs is an
    // implementation detail (measured here at 2 per INSERT, i.e. ~10s for
    // 100 INSERTs) and must not be baked into the bound.
    const N: usize = 100;
    const DELAY_MS: u64 = 50;

    let fast_dir = TempDir::new().unwrap();
    clear_io_delay();
    let fast = measure_inserts(fast_dir.path(), N);

    let slow_dir = TempDir::new().unwrap();
    set_io_delay(DELAY_MS);
    let slow = measure_inserts(slow_dir.path(), N);
    clear_io_delay();

    let net_effect = slow.saturating_sub(fast);
    assert!(
        delay_effect_holds(net_effect, DELAY_MS),
        "the injected I/O delay must show up as a net cost on the save path: \
         no-delay={:?}, delay-{}ms={:?}, net effect={:?} (required >= {:?})",
        fast,
        DELAY_MS,
        slow,
        net_effect,
        expected_delay_effect(DELAY_MS)
    );
}

#[test]
fn test_io_delay_injected_on_load() {
    let _guard = io_delay_guard();
    const DELAY_MS: u64 = 50;

    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().to_path_buf();

    // Build the table first, with no delay in effect.
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

    // #5476: same difference-based judgement as the save path.
    clear_io_delay();
    let fast = measure_load(&data_dir);

    set_io_delay(DELAY_MS);
    let slow = measure_load(&data_dir);
    clear_io_delay();

    let net_effect = slow.saturating_sub(fast);
    assert!(
        delay_effect_holds(net_effect, DELAY_MS),
        "the injected I/O delay must show up as a net cost on the load path: \
         no-delay={:?}, delay-{}ms={:?}, net effect={:?} (required >= {:?})",
        fast,
        DELAY_MS,
        slow,
        net_effect,
        expected_delay_effect(DELAY_MS)
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

// =========================================================================
// Discriminating-power self-proof (#5476)
// =========================================================================
//
// A criterion that never fails and a criterion with real discriminating
// power look identical in CI. These tests pin down **what the judgement
// rejects**, so a future edit that loosens it back into a hollow check
// shows up as a failing test rather than as a silently useless guard.

#[cfg(test)]
mod judgement_tests {
    use super::*;

    /// The delay was not applied at all: the net effect is zero.
    #[test]
    fn a_run_without_the_delay_is_rejected() {
        assert!(
            !delay_effect_holds(Duration::ZERO, 50),
            "a zero net effect must be rejected — the delay was not applied"
        );
    }

    /// The "slow" run was actually faster: the net effect is negative.
    /// A saturating subtraction turns this into zero, which must still be
    /// rejected rather than accepted as "at least the floor".
    #[test]
    fn a_negative_net_effect_is_rejected() {
        let negative = Duration::from_millis(50).saturating_sub(Duration::from_millis(120));
        assert_eq!(negative, Duration::ZERO);
        assert!(
            !delay_effect_holds(negative, 50),
            "a negative net effect must be rejected"
        );
    }

    /// Exactly one injected delay is enough: the criterion is not
    /// over-tightened either.
    #[test]
    fn exactly_one_injected_delay_is_accepted() {
        assert!(
            delay_effect_holds(Duration::from_millis(50), 50),
            "one injected delay must satisfy the criterion"
        );
    }

    /// **The old criterion's false green, spelled out.**
    ///
    /// The old assertion was `elapsed >= 40ms` on the *total* wall time. On a
    /// slow or loaded machine the un-delayed workload alone takes longer
    /// than 40ms, so the old assertion passed **even though the injected
    /// delay was never applied** — the exact "the test is green but it
    /// proved nothing" failure #5476 is about.
    ///
    /// The new judgement looks at the net effect, so the same situation is
    /// rejected.
    #[test]
    fn the_old_total_time_bound_would_have_gone_green_without_the_delay() {
        // A slow machine: 120ms of plain work, no delay applied.
        let slow_machine_total = Duration::from_millis(120);

        // The old criterion: total wall time >= 40ms -> PASS.
        assert!(
            slow_machine_total >= Duration::from_millis(40),
            "precondition: the old total-time bound is satisfied here"
        );

        // The new criterion: net effect (0ms, nothing was injected) -> FAIL.
        let net_effect = Duration::ZERO;
        assert!(
            !delay_effect_holds(net_effect, 50),
            "the new judgement must reject the same situation that the old \
             total-time bound accepted"
        );
    }
}
