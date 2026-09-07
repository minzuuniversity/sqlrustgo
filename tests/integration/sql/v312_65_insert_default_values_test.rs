//! V312-65 / Issue #4643 regression integration test:
//! `INSERT INTO t DEFAULT VALUES` must insert one row populated from
//! each column's declared DEFAULT (or NULL when no default is defined).
//!
//! Companion to the parser unit tests in `crates/parser/src/parser.rs`
//! (test_parse_insert_default_values_basic / _lowercase /
//! _without_values_errors / _values_still_works_after_default_values_added).

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
        Value::Null => i64::MIN, // sentinel for NULL — callers compare via is_null_or()
        other => panic!(
            "expected Integer/Null at [{}][{}], got {:?}",
            row, col, other
        ),
    }
}

fn extract_text(res: &sqlrustgo::ExecutorResult, row: usize, col: usize) -> String {
    match &res.rows[row][col] {
        Value::Text(s) => s.clone(),
        Value::Null => String::new(),
        other => panic!("expected Text/Null at [{}][{}], got {:?}", row, col, other),
    }
}

fn is_null(res: &sqlrustgo::ExecutorResult, row: usize, col: usize) -> bool {
    matches!(&res.rows[row][col], Value::Null)
}

/// Issue #4643: parser accepts DEFAULT VALUES; executor inserts one row
/// with each column populated from its declared DEFAULT (or NULL).
#[test]
fn v312_65_insert_default_values_with_defaults() {
    let mut x = fresh();
    x.execute(
        "CREATE TABLE t (\
         id INTEGER, \
         val INTEGER DEFAULT 100, \
         name VARCHAR(20) DEFAULT 'hello'\
         )",
    )
    .unwrap();
    x.execute("INSERT INTO t DEFAULT VALUES").unwrap();
    let res = x.execute("SELECT id, val, name FROM t").unwrap();
    assert_eq!(
        res.rows.len(),
        1,
        "DEFAULT VALUES must insert exactly one row"
    );
    assert!(is_null(&res, 0, 0), "id column has no DEFAULT → NULL");
    assert_eq!(
        extract_int(&res, 0, 1),
        100,
        "val column has DEFAULT 100 → 100"
    );
    assert_eq!(
        extract_text(&res, 0, 2),
        "hello",
        "name column has DEFAULT 'hello' → 'hello'"
    );
}

/// DEFAULT VALUES on a table without any DEFAULT clauses inserts all-NULL row.
#[test]
fn v312_65_insert_default_values_without_defaults() {
    let mut x = fresh();
    x.execute("CREATE TABLE t (a INTEGER, b TEXT)").unwrap();
    x.execute("INSERT INTO t DEFAULT VALUES").unwrap();
    let res = x.execute("SELECT a, b FROM t").unwrap();
    assert_eq!(res.rows.len(), 1);
    assert!(is_null(&res, 0, 0));
    assert!(is_null(&res, 0, 1));
}

/// DEFAULT VALUES on a non-existent table must surface a binder/storage
/// error (not silently succeed and not parse-error).
#[test]
fn v312_65_insert_default_values_nonexistent_table_errors() {
    let mut x = fresh();
    let err = x
        .execute("INSERT INTO nonexistent DEFAULT VALUES")
        .expect_err("DEFAULT VALUES on missing table must error");
    let msg = err.to_string();
    assert!(
        !msg.to_lowercase().contains("parse error"),
        "DEFAULT VALUES must not produce a parse error, got: {}",
        msg
    );
}

/// DEFAULT VALUES is case-insensitive.
#[test]
fn v312_65_insert_default_values_lowercase_keyword() {
    let mut x = fresh();
    x.execute("CREATE TABLE t (a INTEGER DEFAULT 42)").unwrap();
    x.execute("insert into t default values").unwrap();
    let res = x.execute("SELECT a FROM t").unwrap();
    assert_eq!(res.rows.len(), 1);
    assert_eq!(extract_int(&res, 0, 0), 42);
}

/// Multiple DEFAULT VALUES inserts accumulate independent rows.
#[test]
fn v312_65_insert_default_values_multiple_rows() {
    let mut x = fresh();
    x.execute("CREATE TABLE t (a INTEGER, b INTEGER DEFAULT 7)")
        .unwrap();
    x.execute("INSERT INTO t DEFAULT VALUES").unwrap();
    x.execute("INSERT INTO t DEFAULT VALUES").unwrap();
    x.execute("INSERT INTO t DEFAULT VALUES").unwrap();
    let res = x.execute("SELECT COUNT(*) FROM t").unwrap();
    assert_eq!(res.rows.len(), 1);
    assert_eq!(extract_int(&res, 0, 0), 3);
    let res = x.execute("SELECT a, b FROM t").unwrap();
    assert_eq!(res.rows.len(), 3);
    for row in 0..3 {
        assert!(is_null(&res, row, 0), "a must be NULL for row {}", row);
        assert_eq!(extract_int(&res, row, 1), 7);
    }
}
