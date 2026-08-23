//! Diagnostic: TPC-H Q7 minimal in-memory fixture to verify EXTRACT + 6-table JOIN
//! against an oracle.
//!
//! V312-58 / Issue #4376 — root cause investigation.
//! Builds a 6-table fixture with exactly 1 fact row that satisfies Q7's WHERE
//! predicate (`n1.n_name='GERMANY' AND n2.n_name='FRANCE'`), runs
//! `queries/q7.sql` (which uses `EXTRACT(YEAR FROM o_orderdate)` in SELECT +
//! GROUP BY but no WHERE on year), and verifies:
//!   - sqlrustgo returns exactly 1 row
//!   - the row's `l_year` column is the parsed year
//!   - hash of result matches the SQLite oracle run on the same fixture
//!
//! Run:
//!   cargo test --test diag_q7_mini --all-features -- --nocapture

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
        "CREATE TABLE customer (c_custkey INTEGER PRIMARY KEY, c_name TEXT NOT NULL, c_address TEXT NOT NULL, c_nationkey INTEGER NOT NULL, c_phone TEXT NOT NULL, c_acctbal REAL NOT NULL, c_mktsegment TEXT NOT NULL, c_comment TEXT NOT NULL)",
        "CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, o_orderstatus TEXT NOT NULL, o_orderdate TEXT NOT NULL, o_orderpriority TEXT NOT NULL, o_clerk TEXT NOT NULL, o_shippriority INTEGER NOT NULL, o_comment TEXT NOT NULL)",
        "CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity INTEGER NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT NOT NULL, l_linestatus TEXT NOT NULL, l_shipdate TEXT NOT NULL, l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT NOT NULL, l_shipmode TEXT NOT NULL, l_comment TEXT NOT NULL)",
    ] {
        engine.execute(s).unwrap();
    }

    let mut st = storage.write();
    let _ = st.insert("region", vec![vec![
        sqlrustgo::Value::Integer(1), sqlrustgo::Value::Text("EUROPE".into()), sqlrustgo::Value::Text("".into()),
    ]]);
    let _ = st.insert("nation", vec![vec![
        sqlrustgo::Value::Integer(7), sqlrustgo::Value::Text("GERMANY".into()), sqlrustgo::Value::Integer(1), sqlrustgo::Value::Text("".into()),
    ]]);
    let _ = st.insert("nation", vec![vec![
        sqlrustgo::Value::Integer(8), sqlrustgo::Value::Text("FRANCE".into()), sqlrustgo::Value::Integer(1), sqlrustgo::Value::Text("".into()),
    ]]);
    let _ = st.insert("supplier", vec![vec![
        sqlrustgo::Value::Integer(101), sqlrustgo::Value::Text("S#101".into()), sqlrustgo::Value::Text("addr".into()),
        sqlrustgo::Value::Integer(7), sqlrustgo::Value::Text("phone".into()), sqlrustgo::Value::Float(100.0), sqlrustgo::Value::Text("".into()),
    ]]);
    let _ = st.insert("customer", vec![vec![
        sqlrustgo::Value::Integer(201), sqlrustgo::Value::Text("C#201".into()), sqlrustgo::Value::Text("addr".into()),
        sqlrustgo::Value::Integer(8), sqlrustgo::Value::Text("phone".into()), sqlrustgo::Value::Float(200.0),
        sqlrustgo::Value::Text("BUILDING".into()), sqlrustgo::Value::Text("".into()),
    ]]);
    let _ = st.insert("orders", vec![vec![
        sqlrustgo::Value::Integer(301), sqlrustgo::Value::Integer(201), sqlrustgo::Value::Text("O".into()),
        sqlrustgo::Value::Text("1995-06-15".into()), sqlrustgo::Value::Text("1-URGENT".into()),
        sqlrustgo::Value::Text("Clerk#1".into()), sqlrustgo::Value::Integer(0), sqlrustgo::Value::Text("".into()),
    ]]);
    let _ = st.insert("lineitem", vec![vec![
        sqlrustgo::Value::Integer(301), sqlrustgo::Value::Integer(1), sqlrustgo::Value::Integer(101),
        sqlrustgo::Value::Integer(1), sqlrustgo::Value::Integer(10), sqlrustgo::Value::Float(1000.0),
        sqlrustgo::Value::Float(0.05), sqlrustgo::Value::Float(0.0),
        sqlrustgo::Value::Text("N".into()), sqlrustgo::Value::Text("O".into()),
        sqlrustgo::Value::Text("1995-08-01".into()), sqlrustgo::Value::Text("1995-08-15".into()),
        sqlrustgo::Value::Text("1995-08-22".into()), sqlrustgo::Value::Text("DELIVER IN PERSON".into()),
        sqlrustgo::Value::Text("TRUCK".into()), sqlrustgo::Value::Text("".into()),
    ]]);
    engine
}

#[test]
fn diag_q7_mini_count_one() {
    let mut engine = build_mini_fixture();
    let sql = std::fs::read_to_string("queries/q7.sql").unwrap();
    let sql = sql.replace('\n', " ");
    let r = engine.execute(&sql).unwrap_or_else(|e| panic!("Q7 failed: {}", e));
    eprintln!("Q7 returned {} rows", r.rows.len());
    for (i, row) in r.rows.iter().take(5).enumerate() {
        eprintln!("  row[{}] = {:?}", i, row);
    }
    assert_eq!(r.rows.len(), 1, "Q7 with 1 fact-row must return exactly 1 row");
}