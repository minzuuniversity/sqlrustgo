//! Tests for `SHOW INDEX FROM <table>` (V312-56A / #4251).
//!
//! V312-56A acceptance criteria require SHOW INDEX to return registered
//! index metadata instead of an empty placeholder. The output format
//! mirrors MySQL 5.7 `SHOW INDEX`:
//! Table, Non_unique, Key_name, Seq_in_index, Column_name, Collation,
//! Cardinality, Sub_part, Packed, Null, Index_type, Comment.
//!
//! v3.12 implements the controlled subset: Table, Non_unique, Key_name,
//! Seq_in_index, Column_name, Null, Index_type. Other columns are NULL.
use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine};
use std::sync::Arc;

fn engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

fn index_value(row: &[sqlrustgo::Value], idx: usize) -> String {
    match &row[idx] {
        sqlrustgo::Value::Text(s) => s.clone(),
        sqlrustgo::Value::Null => String::new(),
        sqlrustgo::Value::Integer(i) => i.to_string(),
        other => format!("{:?}", other),
    }
}

#[test]
fn test_show_index_returns_primary_key_index() {
    let mut e = engine();
    e.execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL, email TEXT)")
        .unwrap();

    let r = e.execute("SHOW INDEX FROM users").unwrap();
    // PK index is implicit on `id INTEGER PRIMARY KEY`
    assert!(
        !r.rows.is_empty(),
        "expected at least one index row, got {}",
        r.rows.len()
    );
    // MySQL SHOW INDEX columns:
    // 0=Table, 1=Non_unique, 2=Key_name, 3=Seq_in_index, 4=Column_name,
    // 5=Collation, 6=Cardinality, 7=Sub_part, 8=Packed, 9=Null, 10=Index_type, 11=Comment
    assert_eq!(r.rows[0].len(), 12, "MySQL SHOW INDEX has 12 columns");
    assert_eq!(index_value(&r.rows[0], 0), "users");
    assert_eq!(index_value(&r.rows[0], 2), "PRIMARY");
    assert_eq!(index_value(&r.rows[0], 4), "id");
    assert_eq!(index_value(&r.rows[0], 10), "BTREE");
}

#[test]
fn test_show_index_table_without_indices_returns_empty() {
    let mut e = engine();
    e.execute("CREATE TABLE plain (x INTEGER, y TEXT)").unwrap();

    let r = e.execute("SHOW INDEX FROM plain").unwrap();
    // No PK -> no rows is acceptable as documented "controlled subset".
    assert_eq!(
        r.rows.len(),
        0,
        "table without index should yield empty result (controlled subset)"
    );
}

#[test]
fn test_show_index_missing_table_errors() {
    let mut e = engine();
    let r = e.execute("SHOW INDEX FROM nope");
    assert!(r.is_err(), "expected error for missing table");
}
