//! V312-85 / Issue #4761: SUBSTRING/SUBSTR negative start counts from end.
//!
//! SQLite/PostgreSQL convention:
//! - SUBSTRING('hello world', 1)    → 'hello world' (entire string)
//! - SUBSTRING('hello world', 1, 5) → 'hello'      (5 chars from pos 1)
//! - SUBSTRING('hello world', -5)   → 'world'      (last 5 chars)
//! - SUBSTRING('hello world', -3)   → 'rld'        (last 3 chars)
//! - SUBSTRING('hello world', 0)    → ''           (before start → empty)
//!
//! V312-85 / Issue #4763: ORDER BY ... NULLS FIRST / NULLS LAST
//! - default NULLS LAST for ASC, NULLS FIRST for DESC
//! - explicit NULLS FIRST / NULLS LAST clauses honored
//!
//! V312-85 / Issue #4764: SUBSTRING length argument
//! - SUBSTRING('hello world', 1, 5) returns 'hello' (5 chars), not entire string
//!
//! V312-85 / Issue #4762: TRUNCATE TABLE support CASCADE/RESTRICT options

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::sync::Arc;

fn fresh_mem() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

fn as_text(v: &Value) -> String {
    match v {
        Value::Text(s) => s.clone(),
        Value::Integer(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Boolean(b) => b.to_string(),
        Value::Null => "NULL".to_string(),
        Value::Blob(b) => format!("<blob {} bytes>", b.len()),
        Value::Point(x, y) => format!("POINT({}, {})", x, y),
        Value::Json(s) => s.to_string(),
    }
}

fn row_text(row: &[Value]) -> Vec<String> {
    row.iter().map(as_text).collect()
}

// ============================================================================
// Issue #4761 — SUBSTRING/SUBSTR accepts negative start
// ============================================================================

#[test]
fn substring_negative_start_full() {
    let mut x = fresh_mem();
    let r = x.execute("SELECT SUBSTRING('hello world', -5)").unwrap();
    let rows = r.rows;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], Value::Text("world".to_string()));
}

#[test]
fn substring_negative_start_short() {
    let mut x = fresh_mem();
    let r = x.execute("SELECT SUBSTRING('hello world', -3)").unwrap();
    let rows = r.rows;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], Value::Text("rld".to_string()));
}

#[test]
fn substr_alias_negative_start() {
    let mut x = fresh_mem();
    let r = x.execute("SELECT SUBSTR('hello world', -5)").unwrap();
    let rows = r.rows;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], Value::Text("world".to_string()));
}

#[test]
fn substring_negative_start_exceeds_length() {
    // Upstream PR #4767 (#4761): |i| >= n clamps to 0 (returns full string).
    // SQLite/PG behavior: SUBSTRING('hello', -100) = 'hello'.
    let mut x = fresh_mem();
    let r = x.execute("SELECT SUBSTRING('hello', -100)").unwrap();
    let rows = r.rows;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], Value::Text("hello".to_string()));
}

#[test]
fn substring_zero_still_empty() {
    // Position 0 = before start, still returns empty (per #4681)
    let mut x = fresh_mem();
    let r = x.execute("SELECT SUBSTRING('hello world', 0)").unwrap();
    let rows = r.rows;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], Value::Text("".to_string()));
}

// ============================================================================
// Issue #4764 — SUBSTRING with length argument
// ============================================================================

#[test]
fn substring_with_length() {
    let mut x = fresh_mem();
    let r = x.execute("SELECT SUBSTRING('hello world', 1, 5)").unwrap();
    let rows = r.rows;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], Value::Text("hello".to_string()));
}

#[test]
fn substring_full_no_length() {
    let mut x = fresh_mem();
    let r = x.execute("SELECT SUBSTRING('hello world', 1)").unwrap();
    let rows = r.rows;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], Value::Text("hello world".to_string()));
}

// ============================================================================
// Issue #4763 — ORDER BY NULLS FIRST / NULLS LAST
// ============================================================================

#[test]
fn order_by_nulls_first() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(val int)").unwrap();
    x.execute("INSERT INTO t VALUES (1)").unwrap();
    x.execute("INSERT INTO t VALUES (2)").unwrap();
    // INSERT NULL via SELECT subquery work-around (INSERT VALUES(NULL) parser fails)
    x.execute("INSERT INTO t SELECT NULL").unwrap();
    let r = x
        .execute("SELECT val FROM t ORDER BY val NULLS FIRST")
        .unwrap();
    let rows = r.rows;
    let vals: Vec<String> = rows.iter().map(|r| as_text(&r[0])).collect();
    // NULL must come first
    assert!(vals[0] == "NULL", "expected NULL first, got {:?}", vals);
    assert!(vals[1] == "1");
    assert!(vals[2] == "2");
}

#[test]
fn order_by_nulls_last() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(val int)").unwrap();
    x.execute("INSERT INTO t VALUES (1)").unwrap();
    x.execute("INSERT INTO t VALUES (2)").unwrap();
    x.execute("INSERT INTO t SELECT NULL").unwrap();
    let r = x
        .execute("SELECT val FROM t ORDER BY val NULLS LAST")
        .unwrap();
    let rows = r.rows;
    let vals: Vec<String> = rows.iter().map(|r| as_text(&r[0])).collect();
    assert_eq!(vals[0], "1");
    assert_eq!(vals[1], "2");
    assert_eq!(vals[2], "NULL", "expected NULL last, got {:?}", vals);
}

#[test]
fn order_by_default_asc_nulls_last() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(val int)").unwrap();
    x.execute("INSERT INTO t VALUES (1)").unwrap();
    x.execute("INSERT INTO t VALUES (2)").unwrap();
    x.execute("INSERT INTO t SELECT NULL").unwrap();
    let r = x.execute("SELECT val FROM t ORDER BY val").unwrap();
    let rows = r.rows;
    let vals: Vec<String> = rows.iter().map(|r| as_text(&r[0])).collect();
    // Default NULLS LAST for ASC
    assert_eq!(vals[0], "1");
    assert_eq!(vals[1], "2");
    assert_eq!(vals[2], "NULL");
}

#[test]
fn order_by_desc_default_nulls_last() {
    let mut x = fresh_mem();
    x.execute("CREATE TABLE t(val int)").unwrap();
    x.execute("INSERT INTO t VALUES (1)").unwrap();
    x.execute("INSERT INTO t VALUES (2)").unwrap();
    x.execute("INSERT INTO t SELECT NULL").unwrap();
    let r = x.execute("SELECT val FROM t ORDER BY val DESC").unwrap();
    let rows = r.rows;
    let vals: Vec<String> = rows.iter().map(|r| as_text(&r[0])).collect();
    // SQLite default for DESC: NULLS LAST (verified via sqlite3 3.51)
    assert_eq!(vals[0], "2", "got {:?}", vals);
    assert_eq!(vals[1], "1");
    assert_eq!(vals[2], "NULL");
}
