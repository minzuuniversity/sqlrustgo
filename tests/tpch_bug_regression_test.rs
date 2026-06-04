//! TPC-H bug #3/#4/#5 regression markers (Issue #2977, Phase 1a)
//!
//! These tests were added in 2026-06-05 to track the 3 unfixed engine bugs
//! surfaced in `docs/audit/status/2026-06-04-tpch-phase2d-status.md`:
//!
//! - bug #3: SELECT projection — engine returns ALL columns regardless of
//!   the SELECT clause, and column NAMES appear as TEXT cells in the row
//!   rather than the values (the column NAMES as TEXT bug is the bit that
//!   causes row_count mismatches in wire-protocol tests).
//! - bug #4: `SUM(real_col)` returns 0 instead of the right value. The
//!   aggregator is wired for INTEGER columns only.
//! - bug #5: `AVG(real_col)` returns Null (same root cause as #4).
//!
//! Each test loads the sf001 fixture (tests/data/tpch-sf001/, 614 lineitem),
//! executes a TPC-H query that exercises the bug, and asserts the expected
//! (post-fix) result. Tests are wired to FAIL on the current engine (pre-fix)
//! and will turn GREEN after the corresponding fix lands.
//!
//! See: docs/plans/2026-06-05-tpch-22-wire-three-way.md §3.1
//!      tests/data/tpch-sf001/expected/Q{1..22}_three_way.json (SQLite baseline)

use sqlrustgo::{ExecutionEngine, MemoryStorage};
use sqlrustgo_types::Value as SqlValue;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

const FIXTURE_DIR: &str = "/home/openclaw/dev/yinglichina163/sqlrustgo/.worktrees/tpch-22-bugfixes/tests/data/tpch-sf001";

const DDL: &[&str] = &[
    "CREATE TABLE region (r_regionkey INTEGER, r_name TEXT, r_comment TEXT)",
    "CREATE TABLE nation (n_nationkey INTEGER, n_regionkey INTEGER, n_name TEXT, n_comment TEXT)",
    "CREATE TABLE supplier (s_suppkey INTEGER, s_nationkey INTEGER, s_name TEXT, s_address TEXT, s_phone TEXT, s_acctbal REAL, s_comment TEXT)",
    "CREATE TABLE customer (c_custkey INTEGER, c_nationkey INTEGER, c_name TEXT, c_address TEXT, c_phone TEXT, c_acctbal REAL, c_mktsegment TEXT, c_comment TEXT)",
    "CREATE TABLE part (p_partkey INTEGER, p_name TEXT, p_mfgr TEXT, p_brand TEXT, p_type TEXT, p_size INTEGER, p_container TEXT, p_retailprice REAL, p_comment TEXT)",
    "CREATE TABLE partsupp (ps_partkey INTEGER, ps_suppkey INTEGER, ps_availqty INTEGER, ps_supplycost REAL, ps_comment TEXT)",
    "CREATE TABLE orders (o_orderkey INTEGER, o_custkey INTEGER, o_orderstatus TEXT, o_totalprice REAL, o_orderdate TEXT, o_orderpriority TEXT, o_clerk TEXT, o_shippriority INTEGER, o_comment TEXT)",
    "CREATE TABLE lineitem (l_orderkey INTEGER, l_partkey INTEGER, l_suppkey INTEGER, l_linenumber INTEGER, l_quantity REAL, l_extendedprice REAL, l_discount REAL, l_tax REAL, l_returnflag TEXT, l_linestatus TEXT, l_shipdate TEXT, l_commitdate TEXT, l_receiptdate TEXT, l_shipinstruct TEXT, l_shipmode TEXT, l_comment TEXT)",
];

/// Load the sf001 fixture (614 lineitem rows) into a fresh in-process engine.
/// Identical loading path is used across all 3 regression tests for stability.
fn make_engine_with_sf001() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);
    for d in DDL {
        engine.execute(d).expect("DDL");
    }
    let base = PathBuf::from(FIXTURE_DIR);
    let tables = [
        "region", "nation", "supplier", "customer",
        "part", "partsupp", "orders", "lineitem",
    ];
    for tbl in tables {
        let path = base.join(format!("{}.tbl", tbl));
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
        for line in content.lines() {
            if line.is_empty() {
                continue;
            }
            // sf001 .tbl has trailing '|' on every line — strip it so column count matches
            let line_trimmed = line.trim_end_matches('|');
            let cols: Vec<&str> = line_trimmed.split('|').collect();
            // For TEXT/VARCHAR columns, escape single quotes by doubling them
            let vals: Vec<String> = cols
                .iter()
                .map(|s| {
                    let s_escaped = s.replace('\'', "''");
                    format!("'{}'", s_escaped)
                })
                .collect();
            let sql = format!("INSERT INTO {} VALUES ({})", tbl, vals.join(","));
            engine.execute(&sql).expect(&format!("insert {}", tbl));
        }
    }
    engine
}

