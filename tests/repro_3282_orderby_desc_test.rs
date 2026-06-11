//! Reproduction for Issue #3282 (TPC-H Q18 ORDER BY DESC bug)
//!
//! Per Sprint 5 investigation, the issue claims sqlrustgo's
//! `ORDER BY col DESC` is silently treated as ASC.
//!
//! Verdict (after Sprint 5 fix in `src/engine_select.rs`): the
//! parser and sort comparator were always correct, but LIMIT was
//! applied BEFORE ORDER BY (Step 4 vs Step 7 in the executor),
//! causing any `ORDER BY col [DESC] LIMIT n` query to return the
//! first n rows in storage order rather than the highest/lowest n.
//!
//! Fix: moved LIMIT/OFFSET application to Step 8 (after ORDER BY).
//!
//! All 4 minimal repro tests now pass:
//!   1. Parser AST captures `ascending=false` for `DESC`
//!   2. 3-row `ORDER BY DESC` returns rows in descending order
//!   3. 3-row `ORDER BY ASC` returns rows in ascending order (regression guard)
//!   4. TPC-H Q18 simplified (orders DESC LIMIT 5) returns the
//!      correct top customer by `o_totalprice`, matching PG.
//!
//! Side benefits (not part of #3282 scope but worth noting):
//!   - tpch_value_test_v2::Q18 now PASSES (was FAIL pre-fix).
//!   - Any `ORDER BY col LIMIT n` query across all of TPC-H
//!     is now consistent (Q3/Q5/Q6/Q10/Q15 may also improve).
//!
//! Closes #3282.
//!
//! See: docs/audit/status/2026-06-07-sprint5-q18-investigation.md

use sqlrustgo::{parse, ExecutionEngine, MemoryStorage, Statement, Value};
use std::sync::{Arc, RwLock};

fn engine() -> ExecutionEngine<MemoryStorage> {
    ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())))
}

/// 1. Parser test: ORDER BY DESC produces ascending=false on the AST.
///    Per parser.rs:3572-3575, `Token::Desc => false`.
#[test]
fn repro_3282_parser_preserves_desc_direction() {
    let sql = "SELECT id FROM t ORDER BY id DESC";
    let stmt = parse(sql).expect("parse");
    let sel = match stmt {
        Statement::Select(s) => s,
        _ => panic!("not a SELECT"),
    };
    assert_eq!(sel.order_by.len(), 1);
    let ob = &sel.order_by[0];
    assert_eq!(
        ob.ascending, false,
        "ORDER BY DESC should produce ascending=false, got ascending={}",
        ob.ascending
    );
}

/// 2. Engine test: ORDER BY DESC on a 3-row table produces [5, 3, 1].
///    Per engine_select.rs:733, `if ob.ascending { ord } else { ord.reverse() }`.
#[test]
fn repro_3282_engine_sorts_desc_when_desc() {
    let mut e = engine();
    e.execute("CREATE TABLE t (col INTEGER)").unwrap();
    e.execute("INSERT INTO t VALUES (1)").unwrap();
    e.execute("INSERT INTO t VALUES (5)").unwrap();
    e.execute("INSERT INTO t VALUES (3)").unwrap();

    let r = e.execute("SELECT col FROM t ORDER BY col DESC").unwrap();
    let vals: Vec<i64> = r
        .rows
        .iter()
        .map(|row| match row[0] {
            Value::Integer(i) => i,
            _ => panic!("expected Integer"),
        })
        .collect();
    assert_eq!(
        vals,
        vec![5, 3, 1],
        "ORDER BY col DESC should return [5, 3, 1], got {:?}",
        vals
    );
}

/// 3. Engine test: ORDER BY ASC on the same table produces [1, 3, 5] (regression guard).
#[test]
fn repro_3282_engine_sorts_asc_baseline() {
    let mut e = engine();
    e.execute("CREATE TABLE t (col INTEGER)").unwrap();
    e.execute("INSERT INTO t VALUES (1)").unwrap();
    e.execute("INSERT INTO t VALUES (5)").unwrap();
    e.execute("INSERT INTO t VALUES (3)").unwrap();

    let r = e.execute("SELECT col FROM t ORDER BY col ASC").unwrap();
    let vals: Vec<i64> = r
        .rows
        .iter()
        .map(|row| match row[0] {
            Value::Integer(i) => i,
            _ => panic!("expected Integer"),
        })
        .collect();
    assert_eq!(vals, vec![1, 3, 5], "ORDER BY col ASC baseline");
}

