//! Operator-level regression tests for JOIN operations.
//!
//! Sprint 3 - protects against regression in the join executor and
//! projection layer. Specifically locks in:
//! - Multi-table JOIN column ordering (Issue #3277 Q03/Q10/Q18 symptom:
//!   sqlrustgo returns different column order than expected)
//! - 2-table, 3-table inner joins with WHERE filters
//! - Join with aggregate (TPC-H Q1/Q3 shape)

use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::{Arc, RwLock};

fn engine() -> ExecutionEngine<MemoryStorage> {
    ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())))
}

#[test]
fn join_two_tables_inner_basic() {
    let mut e = engine();
    e.execute("CREATE TABLE customers (id INTEGER, name TEXT)").unwrap();
    e.execute("CREATE TABLE orders (id INTEGER, cust_id INTEGER, amount REAL)").unwrap();
    e.execute("INSERT INTO customers VALUES (1, 'Alice')").unwrap();
    e.execute("INSERT INTO customers VALUES (2, 'Bob')").unwrap();
    e.execute("INSERT INTO orders VALUES (100, 1, 50.0)").unwrap();
    e.execute("INSERT INTO orders VALUES (101, 2, 75.0)").unwrap();
    e.execute("INSERT INTO orders VALUES (102, 3, 999.0)").unwrap();

    let r = e.execute("SELECT customers.name, orders.amount FROM customers, orders WHERE customers.id = orders.cust_id ORDER BY customers.name").unwrap();
    assert_eq!(r.rows.len(), 2);
    assert_eq!(r.rows[0][0].to_string(), "Alice");
    assert_eq!(r.rows[0][1].to_string(), "50");
    assert_eq!(r.rows[1][0].to_string(), "Bob");
    assert_eq!(r.rows[1][1].to_string(), "75");
}

#[test]
fn join_three_tables_preserves_projection_order() {
    let mut e = engine();
    e.execute("CREATE TABLE a (id INTEGER, x TEXT)").unwrap();
    e.execute("CREATE TABLE b (id INTEGER, a_id INTEGER, y TEXT)").unwrap();
    e.execute("CREATE TABLE c (id INTEGER, b_id INTEGER, z TEXT)").unwrap();
    e.execute("INSERT INTO a VALUES (1, 'a1')").unwrap();
    e.execute("INSERT INTO b VALUES (10, 1, 'b1')").unwrap();
    e.execute("INSERT INTO c VALUES (100, 10, 'c1')").unwrap();

    let r = e.execute("SELECT a.x, b.y, c.z FROM a, b, c WHERE a.id = b.a_id AND b.id = c.b_id").unwrap();
    assert_eq!(r.rows.len(), 1);
    let cols: Vec<String> = r.rows[0].iter().map(|v| v.to_string()).collect();
    assert_eq!(cols, vec!["a1", "b1", "c1"]);
}

#[test]
fn join_with_aggregate_group_by_preserves_order() {
    let mut e = engine();
    e.execute("CREATE TABLE dept (id INTEGER, name TEXT)").unwrap();
    e.execute("CREATE TABLE emp (id INTEGER, dept_id INTEGER, salary REAL)").unwrap();
    e.execute("INSERT INTO dept VALUES (1, 'A')").unwrap();
    e.execute("INSERT INTO dept VALUES (2, 'B')").unwrap();
    e.execute("INSERT INTO emp VALUES (1, 1, 100)").unwrap();
    e.execute("INSERT INTO emp VALUES (2, 1, 200)").unwrap();
    e.execute("INSERT INTO emp VALUES (3, 2, 50)").unwrap();

    let r = e.execute("SELECT dept.name, SUM(emp.salary) AS total FROM dept, emp WHERE dept.id = emp.dept_id GROUP BY dept.name ORDER BY dept.name").unwrap();
    assert_eq!(r.rows.len(), 2);
    let r0: Vec<String> = r.rows[0].iter().map(|v| v.to_string()).collect();
    let r1: Vec<String> = r.rows[1].iter().map(|v| v.to_string()).collect();
    assert_eq!(r0[0], "A");
    assert_eq!(r0[1], "300");
    assert_eq!(r1[0], "B");
    assert_eq!(r1[1], "50");
}

#[test]
fn tpch_q1_like_shape_sum_real_columns() {
    let mut e = engine();
    e.execute("CREATE TABLE lineitem (l_returnflag TEXT, l_linestatus TEXT, l_quantity INTEGER, l_extendedprice REAL, l_discount REAL, l_tax REAL, l_shipdate TEXT)").unwrap();
    e.execute("INSERT INTO lineitem VALUES ('A', 'F', 10, 100.5, 0.05, 0.02, '1998-09-01')").unwrap();
    e.execute("INSERT INTO lineitem VALUES ('A', 'F', 20, 200.5, 0.10, 0.02, '1998-09-15')").unwrap();
    e.execute("INSERT INTO lineitem VALUES ('A', 'O', 5, 50.5, 0.05, 0.02, '1998-08-15')").unwrap();

    let r = e.execute("SELECT l_returnflag, l_linestatus, SUM(l_quantity), SUM(l_extendedprice), SUM(l_extendedprice * (1 - l_discount)) FROM lineitem WHERE l_shipdate <= '1998-09-02' GROUP BY l_returnflag, l_linestatus ORDER BY l_returnflag, l_linestatus").unwrap();
    assert_eq!(r.rows.len(), 2);
    let af: Vec<String> = r.rows[0].iter().map(|v| v.to_string()).collect();
    assert_eq!(af[0], "A");
    assert_eq!(af[1], "F");
    assert_eq!(af[2], "10");
    let sum_ext: f64 = af[3].parse().unwrap();
    assert!((sum_ext - 100.5).abs() < 0.01, "SUM(l_extendedprice) AF: expected 100.5, got {sum_ext}");
    let sum_disc: f64 = af[4].parse().unwrap();
    assert!((sum_disc - 100.5 * 0.95).abs() < 0.01, "SUM(l_extendedprice*(1-disc)) AF: expected ~95.475, got {sum_disc}");
}
