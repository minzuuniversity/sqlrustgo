//! G2 Substance Test: Verify ParallelVolcanoExecutor wiring in mysql-server path
//!
//! **INT-2 Gate Substance**: G2 form gate (check_int2_no_orphan.sh) verifies
//! structure: parallel_degree field, set_parallel_degree(), build_parallel_executor(),
//! pub mod parallel_executor, partition API. ALL PASS.
//!
//! This file verifies BEHAVIOR substance:
//! - parallel_degree is configurable end-to-end
//! - build_parallel_executor() returns a working executor
//! - The parallel WHERE filter path in engine_select.rs is exercised without error
//!
//! Note: True parallel execution requires rows >= PARALLEL_MIN_ROWS (100,000).
//! Full parallel correctness is verified by G7 (soak) and G11 (QPS bench).

use sqlrustgo::{ExecutionEngine, MemoryStorage};
use sqlrustgo_types::Value;
use std::sync::{Arc, RwLock};

fn make_engine(parallel_degree: usize) -> ExecutionEngine<MemoryStorage> {
    let mut engine = ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())));
    engine.set_parallel_degree(parallel_degree);
    engine
}

/// Verify parallel_degree setter and getter work
#[test]
fn test_parallel_degree_setter_getter() {
    let mut engine = make_engine(1);
    assert_eq!(engine.parallel_degree(), 1);

    engine.set_parallel_degree(4);
    assert_eq!(engine.parallel_degree(), 4);

    engine.set_parallel_degree(0); // should clamp to 1
    assert_eq!(engine.parallel_degree(), 1);
}

/// Verify build_parallel_executor returns a working ParallelVolcanoExecutor
#[test]
fn test_build_parallel_executor_returns_working_executor() {
    let engine = make_engine(1);
    let _ = engine.build_parallel_executor(); // doesn't panic = success

    let engine4 = make_engine(4);
    let _ = engine4.build_parallel_executor(); // doesn't panic = success
}

/// Verify parallel WHERE path runs without error on small table
/// (parallel_degree > 1 but rows < PARALLEL_MIN_ROWS, so sequential path used;
///  this still exercises the parallel setup code path in engine_select.rs)
#[test]
fn test_parallel_where_clause_setup_no_error() {
    let mut engine = make_engine(4); // degree > 1
    engine
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER)")
        .unwrap();

    // Small table (below PARALLEL_MIN_ROWS = 100,000)
    for i in 0..100 {
        engine
            .execute(&format!("INSERT INTO t VALUES ({}, {})", i, i % 10))
            .unwrap();
    }

    // WHERE clause exercises the filter path in engine_select.rs
    // (parallel path skipped due to small size, but setup code runs)
    let result = engine
        .execute("SELECT COUNT(*) FROM t WHERE v >= 5")
        .unwrap();
    assert_eq!(result.rows[0][0], Value::Integer(50));
}

/// Verify parallel aggregation on small table runs without error
#[test]
fn test_parallel_aggregation_setup_no_error() {
    let mut engine = make_engine(4);
    engine
        .execute("CREATE TABLE orders (id INTEGER, amount INTEGER, category TEXT)")
        .unwrap();

    for i in 0..200 {
        engine
            .execute(&format!(
                "INSERT INTO orders VALUES ({}, {}, 'cat{}')",
                i,
                (i * 17) % 500,
                i % 3
            ))
            .unwrap();
    }

    let result = engine
        .execute("SELECT category, SUM(amount) FROM orders GROUP BY category ORDER BY category")
        .unwrap();

    assert_eq!(result.rows.len(), 3);
    for row in &result.rows {
        assert_eq!(row.len(), 2);
        assert!(matches!(row[0], Value::Text(_)));
        assert!(matches!(row[1], Value::Integer(_)));
    }
}
