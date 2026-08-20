//! Integration tests for MySQL 5.7 function compatibility (Issue #2988 / MySQL-01)

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::Arc;

fn fresh() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

#[test]
fn test_select_coalesce() {
    let mut e = fresh();
    e.execute("CREATE TABLE t (id INTEGER, a INTEGER, b INTEGER, c INTEGER)")
        .unwrap();
    e.execute("INSERT INTO t VALUES (1, NULL, NULL, 5), (2, 3, 4, 5)")
        .unwrap();
    let r = e
        .execute("SELECT COALESCE(a, b, c) FROM t ORDER BY id")
        .unwrap();
    assert_eq!(r.rows.len(), 2);
    // row 1: a=NULL, b=NULL, c=5 -> 5
    // row 2: a=3, b=4, c=5 -> 3
    let v0 = match &r.rows[0][0] {
        sqlrustgo::Value::Integer(n) => *n,
        other => panic!("row 0: expected Integer, got {other:?}"),
    };
    let v1 = match &r.rows[1][0] {
        sqlrustgo::Value::Integer(n) => *n,
        other => panic!("row 1: expected Integer, got {other:?}"),
    };
    assert_eq!(v0, 5);
    assert_eq!(v1, 3);
}

#[test]
fn test_select_nullif() {
    let mut e = fresh();
    e.execute("CREATE TABLE t (id INTEGER, x INTEGER)").unwrap();
    e.execute("INSERT INTO t VALUES (1, 5), (2, 5), (3, 6)")
        .unwrap();
    let r = e.execute("SELECT NULLIF(x, 5) FROM t ORDER BY id").unwrap();
    assert!(matches!(&r.rows[0][0], sqlrustgo::Value::Null));
    assert!(matches!(&r.rows[1][0], sqlrustgo::Value::Null));
    assert_eq!(r.rows[2][0], sqlrustgo::Value::Integer(6));
}

#[test]
fn test_select_date_add() {
    let mut e = fresh();
    e.execute("CREATE TABLE t (id INTEGER, d TEXT)").unwrap();
    e.execute("INSERT INTO t VALUES (1, '2026-06-04'), (2, '2026-12-25')")
        .unwrap();
    let r = e
        .execute("SELECT DATE_ADD(d, 7, 'DAY') FROM t ORDER BY id")
        .unwrap();
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Text("2026-06-11".into()));
    assert_eq!(r.rows[1][0], sqlrustgo::Value::Text("2027-01-01".into()));
}

#[test]
fn test_select_date_sub() {
    let mut e = fresh();
    e.execute("CREATE TABLE t (id INTEGER, d TEXT)").unwrap();
    e.execute("INSERT INTO t VALUES (1, '2026-06-04')").unwrap();
    let r = e.execute("SELECT DATE_SUB(d, 10, 'DAY') FROM t").unwrap();
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Text("2026-05-25".into()));
}
