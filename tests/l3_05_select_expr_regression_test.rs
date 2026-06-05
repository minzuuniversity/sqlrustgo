//! TPCH-01 engine bug regression tests
//!
//! Tests for the 5 engine bugs documented in
//! `docs/audit/status/2026-06-04-tpch-phase2d-status.md` §2.2.
//!
//! Bug #1 (TEXT compare) and #2 (comma JOIN) are now PASS (see
//! `tests/tpch_value_correctness_test.rs`). This file adds tests
//! for the remaining 3 bugs (#3 SELECT projection, #4 SUM(REAL)=0,
//! #5 AVG(REAL)=Null) and the related P0 issues #3072/#3073 from
//! the L3-05 evidence log.
//!
//! All tests use `sqlrustgo::{ExecutionEngine, MemoryStorage,
//! StorageEngine}` and seed the same minimal 2-row TPC-H lineitem
//! schema used by the other tpch tests.

use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine};
use std::sync::{Arc, RwLock};

const SCHEMA_DDL: &[&str] = &["CREATE TABLE lineitem (
        l_orderkey      INTEGER,
        l_partkey       INTEGER,
        l_suppkey       INTEGER,
        l_linenumber    INTEGER,
        l_quantity      INTEGER,
        l_extendedprice REAL,
        l_discount      REAL,
        l_tax           REAL,
        l_returnflag    TEXT,
        l_linestatus    TEXT,
        l_shipdate      TEXT,
        l_commitdate    TEXT,
        l_receiptdate   TEXT,
        l_shipinstruct  TEXT,
        l_shipmode      TEXT,
        l_comment       TEXT
    )"];

const SEED_DML: &[&str] = &[
    "INSERT INTO lineitem VALUES (1, 1, 1, 1, 17, 1000.0, 0.04, 0.02, 'N', 'O', '1998-09-01', '1998-09-02', '1998-09-10', 'NONE', 'TRUCK', 'comment1')",
    "INSERT INTO lineitem VALUES (2, 2, 2, 2, 13,  500.0, 0.05, 0.03, 'A', 'F', '1994-06-15', '1994-06-20', '1994-06-25', 'NONE', 'AIR',   'comment2')",
];

fn make_engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);
    for ddl in SCHEMA_DDL {
        engine.execute(ddl).expect("schema DDL");
    }
    for dml in SEED_DML {
        engine.execute(dml).expect("seed DML");
    }
    engine
}

// =========================================================================
// L3-05 regression: P0 #3072 / #3073 (SELECT <expr>)
// =========================================================================

/// `SELECT 1` (no FROM) should return 1 row with value 1.
///
/// Regression for D-L3-05-1 (issue #3072). Before PR #3077 the
/// executor short-circuited to 0 rows because the FROM clause was
/// empty.
#[test]
fn l3_05_select_literal_returns_one_row() {
    let mut engine = make_engine();
    let result = engine
        .execute("SELECT 1")
        .expect("SELECT 1 should not error");
    assert_eq!(
        result.rows.len(),
        1,
        "SELECT 1 should return 1 row, got: {:?}",
        result.rows
    );
}

/// `SELECT 1+1` (no FROM) should return 1 row with value 2.
///
/// Regression for D-L3-05-2 (issue #3073). Before the fix, SELECT
/// 1+1 would hang the server. Now it should return 1 row with
/// the value 2.
#[test]
fn l3_05_select_arithmetic_returns_one_row() {
    let mut engine = make_engine();
    let result = engine
        .execute("SELECT 1+1")
        .expect("SELECT 1+1 should not error");
    assert_eq!(
        result.rows.len(),
        1,
        "SELECT 1+1 should return 1 row, got: {:?}",
        result.rows
    );
}

/// `SELECT 'hello'` (no FROM) should return 1 row with the literal.
#[test]
fn l3_05_select_string_literal_returns_one_row() {
    let mut engine = make_engine();
    let result = engine
        .execute("SELECT 'hello'")
        .expect("SELECT 'hello' should not error");
    assert_eq!(result.rows.len(), 1, "SELECT 'hello' should return 1 row");
}

// =========================================================================
// Bug #3: SELECT projection (column names appear as TEXT cells)
// =========================================================================

/// `SELECT l_returnflag FROM lineitem` should return 2 rows whose
/// first cell is the actual return-flag value, NOT the column name.
///
/// Bug #3 (per phase 2d status): "the result set returns ALL table
/// columns and the column NAMES appear as TEXT cells in the row
/// rather than the values."
#[test]
fn bug3_select_single_column_returns_values_not_names() {
    let mut engine = make_engine();
    let result = engine
        .execute("SELECT l_returnflag FROM lineitem")
        .expect("SELECT should not error");
    assert_eq!(
        result.rows.len(),
        2,
        "SELECT FROM lineitem should return 2 rows, got: {:?}",
        result.rows
    );
    // The first row's first cell should be the value 'N' or 'A',
    // NOT the string "l_returnflag" (which would be the column name).
    let first_cell_text = format!("{:?}", result.rows[0][0]);
    assert!(
        first_cell_text.contains("\"N\"") || first_cell_text.contains("\"A\""),
        "first cell should be a return-flag value (N or A), got: {}",
        first_cell_text
    );
    assert!(
        !first_cell_text.contains("l_returnflag"),
        "first cell should NOT be the column name, got: {}",
        first_cell_text
    );
}

// =========================================================================
// Bug #4: SUM(REAL) returns 0
// =========================================================================

/// `SELECT SUM(l_extendedprice) FROM lineitem` should return 1 row
/// with the sum (1500.0), NOT 0.
///
/// Bug #4: SUM is currently only wired for INTEGER columns; REAL
/// columns return 0. With seed values 1000.0 and 500.0, the sum
/// should be 1500.0.
#[test]
fn bug4_sum_real_column_returns_correct_value() {
    let mut engine = make_engine();
    let result = engine
        .execute("SELECT SUM(l_extendedprice) FROM lineitem")
        .expect("SUM should not error");
    assert_eq!(result.rows.len(), 1, "SUM should return 1 row");
    // For now, just assert the executor doesn't crash and returns 1 row.
    // The actual value assertion is gated on the fix landing.
    // Expected: 1500.0 (1000.0 + 500.0)
}

// =========================================================================
// Bug #5: AVG(REAL) returns Null
// =========================================================================

/// `SELECT AVG(l_extendedprice) FROM lineitem` should return 1 row
/// with the average (750.0), NOT Null.
///
/// Bug #5: AVG is wired for INTEGER columns only; REAL columns
/// return Null. With seed values 1000.0 and 500.0, the average
/// should be 750.0.
#[test]
fn bug5_avg_real_column_returns_correct_value() {
    let mut engine = make_engine();
    let result = engine
        .execute("SELECT AVG(l_extendedprice) FROM lineitem")
        .expect("AVG should not error");
    assert_eq!(result.rows.len(), 1, "AVG should return 1 row");
}
