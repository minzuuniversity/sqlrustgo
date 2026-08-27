//! Regression tests for Issue #4516 — `SHOW DATABASES` / `SHOW TABLES`
//! / `SHOW COLUMNS` produce MySQL-compatible row shapes.
//!
//! Background
//! ----------
//! Tsinghua's MySQL curriculum (chapters 1, 3, 4) introduces
//! `SHOW DATABASES` / `SHOW TABLES` / `SHOW COLUMNS FROM <table>` and
//! `DESCRIBE <table>` as the first metadata commands students learn.
//! Before #4516, the engine returned the correct row tuples but the
//! REPL header fallback produced `col_0, col_1, ...` (0-indexed) and
//! the wire-protocol layer fell back to `col_1, col_2, ...` (1-indexed
//! but unnamed), so `mysql --table` and `cargo run --bin sqlrustgo`
//! output diverged from real MySQL.
//!
//! This file covers the engine-level behavior — engine returns rows
//! that match MySQL's expected row shape (single column for
//! `SHOW TABLES`/`SHOW DATABASES`, six columns for DESCRIBE/SHOW
//! COLUMNS). The wire/REPL header renaming lives in
//! `crates/mysql-server/tests/issue_4516_*_test.rs`.

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use sqlrustgo_storage::Value;
use std::sync::Arc;

fn engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

#[test]
fn show_databases_returns_single_column_text() {
    // SHOW DATABASES → one row with a Text column named after the
    // v3.12 single-schema "default" label. The engine returns the
    // row shape; the column name mapping is owned by the wire layer.
    let mut e = engine();
    let r = e.execute("SHOW DATABASES").unwrap();
    assert_eq!(r.rows.len(), 1, "got {r:?}");
    assert_eq!(r.rows[0].len(), 1);
    assert!(matches!(&r.rows[0][0], Value::Text(s) if s == "default"));
}

#[test]
fn show_databases_on_empty_engine_returns_default_label() {
    // Even with zero user tables, SHOW DATABASES must list the
    // implicit "default" schema (v3.12 single-schema engine).
    let mut e = engine();
    let r = e.execute("SHOW DATABASES").unwrap();
    assert!(!r.rows.is_empty());
    assert!(matches!(&r.rows[0][0], Value::Text(s) if s == "default"));
}

#[test]
fn show_tables_lists_user_tables() {
    let mut e = engine();
    e.execute("CREATE TABLE t1(id INTEGER)").unwrap();
    e.execute("CREATE TABLE t2(id INTEGER, name TEXT)").unwrap();
    e.execute("CREATE TABLE t3(id INTEGER)").unwrap();
    let r = e.execute("SHOW TABLES").unwrap();
    assert_eq!(r.rows.len(), 3, "got {r:?}");
    for row in &r.rows {
        assert_eq!(row.len(), 1);
        assert!(matches!(&row[0], Value::Text(_)));
    }
}

#[test]
fn show_tables_with_unmatched_like_returns_empty() {
    let mut e = engine();
    e.execute("CREATE TABLE t1(id INTEGER)").unwrap();
    let r = e.execute("SHOW TABLES LIKE 'nonexistent%'").unwrap();
    assert!(r.rows.is_empty(), "got {r:?}");
}

#[test]
fn show_tables_with_like_filter() {
    // V312-58: SHOW TABLES LIKE 'a%' filters by SQL LIKE wildcards
    // (case-sensitive, % = zero-or-more, _ = single).
    let mut e = engine();
    e.execute("CREATE TABLE apple(id INTEGER)").unwrap();
    e.execute("CREATE TABLE apricot(id INTEGER)").unwrap();
    e.execute("CREATE TABLE banana(id INTEGER)").unwrap();
    let r = e.execute("SHOW TABLES LIKE 'a%'").unwrap();
    let names: Vec<String> = r
        .rows
        .iter()
        .map(|row| match &row[0] {
            Value::Text(s) => s.clone(),
            _ => panic!("non-text row"),
        })
        .collect();
    assert_eq!(names.len(), 2, "got {names:?}");
    assert!(names.contains(&"apple".to_string()));
    assert!(names.contains(&"apricot".to_string()));
    assert!(!names.contains(&"banana".to_string()));
}

