//! Operator-level regression test for 3-table JOIN with ON conditions.
//!
//! Sprint 3 (Operator Regression Suite) / Issue #3283 Task 2.
//!
//! Locks in the multi-table JOIN executor (Sprint 4 / Issue #3286):
//! - 3-table JOIN (e.g. customer, orders, lineitem) with proper ON
//!   conditions should produce correct joined rows
//! - GROUP BY + ORDER BY on the joined result should respect the
//!   joined schema, not the inner-table schema
//!
//! Acceptance (per Issue #3283):
//! - Use a minimal fixture (3-5 rows)
//! - Assert cell values, not just row count
//! - Run in < 100ms
//! - Be reproducible

use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::{Arc, RwLock};

fn engine() -> ExecutionEngine<MemoryStorage> {
    ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())))
}

#[test]
fn three_table_join_preserves_all_tables() {
    // 3 customers, 3 orders, 3 lineitems, all linked 1:1:1.
    let mut e = engine();
    e.execute("CREATE TABLE customer (c_custkey INTEGER, c_name TEXT)").unwrap();
    e.execute("CREATE TABLE orders (o_orderkey INTEGER, o_custkey INTEGER)").unwrap();
    e.execute("CREATE TABLE lineitem (l_orderkey INTEGER, l_qty INTEGER)").unwrap();
    e.execute("INSERT INTO customer VALUES (1, 'Alice')").unwrap();
    e.execute("INSERT INTO customer VALUES (2, 'Bob')").unwrap();
    e.execute("INSERT INTO customer VALUES (3, 'Carol')").unwrap();
    e.execute("INSERT INTO orders VALUES (100, 1)").unwrap();
    e.execute("INSERT INTO orders VALUES (101, 2)").unwrap();
    e.execute("INSERT INTO orders VALUES (102, 3)").unwrap();
    e.execute("INSERT INTO lineitem VALUES (100, 5)").unwrap();
    e.execute("INSERT INTO lineitem VALUES (101, 7)").unwrap();
    e.execute("INSERT INTO lineitem VALUES (102, 9)").unwrap();
    let r = e.execute(
        "SELECT c_name, l_qty FROM customer, orders, lineitem \
         WHERE c_custkey = o_custkey AND o_orderkey = l_orderkey \
         ORDER BY c_name",
    )
    .unwrap();
    assert_eq!(r.rows.len(), 3, "expected 3 rows from full 3-table JOIN");
    assert_eq!(r.rows[0][0].to_string(), "Alice");
    assert_eq!(r.rows[0][1].to_string(), "5");
    assert_eq!(r.rows[1][0].to_string(), "Bob");
    assert_eq!(r.rows[1][1].to_string(), "7");
    assert_eq!(r.rows[2][0].to_string(), "Carol");
    assert_eq!(r.rows[2][1].to_string(), "9");
}

#[test]
fn three_table_join_with_group_by_and_sum() {
    // GROUP BY on the joined result, SUM on a joined column.
    // Locks in the multi-JOIN data-corruption issue (Sprint 4).
    let mut e = engine();
    e.execute("CREATE TABLE customer (c_custkey INTEGER, c_name TEXT)").unwrap();
    e.execute("CREATE TABLE orders (o_orderkey INTEGER, o_custkey INTEGER)").unwrap();
    e.execute("CREATE TABLE lineitem (l_orderkey INTEGER, l_qty INTEGER)").unwrap();
    // cust 1 has 2 orders, cust 2 has 1.
    e.execute("INSERT INTO customer VALUES (1, 'Alice')").unwrap();
    e.execute("INSERT INTO customer VALUES (2, 'Bob')").unwrap();
    e.execute("INSERT INTO orders VALUES (100, 1)").unwrap();
    e.execute("INSERT INTO orders VALUES (101, 1)").unwrap();
    e.execute("INSERT INTO orders VALUES (200, 2)").unwrap();
    e.execute("INSERT INTO lineitem VALUES (100, 3)").unwrap();
    e.execute("INSERT INTO lineitem VALUES (101, 4)").unwrap();
    e.execute("INSERT INTO lineitem VALUES (200, 5)").unwrap();
    let r = e.execute(
        "SELECT c_name, SUM(l_qty) AS total \
         FROM customer, orders, lineitem \
         WHERE c_custkey = o_custkey AND o_orderkey = l_orderkey \
         GROUP BY c_name ORDER BY c_name",
    )
    .unwrap();
    assert_eq!(r.rows.len(), 2);
    // Alice: 3 + 4 = 7
    assert_eq!(r.rows[0][0].to_string(), "Alice");
    assert_eq!(r.rows[0][1].to_string(), "7");
    // Bob: 5
    assert_eq!(r.rows[1][0].to_string(), "Bob");
    assert_eq!(r.rows[1][1].to_string(), "5");
}

#[test]
fn three_table_join_with_where_filter() {
    // WHERE filter on a joined table column should be applied
    // correctly (Sprint 4 multi-JOIN data corruption).
    let mut e = engine();
    e.execute("CREATE TABLE customer (c_custkey INTEGER, c_name TEXT)").unwrap();
    e.execute("CREATE TABLE orders (o_orderkey INTEGER, o_custkey INTEGER)").unwrap();
    e.execute("CREATE TABLE lineitem (l_orderkey INTEGER, l_qty INTEGER)").unwrap();
    e.execute("INSERT INTO customer VALUES (1, 'Alice')").unwrap();
    e.execute("INSERT INTO customer VALUES (2, 'Bob')").unwrap();
    e.execute("INSERT INTO orders VALUES (100, 1)").unwrap();
    e.execute("INSERT INTO orders VALUES (200, 2)").unwrap();
    e.execute("INSERT INTO lineitem VALUES (100, 5)").unwrap();
    e.execute("INSERT INTO lineitem VALUES (200, 7)").unwrap();
    // Filter: l_qty > 6 → only Bob (7)
    let r = e.execute(
        "SELECT c_name FROM customer, orders, lineitem \
         WHERE c_custkey = o_custkey AND o_orderkey = l_orderkey \
           AND l_qty > 6 ORDER BY c_name",
    )
    .unwrap();
    assert_eq!(r.rows.len(), 1);
    assert_eq!(r.rows[0][0].to_string(), "Bob");
}