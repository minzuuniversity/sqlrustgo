//! Performance regression test for V312-58 / Issue #4379: TPC-H SF=1
//! Q17 small-order-shortage.
//!
//! Q17 has a correlated scalar subquery in the WHERE clause:
//!
//! ```sql
//! SELECT SUM(l_extendedprice) / 7.0 AS avg_yearly
//! FROM lineitem, part
//! WHERE p_partkey = l_partkey
//!   AND p_brand = 'Brand#23'
//!   AND p_container = 'LG CASE'
//!   AND l_quantity < (SELECT 0.2 * AVG(l_quantity)
//!                       FROM lineitem
//!                       WHERE l_partkey = p_partkey);
//! ```
//!
//! Without decorrelation, the subquery is re-evaluated for each candidate
//! (lineitem, part) row in the post-join filter — scanning all 6M
//! lineitem rows × ~30K candidate rows = ~180B ops, which times out
//! (>1800s).
//!
//! Expected baseline (SQLite oracle, SF=1, queries/q17.sql):
//!   row_count = 1
//!   value      = 249963.75857142857
//!   elapsed    ≤ 1800s (relaxed per #4432 followup; was 300s in original
//!               #4379 AC; Q17 full SF=1 deferred to v3.13 Issue #4426
//!               per upstream commit efbc1e955)
//!
//! Run:
//!   TPCH_SF1_DIR=/tmp/tpch-sf1 cargo test --release \\
//!     --test q17_small_order_shortage_perf --all-features \\
//!     -- --ignored --nocapture q17_small_order_shortage_sf1
//!
//! Fixture: TPC-H SF=1 .tbl files at $TPCH_SF1_DIR (default
//! `/tmp/tpch-sf1`). Required tables: part, lineitem. Load via
//! `bulk_load_tbl_file` (same path as q2_5way_comma_limit_regression.rs).

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// SF=1 fixture directory. Override with TPCH_SF1_DIR env var.
const DATA_DIR: &str = "/tmp/tpch-sf1";

/// Q17 canonical SQL — copy of `queries/q17.sql`.
const Q17_SQL: &str = "SELECT SUM(l_extendedprice) / 7.0 AS avg_yearly \
                       FROM lineitem, part \
                       WHERE p_partkey = l_partkey \
                         AND p_brand = 'Brand#23' \
                         AND p_container = 'LG CASE' \
                         AND l_quantity < (SELECT 0.2 * AVG(l_quantity) \
                                            FROM lineitem \
                                            WHERE l_partkey = p_partkey)";

const SCHEMAS: &[&str] = &[
    "CREATE TABLE part (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, p_comment TEXT)",
    "CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity INTEGER NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT NOT NULL, l_linestatus TEXT NOT NULL, l_shipdate TEXT NOT NULL, l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT NOT NULL, l_shipmode TEXT NOT NULL, l_comment TEXT NOT NULL)",
];

const TBL_FILES: &[&str] = &["part", "lineitem"];

/// Per-query wall-clock budget (relaxed to 1800s per #4432 followup;
/// original #4379 AC specified 300s, but Q17 is deferred to v3.13 Issue
/// #4426 per upstream commit efbc1e955).
const TIMEOUT_BUDGET: Duration = Duration::from_secs(1800);

/// Expected value from the SQLite oracle at SF=1.
/// (cf. docs/releases/v3.12.0/evidence/tpch/cross_engine_sf1/sqlite/q17.tsv)
/// sha256 = 595003bfd069556083c03906c87e59ac0997cc8932995628b038ede3004d759a
const EXPECTED_VALUE: f64 = 249_963.75857142857_f64;

/// Float comparison tolerance. TPC-H 2.18.0 §6.3.3 allows ±epsilon for
/// aggregate queries; we use ±1e-3 to be conservative.
const FLOAT_TOL: f64 = 1e-3;

fn resolve_data_dir() -> String {
    std::env::var("TPCH_SF1_DIR").unwrap_or_else(|_| DATA_DIR.to_string())
}

fn setup() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());
    let data_dir = resolve_data_dir();
    for s in SCHEMAS {
        engine.execute(s).unwrap();
    }
    {
        let mut st = storage.write();
        for t in TBL_FILES {
            let path = format!("{}/{}.tbl", data_dir, t);
            st.bulk_load_tbl_file(t, &path)
                .unwrap_or_else(|e| panic!("bulk_load_tbl_file({}) failed: {}", t, e));
        }
    }
    engine
}

#[test]
#[ignore] // Heavy: SF=1 lineitem has 6M rows
fn q17_small_order_shortage_sf1() {
    let mut engine = setup();

    let start = Instant::now();
    let r = engine
        .execute(Q17_SQL)
        .unwrap_or_else(|e| panic!("Q17 failed: {}", e));
    let elapsed = start.elapsed();
    eprintln!("Q17 elapsed: {:?}", elapsed);

    // 1) Wall-clock budget — the original symptom (#4379) was TIMEOUT.
    assert!(
        elapsed <= TIMEOUT_BUDGET,
        "Q17 elapsed {:?} exceeds TIMEOUT_BUDGET {:?} — correlated subquery \
         not decorrelated",
        elapsed,
        TIMEOUT_BUDGET
    );

    // 2) Row count parity — SQLite oracle returns 1 row.
    assert_eq!(
        r.rows.len(),
        1,
        "Q17 row count must match SQLite oracle (expected 1, got {})",
        r.rows.len()
    );

    // 3) Value parity — within ±epsilon.
    let got = match &r.rows[0][0] {
        sqlrustgo::Value::Float(f) => *f,
        other => panic!("expected Float scalar, got {:?}", other),
    };
    eprintln!("Q17 result: {}", got);
    let diff = (got - EXPECTED_VALUE).abs();
    assert!(
        diff <= FLOAT_TOL,
        "Q17 value must match SQLite oracle (expected {}, got {}, diff {})",
        EXPECTED_VALUE,
        got,
        diff
    );
}
