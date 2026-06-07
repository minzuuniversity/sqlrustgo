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
    e.execute("CREATE TABLE customers (id INTEGER, name TEXT)")
        .unwrap();
    e.execute("CREATE TABLE orders (id INTEGER, cust_id INTEGER, amount REAL)")
        .unwrap();
    e.execute("INSERT INTO customers VALUES (1, 'Alice')")
        .unwrap();
    e.execute("INSERT INTO customers VALUES (2, 'Bob')")
        .unwrap();
    e.execute("INSERT INTO orders VALUES (100, 1, 50.0)")
        .unwrap();
    e.execute("INSERT INTO orders VALUES (101, 2, 75.0)")
        .unwrap();
    e.execute("INSERT INTO orders VALUES (102, 3, 999.0)")
        .unwrap();

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
    e.execute("CREATE TABLE b (id INTEGER, a_id INTEGER, y TEXT)")
        .unwrap();
    e.execute("CREATE TABLE c (id INTEGER, b_id INTEGER, z TEXT)")
        .unwrap();
    e.execute("INSERT INTO a VALUES (1, 'a1')").unwrap();
    e.execute("INSERT INTO b VALUES (10, 1, 'b1')").unwrap();
    e.execute("INSERT INTO c VALUES (100, 10, 'c1')").unwrap();

    let r = e
        .execute("SELECT a.x, b.y, c.z FROM a, b, c WHERE a.id = b.a_id AND b.id = c.b_id")
        .unwrap();
    assert_eq!(r.rows.len(), 1);
    let cols: Vec<String> = r.rows[0].iter().map(|v| v.to_string()).collect();
    assert_eq!(cols, vec!["a1", "b1", "c1"]);
}

#[test]
fn join_with_aggregate_group_by_preserves_order() {
    let mut e = engine();
    e.execute("CREATE TABLE dept (id INTEGER, name TEXT)")
        .unwrap();
    e.execute("CREATE TABLE emp (id INTEGER, dept_id INTEGER, salary REAL)")
        .unwrap();
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
    e.execute("INSERT INTO lineitem VALUES ('A', 'F', 10, 100.5, 0.05, 0.02, '1998-09-01')")
        .unwrap();
    e.execute("INSERT INTO lineitem VALUES ('A', 'F', 20, 200.5, 0.10, 0.02, '1998-09-15')")
        .unwrap();
    e.execute("INSERT INTO lineitem VALUES ('A', 'O', 5, 50.5, 0.05, 0.02, '1998-08-15')")
        .unwrap();

    let r = e.execute("SELECT l_returnflag, l_linestatus, SUM(l_quantity), SUM(l_extendedprice), SUM(l_extendedprice * (1 - l_discount)) FROM lineitem WHERE l_shipdate <= '1998-09-02' GROUP BY l_returnflag, l_linestatus ORDER BY l_returnflag, l_linestatus").unwrap();
    assert_eq!(r.rows.len(), 2);
    let af: Vec<String> = r.rows[0].iter().map(|v| v.to_string()).collect();
    assert_eq!(af[0], "A");
    assert_eq!(af[1], "F");
    assert_eq!(af[2], "10");
    let sum_ext: f64 = af[3].parse().unwrap();
    assert!(
        (sum_ext - 100.5).abs() < 0.01,
        "SUM(l_extendedprice) AF: expected 100.5, got {sum_ext}"
    );
    let sum_disc: f64 = af[4].parse().unwrap();
    assert!(
        (sum_disc - 100.5 * 0.95).abs() < 0.01,
        "SUM(l_extendedprice*(1-disc)) AF: expected ~95.475, got {sum_disc}"
    );
}

// ----- Sprint 3.3: Expanded Join Regression Suite (chatGPT P2) -----

