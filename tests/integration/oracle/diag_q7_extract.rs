//! Diagnostic: TPC-H Q7 EXTRACT(year FROM l_shipdate) parsing path.
//!
//! V312-58 / Issue #4376 — root cause investigation.
//! Run:
//!   cargo test --test diag_q7_extract --all-features -- --ignored --nocapture
//!
//! Expected after fix: diag_extract_year_simple succeeds and returns 1993 (or
//! whichever year the test fixture row ships with). The end-state test
//! (q7_volume_shipping) is added separately once EXTRACT is wired.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine};
use std::sync::Arc;

const DATA_DIR: &str = "/home/openclaw/tpch_baseline/sf1";

const SCHEMAS: &[&str] = &[
    "CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity INTEGER NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT NOT NULL, l_linestatus TEXT NOT NULL, l_shipdate TEXT NOT NULL, l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT NOT NULL, l_shipmode TEXT NOT NULL, l_comment TEXT NOT NULL)",
];

const TBL_FILES: &[&str] = &["lineitem"];

fn setup() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());
    for s in SCHEMAS {
        engine.execute(s).unwrap();
    }
    {
        let mut st = storage.write();
        for t in TBL_FILES {
            let path = format!("{}/{}.tbl", DATA_DIR, t);
            st.bulk_load_tbl_file(t, &path).unwrap();
        }
    }
    engine
}

#[test]
#[ignore] // Heavy: SF=1 lineitem is 6M rows
fn diag_extract_year_simple_parse_only() {
    // Pure parser smoke test: in-memory table with 1 row, EXTRACT(year FROM ...).
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());
    engine.execute("CREATE TABLE t (d TEXT NOT NULL)").unwrap();
    {
        let mut st = storage.write();
        let _ = st.insert("t", vec![vec![sqlrustgo::Value::Text("1995-06-15".into())]]);
    }

    let r = engine
        .execute("SELECT EXTRACT(YEAR FROM d) AS y FROM t")
        .unwrap_or_else(|e| panic!("EXTRACT parse/exec failed: {}", e));
    eprintln!("rows = {}", r.rows.len());
    eprintln!("row[0] = {:?}", r.rows.first());
    assert_eq!(r.rows.len(), 1, "expected 1 row");
}

#[test]
#[ignore] // Heavy: SF=1
fn diag_q7_full_count_against_oracle() {
    let mut engine = setup();
    let sql = std::fs::read_to_string("queries/q7.sql").unwrap();
    let sql = sql.replace('\n', " ");
    let r = engine
        .execute(&sql)
        .unwrap_or_else(|e| panic!("Q7 failed: {}", e));
    eprintln!("Q7 returned {} rows", r.rows.len());
    for (i, row) in r.rows.iter().take(10).enumerate() {
        eprintln!("  row[{}] = {:?}", i, row);
    }
    // Issue #4376 expectation: 7 rows (SQLite oracle sf=1).
    assert_eq!(r.rows.len(), 7, "Q7 must return 7 rows");
}
