//! V312-58 / Issue #4376 — SF~0.001 subset reproduction.
//!
//! Per memory recipe: nation/region all; supplier where s_nationkey∈{6,7};
//! customer where c_nationkey∈{6,7}; orders where o_custkey∈DE/FR customers;
//! lineitem where l_orderkey∈orders (head 60K).
//!
//! SQLite ground truth for SF=0.001 subset (7 rows):
//!   GERMANY|FRANCE|1992|6395998.13
//!   GERMANY|FRANCE|1993|5958710.92
//!   GERMANY|FRANCE|1994|6263829.36
//!   GERMANY|FRANCE|1995|6590755.57
//!   GERMANY|FRANCE|1996|4869595.81
//!   GERMANY|FRANCE|1997|5950228.34
//!   GERMANY|FRANCE|1998|4197154.30
//!
//! Expected (engine, per memory v2): 2 rows × 3 cols (drops volume column):
//!   - ("FRANCE", "FRANCE", "7352")  — n1, n2 conflated
//!   - ("GERMANY", "FRANCE", "2046") — correct alias but GROUP BY year collapsed
//!
//! Run:
//!   cargo test --test diag_q7_sf001_subset --all-features -- --ignored --nocapture

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::Arc;

const DATA_DIR: &str = "/tmp/tpch_sf001_subset";

fn setup() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());
    for s in [
        "CREATE TABLE region (r_regionkey INTEGER PRIMARY KEY, r_name TEXT NOT NULL, r_comment TEXT)",
        "CREATE TABLE nation (n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, n_regionkey INTEGER NOT NULL, n_comment TEXT)",
        "CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT)",
        "CREATE TABLE customer (c_custkey INTEGER PRIMARY KEY, c_name TEXT NOT NULL, c_address TEXT NOT NULL, c_nationkey INTEGER NOT NULL, c_phone TEXT NOT NULL, c_acctbal REAL NOT NULL, c_mktsegment TEXT NOT NULL, c_comment TEXT)",
        "CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, o_orderstatus TEXT NOT NULL, o_totalprice REAL NOT NULL, o_orderdate TEXT NOT NULL, o_orderpriority TEXT NOT NULL, o_clerk TEXT NOT NULL, o_shippriority INTEGER NOT NULL, o_comment TEXT)",
        "CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity INTEGER NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT NOT NULL, l_linestatus TEXT NOT NULL, l_shipdate TEXT NOT NULL, l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT NOT NULL, l_shipmode TEXT NOT NULL, l_comment TEXT)",
    ] {
        engine.execute(s).unwrap();
    }
    {
        let mut st = storage.write();
        for t in ["region", "nation", "supplier", "customer", "orders", "lineitem"] {
            let path = format!("{}/{}.tbl", DATA_DIR, t);
            st.bulk_load_tbl_file(t, &path).unwrap();
        }
    }
    engine
}

const Q7_SQL: &str = "SELECT n1.n_name AS supp_nation, n2.n_name AS cust_nation, EXTRACT(YEAR FROM o_orderdate) AS l_year, SUM(l_extendedprice * (1 - l_discount)) AS volume \
                       FROM supplier, lineitem, orders, customer, nation n1, nation n2 \
                       WHERE s_suppkey = l_suppkey AND o_orderkey = l_orderkey AND c_custkey = o_custkey \
                         AND s_nationkey = n1.n_nationkey AND c_nationkey = n2.n_nationkey \
                         AND n1.n_name = 'GERMANY' AND n2.n_name = 'FRANCE' \
                       GROUP BY n1.n_name, n2.n_name, EXTRACT(YEAR FROM o_orderdate) \
                       ORDER BY n1.n_name, n2.n_name, l_year";

#[test]
#[ignore] // subset reproduction, opt-in only
fn q7_sf001_subset_count() {
    let mut engine = setup();
    let r = engine
        .execute(Q7_SQL)
        .unwrap_or_else(|e| panic!("Q7 failed: {}", e));
    eprintln!("Q7 SF~0.001 subset returned {} rows × {} cols", r.rows.len(), r.rows.first().map(|r| r.len()).unwrap_or(0));
    for (i, row) in r.rows.iter().enumerate() {
        eprintln!("  row[{:2}] = {:?}", i, row);
    }
    eprintln!();
    eprintln!("SQLite ground truth (7 rows, all 4 cols):");
    eprintln!("  GERMANY|FRANCE|1992|6395998.13");
    eprintln!("  GERMANY|FRANCE|1993|5958710.92");
    eprintln!("  GERMANY|FRANCE|1994|6263829.36");
    eprintln!("  GERMANY|FRANCE|1995|6590755.57");
    eprintln!("  GERMANY|FRANCE|1996|4869595.81");
    eprintln!("  GERMANY|FRANCE|1997|5950228.34");
    eprintln!("  GERMANY|FRANCE|1998|4197154.30");
}