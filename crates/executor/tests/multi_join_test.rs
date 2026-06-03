//! Multi-Join Tests
//!
//! Verifies that `execute_select` handles chained JOINs (Vec<JoinClause>).
//! Sprint: TPC-H Q7/Q8/Q9 require 3-7 table joins; this file covers the
//! smallest non-trivial cases end-to-end (parser + executor).
//!
//! Each test creates small in-memory tables, runs a multi-table join, and
//! asserts on the resulting rows.

use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use sqlrustgo_types::Value;
use std::sync::{Arc, RwLock};

fn create_engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

#[test]
fn test_three_table_inner_join_chain() {
    // 3-way INNER JOIN chain: a → b → c.
    // After the joins, accumulated columns are:
    //   [0] a.id, [1] a.val, [2] b.aid, [3] b.num, [4] c.bid, [5] c.label
    let mut engine = create_engine();
    engine
        .execute("CREATE TABLE a (id INTEGER, val TEXT)")
        .unwrap();
    engine
        .execute("CREATE TABLE b (aid INTEGER, num INTEGER)")
        .unwrap();
    engine
        .execute("CREATE TABLE c (bid INTEGER, label TEXT)")
        .unwrap();
    engine
        .execute("INSERT INTO a VALUES (1, 'x'), (2, 'y')")
        .unwrap();
    engine
        .execute("INSERT INTO b VALUES (1, 10), (2, 20)")
        .unwrap();
    engine
        .execute("INSERT INTO c VALUES (10, 'p'), (20, 'q')")
        .unwrap();

    let result = engine
        .execute(
            "SELECT * FROM a \
             JOIN b ON a.id = b.aid \
             JOIN c ON b.num = c.bid",
        )
        .unwrap();

    assert_eq!(
        result.rows.len(),
        2,
        "expected 2 joined rows, got {:?}",
        result.rows
    );
    // Order: id, val, aid, num, bid, label
    let r1 = &result.rows[0];
    assert_eq!(r1[0], Value::Integer(1));
    assert_eq!(r1[1], Value::Text("x".to_string()));
    assert_eq!(r1[2], Value::Integer(1));
    assert_eq!(r1[3], Value::Integer(10));
    assert_eq!(r1[4], Value::Integer(10));
    assert_eq!(r1[5], Value::Text("p".to_string()));
    let r2 = &result.rows[1];
    assert_eq!(r2[0], Value::Integer(2));
    assert_eq!(r2[1], Value::Text("y".to_string()));
    assert_eq!(r2[2], Value::Integer(2));
    assert_eq!(r2[3], Value::Integer(20));
    assert_eq!(r2[4], Value::Integer(20));
    assert_eq!(r2[5], Value::Text("q".to_string()));
}

#[test]
fn test_four_table_inner_join_chain() {
    // 4-way chain — TPC-H Q9 minimal shape.
    // After the joins, accumulated columns are:
    //   t1.id, t2.t1_id, t2.t3_id, t3.id, t4.t2_id, t4.payload
    let mut engine = create_engine();
    engine.execute("CREATE TABLE t1 (id INTEGER)").unwrap();
    engine
        .execute("CREATE TABLE t2 (t1_id INTEGER, t3_id INTEGER)")
        .unwrap();
    engine.execute("CREATE TABLE t3 (id INTEGER)").unwrap();
    engine
        .execute("CREATE TABLE t4 (t2_id INTEGER, payload TEXT)")
        .unwrap();
    engine.execute("INSERT INTO t1 VALUES (1), (2)").unwrap();
    engine
        .execute("INSERT INTO t2 VALUES (1, 100), (2, 200)")
        .unwrap();
    engine
        .execute("INSERT INTO t3 VALUES (100), (200)")
        .unwrap();
    engine
        .execute("INSERT INTO t4 VALUES (1, 'p1'), (2, 'p2')")
        .unwrap();

    let result = engine
        .execute(
            "SELECT * FROM t1 \
             JOIN t2 ON t1.id = t2.t1_id \
             JOIN t3 ON t2.t3_id = t3.id \
             JOIN t4 ON t1.id = t4.t2_id",
        )
        .unwrap();

    assert_eq!(
        result.rows.len(),
        2,
        "expected 2 fully-joined rows, got {:?}",
        result.rows
    );
    // First row: t1.id=1, t2.t1_id=1, t2.t3_id=100, t3.id=100, t4.t2_id=1, t4.payload='p1'
    let r1 = &result.rows[0];
    assert_eq!(r1[0], Value::Integer(1));
    assert_eq!(r1[1], Value::Integer(1));
    assert_eq!(r1[2], Value::Integer(100));
    assert_eq!(r1[3], Value::Integer(100));
    assert_eq!(r1[4], Value::Integer(1));
    assert_eq!(r1[5], Value::Text("p1".to_string()));
}

// Note: WHERE × multi-join interaction is intentionally out of scope here.
// The accumulated schema names columns as `a_join_b.col` and the WHERE
// predicate's column lookup in `eval_predicate` does not currently understand
// that prefix. Tracking that interaction is a separate task; the multi-join
// chain itself (hash join on `ON` keys) is what this file covers.