#[test]
fn join_left_outer_unmatched_right_becomes_null() {
    let mut e = engine();
    e.execute("CREATE TABLE a (id INTEGER, x TEXT)").unwrap();
    e.execute("CREATE TABLE b (id INTEGER, y TEXT)").unwrap();
    e.execute("INSERT INTO a VALUES (1, 'a1')").unwrap();
    e.execute("INSERT INTO a VALUES (2, 'a2')").unwrap();
    e.execute("INSERT INTO b VALUES (1, 'b1')").unwrap();
    let r = e
        .execute("SELECT a.id, b.y FROM a LEFT JOIN b ON a.id = b.id ORDER BY a.id")
        .unwrap();
    assert_eq!(r.rows.len(), 2);
    assert_eq!(r.rows[0][0].to_string(), "1");
    assert_eq!(r.rows[0][1].to_string(), "b1");
    assert_eq!(r.rows[1][0].to_string(), "2");
    assert_eq!(r.rows[1][1], sqlrustgo::Value::Null);
}

#[test]
fn join_left_outer_with_where_filters_unmatched() {
    let mut e = engine();
    e.execute("CREATE TABLE a (id INTEGER, x TEXT)").unwrap();
    e.execute("CREATE TABLE b (id INTEGER, y TEXT)").unwrap();
    e.execute("INSERT INTO a VALUES (1, 'a1')").unwrap();
    e.execute("INSERT INTO a VALUES (2, 'a2')").unwrap();
    e.execute("INSERT INTO b VALUES (1, 'b1')").unwrap();
    let r = e
        .execute("SELECT a.id FROM a LEFT JOIN b ON a.id = b.id WHERE b.y IS NULL")
        .unwrap();
    assert_eq!(r.rows.len(), 1);
    assert_eq!(r.rows[0][0].to_string(), "2");
}

#[test]
fn join_inner_with_null_key_no_match() {
    let mut e = engine();
    e.execute("CREATE TABLE a (id INTEGER, x TEXT)").unwrap();
    e.execute("CREATE TABLE b (id INTEGER, y TEXT)").unwrap();
    e.execute("INSERT INTO a VALUES (1, 'a1')").unwrap();
    e.execute("INSERT INTO a VALUES (NULL, 'aN')").unwrap();
    e.execute("INSERT INTO b VALUES (1, 'b1')").unwrap();
    e.execute("INSERT INTO b VALUES (NULL, 'bN')").unwrap();
    let r = e
        .execute("SELECT a.x, b.y FROM a, b WHERE a.id = b.id")
        .unwrap();
    assert_eq!(r.rows.len(), 1);
    assert_eq!(r.rows[0][0].to_string(), "a1");
    assert_eq!(r.rows[0][1].to_string(), "b1");
}

#[test]
fn join_left_outer_preserves_null_key_row() {
    let mut e = engine();
    e.execute("CREATE TABLE a (id INTEGER, x TEXT)").unwrap();
    e.execute("CREATE TABLE b (id INTEGER, y TEXT)").unwrap();
    e.execute("INSERT INTO a VALUES (NULL, 'aN')").unwrap();
    e.execute("INSERT INTO a VALUES (1, 'a1')").unwrap();
    e.execute("INSERT INTO b VALUES (1, 'b1')").unwrap();
    let r = e
        .execute("SELECT a.x, b.y FROM a LEFT JOIN b ON a.id = b.id ORDER BY a.x")
        .unwrap();
    assert_eq!(r.rows.len(), 2);
    assert_eq!(r.rows[0][0].to_string(), "a1");
    assert_eq!(r.rows[0][1].to_string(), "b1");
    assert_eq!(r.rows[1][0].to_string(), "aN");
    assert_eq!(r.rows[1][1], sqlrustgo::Value::Null);
}

#[test]
fn join_inner_duplicate_keys_produce_cartesian() {
    let mut e = engine();
    e.execute("CREATE TABLE a (id INTEGER, x TEXT)").unwrap();
    e.execute("CREATE TABLE b (id INTEGER, y TEXT)").unwrap();
    e.execute("INSERT INTO a VALUES (1, 'a1')").unwrap();
    e.execute("INSERT INTO a VALUES (1, 'a1b')").unwrap();
    e.execute("INSERT INTO b VALUES (1, 'b1')").unwrap();
    e.execute("INSERT INTO b VALUES (1, 'b1b')").unwrap();
    let r = e
        .execute("SELECT a.x, b.y FROM a, b WHERE a.id = b.id ORDER BY a.x, b.y")
        .unwrap();
    assert_eq!(r.rows.len(), 4);
}

