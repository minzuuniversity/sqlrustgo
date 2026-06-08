//! Operator-level regression tests for EXISTS / NOT EXISTS subqueries.
//!
//! Sprint 3 - protects against regression in the subquery executor.
//! Specifically locks in:
//! - Correlated EXISTS with outer column substitution (Issue #3248)
//! - NOT EXISTS semantics
//! - 0-row subquery handling

use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::{Arc, RwLock};

fn engine() -> ExecutionEngine<MemoryStorage> {
    ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())))
}

#[test]
fn exists_simple_no_correlation() {
    let mut e = engine();
    e.execute("CREATE TABLE tbl_outer (id INTEGER)").unwrap();
    e.execute("CREATE TABLE tbl_inner (id INTEGER)").unwrap();
    e.execute("INSERT INTO tbl_outer VALUES (1)").unwrap();
    e.execute("INSERT INTO tbl_outer VALUES (2)").unwrap();
    e.execute("INSERT INTO tbl_outer VALUES (3)").unwrap();
    e.execute("INSERT INTO tbl_inner VALUES (10)").unwrap();
    e.execute("INSERT INTO tbl_inner VALUES (20)").unwrap();
    e.execute("INSERT INTO tbl_inner VALUES (30)").unwrap();

    let r = e.execute("SELECT id FROM tbl_outer o WHERE EXISTS (SELECT 1 FROM tbl_inner WHERE tbl_inner.id = id) ORDER BY id").unwrap();
    assert_eq!(r.rows.len(), 0, "no outer.id matches inner.id, so EXISTS should yield 0 rows");
}

#[test]
fn exists_simple_with_match() {
    let mut e = engine();
    e.execute("CREATE TABLE tbl_outer (id INTEGER)").unwrap();
    e.execute("CREATE TABLE tbl_inner (id INTEGER)").unwrap();
    e.execute("INSERT INTO tbl_outer VALUES (1)").unwrap();
    e.execute("INSERT INTO tbl_outer VALUES (2)").unwrap();
    e.execute("INSERT INTO tbl_inner VALUES (2)").unwrap();
    e.execute("INSERT INTO tbl_inner VALUES (3)").unwrap();

    let r = e.execute("SELECT id FROM tbl_outer o WHERE EXISTS (SELECT 1 FROM tbl_inner WHERE tbl_inner.id = id) ORDER BY id").unwrap();
    assert_eq!(r.rows.len(), 1);
    assert_eq!(r.rows[0][0].to_string(), "2", "only outer.id=2 matches inner");
}

#[test]
fn exists_correlated_outer_column_substitution() {
    let mut e = engine();
    e.execute("CREATE TABLE supplier (s_suppkey INTEGER, s_name TEXT)").unwrap();
    e.execute("CREATE TABLE partsupp (ps_suppkey INTEGER)").unwrap();
    e.execute("INSERT INTO supplier VALUES (1, 'S1')").unwrap();
    e.execute("INSERT INTO supplier VALUES (2, 'S2')").unwrap();
    e.execute("INSERT INTO supplier VALUES (3, 'S3')").unwrap();
    e.execute("INSERT INTO partsupp VALUES (1)").unwrap();
    e.execute("INSERT INTO partsupp VALUES (3)").unwrap();

    let r = e.execute("SELECT s_name FROM supplier s WHERE EXISTS (SELECT * FROM partsupp WHERE ps_suppkey = s_suppkey) ORDER BY s_suppkey").unwrap();
    assert_eq!(r.rows.len(), 2, "S1 and S3 have parts; S2 does not");
    let names: Vec<String> = r.rows.iter().map(|row| row[0].to_string()).collect();
    assert_eq!(names, vec!["S1", "S3"]);
}

#[test]
fn not_exists_subquery() {
    let mut e = engine();
    e.execute("CREATE TABLE customers (id INTEGER, name TEXT)").unwrap();
    e.execute("CREATE TABLE blocked (cust_id INTEGER)").unwrap();
    e.execute("INSERT INTO customers VALUES (1, 'A')").unwrap();
    e.execute("INSERT INTO customers VALUES (2, 'B')").unwrap();
    e.execute("INSERT INTO customers VALUES (3, 'C')").unwrap();
    e.execute("INSERT INTO blocked VALUES (2)").unwrap();

    let r = e.execute("SELECT name FROM customers c WHERE NOT EXISTS (SELECT 1 FROM blocked WHERE cust_id = id) ORDER BY name").unwrap();
    assert_eq!(r.rows.len(), 2, "A and C are not blocked; B is blocked");
    let names: Vec<String> = r.rows.iter().map(|row| row[0].to_string()).collect();
    assert_eq!(names, vec!["A", "C"]);
}
