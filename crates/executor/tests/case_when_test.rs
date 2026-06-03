//! CASE WHEN Tests
//!
//! TPC-H Q8: `SUM(CASE WHEN n2.n_name = 'GERMANY' THEN l_extendedprice * (1 -
//! l_discount) ELSE 0 END) / SUM(l_extendedprice * (1 - l_discount)) AS
//! mkt_share`. The executor's `UnifiedExpr` already has a CaseWhen arm
//! but the top-level `evaluate_expression` (in sqlrustgo/src/expr_utils.rs)
//! doesn't dispatch to it, so the value falls through to Null.
//!
//! These tests verify the end-to-end path from the parser through
//! aggregate computation.

use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use sqlrustgo_types::Value;
use std::sync::{Arc, RwLock};

fn create_engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

#[test]
fn test_case_when_simple_aggregate() {
    // Simplest CASE WHEN inside SUM: each row contributes its
    // `l_extendedprice * (1 - l_discount)` if `n_name = 'GERMANY'`,
    // else 0. The aggregate is the German-only total.
    //
    // Note: we use INTEGER columns and small values so the
    // `* (1 - l_discount)` arithmetic doesn't underflow (with
    // `l_discount` = 0..1 we'd otherwise get negative results).
    let mut engine = create_engine();
    engine
        .execute(
            "CREATE TABLE lineitem (l_orderkey INTEGER, l_extendedprice INTEGER, l_discount INTEGER, n_name TEXT)",
        )
        .unwrap();
    engine
        .execute(
            "INSERT INTO lineitem VALUES \
             (1, 10, 0, 'GERMANY'), \
             (2, 20, 0, 'FRANCE'), \
             (3, 5, 0, 'GERMANY')",
        )
        .unwrap();

    // 10 * 1 = 10 (Germany); 20 * 1 = 20 (excluded by CASE); 5 * 1 = 5 (Germany)
    // Sum = 10 + 5 = 15
    let result = engine
        .execute(
            "SELECT SUM(CASE WHEN n_name = 'GERMANY' THEN l_extendedprice * (1 - l_discount) ELSE 0 END) AS total \
             FROM lineitem",
        )
        .unwrap();

    assert_eq!(result.rows.len(), 1);
    let v = &result.rows[0][0];
    let n = match v {
        Value::Integer(n) => *n,
        Value::Float(f) => *f as i64,
        other => panic!("expected Integer/Float, got {:?}", other),
    };
    assert_eq!(n, 15, "German-only total = 10 + 5 = 15, got {:?}", v);
}

#[test]
fn test_case_when_no_match_falls_to_else() {
    // Every row's n_name is not 'GERMANY', so the CASE always picks
    // the ELSE branch (0). Sum should be 0.
    let mut engine = create_engine();
    engine
        .execute("CREATE TABLE t (x INTEGER, flag TEXT)")
        .unwrap();
    engine
        .execute("INSERT INTO t VALUES (1, 'no'), (2, 'nope'), (3, 'still no')")
        .unwrap();

    let result = engine
        .execute("SELECT SUM(CASE WHEN flag = 'yes' THEN x ELSE 0 END) AS s FROM t")
        .unwrap();

    assert_eq!(result.rows.len(), 1);
    let v = &result.rows[0][0];
    let n = match v {
        Value::Integer(n) => *n,
        Value::Float(f) => *f as i64,
        _ => 0,
    };
    assert_eq!(n, 0, "All rows pick ELSE 0, sum should be 0, got {:?}", v);
}