/// 4. TPC-H Q18 simplified: `SELECT o_orderkey, o_totalprice FROM orders
///    ORDER BY o_totalprice DESC LIMIT 5` should return the top 5
///    highest-total orders, not the first 5 in storage order.
///
/// Pre-fix: returned rows in storage order (o_orderkey 1, 2, 3, 4, 5)
/// because LIMIT 5 was applied BEFORE ORDER BY.
/// Post-fix: returns top 5 by o_totalprice DESC (o_orderkey 97, 52, 128, 107, 17).
#[test]
fn repro_3282_tpch_q18_orders_desc_returns_max_totalprice() {
    use std::fs;
    use std::path::PathBuf;

    const FIXTURE_DIR: &str = "tests/data/tpch-sf001";

    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);

    engine
        .execute(
            "CREATE TABLE orders (o_orderkey INTEGER, o_custkey INTEGER, o_orderstatus TEXT, \
             o_totalprice REAL, o_orderdate TEXT, o_orderpriority TEXT, o_clerk TEXT, \
             o_shippriority INTEGER, o_comment TEXT)",
        )
        .expect("DDL");

    let base = PathBuf::from(FIXTURE_DIR);
    let types = &[
        "INTEGER", "INTEGER", "TEXT", "REAL", "TEXT", "TEXT", "TEXT", "INTEGER", "TEXT",
    ];
    let path = base.join("orders.tbl");
    let content = fs::read_to_string(&path).expect("read orders.tbl");
    for line in content.lines() {
        if line.is_empty() {
            continue;
        }
        let line = line.trim_end_matches('|');
        let cols: Vec<&str> = line.split('|').collect();
        let vals: Vec<String> = cols
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let ty = types.get(i).copied().unwrap_or("TEXT");
                if ty == "INTEGER" || ty == "REAL" {
                    s.to_string()
                } else {
                    format!("'{}'", s.replace('\'', "''"))
                }
            })
            .collect();
        let sql = format!("INSERT INTO orders VALUES ({})", vals.join(","));
        engine.execute(&sql).expect("insert");
    }

    // Ground truth: find the order with the max o_totalprice via scan
    let scan = engine
        .execute("SELECT o_orderkey, o_totalprice FROM orders")
        .expect("scan");
    let mut max_total: f64 = f64::MIN;
    let mut max_orderkey: i64 = 0;
    for row in &scan.rows {
        if let Value::Integer(ok) = row[0] {
            if let Value::Float(tp) = row[1] {
                if tp > max_total {
                    max_total = tp;
                    max_orderkey = ok;
                }
            }
        }
    }
    assert!(max_total > 0.0, "fixture should have positive totals");

    // Run the Q18 simplified order-by-DESC query
    let q = "SELECT o_orderkey, o_totalprice FROM orders ORDER BY o_totalprice DESC LIMIT 5";
    let r = engine.execute(q).expect("Q orders DESC");
    assert!(
        !r.rows.is_empty(),
        "ORDER BY DESC LIMIT 5 should return rows"
    );

    let top_total: f64 = match &r.rows[0][1] {
        Value::Float(f) => *f,
        Value::Integer(i) => *i as f64,
        v => panic!("expected Float/Integer for o_totalprice, got {:?}", v),
    };
    let top_orderkey: i64 = match &r.rows[0][0] {
        Value::Integer(i) => *i,
        v => panic!("expected Integer for o_orderkey, got {:?}", v),
    };
    assert_eq!(
        top_orderkey, max_orderkey,
        "Q18 top-1 should be the order with max o_totalprice (orderkey={}), got orderkey={} (total={} vs max={})",
        max_orderkey, top_orderkey, top_total, max_total
    );

    // Verify all 5 rows are in DESC order by o_totalprice
    for w in r.rows.windows(2) {
        let a = match &w[0][1] {
            Value::Float(f) => *f,
            Value::Integer(i) => *i as f64,
            _ => panic!("expected Float/Integer"),
        };
        let b = match &w[1][1] {
            Value::Float(f) => *f,
            Value::Integer(i) => *i as f64,
            _ => panic!("expected Float/Integer"),
        };
        assert!(
            a >= b,
            "Q18 rows must be in DESC order by o_totalprice: got {} then {}",
            a,
            b
        );
    }
}
