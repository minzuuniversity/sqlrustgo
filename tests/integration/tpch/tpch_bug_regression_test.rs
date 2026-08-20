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
//! Each test starts the wire server, loads sf001 fixture via LOAD DATA,
//! executes a TPC-H query that exercises the bug, and asserts the expected
//! (post-fix) result.
//!
//! Migration: migrated from in-process ExecutionEngine to wire protocol
//! MySqlTestClient (start_sf001 + client.query_rows).
//!
//! See: docs/plans/2026-06-05-tpch-22-wire-three-way.md §3.1

#[path = "../../common/mod.rs"]
mod common;
use common::tpch_wire_harness::start_sf001;

/// Parse a numeric cell from a string. Returns None if not parseable as f64.
fn first_numeric_cell_str(row: &[String]) -> Option<f64> {
    for cell in row {
        if let Ok(v) = cell.parse::<f64>() {
            return Some(v);
        }
    }
    None
}

/// Parse a TEXT cell from a string (returns first non-numeric cell).
fn first_text_cell_str(row: &[String]) -> Option<String> {
    for cell in row {
        if cell.parse::<f64>().is_err() {
            return Some(cell.clone());
        }
    }
    None
}

// =========================================================================
// bug #3 — SELECT projection
// =========================================================================
//
// TPC-H Q1: SELECT ... GROUP BY l_returnflag, l_linestatus ORDER BY ...
// Expected: 4 groups, each with 6 projected columns.
// Pre-fix symptom: returns full table schema (16 lineitem columns) with
// column NAMES as TEXT cells in the first row.

#[test]
fn test_bug3_tpch_q1_select_projection_returns_4_rows() {
    let mut client = start_sf001();
    let q = "SELECT l_returnflag, l_linestatus, SUM(l_quantity) AS sum_qty, \
             SUM(l_extendedprice) AS sum_base_price, \
             AVG(l_quantity) AS avg_qty, COUNT(*) AS count_order \
             FROM lineitem \
             WHERE l_shipdate <= '1998-09-02' \
             GROUP BY l_returnflag, l_linestatus \
             ORDER BY l_returnflag, l_linestatus";
    let rows = client.query_rows(q).expect("Q1 should not crash");
    // After fix: 4 groups (4 distinct (flag, status) combos in sf001 data)
    assert_eq!(
        rows.len(),
        4,
        "Q1 should return 4 groups, got {} rows",
        rows.len()
    );
    for (i, row) in rows.iter().enumerate() {
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
    let mut client = start_sf001();
    let q = "SELECT l_returnflag, SUM(l_quantity) AS sum_qty \
             FROM lineitem WHERE l_shipdate <= '1998-09-02' \
             GROUP BY l_returnflag ORDER BY l_returnflag";
    let rows = client.query_rows(q).expect("Q1 should not crash");
    // After fix: each row should be 2 cells, and NONE of them should be
    // the column-name strings "l_returnflag" or "sum_qty" (bug #3 symptom).
    for (i, row) in rows.iter().enumerate() {
        assert_eq!(
            row.len(),
            2,
            "row[{}] should have 2 cells, got {}: {:?}",
            i,
            row.len(),
            row
        );
        for (j, _cell) in row.iter().enumerate() {
            if let Some(text) = first_text_cell_str(row) {
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
// TPC-H Q1: SUM(l_extendedprice) over sf001 lineitem must be non-zero
// (l_extendedprice is REAL, e.g. 38018.93 for the first lineitem).
//
// Root cause (re-audited 2026-06-05): the test fixture loader was
// wrapping every value in single quotes ('38018.93'), causing the parser
// to store it as Value::Text rather than Value::Float. The Sum
// aggregator (which only operates on Integer/Float) then skipped
// the Text cell, returning 0.
//
// Wire protocol via LOAD DATA LOCAL INFILE uses the raw .tbl file
// without any quoting, so numeric columns are stored correctly.

#[test]
fn test_bug4_tpch_q1_sum_real_extendedprice_nonzero() {
    let mut client = start_sf001();
    let q = "SELECT l_returnflag, SUM(l_extendedprice) AS sum_base_price \
             FROM lineitem WHERE l_shipdate <= '1998-09-02' \
             GROUP BY l_returnflag ORDER BY l_returnflag";
    let rows = client.query_rows(q).expect("Q1 should not crash");
    assert!(!rows.is_empty(), "should return ≥1 group");
    let total: f64 = rows
        .iter()
        .filter_map(|row| first_numeric_cell_str(row))
        .sum();
    assert!(
        total > 0.0,
        "SUM(l_extendedprice) over sf001 lineitem should be > 0 (real col), got {}",
        total
    );
}

#[test]
fn test_bug4_tpch_q1_sum_real_quantity_nonzero() {
    let mut client = start_sf001();
    let q = "SELECT l_returnflag, SUM(l_quantity) AS sum_qty \
             FROM lineitem WHERE l_shipdate <= '1998-09-02' \
             GROUP BY l_returnflag ORDER BY l_returnflag";
    let rows = client.query_rows(q).expect("Q1 should not crash");
    let total: f64 = rows
        .iter()
        .filter_map(|row| first_numeric_cell_str(row))
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
// TPC-H Q1: AVG(l_quantity) over sf001 lineitem must be non-null, non-zero.
//
// Same root cause as bug #4 (test fixture loader wrapping numerics in
// quotes → Value::Text instead of Value::Float → AVG skipped).

#[test]
fn test_bug5_tpch_q1_avg_real_quantity_nonnull() {
    let mut client = start_sf001();
    let q = "SELECT l_returnflag, AVG(l_quantity) AS avg_qty \
             FROM lineitem WHERE l_shipdate <= '1998-09-02' \
             GROUP BY l_returnflag ORDER BY l_returnflag";
    let rows = client.query_rows(q).expect("Q1 should not crash");
    assert!(!rows.is_empty(), "should return ≥1 group");
    for (i, row) in rows.iter().enumerate() {
        let avg = first_numeric_cell_str(row)
            .unwrap_or_else(|| panic!("row[{}] should have a numeric AVG cell, got: {:?}", i, row));
        assert!(
            avg > 0.0,
            "row[{}] AVG(l_quantity) should be > 0 (real col), got {}",
            i,
            avg
        );
    }
}

#[test]
fn test_bug5_tpch_q1_avg_real_extendedprice_nonnull() {
    let mut client = start_sf001();
    let q = "SELECT l_returnflag, AVG(l_extendedprice) AS avg_price \
             FROM lineitem WHERE l_shipdate <= '1998-09-02' \
             GROUP BY l_returnflag ORDER BY l_returnflag";
    let rows = client.query_rows(q).expect("Q1 should not crash");
    for (i, row) in rows.iter().enumerate() {
        let avg = first_numeric_cell_str(row)
            .unwrap_or_else(|| panic!("row[{}] should have a numeric AVG cell, got: {:?}", i, row));
        assert!(
            avg > 0.0,
            "row[{}] AVG(l_extendedprice) should be > 0 (real col), got {}",
            i,
            avg
        );
    }
}
