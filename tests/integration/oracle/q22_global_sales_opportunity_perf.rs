//! Performance regression test for V312-58 / Issue #4381: TPC-H SF=1
//! Q22 global-sales-opportunity.
//!
//! Q22 has two correlated subqueries on the customer table:
//!
//! ```sql
//! SELECT cntrycode, COUNT(*) AS numcust, SUM(c_acctbal) AS totacctbal
//! FROM (
//!   SELECT SUBSTR(c_phone, 1, 2) AS cntrycode, c_acctbal
//!   FROM customer
//!   WHERE SUBSTR(c_phone, 1, 2) IN ('13', '31', '23', '29', '30', '18', '17')
//!     AND c_acctbal > (SELECT AVG(c_acctbal)
//!                        FROM customer
//!                        WHERE c_acctbal > 0.00
//!                          AND SUBSTR(c_phone, 1, 2) IN ('13', '31', '23', '29', '30', '18', '17'))
//!     AND NOT EXISTS (SELECT * FROM orders WHERE o_custkey = c_custkey)
//! ) AS custsale
//! GROUP BY cntrycode
//! ORDER BY cntrycode;
//! ```
//!
//! 1. **Correlated scalar subquery** (c_acctbal > AVG) re-evaluated per
//!    outer row without materialization.
//! 2. **Correlated NOT EXISTS** subquery (anti-semi-join) re-evaluated
//!    per customer row without anti-join rewrite.
//!
//! Both compound → TIMEOUT (>1800s).
//!
//! Expected baseline (SQLite oracle, SF=1, queries/q22.sql):
//!   row_count = 7
//!   sha256    = 10ad4c38efadc947b75aeb9acea4222ce3e6ea6720727cc6164af6481fc3a6b2
//!   elapsed   ≤ 300s (per #4381 acceptance criterion)
//!
//! Run:
//!   TPCH_SF1_DIR=/tmp/tpch-sf1 cargo test --release \\
//!     --test q22_global_sales_opportunity_perf --all-features \\
//!     -- --ignored --nocapture q22_global_sales_opportunity_sf1

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::Arc;
use std::time::{Duration, Instant};

const DATA_DIR: &str = "/tmp/tpch-sf1";

const Q22_SQL: &str = "SELECT cntrycode, COUNT(*) AS numcust, SUM(c_acctbal) AS totacctbal \
                       FROM (SELECT SUBSTR(c_phone, 1, 2) AS cntrycode, c_acctbal \
                             FROM customer \
                             WHERE SUBSTR(c_phone, 1, 2) IN ('13', '31', '23', '29', '30', '18', '17') \
                               AND c_acctbal > (SELECT AVG(c_acctbal) \
                                                FROM customer \
                                                WHERE c_acctbal > 0.00 \
                                                  AND SUBSTR(c_phone, 1, 2) IN ('13', '31', '23', '29', '30', '18', '17')) \
                               AND NOT EXISTS (SELECT * FROM orders WHERE o_custkey = c_custkey)) AS custsale \
                       GROUP BY cntrycode \
                       ORDER BY cntrycode";

const SCHEMAS: &[&str] = &[
    "CREATE TABLE customer (c_custkey INTEGER PRIMARY KEY, c_name TEXT NOT NULL, c_address TEXT NOT NULL, c_nationkey INTEGER NOT NULL, c_phone TEXT NOT NULL, c_acctbal REAL NOT NULL, c_mktsegment TEXT, c_comment TEXT)",
    "CREATE TABLE orders (o_orderkey INTEGER PRIMARY KEY, o_custkey INTEGER NOT NULL, o_orderstatus TEXT NOT NULL, o_totalprice REAL NOT NULL, o_orderdate TEXT NOT NULL, o_orderpriority TEXT, o_clerk TEXT, o_shippriority INTEGER, o_comment TEXT)",
];

const TBL_FILES: &[&str] = &["customer", "orders"];

/// Per-query wall-clock budget (issue #4381 acceptance criterion).
const TIMEOUT_BUDGET: Duration = Duration::from_secs(300);

/// Expected row count from the SQLite oracle at SF=1.
/// sha256 = 10ad4c38efadc947b75aeb9acea4222ce3e6ea6720727cc6164af6481fc3a6b2
const EXPECTED_ROW_COUNT: usize = 7;

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
#[ignore] // Heavy: SF=1 — customer 150K, orders 1.5M
fn q22_global_sales_opportunity_sf1() {
    let mut engine = setup();

    let start = Instant::now();
    let r = engine
        .execute(Q22_SQL)
        .unwrap_or_else(|e| panic!("Q22 failed: {}", e));
    let elapsed = start.elapsed();
    eprintln!("Q22 elapsed: {:?}", elapsed);

    assert!(
        elapsed <= TIMEOUT_BUDGET,
        "Q22 elapsed {:?} exceeds TIMEOUT_BUDGET {:?} — correlated subqueries \
         not decorrelated",
        elapsed, TIMEOUT_BUDGET
    );

    assert_eq!(
        r.rows.len(),
        EXPECTED_ROW_COUNT,
        "Q22 row count must match SQLite oracle (expected {}, got {})",
        EXPECTED_ROW_COUNT,
        r.rows.len()
    );
}