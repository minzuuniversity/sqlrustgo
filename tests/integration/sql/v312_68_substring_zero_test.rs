//! V312-68 / Issue #4681 regression integration test:
//! `SUBSTRING(s, 0)` MUST return the empty string (1-based indexing
//! per SQL/SQLite/PostgreSQL convention).

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
        Value::Null => String::new(),
        other => panic!("expected Text/Null at [{}][{}], got {:?}", row, col, other),
    }
}

#[test]
fn v312_68_substring_zero_index_returns_empty() {
    let mut x = fresh();
    // Issue body repro: SUBSTRING('hello', 0) should be ''.
    let res = x
        .execute("SELECT SUBSTRING('hello', 0), SUBSTRING('hello', 1), SUBSTRING('hello', 5)")
        .unwrap();
    assert_eq!(res.rows.len(), 1);
    assert_eq!(extract_text(&res, 0, 0), "", "position 0 must be empty");
    assert_eq!(extract_text(&res, 0, 1), "hello", "position 1 = whole string");
    assert_eq!(extract_text(&res, 0, 2), "o", "position 5 = last char");
}

#[test]
fn v312_68_substring_zero_with_length_is_empty() {
    let mut x = fresh();
    let res = x
        .execute("SELECT SUBSTRING('hello', 0, 3), SUBSTRING('hello', 1, 3)")
        .unwrap();
    assert_eq!(res.rows.len(), 1);
    assert_eq!(extract_text(&res, 0, 0), "", "start=0 with length=3 is empty");
    assert_eq!(extract_text(&res, 0, 1), "hel", "start=1 length=3 is first 3");
}

#[test]
fn v312_68_substring_out_of_range_is_empty() {
    let mut x = fresh();
    let res = x.execute("SELECT SUBSTRING('hello', 100)").unwrap();
    assert_eq!(res.rows.len(), 1);
    assert_eq!(extract_text(&res, 0, 0), "", "position 100 is past end");
}

#[test]
fn v312_68_substr_alias_zero_index() {
    // SUBSTR is the alias for SUBSTRING — same semantics.
    let mut x = fresh();
    let res = x
        .execute("SELECT SUBSTR('hello', 0), SUBSTR('hello', 1), SUBSTR('hello', 5)")
        .unwrap();
    assert_eq!(res.rows.len(), 1);
    assert_eq!(extract_text(&res, 0, 0), "");
    assert_eq!(extract_text(&res, 0, 1), "hello");
    assert_eq!(extract_text(&res, 0, 2), "o");
}
