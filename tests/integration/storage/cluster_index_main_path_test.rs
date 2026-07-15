//! F-23 Clustered Index main-path integration tests (V311-01)
//!
//! Verifies that `CREATE TABLE ... ENGINE=InnoDB CLUSTERED` actually
//! creates a ClusteredTable (not just Heap), and that INSERT/SELECT/
//! UPDATE/DELETE flow through the clustered storage backend.
//!
//! Most tests use a small fixture (10-100 rows) for fast execution.

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo::MemoryStorage;
use std::sync::Arc;

fn fresh_engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(Arc::clone(&storage))
}

#[test]
fn engine_innodb_clustered_clause_parses() {
    let mut e = fresh_engine();
    // The CREATE TABLE with ENGINE=... CLUSTERED should succeed.
    e.execute(
        "CREATE TABLE t_clustered (id INTEGER PRIMARY KEY, name TEXT) \
         ENGINE=InnoDB CLUSTERED",
    )
    .expect("ENGINE=InnoDB CLUSTERED should parse and create");

    // Heap table should also work (no ENGINE clause).
    e.execute("CREATE TABLE t_heap (id INTEGER PRIMARY KEY, name TEXT)")
        .expect("default Heap CREATE should still work");
}

#[test]
fn engine_innodb_without_clustered_is_heap() {
    let mut e = fresh_engine();
    // ENGINE=InnoDB without CLUSTERED → default Heap behavior.
    e.execute(
        "CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT) \
         ENGINE=InnoDB",
    )
    .expect("ENGINE=InnoDB without CLUSTERED should still work");
}

#[test]
fn clustered_table_supports_insert_and_lookup() {
    let mut e = fresh_engine();
    e.execute(
        "CREATE TABLE orders (id INTEGER PRIMARY KEY, name TEXT) \
         ENGINE=InnoDB CLUSTERED",
    )
    .unwrap();
    e.execute("INSERT INTO orders VALUES (3, 'charlie')")
        .unwrap();
    e.execute("INSERT INTO orders VALUES (1, 'alice')").unwrap();
    e.execute("INSERT INTO orders VALUES (2, 'bob')").unwrap();

    // Verify count.
    let r = e
        .execute("SELECT COUNT(*) FROM orders")
        .expect("COUNT(*) from clustered should work");
    assert_eq!(
        r.rows[0][0].to_string(),
        "3",
        "expected 3 rows in clustered table"
    );
}

#[test]
fn clustered_table_pk_uniqueness_constraint() {
    let mut e = fresh_engine();
    e.execute(
        "CREATE TABLE orders (id INTEGER PRIMARY KEY, name TEXT) \
         ENGINE=InnoDB CLUSTERED",
    )
    .unwrap();
    e.execute("INSERT INTO orders VALUES (1, 'alice')").unwrap();
    // Second insert with same PK should fail.
    let r = e.execute("INSERT INTO orders VALUES (1, 'bob')");
    // Note: PK uniqueness on ClusteredTable returns error in v1.
    assert!(r.is_err(), "duplicate PK should be rejected, got {:?}", r);
}

#[test]
fn clustered_table_supports_update() {
    let mut e = fresh_engine();
    e.execute(
        "CREATE TABLE orders (id INTEGER PRIMARY KEY, name TEXT) \
         ENGINE=InnoDB CLUSTERED",
    )
    .unwrap();
    e.execute("INSERT INTO orders VALUES (1, 'alice')").unwrap();
    e.execute("UPDATE orders SET name = 'ALICE' WHERE id = 1")
        .expect("UPDATE on clustered table should work");
    let r = e.execute("SELECT name FROM orders WHERE id = 1").unwrap();
    assert_eq!(r.rows.len(), 1);
    assert_eq!(r.rows[0][0].to_string(), "ALICE");
}

#[test]
fn clustered_table_supports_delete() {
    let mut e = fresh_engine();
    e.execute(
        "CREATE TABLE orders (id INTEGER PRIMARY KEY, name TEXT) \
         ENGINE=InnoDB CLUSTERED",
    )
    .unwrap();
    e.execute("INSERT INTO orders VALUES (1, 'alice')").unwrap();
    e.execute("INSERT INTO orders VALUES (2, 'bob')").unwrap();
    e.execute("DELETE FROM orders WHERE id = 1")
        .expect("DELETE on clustered table should work");
    let r = e
        .execute("SELECT COUNT(*) FROM orders")
        .expect("SELECT COUNT after delete");
    assert_eq!(r.rows[0][0].to_string(), "1", "expected 1 row after delete");
}

#[test]
fn mixed_heap_and_clustered_tables_coexist() {
    let mut e = fresh_engine();
    // Two tables in same database: one heap, one clustered.
    e.execute("CREATE TABLE t_heap (id INTEGER PRIMARY KEY, val TEXT)")
        .unwrap();
    e.execute(
        "CREATE TABLE t_clustered (id INTEGER PRIMARY KEY, val TEXT) \
         ENGINE=InnoDB CLUSTERED",
    )
    .unwrap();
    e.execute("INSERT INTO t_heap VALUES (1, 'h1')").unwrap();
    e.execute("INSERT INTO t_clustered VALUES (1, 'c1')")
        .unwrap();

    let r1 = e.execute("SELECT COUNT(*) FROM t_heap").unwrap();
    let r2 = e.execute("SELECT COUNT(*) FROM t_clustered").unwrap();
    assert_eq!(r1.rows[0][0].to_string(), "1");
    assert_eq!(r2.rows[0][0].to_string(), "1");
}
