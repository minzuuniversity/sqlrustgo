//! Tests for DESCRIBE / DESC statements (CLI-02)
use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::Arc;

fn engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

#[test]
fn test_describe_existing_table() {
    let mut e = engine();
    e.execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL, age INTEGER)")
        .unwrap();
    let r = e.execute("DESCRIBE users").unwrap();
    assert_eq!(r.rows.len(), 3);
    // Field, Type, Null, Key, Default, Extra
    assert_eq!(r.rows[0][0], sqlrustgo::Value::Text("id".into()));
    assert_eq!(r.rows[0][1], sqlrustgo::Value::Text("INTEGER".into()));
    assert_eq!(r.rows[0][3], sqlrustgo::Value::Text("PRI".into()));
    assert_eq!(r.rows[1][0], sqlrustgo::Value::Text("name".into()));
    assert_eq!(r.rows[2][0], sqlrustgo::Value::Text("age".into()));
}

#[test]
fn test_describe_alias_desc() {
    let mut e = engine();
    e.execute("CREATE TABLE t (x INTEGER)").unwrap();
    let r1 = e.execute("DESCRIBE t").unwrap();
    let r2 = e.execute("DESC t").unwrap();
    assert_eq!(r1.rows.len(), r2.rows.len());
    assert_eq!(r1.rows.len(), 1);
}

#[test]
fn test_describe_missing_table_errors() {
    let mut e = engine();
    let r = e.execute("DESCRIBE nope");
    assert!(r.is_err());
}
