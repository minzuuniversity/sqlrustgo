//! Operator-level regression test for ORDER BY col DESC.
//!
//! Sprint 3 (Operator Regression Suite) / Issue #3283 Task 7.
//!
//! Locks in the Q18 cell-diff pattern:
//! - ORDER BY o_totalprice DESC, o_orderdate should sort
//!   by totalprice in descending order, then by o_orderdate ascending
//!   as a tiebreaker.
//!
//! Acceptance (per Issue #3283):
//! - Use a minimal fixture (3-5 rows)
//! - Assert cell values, not just row count
//! - Run in < 100ms

use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::{Arc, RwLock};

fn engine() -> ExecutionEngine<MemoryStorage> {
    ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())))
}

#[test]
fn order_by_desc_simple() {
    let mut e = engine();
    e.execute("CREATE TABLE t (val INTEGER)").unwrap();
    e.execute("INSERT INTO t VALUES (1)").unwrap();
    e.execute("INSERT INTO t VALUES (5)").unwrap();
    e.execute("INSERT INTO t VALUES (3)").unwrap();
    e.execute("INSERT INTO t VALUES (2)").unwrap();
    e.execute("INSERT INTO t VALUES (4)").unwrap();
    let r = e.execute("SELECT val FROM t ORDER BY val DESC").unwrap();
    assert_eq!(r.rows[0][0].to_string(), "5");
    assert_eq!(r.rows[1][0].to_string(), "4");
    assert_eq!(r.rows[2][0].to_string(), "3");
    assert_eq!(r.rows[3][0].to_string(), "2");
    assert_eq!(r.rows[4][0].to_string(), "1");
}

#[test]
fn order_by_desc_asc_compound() {
    // Q18 pattern: ORDER BY o_totalprice DESC, o_orderdate (tiebreak ASC)
    let mut e = engine();
    e.execute("CREATE TABLE orders (o_orderkey INTEGER, o_totalprice INTEGER, o_orderdate TEXT)")
        .unwrap();
    // Two orders with same totalprice = 100, different dates
    e.execute("INSERT INTO orders VALUES (1, 100, '1995-03-15')")
        .unwrap();
    e.execute("INSERT INTO orders VALUES (2, 100, '1994-01-01')")
        .unwrap();
    e.execute("INSERT INTO orders VALUES (3, 200, '1995-01-01')")
        .unwrap();
    e.execute("INSERT INTO orders VALUES (4, 50, '1995-05-01')")
        .unwrap();
    let r = e
        .execute("SELECT o_orderkey FROM orders ORDER BY o_totalprice DESC, o_orderdate")
        .unwrap();
    // Expected: 3 (200), 2 (100, 1994-01-01), 1 (100, 1995-03-15), 4 (50)
    assert_eq!(r.rows[0][0].to_string(), "3");
    assert_eq!(r.rows[1][0].to_string(), "2");
    assert_eq!(r.rows[2][0].to_string(), "1");
    assert_eq!(r.rows[3][0].to_string(), "4");
}

#[test]
fn order_by_asc_simple() {
    let mut e = engine();
    e.execute("CREATE TABLE t (val INTEGER)").unwrap();
    e.execute("INSERT INTO t VALUES (3)").unwrap();
    e.execute("INSERT INTO t VALUES (1)").unwrap();
    e.execute("INSERT INTO t VALUES (2)").unwrap();
    let r = e.execute("SELECT val FROM t ORDER BY val ASC").unwrap();
    assert_eq!(r.rows[0][0].to_string(), "1");
    assert_eq!(r.rows[1][0].to_string(), "2");
    assert_eq!(r.rows[2][0].to_string(), "3");
}

#[test]
fn order_by_with_limit() {
    let mut e = engine();
    e.execute("CREATE TABLE t (val INTEGER)").unwrap();
    e.execute("INSERT INTO t VALUES (10)").unwrap();
    e.execute("INSERT INTO t VALUES (30)").unwrap();
    e.execute("INSERT INTO t VALUES (20)").unwrap();
    e.execute("INSERT INTO t VALUES (50)").unwrap();
    e.execute("INSERT INTO t VALUES (40)").unwrap();
    let r = e
        .execute("SELECT val FROM t ORDER BY val DESC LIMIT 3")
        .unwrap();
    assert_eq!(r.rows.len(), 3);
    assert_eq!(r.rows[0][0].to_string(), "50");
    assert_eq!(r.rows[1][0].to_string(), "40");
    assert_eq!(r.rows[2][0].to_string(), "30");
}
