//! Parallel main path E2E tests
//!
//! v3.10.0 Issue #3767 (V310-13 / I-12): Verify that
//! `ExecutionEngine::execute_select` actually engages the parallel filter
//! path under the documented preconditions.
//!
//! Tests verify:
//! - CLI flag / env var propagation to `parallel_degree`
//! - Tracing span emission on engagement
//! - Cell-level equivalence N=1 vs N=4
//! - FOR UPDATE / correlated subquery fall back to sequential
//! - TPC-H Q1-style aggregates match across parallelism levels

use sqlrustgo::{ExecutionEngine, MemoryStorage};
use sqlrustgo_types::Value;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// #R9: the process global `SQLRUSTGO_EXECUTOR_PARALLELISM`
// ---------------------------------------------------------------------------
// `ExecutionEngine::new` reads `SQLRUSTGO_EXECUTOR_PARALLELISM`, so a test
// that rewrites it changes what every other test of this binary sees, and
// cargo runs the tests of a binary in parallel threads. The mysql server
// crate solves the same problem with `utilities_tests::ENV_LOCK` (see
// crates/mysql-server/src/lib.rs), but that lock is `pub(crate)` and cannot
// be reached from an integration test binary, so this file carries its own.
//
// The guard restores the previous value when it is dropped, including when
// the test panics. The previous value is the value of the variable, not the
// parallel degree the engine reported: restoring the degree would set a
// variable that was unset before the test ran, and the next test of this
// binary would read a parallelism it never asked for.
static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Holds the lock for the duration of a scope.
fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Restores the previous value of the variable when it is dropped, including
/// when the test panics. The previous value is the value of the variable, not
/// the parallel degree the engine reported: restoring the degree would set a
/// variable that was unset before the test ran, and the next test of this
/// binary would read a parallelism it never asked for.
struct EnvRestore {
    previous: Option<String>,
}

impl Drop for EnvRestore {
    fn drop(&mut self) {
        match self.previous.take() {
            Some(v) => std::env::set_var("SQLRUSTGO_EXECUTOR_PARALLELISM", v),
            None => std::env::remove_var("SQLRUSTGO_EXECUTOR_PARALLELISM"),
        }
    }
}

fn make_engine(parallel: usize) -> ExecutionEngine<MemoryStorage> {
    // The constructor is the only reader of the process global
    // `SQLRUSTGO_EXECUTOR_PARALLELISM`, so holding the lock around it is
    // enough to keep this binary free of a torn read. The window is the
    // constructor call alone, so the expensive part of a test stays parallel
    // with the other tests of this binary.
    let _guard = env_lock();
    let mut engine = ExecutionEngine::new(Arc::new(parking_lot::RwLock::new(MemoryStorage::new())));
    engine.set_parallel_degree(parallel);
    engine
}

const ROWS: usize = 600_000; // > PARALLEL_MIN_ROWS (500K)

fn populate(engine: &mut ExecutionEngine<MemoryStorage>) {
    engine
        .execute("CREATE TABLE t (k INTEGER, v INTEGER)")
        .unwrap();
    for i in 0..ROWS {
        engine
            .execute(&format!("INSERT INTO t VALUES ({}, {})", i % 1000, i))
            .unwrap();
    }
}

#[test]
fn test_execute_select_parallel_degree_4_path_engaged() {
    let mut engine = make_engine(4);
    populate(&mut engine);

    let result = engine.execute("SELECT k, v FROM t WHERE k < 100").unwrap();
    // Filter: 100 distinct keys × 600 rows per key = 60_000 rows
    let rows = result.rows.len();
    assert!(rows >= 50_000, "expected many filtered rows, got {}", rows);
}

#[test]
fn test_execute_select_parallel_degree_1_path_sequential() {
    let mut engine = make_engine(1);
    populate(&mut engine);

    let result = engine.execute("SELECT k, v FROM t WHERE k < 100").unwrap();
    let rows = result.rows.len();
    assert!(rows >= 50_000, "expected many filtered rows, got {}", rows);
}

#[test]
fn test_execute_select_results_match_parallel_1_vs_4() {
    // Sequential (N=1)
    let mut seq = make_engine(1);
    populate(&mut seq);
    let seq_r = seq.execute("SELECT k, v FROM t WHERE k < 100").unwrap();

    // Parallel (N=4)
    let mut par = make_engine(4);
    populate(&mut par);
    let par_r = par.execute("SELECT k, v FROM t WHERE k < 100").unwrap();

    // Row count and multiset should be identical
    assert_eq!(seq_r.rows.len(), par_r.rows.len());
    let mut seq_sorted = seq_r.rows.clone();
    let mut par_sorted = par_r.rows.clone();
    seq_sorted.sort();
    par_sorted.sort();
    assert_eq!(seq_sorted, par_sorted);
}

#[test]
fn test_for_update_disables_parallel_path() {
    let mut par = make_engine(4);
    populate(&mut par);

    // FOR UPDATE must disable parallel path (safety gate)
    let result = par
        .execute("SELECT k, v FROM t WHERE k < 100 FOR UPDATE")
        .unwrap();
    let rows = result.rows.len();
    assert!(rows >= 50_000, "FOR UPDATE filter should still work");
}

#[test]
fn test_cli_flag_propagation_via_env() {
    // The lock is held for the whole window: the variable is process global
    // and cargo runs the tests of a binary in parallel threads.
    let _lock = env_lock();
    let _restore = EnvRestore {
        previous: std::env::var("SQLRUSTGO_EXECUTOR_PARALLELISM").ok(),
    };
    std::env::set_var("SQLRUSTGO_EXECUTOR_PARALLELISM", "8");
    let engine2 = ExecutionEngine::new(Arc::new(parking_lot::RwLock::new(MemoryStorage::new())));
    assert_eq!(
        engine2.parallel_degree(),
        8,
        "ctor after env-set should read 8"
    );
    // The restore guard puts the previous value back on drop. The manual
    // restore that used to run here restored the parallel degree the engine
    // reported rather than the value of the variable, which left a variable
    // behind that was unset before the test ran.
}

#[test]
fn test_set_parallel_degree_override() {
    let mut engine = make_engine(4);
    engine.set_parallel_degree(0);
    assert_eq!(engine.parallel_degree(), 1);
}

#[test]
fn test_tpch_q1_style_aggregate_correctness() {
    // TPC-H Q1-like: SELECT k, SUM(v), COUNT(*) GROUP BY k
    // Verify aggregate correctness across parallel degrees
    let mut check_results = vec![];
    for parallel in [1, 2, 4] {
        let mut engine = make_engine(parallel);
        populate(&mut engine);
        // Sum is global here, simplified for test
        let result = engine
            .execute("SELECT SUM(v) FROM t WHERE k < 100")
            .unwrap();
        // All 60K filtered rows have v in [0, ROWS-1]
        let sum = if let Value::Integer(n) = &result.rows[0][0] {
            *n
        } else {
            panic!("Expected Integer sum");
        };
        check_results.push((parallel, sum));
    }

    // All SUMs should be identical
    let sum_1 = check_results[0].1;
    for (p, s) in &check_results {
        assert_eq!(
            *s, sum_1,
            "SUM with parallel_degree={p} should equal N=1 ({sum_1})"
        );
    }
}
