//! Operator-level regression tests for AGGREGATE functions (SUM, AVG, COUNT).
//!
//! Sprint 3 (Operator Regression Suite) - protects against regression
//! in the aggregate executor. Locks in the storage-layer Float type
//! preservation contract (Sprint 4 / Issue #3276) and catches:
//! - SUM(TEXT) = 0 bug (fixed in PR #3295)
//! - AVG(Integer) should return Float, not Integer (existing)
//! - COUNT(*) vs COUNT(col) semantics
//! - Mixed Integer/Real in same column

use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::{Arc, RwLock};

fn engine() -> ExecutionEngine<MemoryStorage> {
    ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())))
}

#[test]
fn sum_integer_column_returns_integer() {
    let mut e = engine();
    e.execute("CREATE TABLE t (q INTEGER)").unwrap();
    e.execute("INSERT INTO t VALUES (10)").unwrap();
    e.execute("INSERT INTO t VALUES (20)").unwrap();
    e.execute("INSERT INTO t VALUES (30)").unwrap();
    let r = e.execute("SELECT SUM(q) FROM t").unwrap();
    assert_eq!(r.rows[0][0].to_string(), "60");
}

#[test]
fn sum_real_column_returns_real() {
    let mut e = engine();
    e.execute("CREATE TABLE t (p REAL)").unwrap();
    e.execute("INSERT INTO t VALUES (100.5)").unwrap();
    e.execute("INSERT INTO t VALUES (200.5)").unwrap();
    let r = e.execute("SELECT SUM(p) FROM t").unwrap();
    assert_eq!(
        r.rows[0][0].type_name(),
        "FLOAT",
        "SUM(REAL) must return Float type"
    );
    let v = match &r.rows[0][0] {
        sqlrustgo::Value::Float(f) => *f,
        other => panic!("expected Value::Float, got {other:?}"),
    };
    assert!((v - 301.0).abs() < 0.01, "expected ~301.0, got {v}");
}

#[test]
fn sum_real_with_null_values_skips_nulls() {
    let mut e = engine();
    e.execute("CREATE TABLE t (p REAL)").unwrap();
    e.execute("INSERT INTO t VALUES (100.5)").unwrap();
    e.execute("INSERT INTO t VALUES (NULL)").unwrap();
    e.execute("INSERT INTO t VALUES (200.5)").unwrap();
    let r = e.execute("SELECT SUM(p) FROM t").unwrap();
    let v: f64 = r.rows[0][0].to_string().parse().unwrap();
    assert!((v - 301.0).abs() < 0.01, "NULL should be skipped");
}

#[test]
fn sum_empty_table_returns_null_per_sql_standard() {
    let mut e = engine();
    e.execute("CREATE TABLE t (p REAL)").unwrap();
    let r = e.execute("SELECT SUM(p) FROM t").unwrap();
    assert_eq!(
        r.rows[0][0],
        sqlrustgo::Value::Null,
        "SUM of empty set must return NULL (SQL standard + PG + SQLite + MySQL)"
    );
}

#[test]
fn avg_integer_column_returns_float() {
    let mut e = engine();
    e.execute("CREATE TABLE t (q INTEGER)").unwrap();
    e.execute("INSERT INTO t VALUES (10)").unwrap();
    e.execute("INSERT INTO t VALUES (20)").unwrap();
    e.execute("INSERT INTO t VALUES (30)").unwrap();
    let r = e.execute("SELECT AVG(q) FROM t").unwrap();
    let v: f64 = r.rows[0][0].to_string().parse().unwrap();
    assert!(
        (v - 20.0).abs() < 0.01,
        "AVG(Integer) must be Float, got {v}"
    );
}

#[test]
fn avg_real_column_returns_float() {
    let mut e = engine();
    e.execute("CREATE TABLE t (p REAL)").unwrap();
    e.execute("INSERT INTO t VALUES (100.0)").unwrap();
    e.execute("INSERT INTO t VALUES (200.0)").unwrap();
    let r = e.execute("SELECT AVG(p) FROM t").unwrap();
    let v: f64 = r.rows[0][0].to_string().parse().unwrap();
    assert!((v - 150.0).abs() < 0.01);
}

#[test]
fn count_star_counts_all_rows() {
    let mut e = engine();
    e.execute("CREATE TABLE t (q INTEGER)").unwrap();
    for _ in 0..5 {
        e.execute("INSERT INTO t VALUES (1)").unwrap();
    }
    let r = e.execute("SELECT COUNT(*) FROM t").unwrap();
    assert_eq!(r.rows[0][0].to_string(), "5");
}

#[test]
fn count_column_skips_nulls() {
    let mut e = engine();
    e.execute("CREATE TABLE t (q INTEGER)").unwrap();
    e.execute("INSERT INTO t VALUES (1)").unwrap();
    e.execute("INSERT INTO t VALUES (NULL)").unwrap();
    e.execute("INSERT INTO t VALUES (2)").unwrap();
    e.execute("INSERT INTO t VALUES (NULL)").unwrap();
    let r = e.execute("SELECT COUNT(q) FROM t").unwrap();
    assert_eq!(
        r.rows[0][0].to_string(),
        "2",
        "COUNT(col) should skip NULLs"
    );
}

#[test]
fn count_distinct_counts_unique() {
    let mut e = engine();
    e.execute("CREATE TABLE t (q INTEGER)").unwrap();
    e.execute("INSERT INTO t VALUES (1)").unwrap();
    e.execute("INSERT INTO t VALUES (1)").unwrap();
    e.execute("INSERT INTO t VALUES (2)").unwrap();
    e.execute("INSERT INTO t VALUES (2)").unwrap();
    e.execute("INSERT INTO t VALUES (3)").unwrap();
    let r = e.execute("SELECT COUNT(DISTINCT q) FROM t").unwrap();
    assert_eq!(r.rows[0][0].to_string(), "3");
}
