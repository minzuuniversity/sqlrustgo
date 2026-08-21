//! Regression test for V312-58 / Issue #4375: TPC-H Q2 5-way comma-join
//! with `LIMIT 20` (queries/q2.sql canonical form) returns 15,628 rows
//! instead of 20 because the comma-join fast path's hash-chain
//! optimisation fails to apply either base-table predicate pushdown
//! (`p_size = 15`, `p_type LIKE '%BRASS'`, `r_name = 'EUROPE'`) or the
//! post-join WHERE filter, leaving a fully-joined cross-product before
//! LIMIT trims it.
//!
//! SF=0.1 does NOT reproduce this bug — at SF=0.1 only 3 of the 73
//! qualifying parts have suppliers in EUROPEAN nations, so the result is
//! only 3 rows even with a fully un-pruned cross-product (LIMIT 20 caps
//! at 3). At SF=1 the qualified join yields 642 rows and LIMIT 20 caps
//! at 20. If pushdown is dropped, the cross-product grows to 15,628
//! rows (suppliers_in_EUROPE × partsupp) and LIMIT 20 still caps at 20
//! — so the symptom there is wrong contents, not wrong cardinality.
//!
//! Expected baseline (SQLite oracle, sf=1, runs ~12s):
//!   747 qualifying parts → 642 qualified join rows → top 20 by
//!   ORDER BY s_acctbal ASC, n_name, s_name, p_partkey → LIMIT 20 = 20
//!   rows.
//!
//! Run:
//!   cargo test --test q2_5way_comma_limit_regression --all-features \
//!     -- --ignored --nocapture q2_canonical_5way_comma_limit

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::Arc;

// SF=1 fixture — `/home/openclaw/tpch_baseline/sf1/{table}.tbl`.
// Override with TPCH_SF1_DIR env var to point at another location.
const DATA_DIR: &str = "/home/openclaw/tpch_baseline/sf1";

const SCHEMAS: &[&str] = &[
    "CREATE TABLE region (r_regionkey INTEGER PRIMARY KEY, r_name TEXT NOT NULL, r_comment TEXT)",
    "CREATE TABLE nation (n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, n_regionkey INTEGER NOT NULL, n_comment TEXT)",
    "CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT)",
    "CREATE TABLE part (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, p_comment TEXT)",
    "CREATE TABLE partsupp (ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, ps_availqty INTEGER NOT NULL, ps_supplycost REAL NOT NULL, ps_comment TEXT, PRIMARY KEY (ps_partkey, ps_suppkey))",
];

const TBL_FILES: &[&str] = &[
    "region", "nation", "supplier", "part", "partsupp",
];

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
            st.bulk_load_tbl_file(t, &path).unwrap();
        }
    }
    engine
}

const Q2_SQL: &str = "SELECT s_acctbal, s_name, n_name, p_partkey, p_mfgr, s_address, s_phone, s_comment \
                       FROM part, supplier, partsupp, nation, region \
                       WHERE p_partkey = ps_partkey \
                         AND s_suppkey = ps_suppkey \
                         AND p_size = 15 \
                         AND p_type LIKE '%BRASS' \
                         AND s_nationkey = n_nationkey \
                         AND n_regionkey = r_regionkey \
                         AND r_name = 'EUROPE' \
                       ORDER BY s_acctbal ASC, n_name, s_name, p_partkey \
                       LIMIT 20";

#[test]
#[ignore] // Heavy: 200K part + 800K partsupp + 10K supplier at SF=1
fn q2_canonical_5way_comma_limit() {
    let mut engine = setup();
    let r = engine
        .execute(Q2_SQL)
        .unwrap_or_else(|e| panic!("Q2 failed: {}", e));
    eprintln!("Q2 returned {} rows", r.rows.len());
    for (i, row) in r.rows.iter().take(5).enumerate() {
        eprintln!("  row[{}] = {:?}", i, row);
    }
    assert_eq!(
        r.rows.len(),
        20,
        "Q2 LIMIT 20 must cap the result at 20 rows (got {})",
        r.rows.len()
    );
}