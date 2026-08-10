//! V312-24 Phase 1: managed-test parallel dispatch integration test.
//!
//! Verifies that `TestRunner::run_managed_all` actually fans out
//! `ManagedTest` entries across concurrent tokio tasks, bounded by
//! `config.max_parallel`, and that each entry is dispatched with its
//! declared `timeout_ms`.

use std::sync::Arc;
use std::time::{Duration, Instant};

use test_registry::ManagedTest;
use test_runner::{TestRunConfig, TestRunner, TestStatus};

#[tokio::test(flavor = "current_thread")]
async fn managed_all_returns_one_result_per_entry() {
    let config = TestRunConfig {
        max_parallel: 4,
        timeout_per_test_ms: 5_000,
        ..TestRunConfig::default()
    };
    let runner = Arc::new(TestRunner::new(config));

    let entries: Vec<ManagedTest> = (0..5)
        .map(|i| {
            let name: String = format!("true-{}", i);
            ManagedTest::new(&name, "true").with_timeout_ms(5_000)
        })
        .collect();

    let results = runner.clone().run_managed_all(entries.clone()).await;
    assert_eq!(results.len(), entries.len(), "one result per input entry");
    for (i, r) in results.iter().enumerate() {
        assert_eq!(r.test_id, format!("true-{}", i));
        assert_eq!(
            r.status,
            TestStatus::Passed,
            "entry {} got {:?}",
            i,
            r.status
        );
    }
}

#[tokio::test(flavor = "current_thread")]
async fn managed_all_respects_max_parallel() {
    // 4 entries, each sleeping 2s. With max_parallel=2, the whole
    // batch should take ~4s (2 batches × 2s), not 8s (serial).
    // We allow generous slack to absorb CI scheduler noise.
    let config = TestRunConfig {
        max_parallel: 2,
        timeout_per_test_ms: 10_000,
        ..TestRunConfig::default()
    };
    let runner = Arc::new(TestRunner::new(config));

    let entries: Vec<ManagedTest> = (0..4)
        .map(|i| {
            let name: String = format!("sleep-{}", i);
            ManagedTest::new(&name, "sleep")
                .with_args(vec!["2".to_string()])
                .with_timeout_ms(10_000)
        })
        .collect();

    let start = Instant::now();
    let results = runner.clone().run_managed_all(entries).await;
    let elapsed = start.elapsed();

    assert_eq!(results.len(), 4);
    for r in &results {
        assert_eq!(r.status, TestStatus::Passed, "got {:?}", r.status);
    }
    // Lower bound: 2 batches × 2s = 4s. Upper bound (catch a serialization
    // regression): 8s - 1s slack = 7s. Serial would be ~8s.
    assert!(
        elapsed < Duration::from_secs(7),
        "elapsed={:?} — should be ~4s with max_parallel=2, suggests serial execution",
        elapsed
    );
    assert!(
        elapsed >= Duration::from_secs(3),
        "elapsed={:?} too fast — suggests max_parallel was not enforced (saturated)",
        elapsed
    );
}
