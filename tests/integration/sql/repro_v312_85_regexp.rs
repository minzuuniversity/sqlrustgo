//! V312-85 / Issue #4751: REGEXP/RLIKE tests
//!
//! MySQL/PostgreSQL `col REGEXP pattern` regex match.
//! Examples:
//! - `SELECT 'abc' REGEXP 'a'` returns true
//! - `SELECT 'abc' REGEXP '[0-9]+'` returns false
//! - `SELECT 'user@example.com' REGEXP '@'` returns true
//! - `SELECT REGEXP('abc', 'a')` (function-call form) returns true

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::Arc;

fn fresh_mem() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

#[test]
fn regexp_postfix_after_string_literal() {
    let mut x = fresh_mem();
    let r = x
        .execute("SELECT 'abc123' REGEXP '[0-9]+' AS has_digit")
        .expect("REGEXP after string literal");
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Boolean(true));
}

#[test]
fn regexp_function_call_form() {
    let mut x = fresh_mem();
    let r = x
        .execute("SELECT REGEXP('abc', 'a') AS m, REGEXP('abc', '[0-9]+') AS m2")
        .expect("REGEXP function call");
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Boolean(true));
    assert_eq!(r.rows[0][1], sqlrustgo::Value::Boolean(false));
}

#[test]
fn regexp_rlike_alias() {
    let mut x = fresh_mem();
    let r = x
        .execute("SELECT 'abc' RLIKE 'a' AS m1, 'abc' RLIKE 'b' AS m2")
        .expect("RLIKE");
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Boolean(true));
    assert_eq!(r.rows[0][1], sqlrustgo::Value::Boolean(true));
}

#[test]
fn regexp_infix_with_column() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(s text)").unwrap();
    x.execute("INSERT INTO t VALUES ('abc123')").unwrap();
    x.execute("INSERT INTO t VALUES ('hello')").unwrap();
    let r = x
        .execute("SELECT s, s REGEXP '[0-9]+' AS has_digit FROM t ORDER BY s")
        .expect("column REGEXP");
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Text("abc123".to_string()));
    assert_eq!(r.rows[0][1], sqlrustgo::Value::Boolean(true));
    assert_eq!(r.rows[1][1], sqlrustgo::Value::Boolean(false));
}

#[test]
fn regexp_null_propagation() {
    // V312-85 / Issue #4751 (partial): NULL propagation in REGEXP is
    // a separate upstream concern — `SELECT NULL REGEXP 'a'` is parsed
    // as 3 columns (NULL, REGEXP, 'a') because Token::Null doesn't go
    // through our Token::Identifier path. Document the current
    // behaviour rather than fixing it (out of scope for this PR).
    let mut x = fresh_mem();
    let r = x
        .execute("SELECT 'abc' REGEXP NULL AS m")
        .expect("REGEXP with NULL pattern");
    let rows = r.rows;
    // Second arg NULL → eval_regexp returns Null.
    assert_eq!(rows[0][0], sqlrustgo::Value::Null);
}

#[test]
fn regexp_anchors() {
    let mut x = fresh_mem();
    let r = x
        .execute("SELECT 'abc' REGEXP '^ab' AS pre, 'abc' REGEXP 'bc$' AS suf")
        .expect("anchors");
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Boolean(true));
    assert_eq!(r.rows[0][1], sqlrustgo::Value::Boolean(true));
}
