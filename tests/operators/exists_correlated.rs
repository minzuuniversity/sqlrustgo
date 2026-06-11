//! Operator-level regression test for correlated EXISTS subquery.
//!
//! Sprint 3 (Operator Regression Suite) / Issue #3283 Task 6.
//!
//! Locks in the Q4/Q20/Q21 correlated EXISTS evaluator:
//! - EXISTS(SELECT * FROM lineitem WHERE l_orderkey = orders.o_orderkey
//!   AND <static_predicate>) should be true when at least one inner row
//!   matches.
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
fn correlated_exists_simple_match() {
    let mut e = engine();
    e.execute("CREATE TABLE orders (o_orderkey INTEGER)")
        .unwrap();
    e.execute("CREATE TABLE lineitem (l_orderkey INTEGER, l_qty INTEGER)")
        .unwrap();
    e.execute("INSERT INTO orders VALUES (1)").unwrap();
    e.execute("INSERT INTO orders VALUES (2)").unwrap();
    e.execute("INSERT INTO lineitem VALUES (1, 5)").unwrap();
    e.execute("INSERT INTO lineitem VALUES (1, 6)").unwrap();
    e.execute("INSERT INTO lineitem VALUES (2, 7)").unwrap();
    let r = e
        .execute(
            "SELECT o_orderkey FROM orders \
         WHERE EXISTS (SELECT * FROM lineitem WHERE l_orderkey = orders.o_orderkey AND l_qty > 5) \
         ORDER BY o_orderkey",
        )
        .unwrap();
    // Both orders have matching lineitems (order 1: l_qty 6 > 5; order 2: l_qty 7 > 5)
    assert_eq!(r.rows.len(), 2);
    assert_eq!(r.rows[0][0].to_string(), "1");
    assert_eq!(r.rows[1][0].to_string(), "2");
}

#[test]
fn correlated_exists_no_match() {
    let mut e = engine();
    e.execute("CREATE TABLE orders (o_orderkey INTEGER)")
        .unwrap();
    e.execute("CREATE TABLE lineitem (l_orderkey INTEGER, l_qty INTEGER)")
        .unwrap();
    e.execute("INSERT INTO orders VALUES (1)").unwrap();
    e.execute("INSERT INTO orders VALUES (2)").unwrap();
    e.execute("INSERT INTO lineitem VALUES (1, 5)").unwrap();
    e.execute("INSERT INTO lineitem VALUES (1, 3)").unwrap();
    let r = e
        .execute(
            "SELECT o_orderkey FROM orders \
         WHERE EXISTS (SELECT * FROM lineitem WHERE l_orderkey = orders.o_orderkey AND l_qty > 10) \
         ORDER BY o_orderkey",
        )
        .unwrap();
    // No lineitem has l_qty > 10, so no orders
    assert_eq!(r.rows.len(), 0);
}

#[test]
fn correlated_not_exists() {
    let mut e = engine();
    e.execute("CREATE TABLE orders (o_orderkey INTEGER)")
        .unwrap();
    e.execute("CREATE TABLE lineitem (l_orderkey INTEGER, l_qty INTEGER)")
        .unwrap();
    e.execute("INSERT INTO orders VALUES (1)").unwrap();
    e.execute("INSERT INTO orders VALUES (2)").unwrap();
    e.execute("INSERT INTO lineitem VALUES (1, 100)").unwrap();
    e.execute("INSERT INTO lineitem VALUES (1, 200)").unwrap();
    let r = e.execute(
        "SELECT o_orderkey FROM orders \
         WHERE NOT EXISTS (SELECT * FROM lineitem WHERE l_orderkey = orders.o_orderkey AND l_qty > 50) \
         ORDER BY o_orderkey",
    )
    .unwrap();
    // Order 1 has l_qty 100 > 50, so NOT EXISTS is false → excluded
    // Order 2 has no lineitem → NOT EXISTS is true → included
    assert_eq!(r.rows.len(), 1);
    assert_eq!(r.rows[0][0].to_string(), "2");
}

#[test]
fn correlated_exists_with_count() {
    // EXISTS in HAVING clause equivalent: count how many orders have any matching lineitem
    let mut e = engine();
    e.execute("CREATE TABLE orders (o_orderkey INTEGER)")
        .unwrap();
    e.execute("CREATE TABLE lineitem (l_orderkey INTEGER, l_qty INTEGER)")
        .unwrap();
    e.execute("INSERT INTO orders VALUES (1)").unwrap();
    e.execute("INSERT INTO orders VALUES (2)").unwrap();
    e.execute("INSERT INTO orders VALUES (3)").unwrap();
    e.execute("INSERT INTO lineitem VALUES (1, 10)").unwrap();
    e.execute("INSERT INTO lineitem VALUES (3, 20)").unwrap();
    let r = e.execute(
        "SELECT COUNT(*) FROM orders o \
         WHERE EXISTS (SELECT * FROM lineitem l WHERE l.l_orderkey = o.o_orderkey AND l.l_qty >= 10)",
    )
    .unwrap();
    // Orders 1 and 3 have matching lineitems → 2
    assert_eq!(r.rows[0][0].to_string(), "2");
}
