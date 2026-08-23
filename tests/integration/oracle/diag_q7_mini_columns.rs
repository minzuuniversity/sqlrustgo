//! Diagnostic: TPC-H Q7 in-memory fixture, print per-row column type and count.
//!
//! V312-58 / Issue #4376 — root cause investigation v3.
//! Builds the same 1-fact-row fixture as diag_q7_mini.rs but prints each row's
//! column count + per-column debug repr so we can tell whether the bug is:
//!   (a) GROUP BY year collapse (l_year → Null, all years grouped into 1)
//!   (b) projection truncation (l_year dropped from SELECT list)
//!   (c) alias collapse (n1.n_name and n2.n_name treated as same column)
//!
//! Run:
//!   cargo test --test diag_q7_mini_columns --all-features -- --nocapture
//!
//! Expected: 1 row with 4 columns = [GERMANY, FRANCE, 1995, <SUM>].

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine, Value};
use std::sync::Arc;

fn build_mini_fixture() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());

    for s in [
        "CREATE TABLE region (r_regionkey INTEGER PRIMARY KEY, r_name TEXT NOT NULL, r_comment TEXT)",
        "CREATE TABLE nation (n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, n_regionkey INTEGER NOT NULL, n_comment TEXT)",
        "CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT)",
        "CREATE TABLE customer (c_custkey INTEGER PRIMARY KEY, c_name TEXT NOT NULL, c_address TEXT NOT NULL, c_nationkey INTEGER NOT NULL, c_phone TEXT NOT NULL, c_acctbal REAL NOT NULL, c_mktsegment TEXT NOT NULL, c_comment TEXT)",
        "CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, o_orderstatus TEXT NOT NULL, o_orderdate TEXT NOT NULL, o_orderpriority TEXT NOT NULL, o_clerk TEXT NOT NULL, o_shippriority INTEGER NOT NULL, o_comment TEXT NOT NULL)",
        "CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity INTEGER NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT NOT NULL, l_linestatus TEXT NOT NULL, l_shipdate TEXT NOT NULL, l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT NOT NULL, l_shipmode TEXT NOT NULL, l_comment TEXT NOT NULL)",
    ] {
        engine.execute(s).unwrap();
    }

    let mut st = storage.write();
    let _ = st.insert("region", vec![vec![
        Value::Integer(1), Value::Text("EUROPE".into()), Value::Text("".into()),
    ]]);
    let _ = st.insert("nation", vec![vec![
        Value::Integer(7), Value::Text("GERMANY".into()), Value::Integer(1), Value::Text("".into()),
    ]]);
    let _ = st.insert("nation", vec![vec![
        Value::Integer(8), Value::Text("FRANCE".into()), Value::Integer(1), Value::Text("".into()),
    ]]);
    let _ = st.insert("supplier", vec![vec![
        Value::Integer(101), Value::Text("S#101".into()), Value::Text("addr".into()),
        Value::Integer(7), Value::Text("phone".into()), Value::Float(100.0), Value::Text("".into()),
    ]]);
    let _ = st.insert("customer", vec![vec![
        Value::Integer(201), Value::Text("C#201".into()), Value::Text("addr".into()),
        Value::Integer(8), Value::Text("phone".into()), Value::Float(200.0),
        Value::Text("BUILDING".into()), Value::Text("".into()),
    ]]);
    let _ = st.insert("orders", vec![vec![
        Value::Integer(301), Value::Integer(201), Value::Text("O".into()),
        Value::Text("1995-06-15".into()), Value::Text("1-URGENT".into()),
        Value::Text("Clerk#1".into()), Value::Integer(0), Value::Text("".into()),
    ]]);
    let _ = st.insert("lineitem", vec![vec![
        Value::Integer(301), Value::Integer(1), Value::Integer(101),
        Value::Integer(1), Value::Integer(10), Value::Float(1000.0),
        Value::Float(0.05), Value::Float(0.0),
        Value::Text("N".into()), Value::Text("O".into()),
        Value::Text("1995-08-01".into()), Value::Text("1995-08-15".into()),
        Value::Text("1995-08-22".into()), Value::Text("DELIVER IN PERSON".into()),
        Value::Text("TRUCK".into()), Value::Text("".into()),
    ]]);
    engine
}

#[test]
fn diag_q7_mini_columns_count() {
    let mut engine = build_mini_fixture();
    let sql = std::fs::read_to_string("queries/q7.sql").unwrap();
    let sql = sql.replace('\n', " ");
    let r = engine.execute(&sql).unwrap_or_else(|e| panic!("Q7 failed: {}", e));
    eprintln!("Q7 returned {} rows", r.rows.len());
    for (i, row) in r.rows.iter().enumerate() {
        eprintln!("row[{}] ncols={}", i, row.len());
        for (j, cell) in row.iter().enumerate() {
            eprintln!("  col[{}] = {:?} (variant)", j, std::mem::discriminant(cell));
            eprintln!("         display = {:?}", cell);
        }
    }
    assert!(
        !r.rows.is_empty(),
        "Q7 with 1 fact-row must return at least 1 row"
    );
    let row = &r.rows[0];
    eprintln!("ROW COLUMN COUNT = {}", row.len());
    // The query has 4 SELECT columns: n1.n_name, n2.n_name, EXTRACT(year), SUM(...)
    // If GROUP BY year collapses AND projection drops year, row.len() = 3.
    // If GROUP BY year collapses BUT projection keeps year as Null, row.len() = 4.
    if row.len() == 4 {
        eprintln!("✓ 4 columns: l_year preserved as separate column (likely Null)");
        eprintln!("  -> bug is GROUP BY year collapse only");
    } else if row.len() == 3 {
        eprintln!("✗ 3 columns: l_year is missing entirely");
        eprintln!("  -> bug is BOTH GROUP BY collapse AND projection truncation");
    } else {
        eprintln!("✗ unexpected column count: {}", row.len());
    }
}