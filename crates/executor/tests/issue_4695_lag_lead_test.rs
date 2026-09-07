//! V312-81 / Issue #4695: Window functions LAG and LEAD
//!
//! Before fix: LAG/LEAD returned NULL for all rows (missing arms in
//! `evaluate_window_call` in `src/expr_utils.rs`).
//! After fix: LAG/LEAD return correct preceding/following values.

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

fn row_values(result: &sqlrustgo::ExecutorResult) -> Vec<Vec<Value>> {
    result.rows.clone()
}

// LAG: value from preceding row (offset N, default D)
#[test]
fn test_lag_basic() {
    let mut e = engine();
    e.execute("CREATE TABLE t(id INT, val INT)").unwrap();
    e.execute("INSERT INTO t VALUES (1,10),(2,20),(3,30),(4,40)")
        .unwrap();
    let result = e
        .execute("SELECT id, val, LAG(val, 1, 0) OVER (ORDER BY id) FROM t ORDER BY id")
        .unwrap();
    let rows = row_values(&result);
    assert_eq!(rows.len(), 4);
    // id=1: no preceding row → default=0
    assert_eq!(rows[0][2], Value::Integer(0));
    // id=2: LAG(1)=10
    assert_eq!(rows[1][2], Value::Integer(10));
    // id=3: LAG(1)=20
    assert_eq!(rows[2][2], Value::Integer(20));
    // id=4: LAG(1)=30
    assert_eq!(rows[3][2], Value::Integer(30));
}

#[test]
fn test_lag_no_default() {
    let mut e = engine();
    e.execute("CREATE TABLE t(id INT, val INT)").unwrap();
    e.execute("INSERT INTO t VALUES (1,10),(2,20),(3,30)")
        .unwrap();
    let result = e
        .execute("SELECT id, val, LAG(val) OVER (ORDER BY id) FROM t ORDER BY id")
        .unwrap();
    let rows = row_values(&result);
    assert_eq!(rows.len(), 3);
    // id=1: no preceding row, no default → NULL
    assert_eq!(rows[0][2], Value::Null);
    // id=2: val from id=1
    assert_eq!(rows[1][2], Value::Integer(10));
    // id=3: val from id=2
    assert_eq!(rows[2][2], Value::Integer(20));
}
#[test]
fn test_lag_custom_offset() {
    let mut e = engine();
    e.execute("CREATE TABLE t(id INT, val INT)").unwrap();
    e.execute("INSERT INTO t VALUES (1,10),(2,20),(3,30),(4,40)")
        .unwrap();
    let result = e
        .execute("SELECT id, val, LAG(val, 2, -1) OVER (ORDER BY id) FROM t ORDER BY id")
        .unwrap();
    let rows = row_values(&result);
    // id=1: no 2 rows back → default=-1
    assert_eq!(rows[0][2], Value::Integer(-1));
    // id=2: no 2 rows back → default=-1
    assert_eq!(rows[1][2], Value::Integer(-1));
    // id=3: LAG(2)=10
    assert_eq!(rows[2][2], Value::Integer(10));
    // id=4: LAG(2)=20
    assert_eq!(rows[3][2], Value::Integer(20));
}

#[test]
fn test_lag_partition() {
    let mut e = engine();
    e.execute("CREATE TABLE t(g TEXT, id INT, val INT)")
        .unwrap();
    e.execute("INSERT INTO t VALUES ('A',1,10),('A',2,20),('B',1,100),('B',2,200)")
        .unwrap();
    let result = e
        .execute("SELECT g, id, val, LAG(val, 1, 0) OVER (PARTITION BY g ORDER BY id) FROM t ORDER BY g, id")
        .unwrap();
    let rows = row_values(&result);
    // Partition A: id=1 no preceding → 0, id=2 → 10
    assert_eq!(rows[0][3], Value::Integer(0));
    assert_eq!(rows[1][3], Value::Integer(10));
    // Partition B: id=1 no preceding → 0, id=2 → 100
    assert_eq!(rows[2][3], Value::Integer(0));
    assert_eq!(rows[3][3], Value::Integer(100));
}

