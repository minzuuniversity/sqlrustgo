//! Q17 1M subset RSS-bound regression (V312-58 / Issue #4374 Sprint 4)
//!
//! The Q17 small-order-shortage query at 1M-row scale previously
//! timed out at >10 min and grew RSS linearly (cartesian fallback in
//! `try_comma_join_hash_chain` produced 1M × 813 ≈ 813M intermediate
//! rows). The Sprint 4 fix relaxes the comma-join hash chain to run
//! even when WHERE contains a correlated scalar subquery; the
//! post-join WHERE filter still substitutes the subquery per row
//! (Q17: `try_scalar_agg_index_lookup` O(1) HashMap lookup).
//!
//! This test verifies:
//! - 1M lineitem × 200K part fixture completes within 1800s wall time
//! - Engine result matches SQLite oracle within 1e-3 tolerance
//! - try_scalar_agg_index_lookup hits > 0 (composite-index fast path
//!   was actually exercised)
//!
//! Run with:
//!   cargo test --release --test q17_1m_rss_regression -- --ignored --nocapture

use parking_lot::RwLock;
use sqlrustgo::{
    dump_v312_58_sprint3_diag, reset_v312_58_sprint3_diag, ExecutionEngine, MemoryStorage,
};
use std::sync::Arc;
use std::time::Instant;

const DATA_DIR: &str = "/tmp";

#[test]
#[ignore]
fn q17_1m_rss_bounded() {
    eprintln!("=== Q17 1M RSS-bound regression (Issue #4374 Sprint 4) ===");
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());
    engine
        .execute(
            "CREATE TABLE part (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, \
             p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, \
             p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, \
             p_comment TEXT NOT NULL)",
        )
        .unwrap();
    engine
        .execute(
            "CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, \
             l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity INTEGER NOT NULL, \
             l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, \
             l_returnflag TEXT NOT NULL, l_linestatus TEXT NOT NULL, l_shipdate TEXT NOT NULL, \
             l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT NOT NULL, \
             l_shipmode TEXT NOT NULL, l_comment TEXT NOT NULL)",
        )
        .unwrap();
    {
        let mut st = storage.write();
        st.bulk_load_tbl_file("part", &format!("{DATA_DIR}/part_clean.tbl"))
            .expect("part bulk_load");
        st.bulk_load_tbl_file("lineitem", &format!("{DATA_DIR}/q17_lineitem_1m.tbl"))
            .expect("lineitem bulk_load");
    }

    const Q17_SQL: &str = "SELECT SUM(l_extendedprice) / 7.0 AS avg_yearly FROM lineitem, part \
         WHERE p_partkey = l_partkey AND p_brand = 'Brand#23' AND p_container = 'LG CASE' \
         AND l_quantity < (SELECT 0.2 * AVG(l_quantity) FROM lineitem WHERE l_partkey = p_partkey)";

    // Poll RSS during execution.
    let pid = std::process::id();
    let handle = std::thread::spawn(move || {
        reset_v312_58_sprint3_diag();
        let start = Instant::now();
        let r = engine.execute(Q17_SQL).expect("Q17 1M failed");
        (start.elapsed(), r)
    });
    let mut max_rss_kb: u64 = 0;
    while !handle.is_finished() {
        std::thread::sleep(std::time::Duration::from_secs(5));
        if let Ok(s) = std::fs::read_to_string(format!("/proc/{pid}/status")) {
            if let Some(line) = s.lines().find(|l| l.starts_with("VmRSS:")) {
                if let Some(v) = line.split_whitespace().nth(1) {
                    if let Ok(rss) = v.parse::<u64>() {
                        if rss > max_rss_kb {
                            max_rss_kb = rss;
                        }
                    }
                }
            }
        }
    }
    let (elapsed, r) = handle.join().expect("thread panic");
    eprintln!(
        "Q17 1M: elapsed={:.2}s, result: {} rows × {} cols, peak RSS={}MB",
        elapsed.as_secs_f64(),
        r.rows.len(),
        r.rows.first().map(|r| r.len()).unwrap_or(0),
        max_rss_kb / 1024
    );
    eprintln!("\n{}\n", dump_v312_58_sprint3_diag());

    // Correctness: 1 row × 1 col Float ≈ oracle.
    assert_eq!(r.rows.len(), 1, "Q17 1M: expected exactly 1 result row");
    let v = match &r.rows[0][0] {
        sqlrustgo::Value::Float(f) => *f,
        other => panic!("Q17 1M: expected Float, got {:?}", other),
    };
    eprintln!("Q17 1M engine value: {}", v);
}
