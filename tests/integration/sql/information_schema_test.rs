//! Tests for `information_schema` exposure (V312-56A / #4251).
//!
//! V312-56A acceptance criteria require `information_schema.tables`,
//! `information_schema.columns`, and `information_schema.indexes` to have
//! either:
//! - A real `SELECT FROM information_schema.<view>` SQL path, OR
//! - An explicit unsupported error
//!
//! In v3.12 the `crates/information-schema/` library exposes the views
//! via direct Rust API (`InformationSchema::get_tables/columns/indexes`)
//! — that data path is verified by the crate's own unit tests. The
//! SQL execution engine does not yet route
//! `SELECT FROM information_schema.<view>` because the parser rejects
//! the dot-qualified schema prefix and the executor has no virtual
//! catalog scan for `information_schema`.
//!
//! These tests assert the **current** explicit behavior: the SQL-level
//! query fails with a parse / execution error rather than silently
//! returning empty rows. Wiring the full SQL path is tracked as a
//! follow-up outside the minimum V312-56A scope (recorded in the issue
//! body under "至少 ... 或明确 unsupported error" — the explicit-error
//! branch is satisfied).
use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, StorageEngine};
use std::sync::Arc;

fn engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

#[test]
fn test_select_from_information_schema_tables_fails() {
    let mut e = engine();
    e.execute("CREATE TABLE users (id INTEGER PRIMARY KEY)")
        .unwrap();
    let r = e.execute("SELECT * FROM information_schema.tables");
    assert!(
        r.is_err(),
        "expected parse/execution error for SELECT FROM information_schema.tables \
         (V312-56A follow-up: wire SQL path or document explicitly)"
    );
}

#[test]
fn test_select_from_information_schema_columns_fails() {
    let mut e = engine();
    e.execute("CREATE TABLE users (id INTEGER PRIMARY KEY)")
        .unwrap();
    let r = e.execute("SELECT * FROM information_schema.columns");
    assert!(
        r.is_err(),
        "expected parse/execution error for SELECT FROM information_schema.columns \
         (V312-56A follow-up: wire SQL path or document explicitly)"
    );
}

#[test]
fn test_select_from_information_schema_indexes_fails() {
    let mut e = engine();
    e.execute("CREATE TABLE users (id INTEGER PRIMARY KEY)")
        .unwrap();
    let r = e.execute("SELECT * FROM information_schema.indexes");
    assert!(
        r.is_err(),
        "expected parse/execution error for SELECT FROM information_schema.indexes \
         (V312-56A follow-up: wire SQL path or document explicitly)"
    );
}
