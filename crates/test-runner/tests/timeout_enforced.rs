//! V312-24 Phase 1: timeout enforcement integration test.
//!
//! Verifies that `TestRunner::run_managed` actually kills a runaway
//! child process via `tokio::time::timeout` + `kill_on_drop`, and
//! reports it as `TestStatus::TimedOut` within the declared budget.
//!
//! This guards against regressions where the timeout field is silently
//! ignored (the v3.10.0 beta-report bug — the runner would block
//! forever on a hung child).

use std::sync::Arc;
use std::time::{Duration, Instant};

use test_registry::ManagedTest;
use test_runner::{TestRunConfig, TestRunner, TestStatus};

/// Spawn the configured command and return elapsed wall-clock time + status.
async fn run_one(timeout_ms: u64) -> (Duration, test_runner::TestStatus) {
    let config = TestRunConfig {
        timeout_per_test_ms: timeout_ms,
        ..TestRunConfig::default()
    };
    let runner = Arc::new(TestRunner::new(config));

    // Use a ManagedTest that runs `sleep 30` — guaranteed to exceed
    // any sane timeout. The shell command is portable on Linux/macOS.
    let entry = ManagedTest::new("sleep-probe", "sleep")
        .with_args(vec!["30".to_string()])
        .with_timeout_ms(timeout_ms);

    let start = Instant::now();
    let result = runner.run_managed(&entry).await;
    let elapsed = start.elapsed();
    (elapsed, result.status)
}

#[tokio::test(flavor = "current_thread")]
async fn managed_timeout_kills_hung_child() {
    // Budget 500ms; sleep 30s. We expect:
    //   - Status == TimedOut
    //   - elapsed < 5s (well under the 30s sleep) — proves the child was killed.
    let (elapsed, status) = run_one(500).await;
    assert_eq!(
        status,
        TestStatus::TimedOut,
        "expected TimedOut, got {:?}",
        status
    );
    assert!(
        elapsed < Duration::from_secs(5),
        "elapsed={:?} suggests the child was NOT killed by timeout",
        elapsed
    );
    // Lower bound: at least the budget, plus some scheduling slack.
    assert!(
        elapsed >= Duration::from_millis(400),
        "elapsed={:?} shorter than the budget — timeout fired too early",
        elapsed
    );
}

#[tokio::test(flavor = "current_thread")]
async fn managed_timeout_passes_fast_command() {
    // A 5s budget against `true` (instant exit) should pass.
    let config = TestRunConfig {
        timeout_per_test_ms: 5_000,
        ..TestRunConfig::default()
    };
    let runner = Arc::new(TestRunner::new(config));
    let entry = ManagedTest::new("true-probe", "true").with_timeout_ms(5_000);
    let result = runner.run_managed(&entry).await;
    assert_eq!(
        result.status,
        TestStatus::Passed,
        "expected Passed, got {:?}",
        result.status
    );
    assert!(
        result.duration_ms < 2_000,
        "true should exit near-instantly, took {} ms",
        result.duration_ms
    );
}

#[tokio::test(flavor = "current_thread")]
async fn managed_missing_binary_crashes_not_hangs() {
    // A non-existent binary must return Crashed quickly, not block on
    // a timeout. This exercises the `Ok(Err(e))` arm of the join.
    let config = TestRunConfig {
        timeout_per_test_ms: 5_000,
        ..TestRunConfig::default()
    };
    let runner = Arc::new(TestRunner::new(config));
    let entry = ManagedTest::new("missing", "/nonexistent/path/__definitely_not_a_binary__")
        .with_timeout_ms(5_000);
    let start = Instant::now();
    let result = runner.run_managed(&entry).await;
    let elapsed = start.elapsed();
    assert_eq!(
        result.status,
        TestStatus::Crashed,
        "expected Crashed, got {:?}",
        result.status
    );
    assert!(
        elapsed < Duration::from_secs(3),
        "missing-binary detection took {:?} — should be near-instant",
        elapsed
    );
}
