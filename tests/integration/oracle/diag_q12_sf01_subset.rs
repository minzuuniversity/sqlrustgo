//! V312-58 / Issue #4378 — Q12 SF~0.01 subset reproduction.
//!
//! Subset: 15K orders (head of SF=1 orders), 60K lineitem (matching l_orderkey),
// all 25 nations, all 10K suppliers, all 15K customers.
//!
//! SQLite ground truth for SF~0.01 subset (2 rows):
//!   MAIL|64|86
//!   SHIP|61|96
//!
//! Pre-fix expected (per issue #4378 on SF=1): 7 rows.
//!
//! Run:
//!   cargo test --test diag_q12_sf01_subset --all-features -- --ignored --nocapture

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::Arc;

const DATA_DIR: &str = "/tmp/tpch_sf01_subset";

fn setup() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());
    engine.execute("CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, o_orderstatus TEXT NOT NULL, o_totalprice REAL NOT NULL, o_orderdate TEXT NOT NULL, o_orderpriority TEXT NOT NULL, o_clerk TEXT NOT NULL, o_shippriority INTEGER NOT NULL, o_comment TEXT)").unwrap();
    engine.execute("CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity INTEGER NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT NOT NULL, l_linestatus TEXT NOT NULL, l_shipdate TEXT NOT NULL, l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT NOT NULL, l_shipmode TEXT NOT NULL, l_comment TEXT)").unwrap();
    {
        let mut st = storage.write();
        st.bulk_load_tbl_file("orders", &format!("{}/orders.tbl", DATA_DIR))
            .unwrap();
        st.bulk_load_tbl_file("lineitem", &format!("{}/lineitem.tbl", DATA_DIR))
            .unwrap();
    }
    engine
}

const Q12_SQL: &str = "SELECT l_shipmode, SUM(CASE WHEN o_orderpriority = '1-URGENT' OR o_orderpriority = '2-HIGH' THEN 1 ELSE 0 END) AS high_line_count, SUM(CASE WHEN o_orderpriority <> '1-URGENT' AND o_orderpriority <> '2-HIGH' THEN 1 ELSE 0 END) AS low_line_count FROM orders, lineitem WHERE l_orderkey = o_orderkey AND l_shipmode IN ('MAIL', 'SHIP') AND l_commitdate < l_receiptdate AND l_shipdate < l_commitdate AND l_receiptdate >= '1994-01-01' AND l_receiptdate < '1995-01-01' GROUP BY l_shipmode ORDER BY l_shipmode";

#[test]
#[ignore] // subset reproduction, opt-in only
fn q12_sf01_subset_count() {
    let mut engine = setup();
    let r = engine
        .execute(Q12_SQL)
        .unwrap_or_else(|e| panic!("Q12 failed: {}", e));
    eprintln!(
        "Q12 SF~0.01 subset returned {} rows × {} cols",
        r.rows.len(),
        r.rows.first().map(|r| r.len()).unwrap_or(0)
    );
    for (i, row) in r.rows.iter().enumerate() {
        eprintln!("  row[{:2}] = {:?}", i, row);
    }
    eprintln!();
    eprintln!("SQLite ground truth (2 rows, all 3 cols):");
    eprintln!("  MAIL|64|86");
    eprintln!("  SHIP|61|96");
}