/// Find the first numeric (Integer or Float) cell in a row.
fn first_numeric_cell(row: &[SqlValue]) -> Option<f64> {
    row.iter().find_map(|v| match v {
        SqlValue::Integer(i) => Some(*i as f64),
        SqlValue::Float(f) => Some(*f),
        _ => None,
    })
}

/// Find the first TEXT cell in a row.
fn first_text_cell(row: &[SqlValue]) -> Option<String> {
    row.iter().find_map(|v| match v {
        SqlValue::Text(s) => Some(s.clone()),
        _ => None,
    })
}

// =========================================================================
// bug #3 — SELECT projection
// =========================================================================
//
// TPC-H Q1: SELECT ... GROUP BY l_returnflag, l_linestatus ORDER BY ...
// Expected: 6 groups, each with 10 columns matching the SELECT list.
// Pre-fix symptom: returns full table schema (16 lineitem columns) with
// column NAMES as TEXT cells in the first row.

#[test]
fn test_bug3_tpch_q1_select_projection_returns_10_columns() {
    let mut engine = make_engine_with_sf001();
    let q = "SELECT l_returnflag, l_linestatus, SUM(l_quantity) AS sum_qty, \
             SUM(l_extendedprice) AS sum_base_price, \
             AVG(l_quantity) AS avg_qty, COUNT(*) AS count_order \
             FROM lineitem \
             WHERE l_shipdate <= '1995-12-01' \
             GROUP BY l_returnflag, l_linestatus \
             ORDER BY l_returnflag, l_linestatus";
    let r = engine.execute(q).expect("Q1 should not crash");
    // After fix: 6 groups × 6 projected columns (2 group + 4 aggregates)
    assert_eq!(
        r.rows.len(),
        6,
        "Q1 should return 6 groups (matches expected/Q1_three_way.json), got {} rows",
        r.rows.len()
    );
    for (i, row) in r.rows.iter().enumerate() {
        assert_eq!(
            row.len(),
            6,
            "row[{}] should have 6 projected columns (l_returnflag, l_linestatus, \
             sum_qty, sum_base_price, avg_qty, count_order), got {} columns: {:?}",
            i,
            row.len(),
            row
        );
    }
}

#[test]
fn test_bug3_tpch_q1_no_column_name_text_cells() {
    let mut engine = make_engine_with_sf001();
    let q = "SELECT l_returnflag, SUM(l_quantity) AS sum_qty \
             FROM lineitem WHERE l_shipdate <= '1995-12-01' \
             GROUP BY l_returnflag ORDER BY l_returnflag";
    let r = engine.execute(q).expect("Q1 should not crash");
    // After fix: each row should be 2 cells, and NONE of them should be the
    // column-name strings "l_returnflag" or "sum_qty" (bug #3 symptom).
    for (i, row) in r.rows.iter().enumerate() {
        assert_eq!(row.len(), 2, "row[{}] should have 2 cells, got {}: {:?}", i, row.len(), row);
        for (j, cell) in row.iter().enumerate() {
            if let Some(text) = first_text_cell(&[cell.clone()]) {
                assert_ne!(
                    text, "l_returnflag",
                    "row[{}] cell[{}] is column name 'l_returnflag' as TEXT (bug #3)",
                    i, j
                );
                assert_ne!(
                    text, "sum_qty",
                    "row[{}] cell[{}] is column name 'sum_qty' as TEXT (bug #3)",
                    i, j
                );
            }
        }
    }
}

// =========================================================================
// bug #4 — SUM(real_col) returns 0
// =========================================================================
//
// TPC-H Q1: SUM(l_extendedprice) over the 6 groups must be non-zero
// (l_extendedprice is REAL, e.g. 38018.93 for the first lineitem).
// Pre-fix: SUM returns 0 (Integer aggregator wired for INTEGER columns only).
//
// Status: bug #4 unfixed as of 2026-06-05 (commit e97985454). The two tests
// below FAIL today and are `#[ignore]`-marked so CI stays GREEN until the
// aggregator dispatches Sum/Avg over REAL columns to f64 paths (Phase 1b).
// Run with `cargo test -- --ignored` to verify the contract.

