// Reproduction script for Issue #3285 (storage REAL preservation)
// Sprint 4 diagnostic: does MemoryStorage::insert(Value::Float(100.5)) preserve type on scan()?
//
// Expected (per sprint4-sum-real-investigation.md): Value::Float(100.5) on scan
// Actual (suspected): Value::Integer(100) or Value::Integer(0) — type lost in storage layer

use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::sync::{Arc, RwLock};

fn engine() -> ExecutionEngine<MemoryStorage> {
    ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())))
}

#[test]
fn repro_3285_real_insert_preserves_type() {
    let mut e = engine();
    e.execute("CREATE TABLE t (q REAL)").unwrap();
    e.execute("INSERT INTO t VALUES (100.5)").unwrap();
    let r = e.execute("SELECT q FROM t").unwrap();
    let val = &r.rows[0][0];
    println!("\n  Result type: {:?}, value: {:?}", val, val);
    assert_eq!(*val, Value::Float(100.5), "REAL preservation: expected Value::Float(100.5)");
}

#[test]
fn repro_3285_sum_real_returns_correct_value() {
    let mut e = engine();
    e.execute("CREATE TABLE t (q REAL)").unwrap();
    e.execute("INSERT INTO t VALUES (100.5)").unwrap();
    e.execute("INSERT INTO t VALUES (200.5)").unwrap();
    let r = e.execute("SELECT SUM(q) FROM t").unwrap();
    let val = &r.rows[0][0];
    println!("\n  SUM result type: {:?}, value: {:?}", val, val);
    assert_eq!(*val, Value::Float(301.0), "SUM(REAL) should preserve float type");
}

#[test]
fn repro_3285_integer_column_works_baseline() {
    // Confirms INTEGER columns work — baseline for comparison with REAL
    let mut e = engine();
    e.execute("CREATE TABLE t (q INTEGER)").unwrap();
    e.execute("INSERT INTO t VALUES (100)").unwrap();
    e.execute("INSERT INTO t VALUES (200)").unwrap();
    let r = e.execute("SELECT SUM(q) FROM t").unwrap();
    let val = &r.rows[0][0];
    println!("\n  SUM(INTEGER) result type: {:?}, value: {:?}", val, val);
    assert_eq!(*val, Value::Integer(300), "SUM(INTEGER) baseline");
}
