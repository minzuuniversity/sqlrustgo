//! PR-SHOW-TABLES — P1 backlog fix for v3.7.0
//!
//! **Issue**: `SHOW TABLES` returned "Unsupported statement type" because
//! `ExecutionEngine::execute()` had no match arm for `Statement::Show`.
//! See docs/releases/v3.7.0/GA_GAP_REPORT.md §3.1.
//!
//! **Fix**: Add `Statement::Show` dispatch + `execute_show_databases` and
//! `execute_show_tables` handlers using `StorageEngine::list_tables()`.

use sqlrustgo::MemoryExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use std::sync::{Arc, RwLock};

fn make_engine() -> MemoryExecutionEngine {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    MemoryExecutionEngine::new(storage)
}

#[test]
fn show_tables_on_empty_db_returns_empty_result() {
    let mut engine = make_engine();
    let result = engine.execute("SHOW TABLES").unwrap();
    assert_eq!(
        result.rows.len(),
        0,
        "SHOW TABLES on empty DB should return 0 rows, got {}",
        result.rows.len()
    );
}

#[test]
fn show_tables_lists_all_created_tables() {
    let mut engine = make_engine();
    engine.execute("CREATE TABLE t1 (id INTEGER)").unwrap();
    engine
        .execute("CREATE TABLE t2 (id INTEGER, name TEXT)")
        .unwrap();
    engine.execute("CREATE TABLE t3 (id INTEGER)").unwrap();

    let result = engine.execute("SHOW TABLES").unwrap();
    let names: Vec<String> = result
        .rows
        .iter()
        .filter_map(|row| match row.first() {
            Some(sqlrustgo::Value::Text(s)) => Some(s.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(names.len(), 3, "expected 3 tables, got {:?}", names);
    assert!(names.contains(&"t1".to_string()));
    assert!(names.contains(&"t2".to_string()));
    assert!(names.contains(&"t3".to_string()));
}

#[test]
fn show_databases_returns_one_row() {
    let mut engine = make_engine();
    // v3.7.0 has a single in-memory catalog. SHOW DATABASES returns one
    // row representing the current (only) database.
    let result = engine.execute("SHOW DATABASES").unwrap();
    assert!(
        result.rows.len() >= 1,
        "SHOW DATABASES should return at least one row, got {}",
        result.rows.len()
    );
}

#[test]
fn show_tables_after_drop_reflects_drop() {
    let mut engine = make_engine();
    engine.execute("CREATE TABLE keep_me (id INTEGER)").unwrap();
    engine.execute("CREATE TABLE drop_me (id INTEGER)").unwrap();
    engine.execute("DROP TABLE drop_me").unwrap();

    let result = engine.execute("SHOW TABLES").unwrap();
    let names: Vec<String> = result
        .rows
        .iter()
        .filter_map(|row| match row.first() {
            Some(sqlrustgo::Value::Text(s)) => Some(s.clone()),
            _ => None,
        })
        .collect();
    assert!(names.contains(&"keep_me".to_string()));
    assert!(
        !names.contains(&"drop_me".to_string()),
        "drop_me should not appear after DROP"
    );
}
