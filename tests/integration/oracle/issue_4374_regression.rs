//! Focused regression coverage for V312-58 / Issue #4374.
//!
//! The fixtures are intentionally in-memory and dependency-free.  They assert
//! the observable correlated-scalar behavior without requiring a full TPC-H
//! dbgen dataset.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use std::sync::Arc;

fn fresh_engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

#[test]
fn q17_correlated_avg_filter_is_applied_before_aggregation() {
    let mut e = fresh_engine();
    e.execute("CREATE TABLE part (p_partkey INTEGER PRIMARY KEY, p_brand TEXT, p_container TEXT)")
        .unwrap();
    e.execute("CREATE TABLE lineitem (l_partkey INTEGER, l_quantity INTEGER, l_extendedprice REAL, l_shipdate TEXT)").unwrap();
    e.execute("INSERT INTO part VALUES (1, 'Brand#23', 'LG CASE')")
        .unwrap();
    e.execute("INSERT INTO lineitem VALUES (1, 1, 10.0, '1994-01-01')")
        .unwrap();
    e.execute("INSERT INTO lineitem VALUES (1, 2, 20.0, '1994-01-01')")
        .unwrap();
    e.execute("INSERT INTO lineitem VALUES (1, 100, 100.0, '1994-01-01')")
        .unwrap();
    e.execute("INSERT INTO lineitem VALUES (1, 200, 200.0, '1994-01-01')")
        .unwrap();

    let r = e.execute("SELECT SUM(l_extendedprice) / 7.0 AS avg_yearly FROM lineitem, part WHERE p_partkey = l_partkey AND p_brand = 'Brand#23' AND p_container = 'LG CASE' AND l_quantity < (SELECT 0.2 * AVG(l_quantity) FROM lineitem WHERE l_partkey = p_partkey)").unwrap();
    assert_eq!(r.rows.len(), 1);
    let got = match r.rows[0][0] {
        Value::Float(v) => v,
        ref other => panic!("expected Float, got {:?}", other),
    };
    let expected = (10.0 + 20.0) / 7.0;
    assert!(
        (got - expected).abs() < 1e-9,
        "got {}, expected {}",
        got,
        expected
    );
}

#[test]
fn q20_scalar_sum_respects_supplier_and_date_filters() {
    let mut e = fresh_engine();
    e.execute(
        "CREATE TABLE partsupp (ps_partkey INTEGER, ps_suppkey INTEGER, ps_availqty INTEGER)",
    )
    .unwrap();
    e.execute("INSERT INTO partsupp VALUES (10, 1, 1000)")
        .unwrap();
    e.execute("INSERT INTO partsupp VALUES (10, 2, 50)")
        .unwrap();
    e.execute("CREATE TABLE lineitem (l_partkey INTEGER, l_suppkey INTEGER, l_quantity INTEGER, l_shipdate TEXT)").unwrap();
    for (suppkey, quantity) in [(1, 10), (1, 20), (1, 100), (2, 10), (2, 20), (2, 100)] {
        e.execute(&format!(
            "INSERT INTO lineitem VALUES (10, {suppkey}, {quantity}, '1994-01-01')"
        ))
        .unwrap();
    }
    e.execute("INSERT INTO lineitem VALUES (10, 1, 1000000, '1995-01-01')")
        .unwrap();
    e.execute("INSERT INTO lineitem VALUES (10, 2, 1000000, '1995-01-01')")
        .unwrap();
    let r = e.execute("SELECT ps_partkey, ps_suppkey FROM partsupp WHERE ps_partkey = 10 AND ps_availqty > (SELECT 0.5 * SUM(l_quantity) FROM lineitem WHERE l_partkey = ps_partkey AND l_suppkey = ps_suppkey AND l_shipdate >= '1994-01-01' AND l_shipdate < '1995-01-01') ORDER BY ps_suppkey").unwrap();
    assert_eq!(r.rows, vec![vec![Value::Integer(10), Value::Integer(1)]]);
}
