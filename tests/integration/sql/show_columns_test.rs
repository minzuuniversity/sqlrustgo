//! Tests for `SHOW COLUMNS FROM <table>` (V312-56A / #4251).
//!
//! V312-56A acceptance criteria require SHOW COLUMNS to no longer be a
//! placeholder. The behavior must mirror `DESCRIBE <table>` and accept an
//! optional LIKE pattern.
use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine};
use std::sync::Arc;

fn engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

/// Convenience helper: build a table with one PK column, one NOT NULL
/// column, and one nullable column.
fn make_table(e: &mut ExecutionEngine<MemoryStorage>) {
    e.execute(
        "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL, email TEXT)",
    )
    .unwrap();
}

#[test]
fn test_show_columns_basic_returns_rows() {
    let mut e = engine();
    make_table(&mut e);

    let r = e.execute("SHOW COLUMNS FROM users").unwrap();
    assert_eq!(r.rows.len(), 3, "expected 3 rows for users");
    // Each row has 6 columns: Field, Type, Null, Key, Default, Extra
    assert_eq!(r.rows[0].len(), 6);
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Text("id".into()));
    assert_eq!(r.rows[0][1], sqlrustgo::Value::Text("INTEGER".into()));
    assert_eq!(r.rows[0][3], sqlrustgo::Value::Text("PRI".into()));
}

#[test]
fn test_show_columns_field_type_null_match_describe() {
    let mut e = engine();
    make_table(&mut e);

    let cols = e.execute("SHOW COLUMNS FROM users").unwrap();
    let desc = e.execute("DESCRIBE users").unwrap();

    assert_eq!(cols.rows.len(), desc.rows.len());
    for (c_row, d_row) in cols.rows.iter().zip(desc.rows.iter()) {
        // Field / Type / Null / Key / Default / Extra should be identical
        assert_eq!(c_row, d_row);
    }
}

#[test]
fn test_show_columns_like_pattern_filters_columns() {
    let mut e = engine();
    make_table(&mut e);

    // Only match columns whose name contains "e" -> "name" has no "e", "email" does.
    // MySQL LIKE semantics: case-insensitive for unescaped ASCII letters.
    let r = e
        .execute("SHOW COLUMNS FROM users LIKE '%e%'")
        .unwrap();
    assert_eq!(r.rows.len(), 2, "expected 2 rows matching LIKE '%e%'");
    let names: Vec<&sqlrustgo::Value> = r.rows.iter().map(|row| &row[0]).collect();
    // id: no 'e'; name: no 'e'; email: has 'e'
    let name_strs: Vec<String> = names
        .iter()
        .map(|v| match v {
            sqlrustgo::Value::Text(s) => s.clone(),
            _ => String::new(),
        })
        .collect();
    assert!(name_strs.iter().any(|s| s == "email"));
}

#[test]
fn test_show_columns_missing_table_errors() {
    let mut e = engine();
    let r = e.execute("SHOW COLUMNS FROM nope");
    assert!(r.is_err(), "expected error for missing table");
}
