//! V312-58 Sprint 3 Phase 1.1 — Path diagnostic
//!
//! Run Q17 on 100K and 1M subsets, dump the path counters added in
//! `engine_select.rs` to identify which path Q17 actually takes:
//! - `try_scalar_agg_index_lookup` calls/hits/pattern_fail/build
//! - per-row `scalar_subq_cache` hits/misses
//! - `execute_select` fallback calls
//!
//! Run:
//!   cargo test --test diag_q17_sprint3_path --all-features -- --ignored --nocapture

use parking_lot::RwLock;
use sqlrustgo::{
    dump_v312_58_sprint3_diag, reset_v312_58_sprint3_diag, ExecutionEngine, MemoryStorage, Value,
};
use std::sync::Arc;
use std::time::Instant;

const DATA_DIR: &str = "/tmp";

// SQLite oracle value for Q17 on the 100K subset (Brand#23, LG CASE):
//   SELECT SUM(l_extendedprice) / 7.0 FROM lineitem, part WHERE ...
//   → 224.5214285714286 (high-precision float)
const Q17_100K_ORACLE: f64 = 224.5214285714286;
const Q17_100K_TOLERANCE: f64 = 1e-6;

fn run_q17(lineitem_tbl: &str) -> (f64, String, Option<f64>) {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());
    engine.execute("CREATE TABLE part (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, p_comment TEXT NOT NULL)").unwrap();
    engine.execute("CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity INTEGER NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT NOT NULL, l_linestatus TEXT NOT NULL, l_shipdate TEXT NOT NULL, l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT NOT NULL, l_shipmode TEXT NOT NULL, l_comment TEXT NOT NULL)").unwrap();
    {
        let mut st = storage.write();
        st.bulk_load_tbl_file("part", &format!("{}/part_clean.tbl", DATA_DIR))
            .unwrap();
        st.bulk_load_tbl_file("lineitem", &format!("{}/{}", DATA_DIR, lineitem_tbl))
            .unwrap();
    }

    const Q17_SQL: &str = "SELECT SUM(l_extendedprice) / 7.0 AS avg_yearly FROM lineitem, part WHERE p_partkey = l_partkey AND p_brand = 'Brand#23' AND p_container = 'LG CASE' AND l_quantity < (SELECT 0.2 * AVG(l_quantity) FROM lineitem WHERE l_partkey = p_partkey)";

    reset_v312_58_sprint3_diag();
    let start = Instant::now();
    let r = engine
        .execute(Q17_SQL)
        .unwrap_or_else(|e| panic!("Q17 failed: {}", e));
    let elapsed = start.elapsed().as_secs_f64();
    let result = format!(
        "{} rows × {} cols",
        r.rows.len(),
        r.rows.first().map(|r| r.len()).unwrap_or(0)
    );
    let value = match r.rows.first().and_then(|row| row.first()) {
        Some(Value::Float(f)) => Some(*f),
        Some(Value::Integer(i)) => Some(*i as f64),
        Some(Value::Text(s)) => s.parse::<f64>().ok(),
        _ => None,
    };
    (elapsed, result, value)
}

#[test]
#[ignore]
fn diag_q17_100k_path() {
    eprintln!("=== Q17 100K subset path diagnostic ===");
    let (elapsed, result, value) = run_q17("q17_lineitem_100k.tbl");
    eprintln!("Q17 100K: elapsed {:.2}s, result: {}", elapsed, result);
    eprintln!("Q17 100K engine value: {:?}", value);
    eprintln!("Q17 100K SQLite oracle: {}", Q17_100K_ORACLE);
    eprintln!("\n{}\n", dump_v312_58_sprint3_diag());

    // Verify oracle match for 100K subset.
    if let Some(v) = value {
        let diff = (v - Q17_100K_ORACLE).abs();
        assert!(
            diff < Q17_100K_TOLERANCE,
            "Q17 100K oracle MISMATCH: engine={}, sqlite={}, diff={}",
            v,
            Q17_100K_ORACLE,
            diff
        );
        eprintln!(
            "✓ Q17 100K oracle MATCH (diff = {:.2e})",
            diff
        );
    } else {
        panic!("Q17 100K: no scalar value returned");
    }
}

#[test]
#[ignore]
fn diag_q17_1m_path() {
    eprintln!("=== Q17 1M subset path diagnostic (timeout 600s) ===");
    let (elapsed, result, value) = run_q17("q17_lineitem_1m.tbl");
    eprintln!("Q17 1M: elapsed {:.2}s, result: {}", elapsed, result);
    eprintln!("Q17 1M engine value: {:?}", value);
    eprintln!("\n{}\n", dump_v312_58_sprint3_diag());
}
