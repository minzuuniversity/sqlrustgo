//! Operator-level regression tests for `PRAGMA table_info(<table>)`.
//!
//! Two defects motivated this suite:
//!
//! 1. **PRAGMA was unrecognised.** `parse_statement` had no arm for `PRAGMA`,
//!    so every pragma hit the final `Unexpected token: Identifier("PRAGMA")` error.
//!    A dedicated parser and executor now accepts the syntax and returns a
//!    structured result.
//!
//! 2. **`pk` was always 0 for table-level `PRIMARY KEY` constraints.** The
//!    executor only propagated `primary_key = true` for the column-level form
//!    (`col TYPE PRIMARY KEY`). Table-level `PRIMARY KEY (col)` constraints were
//!    stored in `TableInfo.constraints` but never used to backfill the column
//!    flag. Now both forms produce `pk=1`.

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::Arc;

fn engine() -> ExecutionEngine<MemoryStorage> {
    ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())))
}

fn rows_on(e: &mut ExecutionEngine<MemoryStorage>, sql: &str) -> Vec<Vec<String>> {
    e.execute(sql)
        .unwrap()
        .rows
        .into_iter()
        .map(|row| row.into_iter().map(|v| v.to_string()).collect())
        .collect()
}

fn first_row_on(e: &mut ExecutionEngine<MemoryStorage>, sql: &str) -> Vec<String> {
    rows_on(e, sql).into_iter().next().unwrap_or_default()
}

/// Columns of PRAGMA table_info: cid, name, type, notnull, dflt_value, pk
const COLS: [&str; 6] = ["cid", "name", "type", "notnull", "dflt_value", "pk"];

fn check_col(row: &[String], idx: usize, expected: &str) {
    assert_eq!(
        row[idx],
        expected,
        "row {:?}: col[{}] = {:?}, expected {:?}",
        row,
        idx,
        row.get(idx),
        expected
    );
}

// --------------------------------------------------------------------------
// Basic acceptance
// --------------------------------------------------------------------------

#[test]
fn pragma_table_info_unknown_table_error() {
    let mut e = engine();
    let r = e.execute("PRAGMA table_info(nonexistent)");
    assert!(r.is_err());
    let err = r.unwrap_err().to_string();
    assert!(
        err.contains("does not exist"),
        "expected 'does not exist' in error, got: {}",
        err
    );
}

#[test]
fn pragma_table_info_no_arg_error() {
    let mut e = engine();
    e.execute("CREATE TABLE t (id INT PRIMARY KEY)").unwrap();
    let r = e.execute("PRAGMA table_info");
    assert!(r.is_err());
}

#[test]
fn pragma_unsupported_name_error() {
    let mut e = engine();
    e.execute("CREATE TABLE t (id INT)").unwrap();
    let r = e.execute("PRAGMA database_list");
    assert!(r.is_err());
    let err = r.unwrap_err().to_string();
    assert!(
        err.contains("not supported"),
        "expected 'not supported' in error, got: {}",
        err
    );
}

// --------------------------------------------------------------------------
// Column metadata: name, type, notnull
// --------------------------------------------------------------------------

#[test]
fn pragma_table_info_single_column() {
    let mut e = engine();
    e.execute("CREATE TABLE t (name VARCHAR(20) NOT NULL)")
        .unwrap();
    let row = first_row_on(&mut e, "PRAGMA table_info(t)");
    assert_eq!(row.len(), 6);
    check_col(&row, 0, "0"); // cid
    check_col(&row, 1, "name"); // name
    assert!(
        row[2].contains("VARCHAR"),
        "type should contain VARCHAR, got: {}",
        row[2]
    );
    check_col(&row, 3, "1"); // notnull: 1 = NOT NULL
                             // dflt_value: absent
    check_col(&row, 5, "0"); // pk: 0 (no PRIMARY KEY)
}

