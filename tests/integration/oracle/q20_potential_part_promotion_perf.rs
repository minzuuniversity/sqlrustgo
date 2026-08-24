//! Performance regression test for V312-58 / Issue #4380: TPC-H SF=1
//! Q20 potential-part-promotion.
//!
//! Q20 has two layers of correlated subqueries:
//!
//! ```sql
//! SELECT s_name, s_address
//! FROM supplier, nation
//! WHERE s_nationkey = n_nationkey
//!   AND n_name = 'GERMANY'
//!   AND EXISTS (
//!     SELECT * FROM partsupp
//!     WHERE ps_suppkey = s_suppkey
//!       AND ps_partkey IN (SELECT p_partkey FROM part WHERE p_name LIKE 'forest%')
//!       AND ps_availqty > (SELECT 0.5 * SUM(l_quantity)
//!                            FROM lineitem
//!                            WHERE l_partkey = ps_partkey
//!                              AND l_suppkey = ps_suppkey
//!                              AND l_shipdate >= '1994-01-01'
//!                              AND l_shipdate <  '1995-01-01')
//!   )
//! ORDER BY s_name;
//! ```
//!
//! 1. **Outer** EXISTS subquery is correlated to outer supplier (s_suppkey).
//! 2. **Inner** scalar subquery is correlated to partsupp (ps_partkey, ps_suppkey).
//!
//! Without EXISTS→semi-join decorrelation and correlated scalar materialization,
//! the engine re-scans 800K partsupp × 6M lineitem per outer supplier row,
//! producing TIMEOUT (>1800s).
//!
//! Expected baseline (SQLite oracle, SF=1, queries/q20.sql):
//!   row_count = 172
//!   sha256    = 985b249c6cba0a680721cd547f710951dcf5695f18ff4b19b53ce42b90ef8f5b
//!   elapsed   ≤ 300s (per #4380 acceptance criterion)
//!
//! Run:
//!   TPCH_SF1_DIR=/tmp/tpch-sf1 cargo test --release \\
//!     --test q20_potential_part_promotion_perf --all-features \\
//!     -- --ignored --nocapture q20_potential_part_promotion_sf1

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::Arc;
use std::time::{Duration, Instant};

const DATA_DIR: &str = "/tmp/tpch-sf1";

const Q20_SQL: &str = "SELECT s_name, s_address \
                       FROM supplier, nation \
                       WHERE s_nationkey = n_nationkey \
                         AND n_name = 'GERMANY' \
                         AND EXISTS (SELECT * FROM partsupp \
                                      WHERE ps_suppkey = s_suppkey \
                                        AND ps_partkey IN (SELECT p_partkey FROM part WHERE p_name LIKE 'forest%') \
                                        AND ps_availqty > (SELECT 0.5 * SUM(l_quantity) \
                                                             FROM lineitem \
                                                             WHERE l_partkey = ps_partkey \
                                                               AND l_suppkey = ps_suppkey \
                                                               AND l_shipdate >= '1994-01-01' \
                                                               AND l_shipdate < '1995-01-01')) \
                       ORDER BY s_name";

const SCHEMAS: &[&str] = &[
    "CREATE TABLE nation (n_nationkey INTEGER PRIMARY KEY, n_name TEXT NOT NULL, n_regionkey INTEGER NOT NULL, n_comment TEXT)",
    "CREATE TABLE supplier (s_suppkey INTEGER PRIMARY KEY, s_name TEXT NOT NULL, s_address TEXT NOT NULL, s_nationkey INTEGER NOT NULL, s_phone TEXT NOT NULL, s_acctbal REAL NOT NULL, s_comment TEXT)",
    "CREATE TABLE part (p_partkey INTEGER PRIMARY KEY, p_name TEXT NOT NULL, p_mfgr TEXT NOT NULL, p_brand TEXT NOT NULL, p_type TEXT NOT NULL, p_size INTEGER NOT NULL, p_container TEXT NOT NULL, p_retailprice REAL NOT NULL, p_comment TEXT)",
    "CREATE TABLE partsupp (ps_partkey INTEGER NOT NULL, ps_suppkey INTEGER NOT NULL, ps_availqty INTEGER NOT NULL, ps_supplycost REAL NOT NULL, ps_comment TEXT, PRIMARY KEY (ps_partkey, ps_suppkey))",
    "CREATE TABLE lineitem (l_orderkey INTEGER NOT NULL, l_partkey INTEGER NOT NULL, l_suppkey INTEGER NOT NULL, l_linenumber INTEGER NOT NULL, l_quantity INTEGER NOT NULL, l_extendedprice REAL NOT NULL, l_discount REAL NOT NULL, l_tax REAL NOT NULL, l_returnflag TEXT NOT NULL, l_linestatus TEXT NOT NULL, l_shipdate TEXT NOT NULL, l_commitdate TEXT NOT NULL, l_receiptdate TEXT NOT NULL, l_shipinstruct TEXT NOT NULL, l_shipmode TEXT NOT NULL, l_comment TEXT NOT NULL)",
];

const TBL_FILES: &[&str] = &["nation", "supplier", "part", "partsupp", "lineitem"];

/// Per-query wall-clock budget (issue #4380 acceptance criterion).
const TIMEOUT_BUDGET: Duration = Duration::from_secs(300);

/// Expected row count from the SQLite oracle at SF=1.
/// sha256 = 985b249c6cba0a680721cd547f710951dcf5695f18ff4b19b53ce42b90ef8f5b
const EXPECTED_ROW_COUNT: usize = 172;

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
#[ignore] // Heavy: SF=1 — lineitem 6M, partsupp 800K, supplier 10K
fn q20_potential_part_promotion_sf1() {
    let mut engine = setup();

    let start = Instant::now();
    let r = engine
        .execute(Q20_SQL)
        .unwrap_or_else(|e| panic!("Q20 failed: {}", e));
    let elapsed = start.elapsed();
    eprintln!("Q20 elapsed: {:?}", elapsed);

    assert!(
        elapsed <= TIMEOUT_BUDGET,
        "Q20 elapsed {:?} exceeds TIMEOUT_BUDGET {:?} — correlated subqueries \
         not decorrelated",
        elapsed, TIMEOUT_BUDGET
    );

    assert_eq!(
        r.rows.len(),
        EXPECTED_ROW_COUNT,
        "Q20 row count must match SQLite oracle (expected {}, got {})",
        EXPECTED_ROW_COUNT,
        r.rows.len()
    );
}