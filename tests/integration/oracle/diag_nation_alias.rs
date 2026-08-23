//! V312-58 / Issue #4376 — root-cause v5 hypothesis: duplicate-table-alias
//! (n1, n2) self-join collapses and breaks WHERE filtering.
//!
//! Construct 5-nation × 3-year fixture, run Q7 with `n1.n_name='GERMANY'
//! AND n2.n_name='FRANCE'`, count rows:
//!
//! - 3 rows → both n1, n2 filters work; n1, n2 are distinct table refs
//! - 75 rows → neither filter works; n1 and n2 collapse to same table
//! - 15 rows → only n1 filter applies (n2 conflated with n1)
//!
//! Run:
//!   cargo test --test diag_nation_alias --all-features -- --nocapture

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine};
use std::sync::Arc;

fn build_mini_fixture() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());

    for s in [
        "CREATE TABLE region (r_regionkey INTEGER PRIMARY KEY, r_name TEXT NOT NULL, r_comment TEXT)",
        "CREATE TABLE nation (n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, n_regionkey INTEGER NOT NULL, n_comment TEXT)",
        "CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT)",
        "CREATE TABLE customer (c_custkey INTEGER PRIMARY KEY, c_name TEXT NOT NULL, c_address TEXT NOT NULL, c_nationkey INTEGER NOT NULL, c_phone TEXT NOT NULL, c_acctbal REAL NOT NULL, c_mktsegment TEXT NOT NULL, c_comment TEXT)",
        "CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, o_orderstatus TEXT NOT NULL, o_totalprice REAL NOT NULL, o_orderdate TEXT NOT NULL, o_orderpriority TEXT NOT NULL, o_clerk TEXT NOT NULL, o_shippriority INTEGER NOT NULL, o_comment TEXT NOT NULL)",
        "CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity INTEGER NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT NOT NULL, l_linestatus TEXT NOT NULL, l_shipdate TEXT NOT NULL, l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT NOT NULL, l_shipmode TEXT NOT NULL, l_comment TEXT NOT NULL)",
    ] {
        engine.execute(s).unwrap();
    }

    let mut st = storage.write();
    let _ = st.insert(
        "region",
        vec![vec![
            sqlrustgo::Value::Integer(1),
            sqlrustgo::Value::Text("EUROPE".into()),
            sqlrustgo::Value::Text("".into()),
        ]],
    );
    // 5 nations
    for (k, n) in [
        (0, "GERMANY"),
        (1, "FRANCE"),
        (2, "UK"),
        (3, "USA"),
        (4, "JAPAN"),
    ] {
        let _ = st.insert(
            "nation",
            vec![vec![
                sqlrustgo::Value::Integer(k),
                sqlrustgo::Value::Text(n.into()),
                sqlrustgo::Value::Integer(1),
                sqlrustgo::Value::Text("".into()),
            ]],
        );
    }
    // 5 suppliers (one per nation) — supplier 0 (GERMANY) for ALL lineitem
    for k in 0..5 {
        let _ = st.insert(
            "supplier",
            vec![vec![
                sqlrustgo::Value::Integer(100 + k),
                sqlrustgo::Value::Text(format!("S#{}", 100 + k)),
                sqlrustgo::Value::Text("addr".into()),
                sqlrustgo::Value::Integer(k),
                sqlrustgo::Value::Text("phone".into()),
                sqlrustgo::Value::Float(100.0),
                sqlrustgo::Value::Text("".into()),
            ]],
        );
    }
    // 5 customers (one per nation) — used by orders
    for k in 0..5 {
        let _ = st.insert(
            "customer",
            vec![vec![
                sqlrustgo::Value::Integer(200 + k),
                sqlrustgo::Value::Text(format!("C#{}", 200 + k)),
                sqlrustgo::Value::Text("addr".into()),
                sqlrustgo::Value::Integer(k),
                sqlrustgo::Value::Text("phone".into()),
                sqlrustgo::Value::Float(200.0),
                sqlrustgo::Value::Text("BUILDING".into()),
                sqlrustgo::Value::Text("".into()),
            ]],
        );
    }
    // 3 orders across 3 years, each to a different customer
    for (k, year, custk) in [(0, "1995", 0), (1, "1996", 1), (2, "1997", 2)] {
        let _ = st.insert(
            "orders",
            vec![vec![
                sqlrustgo::Value::Integer(300 + k),
                sqlrustgo::Value::Integer(200 + custk),
                sqlrustgo::Value::Text("O".into()),
                sqlrustgo::Value::Float(1000.0),
                sqlrustgo::Value::Text(format!("{}-06-15", year)),
                sqlrustgo::Value::Text("1-URGENT".into()),
                sqlrustgo::Value::Text("Clerk#1".into()),
                sqlrustgo::Value::Integer(0),
                sqlrustgo::Value::Text("".into()),
            ]],
        );
    }
    // 3 lineitems — all use supplier 0 (GERMANY)
    for k in 0..3 {
        let _ = st.insert(
            "lineitem",
            vec![vec![
                sqlrustgo::Value::Integer(300 + k),
                sqlrustgo::Value::Integer(1),
                sqlrustgo::Value::Integer(100),
                sqlrustgo::Value::Integer(1),
                sqlrustgo::Value::Integer(10),
                sqlrustgo::Value::Float(1000.0),
                sqlrustgo::Value::Float(0.05),
                sqlrustgo::Value::Float(0.0),
                sqlrustgo::Value::Text("N".into()),
                sqlrustgo::Value::Text("O".into()),
                sqlrustgo::Value::Text("1995-08-01".into()),
                sqlrustgo::Value::Text("1995-08-15".into()),
                sqlrustgo::Value::Text("1995-08-22".into()),
                sqlrustgo::Value::Text("DELIVER IN PERSON".into()),
                sqlrustgo::Value::Text("TRUCK".into()),
                sqlrustgo::Value::Text("".into()),
            ]],
        );
    }
    engine
}

#[test]
fn diag_nation_alias_5x3() {
    let mut engine = build_mini_fixture();
    let sql = std::fs::read_to_string("queries/q7.sql").unwrap();
    let sql = sql.replace('\n', " ");
    let r = engine
        .execute(&sql)
        .unwrap_or_else(|e| panic!("Q7 failed: {}", e));
    eprintln!("Q7 returned {} rows", r.rows.len());
    for (i, row) in r.rows.iter().take(20).enumerate() {
        eprintln!("  row[{:2}] = {:?}", i, row);
    }
    eprintln!();
    eprintln!("SQLite oracle (same data): only (GERMANY, FRANCE) → 3 rows for 3 years");
    eprintln!("If duplicate-table-alias collapses: 5 nations × 5 nations × 3 years = 75 rows");
    eprintln!("If only n1 filter: 1 × 5 × 3 = 15 rows");
    eprintln!("If both filters work but EXTRACT collapses: 1 row");
}