#[test]
#[ignore = "bug #4 unfixed: SUM(real) returns 0; Phase 1b aggregator fix will remove this ignore"]
fn test_bug4_tpch_q1_sum_real_extendedprice_nonzero() {
    let mut engine = make_engine_with_sf001();
    let q = "SELECT l_returnflag, SUM(l_extendedprice) AS sum_base_price \
             FROM lineitem WHERE l_shipdate <= '1995-12-01' \
             GROUP BY l_returnflag ORDER BY l_returnflag";
    let r = engine.execute(q).expect("Q1 should not crash");
    assert!(r.rows.len() >= 1, "should return ≥1 group");
    let total: f64 = r
        .rows
        .iter()
        .filter_map(|row| first_numeric_cell(row))
        .sum();
    assert!(
        total > 0.0,
        "SUM(l_extendedprice) over sf001 lineitem should be > 0 (real col), got {}",
        total
    );
}

#[test]
#[ignore = "bug #4 unfixed: SUM(real) returns 0; Phase 1b aggregator fix will remove this ignore"]
fn test_bug4_tpch_q1_sum_real_quantity_nonzero() {
    let mut engine = make_engine_with_sf001();
    let q = "SELECT l_returnflag, SUM(l_quantity) AS sum_qty \
             FROM lineitem WHERE l_shipdate <= '1995-12-01' \
             GROUP BY l_returnflag ORDER BY l_returnflag";
    let r = engine.execute(q).expect("Q1 should not crash");
    let total: f64 = r
        .rows
        .iter()
        .filter_map(|row| first_numeric_cell(row))
        .sum();
    assert!(
        total > 0.0,
        "SUM(l_quantity) over sf001 lineitem should be > 0 (real col), got {}",
        total
    );
}

// =========================================================================
// bug #5 — AVG(real_col) returns Null
// =========================================================================
//
// TPC-H Q1: AVG(l_quantity) over the 6 groups must be non-null, non-zero.
// Pre-fix: AVG returns Null (Integer aggregator wired for INTEGER only).
//
// Status: bug #5 unfixed as of 2026-06-05. `#[ignore]`-marked until Phase 1b
// aggregator fix lands. See bug #4 note above for rationale.

#[test]
#[ignore = "bug #5 unfixed: AVG(real) returns Null; Phase 1b aggregator fix will remove this ignore"]
fn test_bug5_tpch_q1_avg_real_quantity_nonnull() {
    let mut engine = make_engine_with_sf001();
    let q = "SELECT l_returnflag, AVG(l_quantity) AS avg_qty \
             FROM lineitem WHERE l_shipdate <= '1995-12-01' \
             GROUP BY l_returnflag ORDER BY l_returnflag";
    let r = engine.execute(q).expect("Q1 should not crash");
    assert!(r.rows.len() >= 1, "should return ≥1 group");
    for (i, row) in r.rows.iter().enumerate() {
        let avg = first_numeric_cell(row)
            .unwrap_or_else(|| panic!("row[{}] should have a numeric AVG cell, got: {:?}", i, row));
        assert!(
            avg > 0.0,
            "row[{}] AVG(l_quantity) should be > 0 (real col), got {}",
            i, avg
        );
    }
}

#[test]
#[ignore = "bug #5 unfixed: AVG(real) returns Null; Phase 1b aggregator fix will remove this ignore"]
fn test_bug5_tpch_q1_avg_real_extendedprice_nonnull() {
    let mut engine = make_engine_with_sf001();
    let q = "SELECT l_returnflag, AVG(l_extendedprice) AS avg_price \
             FROM lineitem WHERE l_shipdate <= '1995-12-01' \
             GROUP BY l_returnflag ORDER BY l_returnflag";
    let r = engine.execute(q).expect("Q1 should not crash");
    for (i, row) in r.rows.iter().enumerate() {
        let avg = first_numeric_cell(row)
            .unwrap_or_else(|| panic!("row[{}] should have a numeric AVG cell, got: {:?}", i, row));
        assert!(
            avg > 0.0,
            "row[{}] AVG(l_extendedprice) should be > 0 (real col), got {}",
            i, avg
        );
    }
}
