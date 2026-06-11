//! Operator-level regression test for SUM over REAL column.
//!
//! Sprint 3 (Operator Regression Suite) / Issue #3283 Task 1.
//!
//! Locks in the storage-layer Float type preservation contract
//! (Sprint 4 / Issue #3276).  The original bug: SUM over a REAL column
//! silently coerced values to integer 0 due to a storage-layer
//! truncation, producing 0 for any sum that should be > 0.
//!
//! Acceptance (per Issue #3283):
//! - Use a minimal fixture (1KB .tbl equivalent, 3-5 rows)
//! - Assert cell values, not just row count
//! - Run in < 100ms
//! - Be reproducible

use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::{Arc, RwLock};

fn engine() -> ExecutionEngine<MemoryStorage> {
    ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())))
}

#[test]
fn sum_real_values_returns_real_sum() {
    // 5 rows of REAL values; sum should be 1.5, not 0.
    let mut e = engine();
    e.execute("CREATE TABLE t (price REAL)").unwrap();
    e.execute("INSERT INTO t VALUES (0.1)").unwrap();
    e.execute("INSERT INTO t VALUES (0.2)").unwrap();
    e.execute("INSERT INTO t VALUES (0.3)").unwrap();
    e.execute("INSERT INTO t VALUES (0.4)").unwrap();
    e.execute("INSERT INTO t VALUES (0.5)").unwrap();
    let r = e.execute("SELECT SUM(price) FROM t").unwrap();
    let sum_str = r.rows[0][0].to_string();
    let sum: f64 = sum_str.parse().unwrap_or(0.0);
    assert!(
        (sum - 1.5).abs() < 1e-6,
        "SUM(price) should be 1.5, got {}",
        sum_str
    );
}

#[test]
fn sum_real_with_fractional_does_not_truncate() {
    // The exact regression: SUM(REAL) used to return 0 due to
    // integer truncation. This test pins that to never happen
    // again for fractional values.
    let mut e = engine();
    e.execute("CREATE TABLE t (x REAL)").unwrap();
    for v in &["1.1", "2.2", "3.3", "4.4", "5.5"] {
        e.execute(&format!("INSERT INTO t VALUES ({})", v)).unwrap();
    }
    let r = e.execute("SELECT SUM(x) FROM t").unwrap();
    let s: f64 = r.rows[0][0].to_string().parse().unwrap_or(0.0);
    assert!(s > 16.0, "SUM(x) should be ~16.5, got {}", s);
}

#[test]
fn sum_real_mixed_with_integer_preserves_real() {
    // Mixed INTEGER and REAL — the sum should be the REAL aggregate.
    let mut e = engine();
    e.execute("CREATE TABLE t (v REAL)").unwrap();
    e.execute("INSERT INTO t VALUES (10)").unwrap();
    e.execute("INSERT INTO t VALUES (10.5)").unwrap();
    e.execute("INSERT INTO t VALUES (20.5)").unwrap();
    let r = e.execute("SELECT SUM(v) FROM t").unwrap();
    let s: f64 = r.rows[0][0].to_string().parse().unwrap_or(0.0);
    assert!(
        (s - 41.0).abs() < 1e-6,
        "SUM(v) should be 41.0, got {}",
        s
    );
}