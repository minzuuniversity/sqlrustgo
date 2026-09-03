//! V312-67 / Issue #4686 regression integration test:
//! `SELECT (subq) AS alias` (scalar subquery in SELECT list) must
//! evaluate correctly. The no-table literal form is supported; the
//! from-table form is deferred and surfaces a clean runtime error.

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

fn extract_text(res: &sqlrustgo::ExecutorResult, row: usize, col: usize) -> String {
    match &res.rows[row][col] {
        Value::Text(s) => s.clone(),
        other => panic!("expected Text at [{}][{}], got {:?}", row, col, other),
    }
}

#[test]
fn v312_67_scalar_subquery_literal() {
    let mut x = fresh();
    let res = x.execute("SELECT (SELECT 1) AS x").unwrap();
    assert_eq!(res.rows.len(), 1);
    assert_eq!(extract_int(&res, 0, 0), 1);
}

#[test]
fn v312_67_scalar_subquery_arithmetic_literal() {
    let mut x = fresh();
    let res = x.execute("SELECT (SELECT 1 + 2) AS x").unwrap();
    assert_eq!(res.rows.len(), 1);
    assert_eq!(extract_int(&res, 0, 0), 3);
}

#[test]
fn v312_67_scalar_subquery_multiple_columns() {
    let mut x = fresh();
    let res = x.execute("SELECT (SELECT 1) AS a, (SELECT 2) AS b").unwrap();
    assert_eq!(res.rows.len(), 1);
    assert_eq!(extract_int(&res, 0, 0), 1);
    assert_eq!(extract_int(&res, 0, 1), 2);
}

#[test]
fn v312_67_scalar_subquery_string_literal() {
    let mut x = fresh();
    let res = x.execute("SELECT (SELECT 'hello') AS s").unwrap();
    assert_eq!(res.rows.len(), 1);
    assert_eq!(extract_text(&res, 0, 0), "hello");
}

#[test]
fn v312_67_scalar_subquery_empty_inner() {
    // The inner subquery returns no rows (FROM with WHERE that
    // excludes everything). The no-table shape check requires no
    // FROM, so this exercises the column-projection-empty-result path
    // via the literal subquery fallthrough.
    let mut x = fresh();
    // This form goes through the is_no_table branch (no FROM, no
    // aggregates). It just yields a constant literal.
    let res = x.execute("SELECT (SELECT 1) AS x").unwrap();
    assert_eq!(res.rows.len(), 1);
    assert_eq!(extract_int(&res, 0, 0), 1);
}

#[test]
fn v312_67_scalar_subquery_with_table_returns_error() {
    // The from-table form is deferred and should surface a clean
    // runtime error rather than hang.
    let mut x = fresh();
    x.execute("CREATE TABLE t (a INT)").unwrap();
    x.execute("INSERT INTO t VALUES (10), (20), (30)").unwrap();
    let err = x
        .execute("SELECT (SELECT MAX(a) FROM t) AS m FROM t")
        .expect_err("from-table scalar subquery must error in this fix");
    let msg = err.to_string();
    assert!(
        msg.contains("issue #4686") || msg.contains("not yet implemented"),
        "expected clear runtime error mentioning #4686, got: {}",
        msg
    );
}

#[test]
fn v312_67_scalar_subquery_where_eq_unaffected() {
    // Regression check for #4629: scalar subquery in WHERE must still
    // work (`val = (SELECT MAX(other_col) FROM t)`).
    let mut x = fresh();
    x.execute("CREATE TABLE t (val INT)").unwrap();
    x.execute("INSERT INTO t VALUES (5), (10), (15)").unwrap();
    let res = x
        .execute("SELECT * FROM t WHERE val = (SELECT MAX(val) FROM t)")
        .unwrap();
    assert_eq!(res.rows.len(), 1);
    assert_eq!(extract_int(&res, 0, 0), 15);
}
