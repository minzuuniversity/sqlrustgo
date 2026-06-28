//! Multi-Database E2E Tests
//!
//! 验证 v3.10.0 多数据库支持 (CREATE DATABASE / DROP DATABASE).
//! 当前 USE 仅作 no-op，因为 table lookup 路径尚未按数据库隔离.

use sqlrustgo::MemoryExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use std::sync::{Arc, RwLock};

fn make_engine() -> MemoryExecutionEngine {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    MemoryExecutionEngine::new(storage)
}

#[test]
fn test_create_database() {
    let mut engine = make_engine();
    let r = engine.execute("CREATE DATABASE mydb");
    assert!(r.is_ok(), "CREATE DATABASE failed: {:?}", r.err());
}

#[test]
fn test_create_database_if_not_exists() {
    let mut engine = make_engine();
    let _ = engine.execute("CREATE DATABASE mydb");
    let r = engine.execute("CREATE DATABASE IF NOT EXISTS mydb");
    assert!(
        r.is_ok(),
        "CREATE DATABASE IF NOT EXISTS should not error: {:?}",
        r.err()
    );
}

#[test]
fn test_create_reserved_database_rejected() {
    let mut engine = make_engine();
    let r = engine.execute("CREATE DATABASE default");
    assert!(r.is_err(), "CREATE DATABASE default should be rejected");
    let r = engine.execute("CREATE DATABASE postgres");
    assert!(r.is_err(), "CREATE DATABASE postgres should be rejected");
    let r = engine.execute("CREATE DATABASE mysql");
    assert!(r.is_err(), "CREATE DATABASE mysql should be rejected");
}

#[test]
fn test_drop_database() {
    let mut engine = make_engine();
    let _ = engine.execute("CREATE DATABASE mydb").unwrap();
    let r = engine.execute("DROP DATABASE mydb");
    assert!(r.is_ok(), "DROP DATABASE failed: {:?}", r.err());
}

#[test]
fn test_drop_reserved_database_rejected() {
    let mut engine = make_engine();
    let r = engine.execute("DROP DATABASE default");
    assert!(r.is_err(), "DROP DATABASE default should be rejected");
    let r = engine.execute("DROP DATABASE postgres");
    assert!(r.is_err(), "DROP DATABASE postgres should be rejected");
    let r = engine.execute("DROP DATABASE mysql");
    assert!(r.is_err(), "DROP DATABASE mysql should be rejected");
}

#[test]
fn test_create_multiple_databases() {
    let mut engine = make_engine();
    let _ = engine.execute("CREATE DATABASE db1");
    let _ = engine.execute("CREATE DATABASE db2");
    let _ = engine.execute("CREATE DATABASE db3");
    // Should not panic
}

#[test]
fn test_use_database_noop() {
    let mut engine = make_engine();
    let _ = engine.execute("CREATE DATABASE mydb");
    let r = engine.execute("USE mydb");
    assert!(r.is_ok(), "USE mydb failed: {:?}", r.err());
}
