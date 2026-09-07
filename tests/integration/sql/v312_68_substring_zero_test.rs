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
    assert_eq!(
        extract_text(&res, 0, 1),
        "hello",
        "position 1 = whole string"
    );
    assert_eq!(extract_text(&res, 0, 2), "o", "position 5 = last char");
}

#[test]
fn v312_68_substring_zero_with_length_is_empty() {
    let mut x = fresh();
    let res = x
        .execute("SELECT SUBSTRING('hello', 0, 3), SUBSTRING('hello', 1, 3)")
        .unwrap();
    assert_eq!(res.rows.len(), 1);
    assert_eq!(
        extract_text(&res, 0, 0),
        "",
        "start=0 with length=3 is empty"
    );
    assert_eq!(
        extract_text(&res, 0, 1),
        "hel",
        "start=1 length=3 is first 3"
    );
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

// ============================================================================
// V312-64h / Issue #4761 — SUBSTRING(str, -n) must count from the end.
// SQLite convention: negative start position = |i|th char from the right.
// This block extends the SUBSTRING test surface; the zero-index tests above
// (#4681) and the negative-start tests below (#4761) together pin down the
// full SUBSTRING semantics in the spirit of SQLite's docs.
// ============================================================================

#[test]
fn v312_64h_substring_negative_start_no_length() {
    // SQLite: SUBSTR('hello world', -5) = 'world' (last 5 chars).
    let mut x = fresh();
    let res = x.execute("SELECT SUBSTRING('hello world', -5)").unwrap();
    assert_eq!(res.rows.len(), 1);
    assert_eq!(extract_text(&res, 0, 0), "world", "must take last 5 chars");
}

#[test]
fn v312_64h_substring_negative_start_with_length() {
    // SQLite: SUBSTR('hello world', -5, 5) = 'world' (start 5 from right, 5 chars).
    let mut x = fresh();
    let res = x.execute("SELECT SUBSTRING('hello world', -5, 5)").unwrap();
    assert_eq!(res.rows.len(), 1);
    assert_eq!(extract_text(&res, 0, 0), "world");
}

#[test]
fn v312_64h_substring_negative_start_clamps_to_zero() {
    // SQLite: SUBSTR('hello', -100) = 'hello' (clamped to start of string).
    let mut x = fresh();
    let res = x.execute("SELECT SUBSTRING('hello', -100)").unwrap();
    assert_eq!(res.rows.len(), 1);
    assert_eq!(extract_text(&res, 0, 0), "hello");
}

#[test]
fn v312_64h_substring_negative_start_exact() {
    // SQLite: SUBSTR('hello', -5) = 'hello' (start at index 0, full string).
    let mut x = fresh();
    let res = x.execute("SELECT SUBSTRING('hello', -5)").unwrap();
    assert_eq!(res.rows.len(), 1);
    assert_eq!(extract_text(&res, 0, 0), "hello");
}

#[test]
fn v312_64h_substring_negative_start_partial() {
    // SQLite: SUBSTR('hello', -3, 2) = 'll' (start 3rd from right, take 2 chars).
    let mut x = fresh();
    let res = x.execute("SELECT SUBSTRING('hello', -3, 2)").unwrap();
    assert_eq!(res.rows.len(), 1);
    assert_eq!(extract_text(&res, 0, 0), "ll");
}

#[test]
fn v312_64h_substring_substr_alias_negative_start() {
    // SUBSTR is the alias for SUBSTRING — same negative-start semantics.
    let mut x = fresh();
    let res = x
        .execute("SELECT SUBSTR('hello world', -5), SUBSTR('hello', -5)")
        .unwrap();
    assert_eq!(res.rows.len(), 1);
    assert_eq!(extract_text(&res, 0, 0), "world");
    assert_eq!(extract_text(&res, 0, 1), "hello");
}

#[test]
fn v312_64h_substring_positive_unchanged_regression() {
    // Regression: V312-68 (#4681) start=0 → empty must still hold, and the
    // positive-start branch must be untouched by the V312-64h change.
    let mut x = fresh();
    let res = x
        .execute("SELECT SUBSTRING('hello', 0), SUBSTRING('hello', 1, 5)")
        .unwrap();
    assert_eq!(res.rows.len(), 1);
    assert_eq!(extract_text(&res, 0, 0), "", "start=0 still empty");
    assert_eq!(
        extract_text(&res, 0, 1),
        "hello",
        "start=1 length=5 whole string"
    );
}
