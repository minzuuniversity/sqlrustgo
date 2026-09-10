//! Regression tests for issue #4848 — `ALTER TABLE ... RENAME COLUMN`
//! in v3.12.0 GA CLI batch mode.
//!
//! Issue body (2026-09-07):
//! The default `StorageEngine::rename_column` trait impl returned
//! "rename_column not supported by this storage engine", so all three
//! execution paths (CLI sqlite --batch, serve+cli, soak) failed at
//! `ALTER TABLE order_item RENAME COLUMN note TO memo;`. SQLite 3.45.1
//! and MySQL 8.0 both support this. The fix implements the trait on
//! `FileStorage` (the CLI default), keeping the positional row layout
//! (Vec<Vec<Value>>) intact and only updating `info.columns[idx].name`.
//!
//! BustubX-EDU week-11 (Catalog 演化) 教材快检 #4 was blocked by
//! this — the v3.12.0 GA release notes have the v3.13 deferred item
//! for "rename_column" in CLAIM_DOWNGRADE_MANIFEST.md §3.

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::FileStorage;
use std::sync::Arc;

fn create_engine(dir: &std::path::Path) -> ExecutionEngine<FileStorage> {
    let storage = FileStorage::new_with_buffer_config(dir.to_path_buf(), 100, false)
        .expect("FileStorage::new");
    ExecutionEngine::new(Arc::new(RwLock::new(storage)))
}

#[test]
fn test_issue_4848_alter_rename_column_basic() {
    // Issue minimum reproducer.
    let dir = tempfile::tempdir().unwrap();
    let mut engine = create_engine(dir.path());
    engine
        .execute("CREATE TABLE order_item (id INT, qty INT)")
        .unwrap();
    engine
        .execute("ALTER TABLE order_item ADD COLUMN note VARCHAR(100)")
        .unwrap();
    engine
        .execute("INSERT INTO order_item VALUES (1, 10, 'old')")
        .unwrap();
    // The fix: this should no longer error with "rename_column not
    // supported".
    engine
        .execute("ALTER TABLE order_item RENAME COLUMN note TO memo")
        .unwrap();
    // Verify the new column name is queryable.
    let r = engine
        .execute("SELECT id, qty, memo FROM order_item")
        .unwrap();
    assert_eq!(r.rows.len(), 1);
    assert_eq!(r.rows[0][2].as_string(), "old");
}

#[test]
fn test_issue_4848_alter_rename_column_duplicate_destination_rejected() {
    // MySQL 8.0 (ERROR_DUP_FIELDNAME 1060) and SQLite both reject
    // renaming a column to a name that already exists.
    let dir = tempfile::tempdir().unwrap();
    let mut engine = create_engine(dir.path());
    engine.execute("CREATE TABLE t (a INT, b INT)").unwrap();
    let err = engine
        .execute("ALTER TABLE t RENAME COLUMN a TO b")
        .unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("Duplicate") || msg.contains("duplicate"),
        "expected duplicate-column error, got: {}",
        msg
    );
}

#[test]
fn test_issue_4848_alter_rename_column_missing_source_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let mut engine = create_engine(dir.path());
    engine.execute("CREATE TABLE t (a INT)").unwrap();
    let err = engine
        .execute("ALTER TABLE t RENAME COLUMN xyz TO y")
        .unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("Column not found") || msg.contains("not found"),
        "expected column-not-found error, got: {}",
        msg
    );
}

trait AsString {
    fn as_string(&self) -> &str;
}

impl AsString for sqlrustgo_types::Value {
    fn as_string(&self) -> &str {
        match self {
            sqlrustgo_types::Value::Text(s) => s,
            _ => "",
        }
    }
}