// LEAD: value from following row (offset N, default D)
#[test]
fn test_lead_basic() {
    let mut e = engine();
    e.execute("CREATE TABLE t(id INT, val INT)").unwrap();
    e.execute("INSERT INTO t VALUES (1,10),(2,20),(3,30),(4,40)")
        .unwrap();
    let result = e
        .execute("SELECT id, val, LEAD(val, 1, 99) OVER (ORDER BY id) FROM t ORDER BY id")
        .unwrap();
    let rows = row_values(&result);
    assert_eq!(rows.len(), 4);
    // id=1: LEAD(1)=20
    assert_eq!(rows[0][2], Value::Integer(20));
    // id=2: LEAD(1)=30
    assert_eq!(rows[1][2], Value::Integer(30));
    // id=3: LEAD(1)=40
    assert_eq!(rows[2][2], Value::Integer(40));
    // id=4: no following row → default=99
    assert_eq!(rows[3][2], Value::Integer(99));
}

#[test]
fn test_lead_no_default() {
    let mut e = engine();
    e.execute("CREATE TABLE t(id INT, val INT)").unwrap();
    e.execute("INSERT INTO t VALUES (1,10),(2,20),(3,30)")
        .unwrap();
    let result = e
        .execute("SELECT id, val, LEAD(val) OVER (ORDER BY id) FROM t ORDER BY id")
        .unwrap();
    let rows = row_values(&result);
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0][2], Value::Integer(20)); // id=1: val from id=2
    assert_eq!(rows[1][2], Value::Integer(30)); // id=2: val from id=3
    assert_eq!(rows[2][2], Value::Null); // id=3: no following row, no default
}

#[test]
fn test_lead_custom_offset() {
    let mut e = engine();
    e.execute("CREATE TABLE t(id INT, val INT)").unwrap();
    e.execute("INSERT INTO t VALUES (1,10),(2,20),(3,30),(4,40)")
        .unwrap();
    let result = e
        .execute("SELECT id, val, LEAD(val, 2, -1) OVER (ORDER BY id) FROM t ORDER BY id")
        .unwrap();
    let rows = row_values(&result);
    // id=1: LEAD(2)=30
    assert_eq!(rows[0][2], Value::Integer(30));
    // id=2: LEAD(2)=40
    assert_eq!(rows[1][2], Value::Integer(40));
    // id=3: no 2 rows ahead → default=-1
    assert_eq!(rows[2][2], Value::Integer(-1));
    // id=4: no 2 rows ahead → default=-1
    assert_eq!(rows[3][2], Value::Integer(-1));
}

#[test]
fn test_lead_partition() {
    let mut e = engine();
    e.execute("CREATE TABLE t(g TEXT, id INT, val INT)")
        .unwrap();
    e.execute("INSERT INTO t VALUES ('A',1,10),('A',2,20),('B',1,100),('B',2,200)")
        .unwrap();
    let result = e
        .execute("SELECT g, id, val, LEAD(val, 1, 0) OVER (PARTITION BY g ORDER BY id) FROM t ORDER BY g, id")
        .unwrap();
    let rows = row_values(&result);
    // Partition A: id=1 → 20, id=2 no following → 0
    assert_eq!(rows[0][3], Value::Integer(20));
    assert_eq!(rows[1][3], Value::Integer(0));
    // Partition B: id=1 → 200, id=2 no following → 0
    assert_eq!(rows[2][3], Value::Integer(200));
    assert_eq!(rows[3][3], Value::Integer(0));
}

// Both LAG and LEAD in same query
#[test]
fn test_lag_and_lead_same_query() {
    let mut e = engine();
    e.execute("CREATE TABLE t(id INT, val INT)").unwrap();
    e.execute("INSERT INTO t VALUES (1,10),(2,20),(3,30),(4,40)")
        .unwrap();
    let result = e
        .execute("SELECT id, val, LAG(val, 1, 0) OVER (ORDER BY id) AS lag_val, LEAD(val, 1, 0) OVER (ORDER BY id) AS lead_val FROM t ORDER BY id")
        .unwrap();
    let rows = row_values(&result);
    assert_eq!(rows[0][2], Value::Integer(0)); // LAG: no preceding
    assert_eq!(rows[0][3], Value::Integer(20)); // LEAD: val from id=2
    assert_eq!(rows[1][2], Value::Integer(10)); // LAG: val from id=1
    assert_eq!(rows[1][3], Value::Integer(30)); // LEAD: val from id=3
    assert_eq!(rows[2][2], Value::Integer(20)); // LAG: val from id=2
    assert_eq!(rows[2][3], Value::Integer(40)); // LEAD: val from id=4
    assert_eq!(rows[3][2], Value::Integer(30)); // LAG: val from id=3
    assert_eq!(rows[3][3], Value::Integer(0)); // LEAD: no following
}