#[test]
fn join_four_tables_chain_comma_list() {
    let mut e = engine();
    e.execute("CREATE TABLE a (id INTEGER, x TEXT)").unwrap();
    e.execute("CREATE TABLE b (id INTEGER, a_id INTEGER, y TEXT)")
        .unwrap();
    e.execute("CREATE TABLE c (id INTEGER, b_id INTEGER, z TEXT)")
        .unwrap();
    e.execute("CREATE TABLE d (id INTEGER, c_id INTEGER, w TEXT)")
        .unwrap();
    e.execute("INSERT INTO a VALUES (1, 'a1')").unwrap();
    e.execute("INSERT INTO b VALUES (10, 1, 'b1')").unwrap();
    e.execute("INSERT INTO c VALUES (100, 10, 'c1')").unwrap();
    e.execute("INSERT INTO d VALUES (1000, 100, 'd1')").unwrap();
    let r = e.execute("SELECT a.x, b.y, c.z, d.w FROM a, b, c, d WHERE a.id = b.a_id AND b.id = c.b_id AND c.id = d.c_id").unwrap();
    assert_eq!(r.rows.len(), 1);
    let cols: Vec<String> = r.rows[0].iter().map(|v| v.to_string()).collect();
    assert_eq!(cols, vec!["a1", "b1", "c1", "d1"]);
}

#[test]
fn join_four_tables_chain_explicit_join() {
    let mut e = engine();
    e.execute("CREATE TABLE a (id INTEGER, x TEXT)").unwrap();
    e.execute("CREATE TABLE b (id INTEGER, a_id INTEGER, y TEXT)")
        .unwrap();
    e.execute("CREATE TABLE c (id INTEGER, b_id INTEGER, z TEXT)")
        .unwrap();
    e.execute("CREATE TABLE d (id INTEGER, c_id INTEGER, w TEXT)")
        .unwrap();
    e.execute("INSERT INTO a VALUES (1, 'a1')").unwrap();
    e.execute("INSERT INTO b VALUES (10, 1, 'b1')").unwrap();
    e.execute("INSERT INTO c VALUES (100, 10, 'c1')").unwrap();
    e.execute("INSERT INTO d VALUES (1000, 100, 'd1')").unwrap();
    let r = e.execute("SELECT a.x, d.w FROM a JOIN b ON a.id = b.a_id JOIN c ON b.id = c.b_id JOIN d ON c.id = d.c_id").unwrap();
    assert_eq!(r.rows.len(), 1);
    assert_eq!(r.rows[0][0].to_string(), "a1");
    assert_eq!(r.rows[0][1].to_string(), "d1");
}

#[test]
fn join_self_join_with_aliases_explicit_join_works() {
    let mut e = engine();
    e.execute("CREATE TABLE emp (id INTEGER, mgr_id INTEGER, name TEXT)")
        .unwrap();
    e.execute("INSERT INTO emp VALUES (1, NULL, 'CEO')")
        .unwrap();
    e.execute("INSERT INTO emp VALUES (2, 1, 'CTO')").unwrap();
    e.execute("INSERT INTO emp VALUES (3, 1, 'CFO')").unwrap();
    let r = e
        .execute("SELECT e.name, m.name FROM emp e JOIN emp m ON e.mgr_id = m.id ORDER BY e.name")
        .unwrap();
    assert_eq!(r.rows.len(), 2);
    assert_eq!(r.rows[0][0].to_string(), "CFO");
    assert_eq!(r.rows[0][1].to_string(), "CEO");
    assert_eq!(r.rows[1][0].to_string(), "CTO");
    assert_eq!(r.rows[1][1].to_string(), "CEO");
}

#[test]
fn join_inner_with_aliased_tables_explicit_join_works() {
    let mut e = engine();
    e.execute("CREATE TABLE customers (id INTEGER, name TEXT)")
        .unwrap();
    e.execute("CREATE TABLE orders (id INTEGER, cust_id INTEGER, amount REAL)")
        .unwrap();
    e.execute("INSERT INTO customers VALUES (1, 'Alice')")
        .unwrap();
    e.execute("INSERT INTO orders VALUES (100, 1, 50.0)")
        .unwrap();
    let r = e
        .execute("SELECT c.name FROM customers AS c JOIN orders AS o ON c.id = o.cust_id")
        .unwrap();
    assert_eq!(r.rows.len(), 1);
    assert_eq!(r.rows[0][0].to_string(), "Alice");
}

