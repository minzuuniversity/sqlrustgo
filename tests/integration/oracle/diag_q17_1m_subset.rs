//! V312-58 / Issue #4379 — Q17 1M-lineitem subset perf check.
//!
//! Subset: 1M lineitem rows (random sample from SF=1) + 200K SF=1 part table.
//!
//! Purpose: validate Q17 scaling at 1/6 of SF=1 to extrapolate full SF=1 timing.
//!
//! Pre-fix expected (per issue #4379 on SF=1): TIMEOUT (>1800s).
//!
//! Run:
//!   cargo test --test diag_q17_1m_subset --all-features -- --ignored --nocapture

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::Arc;
use std::time::Instant;

const DATA_DIR: &str = "/tmp";

fn setup() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());
    engine.execute("CREATE TABLE part (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, p_comment TEXT NOT NULL)").unwrap();
    engine.execute("CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity INTEGER NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT NOT NULL, l_linestatus TEXT NOT NULL, l_shipdate TEXT NOT NULL, l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT NOT NULL, l_shipmode TEXT NOT NULL, l_comment TEXT NOT NULL)").unwrap();
    {
        let mut st = storage.write();
        st.bulk_load_tbl_file("part", &format!("{}/part_clean.tbl", DATA_DIR))
            .unwrap();
        st.bulk_load_tbl_file("lineitem", &format!("{}/q17_lineitem_1m.tbl", DATA_DIR))
            .unwrap();
    }
    engine
}

const Q17_SQL: &str = "SELECT SUM(l_extendedprice) / 7.0 AS avg_yearly FROM lineitem, part WHERE p_partkey = l_partkey AND p_brand = 'Brand#23' AND p_container = 'LG CASE' AND l_quantity < (SELECT 0.2 * AVG(l_quantity) FROM lineitem WHERE l_partkey = p_partkey)";

#[test]
#[ignore] // subset reproduction, opt-in only
fn diag_q17_1m_subset() {
    let mut engine = setup();
    let start = Instant::now();
    let r = engine
        .execute(Q17_SQL)
        .unwrap_or_else(|e| panic!("Q17 failed: {}", e));
    let elapsed = start.elapsed();
    eprintln!(
        "Q17 1M subset: {} rows × {} cols, elapsed {:.2}s",
        r.rows.len(),
        r.rows.first().map(|r| r.len()).unwrap_or(0),
        elapsed.as_secs_f64()
    );
    for (i, row) in r.rows.iter().enumerate() {
        eprintln!("  row[{:2}] = {:?}", i, row);
    }
    eprintln!();
    eprintln!(
        "Extrapolation (linear): SF=1 6M rows ≈ {:.0}s",
        elapsed.as_secs_f64() * 6.0
    );
    eprintln!("SQLite SF=1 oracle: 3713854.4285714287 (sha256 595003bf...)");
}
