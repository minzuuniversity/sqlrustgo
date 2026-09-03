//! V312-69 / Issue #4653 regression integration test:
//! `INSERT ... RETURNING col_list` (PostgreSQL/MySQL 8.0+) must project
//! the just-inserted rows to the requested column list.

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
        Value::Null => String::new(),
        other => panic!("expected Text/Null at [{}][{}], got {:?}", row, col, other),
    }
}

#[test]
fn v312_69_insert_returning_returns_inserted_row() {
    let mut x = fresh();
    x.execute("CREATE TABLE t(id INT, val INT)").unwrap();
    let res = x
        .execute("INSERT INTO t VALUES (1, 100) RETURNING id, val")
        .unwrap();
    assert_eq!(res.rows.len(), 1);
    assert_eq!(extract_int(&res, 0, 0), 1);
    assert_eq!(extract_int(&res, 0, 1), 100);
}

#[test]
fn v312_69_insert_returning_with_no_clause_unaffected() {
    // Regression: INSERT without RETURNING must still return empty result.
    let mut x = fresh();
    x.execute("CREATE TABLE t(id INT, val INT)").unwrap();
    let res = x.execute("INSERT INTO t VALUES (1, 100)").unwrap();
    assert_eq!(res.rows.len(), 0);
}

#[test]
fn v312_69_insert_returning_star_projects_all_columns() {
    let mut x = fresh();
    x.execute("CREATE TABLE t(id INT, name VARCHAR(20))").unwrap();
    let res = x
        .execute("INSERT INTO t VALUES (1, 'alice') RETURNING *")
        .unwrap();
    assert_eq!(res.rows.len(), 1);
    assert_eq!(res.rows[0].len(), 2);
    assert_eq!(extract_int(&res, 0, 0), 1);
    assert_eq!(extract_text(&res, 0, 1), "alice");
}

#[test]
fn v312_69_insert_returning_with_multi_row_insert() {
    let mut x = fresh();
    x.execute("CREATE TABLE t(id INT, val INT)").unwrap();
    let res = x
        .execute("INSERT INTO t VALUES (1, 10), (2, 20), (3, 30) RETURNING id")
        .unwrap();
    assert_eq!(res.rows.len(), 3);
    assert_eq!(extract_int(&res, 0, 0), 1);
    assert_eq!(extract_int(&res, 1, 0), 2);
    assert_eq!(extract_int(&res, 2, 0), 3);
}

#[test]
fn v312_69_insert_returning_with_nonexistent_column_yields_null() {
    let mut x = fresh();
    x.execute("CREATE TABLE t(id INT, val INT)").unwrap();
    let res = x
        .execute("INSERT INTO t VALUES (1, 100) RETURNING nonexistent")
        .unwrap();
    assert_eq!(res.rows.len(), 1);
    assert!(matches!(res.rows[0][0], Value::Null));
}