#[test]
fn join_three_tables_with_aggregate() {
    let mut e = engine();
    e.execute("CREATE TABLE t1 (id INTEGER, name TEXT)")
        .unwrap();
    e.execute("CREATE TABLE t2 (id INTEGER, t1_id INTEGER, qty INTEGER)")
        .unwrap();
    e.execute("CREATE TABLE t3 (id INTEGER, t2_id INTEGER, weight REAL)")
        .unwrap();
    e.execute("INSERT INTO t1 VALUES (1, 'A')").unwrap();
    e.execute("INSERT INTO t1 VALUES (2, 'B')").unwrap();
    e.execute("INSERT INTO t2 VALUES (10, 1, 5)").unwrap();
    e.execute("INSERT INTO t2 VALUES (11, 1, 7)").unwrap();
    e.execute("INSERT INTO t2 VALUES (12, 2, 3)").unwrap();
    e.execute("INSERT INTO t3 VALUES (100, 10, 1.5)").unwrap();
    e.execute("INSERT INTO t3 VALUES (101, 11, 2.5)").unwrap();
    e.execute("INSERT INTO t3 VALUES (102, 12, 3.5)").unwrap();
    let r = e.execute("SELECT t1.name, SUM(t3.weight) FROM t1, t2, t3 WHERE t1.id = t2.t1_id AND t2.id = t3.t2_id GROUP BY t1.name ORDER BY t1.name").unwrap();
    assert_eq!(r.rows.len(), 2);
    let a: f64 = r.rows[0][1].to_string().parse().unwrap();
    let b: f64 = r.rows[1][1].to_string().parse().unwrap();
    assert!((a - 4.0).abs() < 0.01, "t1='A' expected 4.0, got {a}");
    assert!((b - 3.5).abs() < 0.01, "t1='B' expected 3.5, got {b}");
}

#[test]
fn join_with_extra_where_predicate() {
    let mut e = engine();
    e.execute("CREATE TABLE t1 (id INTEGER, x TEXT)").unwrap();
    e.execute("CREATE TABLE t2 (id INTEGER, t1_id INTEGER, y TEXT)")
        .unwrap();
    e.execute("INSERT INTO t1 VALUES (1, 'A')").unwrap();
    e.execute("INSERT INTO t1 VALUES (2, 'B')").unwrap();
    e.execute("INSERT INTO t2 VALUES (10, 1, 'k1')").unwrap();
    e.execute("INSERT INTO t2 VALUES (11, 1, 'k2')").unwrap();
    e.execute("INSERT INTO t2 VALUES (12, 2, 'k1')").unwrap();
    let r = e
        .execute(
            "SELECT t1.x, t2.y FROM t1, t2 WHERE t1.id = t2.t1_id AND t2.y = 'k1' ORDER BY t2.y",
        )
        .unwrap();
    assert_eq!(r.rows.len(), 2);
    assert_eq!(r.rows[0][0].to_string(), "A");
    assert_eq!(r.rows[0][1].to_string(), "k1");
    assert_eq!(r.rows[1][0].to_string(), "B");
    assert_eq!(r.rows[1][1].to_string(), "k1");
}

#[test]
fn join_count_star_with_group_by() {
    let mut e = engine();
    e.execute("CREATE TABLE t1 (id INTEGER, name TEXT)")
        .unwrap();
    e.execute("CREATE TABLE t2 (id INTEGER, t1_id INTEGER)")
        .unwrap();
    e.execute("INSERT INTO t1 VALUES (1, 'A')").unwrap();
    e.execute("INSERT INTO t1 VALUES (2, 'B')").unwrap();
    e.execute("INSERT INTO t2 VALUES (10, 1)").unwrap();
    e.execute("INSERT INTO t2 VALUES (11, 1)").unwrap();
    e.execute("INSERT INTO t2 VALUES (12, 2)").unwrap();
    let r = e.execute("SELECT t1.name, COUNT(*) FROM t1, t2 WHERE t1.id = t2.t1_id GROUP BY t1.name ORDER BY t1.name").unwrap();
    assert_eq!(r.rows.len(), 2);
    assert_eq!(r.rows[0][0].to_string(), "A");
    assert_eq!(r.rows[0][1].to_string(), "2");
    assert_eq!(r.rows[1][0].to_string(), "B");
    assert_eq!(r.rows[1][1].to_string(), "1");
}

