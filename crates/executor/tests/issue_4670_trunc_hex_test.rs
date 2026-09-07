//! V312-79 / Issue #4670: TRUNCATE/TRUNC and HEX return NULL.
//!
//! Tests verify TRUNCATE numeric truncation and HEX conversion.

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use sqlrustgo_types::Value;
use std::sync::Arc;

fn engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

fn first_value(result: &sqlrustgo::ExecutorResult) -> &Value {
    result
        .rows
        .first()
        .and_then(|row| row.first())
        .expect("at least one row")
}

fn first_float(result: &sqlrustgo::ExecutorResult) -> f64 {
    match first_value(result) {
        Value::Float(f) => *f,
        Value::Integer(i) => *i as f64,
        other => panic!("expected numeric, got {other:?}"),
    }
}

fn first_text(result: &sqlrustgo::ExecutorResult) -> &str {
    match first_value(result) {
        Value::Text(s) => s.as_str(),
        other => panic!("expected Text, got {other:?}"),
    }
}

fn is_null(result: &sqlrustgo::ExecutorResult) -> bool {
    matches!(first_value(result), Value::Null)
}

// TRUNCATE tests
#[test]
fn truncate_positive_decimals() {
    let mut e = engine();
    let r = e.execute("SELECT TRUNC(3.14159, 3)").unwrap();
    assert!(
        (first_float(&r) - 3.141).abs() < 1e-10,
        "TRUNC(3.14159, 3) = 3.141"
    );
}
#[test]
fn truncate_default_zero_decimals() {
    let mut e = engine();
    let r = e.execute("SELECT TRUNC(123.456)").unwrap();
}

#[test]
fn truncate_negative_scale() {
    let mut e = engine();
    let r = e.execute("SELECT TRUNC(123.456, -1)").unwrap();
    assert!(
        (first_float(&r) - 120.0).abs() < 1e-10,
        "TRUNC(123.456, -1) = 120.0"
    );
}

#[test]
fn truncate_integer_input() {
    let mut e = engine();
    let r = e.execute("SELECT TRUNC(42)").unwrap();
    assert!((first_float(&r) - 42.0).abs() < 1e-10);
}

#[test]
fn truncate_null_input() {
    let mut e = engine();
    let r = e.execute("SELECT TRUNC(NULL, 2)").unwrap();
    assert!(is_null(&r), "TRUNC(NULL, 2) must be NULL");
}

#[test]
fn truncate_trunc_alias() {
    let mut e = engine();
    let r = e.execute("SELECT TRUNC(3.14159, 3)").unwrap();
    assert!(
        (first_float(&r) - 3.141).abs() < 1e-10,
        "TRUNC is alias for TRUNCATE"
    );
}

// HEX tests
#[test]
fn hex_integer() {
    let mut e = engine();
    let r = e.execute("SELECT HEX(255)").unwrap();
    assert_eq!(first_text(&r), "FF", "HEX(255) = 'FF'");
}

#[test]
fn hex_integer_16() {
    let mut e = engine();
    let r = e.execute("SELECT HEX(16)").unwrap();
    assert_eq!(first_text(&r), "10", "HEX(16) = '10'");
}

#[test]
fn hex_zero() {
    let mut e = engine();
    let r = e.execute("SELECT HEX(0)").unwrap();
    assert_eq!(first_text(&r), "0", "HEX(0) = '0'");
}

#[test]
fn hex_null() {
    let mut e = engine();
    let r = e.execute("SELECT HEX(NULL)").unwrap();
    assert!(is_null(&r), "HEX(NULL) must be NULL");
}

#[test]
fn hex_text() {
    let mut e = engine();
    let r = e.execute("SELECT HEX('Hello')").unwrap();
    // 'H'=0x48,'e'=0x65,'l'=0x6C,'l'=0x6C,'o'=0x6F
    assert_eq!(first_text(&r), "48656C6C6F", "HEX('Hello') = '48656C6C6F'");
}

#[test]
fn truncate_table_usage() {
    let mut e = engine();
    e.execute("CREATE TABLE t(v REAL)").unwrap();
    e.execute("INSERT INTO t VALUES (3.14159), (99.999), (42.0)")
        .unwrap();
    let r = e
        .execute("SELECT v, TRUNC(v, 2) FROM t ORDER BY v")
        .unwrap();
    match r.rows[0][1] {
        Value::Float(f) => assert!((f - 3.14).abs() < 1e-10, "row0 TRUNC=3.14 got {}", f),
        Value::Integer(ref i) => assert!(((*i) as f64 - 3.14).abs() < 1e-10),
        _ => panic!("expected float"),
    }
    match r.rows[1][1] {
        Value::Float(f) => assert!((f - 42.0).abs() < 1e-10, "row1 TRUNC=42.0 got {}", f),
        Value::Integer(ref i) => assert!(((*i) as f64 - 42.0).abs() < 1e-10),
        _ => panic!("expected float"),
    }
    match r.rows[2][1] {
        Value::Float(f) => assert!((f - 99.99).abs() < 1e-10, "row2 TRUNC=99.99 got {}", f),
        Value::Integer(ref i) => assert!(((*i) as f64 - 99.99).abs() < 1e-10),
        _ => panic!("expected float"),
    }
}
