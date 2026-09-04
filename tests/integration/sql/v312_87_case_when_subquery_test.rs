//! V312-87 / Issue #4760: regression integration test —
//! scalar subquery inside a `CASE WHEN ... > ... THEN ... ELSE ... END`
//! predicate must evaluate against the FROM-table values, not silently
//! collapse to `Value::Null`.
//!
//! Before the fix:
//!   `evaluate_expression_with_subq`'s CaseWhen arm closed over
//!   `evaluate_expression(e, row, table_info)` (which has a hardcoded
//!   `subq_eval = |_| Ok(Value::Null)`), so any Subquery inside a
//!   WHEN condition evaluated to Null → the BinaryOp became Null → no
//!   WHEN matched → ELSE branch always won.
//!
//! After the fix the CaseWhen closure threads the caller's real
//! `subq_eval` through, and the projection-time closure dispatches
//! the from-table scalar subquery to
//! `execute_subquery_for_scalar_from_table` so it sees the storage
//! state (V312-75 #4636 path).

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::sync::Arc;

fn fresh() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

fn extract_text(res: &sqlrustgo::ExecutorResult, row: usize, col: usize) -> String {
    match &res.rows[row][col] {
        Value::Text(s) => s.clone(),
        other => panic!("expected Text at [{}][{}], got {:?}", row, col, other),
    }
}

/// Exact repro from issue #4760: AVG(val) = 20 > 15 → every row should
/// classify as `'high'`, not `'low'`.
#[test]
fn v312_87_case_when_from_table_subquery_high_branch() {
    let mut x = fresh();
    x.execute("CREATE TABLE t(id INTEGER, val INTEGER)")
        .unwrap();
    x.execute("INSERT INTO t VALUES (1,10),(2,20),(3,30)")
        .unwrap();
    let r = x
        .execute(
            "SELECT id, CASE WHEN (SELECT AVG(val) FROM t) > 15 \
             THEN 'high' ELSE 'low' END AS cls \
             FROM t",
        )
        .unwrap();
    assert_eq!(r.rows.len(), 3);
    for i in 0..3 {
        assert_eq!(
            extract_text(&r, i, 1),
            "high",
            "row {} should classify as 'high' (AVG=20 > 15)",
            i
        );
    }
}

/// Same shape, opposite branch: `> 25` should fail and route to ELSE.
#[test]
fn v312_87_case_when_from_table_subquery_low_branch() {
    let mut x = fresh();
    x.execute("CREATE TABLE t(id INTEGER, val INTEGER)")
        .unwrap();
    x.execute("INSERT INTO t VALUES (1,10),(2,20),(3,30)")
        .unwrap();
    let r = x
        .execute(
            "SELECT id, CASE WHEN (SELECT AVG(val) FROM t) > 25 \
             THEN 'high' ELSE 'low' END AS cls \
             FROM t",
        )
        .unwrap();
    assert_eq!(r.rows.len(), 3);
    for i in 0..3 {
        assert_eq!(
            extract_text(&r, i, 1),
            "low",
            "row {} should classify as 'low' (AVG=20 ≤ 25)",
            i
        );
    }
}

/// No-table scalar subquery inside CASE WHEN — should also work
/// after the fix (no-table form has been supported since V312-67
/// #4686, but the CaseWhen closure used the Null subq_eval).
#[test]
fn v312_87_case_when_no_table_subquery() {
    let mut x = fresh();
    let r = x
        .execute(
            "SELECT CASE WHEN (SELECT 1 + 2) > 2 \
             THEN 'gt' ELSE 'le' END",
        )
        .unwrap();
    assert_eq!(r.rows.len(), 1);
    assert_eq!(extract_text(&r, 0, 0), "gt");
}

/// The bare subquery in SELECT-list still works (this is the existing
/// #4686 path, regression guard for the projection-time subq_eval
/// refactor).
#[test]
fn v312_87_select_list_subquery_still_works() {
    let mut x = fresh();
    x.execute("CREATE TABLE t(id INTEGER, val INTEGER)")
        .unwrap();
    x.execute("INSERT INTO t VALUES (1,10),(2,20),(3,30)")
        .unwrap();
    let r = x
        .execute("SELECT id, (SELECT AVG(val) FROM t) AS a FROM t")
        .unwrap();
    assert_eq!(r.rows.len(), 3);
    match &r.rows[0][1] {
        Value::Float(f) => assert!((*f - 20.0).abs() < 1e-9),
        other => panic!("expected Float(20.0), got {:?}", other),
    }
}