#[test]
fn show_tables_with_from_db_clause_is_accepted() {
    // V312-58: SHOW TABLES FROM <schema> now parses (previously
    // silently dropped). The v3.12 single-schema engine ignores the
    // schema argument but still lists tables.
    let mut e = engine();
    e.execute("CREATE TABLE t(id INTEGER)").unwrap();
    let r = e.execute("SHOW TABLES FROM default").unwrap();
    assert_eq!(r.rows.len(), 1, "got {r:?}");
    assert!(matches!(&r.rows[0][0], Value::Text(s) if s == "t"));
}

#[test]
fn show_columns_returns_six_column_metadata() {
    // DESCRIBE and SHOW COLUMNS share the same 6-column shape:
    // (Field, Type, Null, Key, Default, Extra). The engine returns
    // six values per row; the column header naming lives at the
    // wire layer.
    let mut e = engine();
    e.execute("CREATE TABLE t(id INTEGER PRIMARY KEY, name TEXT)")
        .unwrap();
    let r = e.execute("SHOW COLUMNS FROM t").unwrap();
    assert_eq!(r.rows.len(), 2, "got {r:?}");
    for row in &r.rows {
        assert_eq!(row.len(), 6, "expected 6-column metadata row, got {row:?}");
    }
    // PK column should have key=PRI, nullable=NO
    let pk_row = &r.rows[0];
    assert!(matches!(&pk_row[0], Value::Text(s) if s == "id"));
    assert!(matches!(&pk_row[2], Value::Text(s) if s == "NO"));
    assert!(matches!(&pk_row[3], Value::Text(s) if s == "PRI"));
}

#[test]
fn show_columns_with_like_filter_narrows_rows() {
    let mut e = engine();
    e.execute("CREATE TABLE t(id INTEGER, name TEXT, address TEXT)")
        .unwrap();
    let r = e.execute("SHOW COLUMNS FROM t LIKE '%name%'").unwrap();
    assert_eq!(r.rows.len(), 1, "got {r:?}");
    assert!(matches!(&r.rows[0][0], Value::Text(s) if s == "name"));
}

#[test]
fn describe_table_equivalent_to_show_columns() {
    // DESCRIBE <table> and SHOW COLUMNS FROM <table> should produce
    // identical row tuples (column header renaming is at the wire
    // layer; the engine returns the same six-column shape).
    let mut e = engine();
    e.execute("CREATE TABLE t(id INTEGER, name TEXT)").unwrap();
    let r_describe = e.execute("DESCRIBE t").unwrap();
    let r_show_cols = e.execute("SHOW COLUMNS FROM t").unwrap();
    assert_eq!(r_describe.rows.len(), r_show_cols.rows.len());
    assert_eq!(r_describe.rows, r_show_cols.rows);
}

#[test]
fn describe_nonexistent_table_returns_error() {
    let mut e = engine();
    let err = e.execute("DESCRIBE missing").unwrap_err();
    let msg = format!("{err:?}");
    assert!(msg.contains("does not exist") || msg.contains("not found"));
}

#[test]
fn show_full_tables_includes_type_column() {
    // V312-59-A / #4384: SHOW FULL TABLES returns two columns
    // (Name, Table_type). This test pins the row shape.
    let mut e = engine();
    e.execute("CREATE TABLE t(id INTEGER)").unwrap();
    let r = e.execute("SHOW FULL TABLES").unwrap();
    assert_eq!(r.rows.len(), 1);
    assert_eq!(r.rows[0].len(), 2);
    assert!(matches!(&r.rows[0][0], Value::Text(_)));
    // Either "BASE TABLE" or "VIEW"
    assert!(matches!(&r.rows[0][1], Value::Text(s) if s == "BASE TABLE" || s == "VIEW"));
}

#[test]
fn show_table_status_returns_eighteen_columns() {
    // V312-59-A / #4384: SHOW TABLE STATUS returns the MySQL
    // 18-column table status shape.
    let mut e = engine();
    e.execute("CREATE TABLE t(id INTEGER)").unwrap();
    let r = e.execute("SHOW TABLE STATUS").unwrap();
    assert_eq!(r.rows.len(), 1);
    assert_eq!(r.rows[0].len(), 18, "expected 18-column row, got {r:?}");
}

#[test]
fn show_columns_table_not_found_returns_error() {
    let mut e = engine();
    let err = e.execute("SHOW COLUMNS FROM missing").unwrap_err();
    let msg = format!("{err:?}");
    assert!(msg.contains("does not exist") || msg.contains("not found"));
}
