//! V312-77 / Issue #4676: MOD / POWER / LOG / EXP / SQRT return NULL.
//!
//! Tests verify all five functions produce correct f64 results,
//! NULL propagation, and domain-error handling.

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

fn is_null(result: &sqlrustgo::ExecutorResult) -> bool {
    matches!(first_value(result), Value::Null)
}

fn get_float(result: &sqlrustgo::ExecutorResult, row: usize, col: usize) -> f64 {
    match &result.rows[row][col] {
        Value::Float(f) => *f,
        Value::Integer(i) => *i as f64,
        other => panic!("expected numeric at [{}][{}], got {other:?}", row, col),
    }
}

#[test]
fn mod_integer_args() {
    let mut e = engine();
    let r = e.execute("SELECT MOD(10, 3)").unwrap();
    assert!((first_float(&r) - 1.0).abs() < 1e-10, "MOD(10,3) = 1.0");
}

#[test]
fn mod_float_args() {
    let mut e = engine();
    let r = e.execute("SELECT MOD(4.5, 2.1)").unwrap();
    let v = first_float(&r);
    assert!((v - 0.3).abs() < 0.001, "MOD(4.5, 2.1) ≈ 0.3, got {}", v);
}

#[test]
fn mod_div_by_zero() {
    let mut e = engine();
    let r = e.execute("SELECT MOD(10, 0)").unwrap();
    assert!(is_null(&r), "MOD(10, 0) must be NULL");
}

#[test]
fn power_integer_args() {
    let mut e = engine();
    let r = e.execute("SELECT POWER(2, 3)").unwrap();
    assert!((first_float(&r) - 8.0).abs() < 1e-10, "POWER(2,3) = 8.0");
}

#[test]
fn power_half_for_sqrt() {
    let mut e = engine();
    let r = e.execute("SELECT POWER(100, 0.5)").unwrap();
    assert!(
        (first_float(&r) - 10.0).abs() < 1e-10,
        "POWER(100, 0.5) = 10.0"
    );
}

#[test]
fn sqrt_positive_integer() {
    let mut e = engine();
    let r = e.execute("SELECT SQRT(16)").unwrap();
    assert!((first_float(&r) - 4.0).abs() < 1e-10, "SQRT(16) = 4.0");
}

#[test]
fn sqrt_two() {
    let mut e = engine();
    let r = e.execute("SELECT SQRT(2)").unwrap();
    let v = first_float(&r);
    assert!(
        (v - 1.4142135623730951).abs() < 1e-10,
        "SQRT(2) ≈ 1.414..., got {}",
        v
    );
}

#[test]
fn sqrt_negative_returns_null() {
    let mut e = engine();
    let r = e.execute("SELECT SQRT(-1)").unwrap();
    assert!(is_null(&r), "SQRT(-1) must be NULL (domain error)");
}

#[test]
fn log_positive() {
    let mut e = engine();
    let r = e.execute("SELECT LOG(2.718281828)").unwrap();
    let v = first_float(&r);
    assert!((v - 1.0).abs() < 0.0001, "LOG(e) ≈ 1.0, got {}", v);
}

#[test]
fn log_negative_returns_null() {
    let mut e = engine();
    let r = e.execute("SELECT LOG(-1)").unwrap();
    assert!(is_null(&r), "LOG(-1) must be NULL (domain error)");
}

#[test]
fn log_zero_returns_null() {
    let mut e = engine();
    let r = e.execute("SELECT LOG(0)").unwrap();
    assert!(is_null(&r), "LOG(0) must be NULL (domain error)");
}

#[test]
fn exp_one() {
    let mut e = engine();
    let r = e.execute("SELECT EXP(1)").unwrap();
    let v = first_float(&r);
    assert!(
        (v - 2.718281828).abs() < 1e-6,
        "EXP(1) ≈ 2.718..., got {}",
        v
    );
}

#[test]
fn exp_zero() {
    let mut e = engine();
    let r = e.execute("SELECT EXP(0)").unwrap();
    assert!((first_float(&r) - 1.0).abs() < 1e-10, "EXP(0) = 1.0");
}

#[test]
fn all_functions_null_propagation() {
    let mut e = engine();
    for sql in &[
        "SELECT MOD(NULL, 3)",
        "SELECT POWER(NULL, 2)",
        "SELECT LOG(NULL)",
        "SELECT EXP(NULL)",
        "SELECT SQRT(NULL)",
    ] {
        let r = e.execute(sql).unwrap();
        assert!(is_null(&r), "{} must return NULL", sql);
    }
}

#[test]
fn mod_null_second_arg() {
    let mut e = engine();
    let r = e.execute("SELECT MOD(10, NULL)").unwrap();
    assert!(is_null(&r), "MOD(10, NULL) must be NULL");
}

#[test]
fn table_column_usage() {
    let mut e = engine();
    e.execute("CREATE TABLE t(v REAL)").unwrap();
    e.execute("INSERT INTO t VALUES (2), (4), (100)").unwrap();
    let r = e
        .execute("SELECT v, MOD(v, 3), POWER(v, 2), SQRT(v) FROM t ORDER BY v")
        .unwrap();

    // Row 1: v=2
    assert!((get_float(&r, 0, 0) - 2.0).abs() < 1e-10);
    assert!((get_float(&r, 0, 1) - 2.0).abs() < 1e-10, "MOD(2,3)=2.0");
    assert!((get_float(&r, 0, 2) - 4.0).abs() < 1e-10, "POWER(2,2)=4.0");
    assert!(
        (get_float(&r, 0, 3) - 1.4142135623730951).abs() < 1e-10,
        "SQRT(2)"
    );

    // Row 2: v=4
    assert!((get_float(&r, 1, 0) - 4.0).abs() < 1e-10);
    assert!((get_float(&r, 1, 1) - 1.0).abs() < 1e-10, "MOD(4,3)=1.0");
    assert!(
        (get_float(&r, 1, 2) - 16.0).abs() < 1e-10,
        "POWER(4,2)=16.0"
    );
    assert!((get_float(&r, 1, 3) - 2.0).abs() < 1e-10, "SQRT(4)=2.0");

    // Row 3: v=100
    assert!((get_float(&r, 2, 0) - 100.0).abs() < 1e-10);
    assert!((get_float(&r, 2, 1) - 1.0).abs() < 1e-10, "MOD(100,3)=1.0");
    assert!(
        (get_float(&r, 2, 2) - 10000.0).abs() < 1e-10,
        "POWER(100,2)=10000.0"
    );
    assert!((get_float(&r, 2, 3) - 10.0).abs() < 1e-10, "SQRT(100)=10.0");
}