#[test]
fn pragma_table_info_nullable() {
    let mut e = engine();
    e.execute("CREATE TABLE t (c INT)").unwrap();
    let row = first_row_on(&mut e, "PRAGMA table_info(t)");
    check_col(&row, 3, "0"); // notnull: 0 = nullable
}

#[test]
fn pragma_table_info_char_length() {
    let mut e = engine();
    e.execute("CREATE TABLE t (c CHAR(10))").unwrap();
    let row = first_row_on(&mut e, "PRAGMA table_info(t)");
    assert!(
        row[2].contains("CHAR"),
        "type should contain CHAR, got: {}",
        row[2]
    );
    assert!(
        row[2].contains("(10)"),
        "type should contain (10), got: {}",
        row[2]
    );
}

// --------------------------------------------------------------------------
// Primary key: column-level form
// --------------------------------------------------------------------------

#[test]
fn pragma_table_info_pk_column_level() {
    let mut e = engine();
    e.execute("CREATE TABLE t (id INT PRIMARY KEY)").unwrap();
    let row = first_row_on(&mut e, "PRAGMA table_info(t)");
    check_col(&row, 5, "1"); // pk = 1
    check_col(&row, 3, "1"); // notnull = 1 (PK implies NOT NULL)
}

// --------------------------------------------------------------------------
// Primary key: table-level form (the fixed defect)
// --------------------------------------------------------------------------

#[test]
fn pragma_table_info_pk_table_level() {
    let mut e = engine();
    e.execute("CREATE TABLE t (id INT, PRIMARY KEY (id))")
        .unwrap();
    let row = first_row_on(&mut e, "PRAGMA table_info(t)");
    // pk should be 1 even when declared as table-level constraint
    check_col(&row, 5, "1");
}

#[test]
fn pragma_table_info_multi_col_pk() {
    let mut e = engine();
    // Two-column table-level PK
    e.execute("CREATE TABLE t (a INT, b INT, PRIMARY KEY (a, b))")
        .unwrap();
    let rows = rows_on(&mut e, "PRAGMA table_info(t)");
    assert_eq!(rows.len(), 2);
    // Both columns should have pk=1
    check_col(&rows[0], 5, "1");
    check_col(&rows[1], 5, "1");
}

// --------------------------------------------------------------------------
// Default value
// --------------------------------------------------------------------------

#[test]
fn pragma_table_info_default_null() {
    let mut e = engine();
    e.execute("CREATE TABLE t (c INT)").unwrap();
    let row = first_row_on(&mut e, "PRAGMA table_info(t)");
    // No explicit DEFAULT → dflt_value should be NULL
    check_col(&row, 4, "NULL");
}

#[test]
fn pragma_table_info_default_int() {
    let mut e = engine();
    e.execute("CREATE TABLE t (c INT DEFAULT 42)").unwrap();
    let row = first_row_on(&mut e, "PRAGMA table_info(t)");
    assert_eq!(row[4], "42", "dflt_value should be 42");
}

#[test]
fn pragma_table_info_default_text() {
    let mut e = engine();
    e.execute("CREATE TABLE t (c TEXT DEFAULT 'hello')")
        .unwrap();
    let row = first_row_on(&mut e, "PRAGMA table_info(t)");
    assert_eq!(
        row[4], "'hello'",
        "dflt_value should be quoted 'hello', got: {}",
        row[4]
    );
}

// --------------------------------------------------------------------------
// Multi-column table
// --------------------------------------------------------------------------

#[test]
fn pragma_table_info_three_columns() {
    let mut e = engine();
    e.execute("CREATE TABLE t (a INT, b TEXT, c FLOAT)")
        .unwrap();
    let rows = rows_on(&mut e, "PRAGMA table_info(t)");
    assert_eq!(rows.len(), 3);
    check_col(&rows[0], 0, "0");
    check_col(&rows[1], 0, "1");
    check_col(&rows[2], 0, "2");
    check_col(&rows[0], 1, "a");
    check_col(&rows[1], 1, "b");
    check_col(&rows[2], 1, "c");
}
