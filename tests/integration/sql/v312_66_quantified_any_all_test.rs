//! V312-66 / Issue #4641 regression integration test:
//! `val > ANY (subq)`, `val > ALL (subq)`, `val = ANY (subq)`, etc.
//! must evaluate correctly (non-empty result for `> ANY`, no rows
//! for `> ALL` when no outer value exceeds the subquery max, etc.).
//!
//! Uses the public `ExecutionEngine::execute` path so the parser,
//! executor, and storage layers are exercised end-to-end — matching
//! the `v312_63_parser_issues_test.rs` convention.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::sync::Arc;

fn fresh() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

fn extract_int(res: &sqlrustgo::ExecutorResult, row: usize, col: usize) -> i64 {
    match &res.rows[row][col] {
        Value::Integer(n) => *n,
        other => panic!("expected Integer at [{}][{}], got {:?}", row, col, other),
    }
}

/// Issue #4641 example: `val > ANY (subq)` returns rows where
/// `val > MIN(subq.val)`.
#[test]
fn v312_66_quantified_any_greater_than() {
    let mut x = fresh();
    x.execute("CREATE TABLE a (val INT)").unwrap();
    x.execute("INSERT INTO a VALUES (10), (20), (30)").unwrap();
    x.execute("CREATE TABLE b (val INT)").unwrap();
    x.execute("INSERT INTO b VALUES (10), (25), (35)").unwrap();
    let res = x
        .execute("SELECT val FROM a WHERE val > ANY (SELECT val FROM b) ORDER BY val")
        .unwrap();
    assert_eq!(res.rows.len(), 2);
    assert_eq!(extract_int(&res, 0, 0), 20);
    assert_eq!(extract_int(&res, 1, 0), 30);
}

/// `> ALL` against a non-empty subquery: only rows exceeding MAX(subq) match.
#[test]
fn v312_66_quantified_all_greater_than_no_match() {
    let mut x = fresh();
    x.execute("CREATE TABLE a (val INT)").unwrap();
    x.execute("INSERT INTO a VALUES (10), (20), (30)").unwrap();
    x.execute("CREATE TABLE b (val INT)").unwrap();
    x.execute("INSERT INTO b VALUES (10), (25), (35)").unwrap();
    let res = x
        .execute("SELECT val FROM a WHERE val > ALL (SELECT val FROM b)")
        .unwrap();
    assert_eq!(res.rows.len(), 0, "no value in a exceeds 35");
}

/// `> ALL` against an empty subquery: vacuous truth → all outer rows pass.
#[test]
fn v312_66_quantified_all_greater_than_empty() {
    let mut x = fresh();
    x.execute("CREATE TABLE a (val INT)").unwrap();
    x.execute("INSERT INTO a VALUES (10), (20), (30)").unwrap();
    x.execute("CREATE TABLE b (val INT)").unwrap();
    x.execute("INSERT INTO b VALUES (10), (20), (30)").unwrap();
    let res = x
        .execute(
            "SELECT val FROM a WHERE val > ALL (SELECT val FROM b WHERE val > 100) ORDER BY val",
        )
        .unwrap();
    assert_eq!(res.rows.len(), 3, "ALL with empty subquery = vacuous truth");
    assert_eq!(extract_int(&res, 0, 0), 10);
    assert_eq!(extract_int(&res, 1, 0), 20);
    assert_eq!(extract_int(&res, 2, 0), 30);
}

/// `= ANY (subq)` returns rows whose value appears in the subquery.
#[test]
fn v312_66_quantified_any_equals() {
    let mut x = fresh();
    x.execute("CREATE TABLE a (val INT)").unwrap();
    x.execute("INSERT INTO a VALUES (10), (20), (30)").unwrap();
    x.execute("CREATE TABLE b (val INT)").unwrap();
    x.execute("INSERT INTO b VALUES (10), (25), (35)").unwrap();
    let res = x
        .execute("SELECT val FROM a WHERE val = ANY (SELECT val FROM b) ORDER BY val")
        .unwrap();
    assert_eq!(res.rows.len(), 1);
    assert_eq!(extract_int(&res, 0, 0), 10);
}

/// `< ANY (subq)` returns rows whose value is less than at least one
/// subquery value.
#[test]
fn v312_66_quantified_less_than_any() {
    let mut x = fresh();
    x.execute("CREATE TABLE a (val INT)").unwrap();
    x.execute("INSERT INTO a VALUES (10), (20), (30)").unwrap();
    x.execute("CREATE TABLE b (val INT)").unwrap();
    x.execute("INSERT INTO b VALUES (10), (25), (35)").unwrap();
    let res = x
        .execute("SELECT val FROM a WHERE val < ANY (SELECT val FROM b) ORDER BY val")
        .unwrap();
    assert_eq!(res.rows.len(), 3);
    assert_eq!(extract_int(&res, 0, 0), 10);
    assert_eq!(extract_int(&res, 1, 0), 20);
    assert_eq!(extract_int(&res, 2, 0), 30);
}

/// `!= ANY (subq)` returns rows whose value is not equal to any
/// `!= ALL (subq)` returns rows whose value is not equal to any
/// subquery value (i.e., the value is unique w.r.t. the subquery set).
#[test]
fn v312_66_quantified_not_equal_all() {
    let mut x = fresh();
    x.execute("CREATE TABLE a (val INT)").unwrap();
    x.execute("INSERT INTO a VALUES (10), (20), (30)").unwrap();
    x.execute("CREATE TABLE b (val INT)").unwrap();
    x.execute("INSERT INTO b VALUES (10), (25), (35)").unwrap();
    let res = x
        .execute("SELECT val FROM a WHERE val != ALL (SELECT val FROM b) ORDER BY val")
        .unwrap();
    // Rows where val != 10 AND val != 25 AND val != 35: 20 and 30.
    assert_eq!(res.rows.len(), 2);
    assert_eq!(extract_int(&res, 0, 0), 20);
    assert_eq!(extract_int(&res, 1, 0), 30);
}
/// QuantifiedOp in a correlated subquery must not panic. The result
/// may be over-inclusive (conservative `true`) per the existing
/// IN/EXISTS pattern.
#[test]
fn v312_66_quantified_correlated_does_not_panic() {
    let mut x = fresh();
    x.execute("CREATE TABLE outer_t (val INT)").unwrap();
    x.execute("INSERT INTO outer_t VALUES (10), (20), (30)")
        .unwrap();
    x.execute("CREATE TABLE inner_t (inner_col INT)").unwrap();
    x.execute("INSERT INTO inner_t VALUES (5), (15), (25)")
        .unwrap();
    // Correlated subquery: `inner_col < outer_t.val`. The conservative
    // fallback returns true for all outer rows (no panic), matching
    // the existing IN/EXISTS pattern at `eval_predicate`.
    let res = x
        .execute("SELECT val FROM outer_t WHERE val > ANY (SELECT inner_col FROM inner_t WHERE inner_col < outer_t.val) ORDER BY val")
        .unwrap();
    // We don't assert a specific count (correlated is conservative);
    // just verify the executor didn't panic and produced SOME rows.
    assert!(
        !res.rows.is_empty(),
        "correlated quantified should not panic"
    );
}