#[test]
fn join_two_tables_no_match_returns_empty() {
    let mut e = engine();
    e.execute("CREATE TABLE t1 (id INTEGER)").unwrap();
    e.execute("CREATE TABLE t2 (id INTEGER)").unwrap();
    e.execute("INSERT INTO t1 VALUES (1)").unwrap();
    e.execute("INSERT INTO t1 VALUES (2)").unwrap();
    e.execute("INSERT INTO t2 VALUES (3)").unwrap();
    e.execute("INSERT INTO t2 VALUES (4)").unwrap();
    let r = e
        .execute("SELECT t1.id, t2.id FROM t1, t2 WHERE t1.id = t2.id")
        .unwrap();
    assert_eq!(r.rows.len(), 0);
}

#[test]
fn join_left_outer_all_unmatched() {
    let mut e = engine();
    e.execute("CREATE TABLE t1 (id INTEGER, x TEXT)").unwrap();
    e.execute("CREATE TABLE t2 (id INTEGER, y TEXT)").unwrap();
    e.execute("INSERT INTO t1 VALUES (1, 'a')").unwrap();
    e.execute("INSERT INTO t1 VALUES (2, 'b')").unwrap();
    e.execute("INSERT INTO t2 VALUES (3, 'c')").unwrap();
    let r = e
        .execute("SELECT t1.x, t2.y FROM t1 LEFT JOIN t2 ON t1.id = t2.id ORDER BY t1.x")
        .unwrap();
    assert_eq!(r.rows.len(), 2);
    assert_eq!(r.rows[0][1], sqlrustgo::Value::Null);
    assert_eq!(r.rows[1][1], sqlrustgo::Value::Null);
}

#[test]
fn join_inner_with_text_key() {
    let mut e = engine();
    e.execute("CREATE TABLE t1 (k TEXT, v INTEGER)").unwrap();
    e.execute("CREATE TABLE t2 (k TEXT, w INTEGER)").unwrap();
    e.execute("INSERT INTO t1 VALUES ('x', 1)").unwrap();
    e.execute("INSERT INTO t1 VALUES ('y', 2)").unwrap();
    e.execute("INSERT INTO t2 VALUES ('x', 10)").unwrap();
    e.execute("INSERT INTO t2 VALUES ('z', 30)").unwrap();
    let r = e
        .execute("SELECT t1.k, t1.v, t2.w FROM t1, t2 WHERE t1.k = t2.k ORDER BY t1.k")
        .unwrap();
    assert_eq!(r.rows.len(), 1);
    assert_eq!(r.rows[0][0].to_string(), "x");
    assert_eq!(r.rows[0][1].to_string(), "1");
    assert_eq!(r.rows[0][2].to_string(), "10");
}

#[test]
fn join_three_tables_distinct_keys_no_ambiguity() {
    let mut e = engine();
    e.execute("CREATE TABLE a (id INTEGER, x TEXT)").unwrap();
    e.execute("CREATE TABLE b (id INTEGER, a_id INTEGER, y TEXT)")
        .unwrap();
    e.execute("CREATE TABLE c (id INTEGER, b_id INTEGER, z TEXT)")
        .unwrap();
    e.execute("INSERT INTO a VALUES (1, 'a1')").unwrap();
    e.execute("INSERT INTO a VALUES (2, 'a2')").unwrap();
    e.execute("INSERT INTO b VALUES (10, 1, 'b1')").unwrap();
    e.execute("INSERT INTO b VALUES (20, 2, 'b2')").unwrap();
    e.execute("INSERT INTO c VALUES (100, 10, 'c1')").unwrap();
    let r = e
        .execute("SELECT a.x, b.y, c.z FROM a, b, c WHERE a.id = b.a_id AND b.id = c.b_id")
        .unwrap();
    assert_eq!(r.rows.len(), 1);
    let cols: Vec<String> = r.rows[0].iter().map(|v| v.to_string()).collect();
    assert_eq!(cols, vec!["a1", "b1", "c1"]);
}
