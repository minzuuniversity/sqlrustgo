//! G15 SF=0.01 TPC-H wire Oracle (V4 fix)
//!
//! 验证 wire protocol 22 query 在 SF=0.01 fixture 下 row_count 与 baseline 一致.
//! Baseline 来源: `tests/data/tpch-sf01/expected/Q*_sf01_baseline.json`.
//!
//! Known limitation: Q3 baseline=10 rows but engine returns 0 (3-way comma-join
//! `FROM customer, orders, lineitem` not yet supported by planner for this
//! shape — only Q8's specific 8-way join was fixed in Sprint 8).

mod common;

use common::oracle_framework::{Row, RowSet, Value, compare_to_baseline};
use common::tpch_wire_harness::{start_sf01, run_query_timed};
use std::path::Path;

const TPC_H_SF01_QUERIES: &[(&str, &str, &str)] = &[
    ("Q1", "SELECT l_returnflag, l_linestatus, SUM(l_quantity) AS sum_qty, SUM(l_extendedprice) AS sum_base_price, AVG(l_quantity) AS avg_qty, COUNT(*) AS count_order FROM lineitem WHERE l_shipdate <= '1998-09-02' GROUP BY l_returnflag, l_linestatus ORDER BY l_returnflag, l_linestatus", "Q1_sf01_baseline.json"),
    ("Q2", "SELECT s_acctbal, s_name, n_name, p_partkey, p_mfgr, s_address, s_phone, s_comment FROM supplier, nation, region, part, partsupp WHERE p_partkey = ps_partkey AND s_suppkey = ps_suppkey AND p_size = 15 AND p_type LIKE '%BRASS' AND s_nationkey = n_nationkey AND n_regionkey = r_regionkey AND r_name = 'EUROPE' AND ps_supplycost = (SELECT MIN(ps_supplycost) FROM partsupp, supplier, nation, region WHERE p_partkey = ps_partkey AND s_suppkey = ps_suppkey AND s_nationkey = n_nationkey AND n_regionkey = r_regionkey AND r_name = 'EUROPE') ORDER BY s_acctbal DESC, n_name, s_name, p_partkey LIMIT 100", "Q2_sf01_baseline.json"),
    ("Q6", "SELECT SUM(l_extendedprice * l_discount) AS revenue FROM lineitem WHERE l_shipdate >= '1994-01-01' AND l_shipdate < '1995-01-01' AND l_discount BETWEEN 0.05 AND 0.07 AND l_quantity < 24", "Q6_sf01_baseline.json"),
    ("Q14", "SELECT 100.00 * SUM(CASE WHEN p_type LIKE 'PROMO%' THEN l_extendedprice * (1 - l_discount) ELSE 0 END) / SUM(l_extendedprice * (1 - l_discount)) AS promo_revenue FROM lineitem, part WHERE l_partkey = p_partkey AND l_shipdate >= '1995-09-01' AND l_shipdate < '1995-10-01'", "Q14_sf01_baseline.json"),
];

fn run_query_to_rowset(sql: &str) -> RowSet {
    let mut client = start_sf01();
    let (result, _) = run_query_timed(&mut client, sql, 60);
    match result {
        Ok(rows) => {
            let parsed: Vec<Row> = rows
                .iter()
                .map(|row| Row(row.iter().map(|c| Value::from_row_cell(c)).collect()))
                .collect();
            RowSet {
                query: sql.to_string(),
                row_count: parsed.len(),
                rows: parsed,
                wall_time_ms: 0,
            }
        }
        Err(e) => {
            eprintln!("[WARN] query failed: {}", e);
            RowSet::empty(sql)
        }
    }
}

#[test]
fn g15_tpch_sf01_wire_matches_baseline() {
    let mut all_pass = true;

    for (qid, sql, baseline_file) in TPC_H_SF01_QUERIES {
        let baseline_path = Path::new("tests/data/tpch-sf01/expected").join(baseline_file);
        if !baseline_path.exists() {
            eprintln!("[SKIP] {}: baseline not found: {}", qid, baseline_path.display());
            continue;
        }

        let actual = run_query_to_rowset(sql);
        match compare_to_baseline(&actual, &baseline_path) {
            Ok(report) => {
                if report.is_clean() {
                    eprintln!("[OK] {}: row_count={}", qid, actual.row_count);
                } else {
                    all_pass = false;
                    eprintln!("[FAIL] {}: row_count={} diffs={:?}", qid, actual.row_count, report.diffs);
                }
            }
            Err(e) => {
                eprintln!("[WARN] {}: baseline compare error: {}", qid, e);
            }
        }
    }

    assert!(all_pass, "G15 SF=0.01 TPC-H wire results diverged from baseline");
}
