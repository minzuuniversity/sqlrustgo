//! View / Procedure / Trigger 端到端测试
//!
//! 对应的功能矩阵: docs/standard/SQL92_FUNCTIONALITY_MATRIX.md §3.3

use sqlrustgo::MemoryExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use std::sync::{Arc, RwLock};

fn make_engine() -> MemoryExecutionEngine {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    MemoryExecutionEngine::new(storage)
}

fn make_engine_with_catalog() -> MemoryExecutionEngine {
    use sqlrustgo_catalog::Catalog;
    let catalog = Arc::new(RwLock::new(Catalog::new("test")));
    MemoryExecutionEngine::with_memory_and_catalog(catalog)
}

// =============================================================================
// CREATE / DROP VIEW
// =============================================================================

#[test]
fn test_create_view_parse() {
    use sqlrustgo_parser::parse;
    let r = parse("CREATE VIEW v AS SELECT id, name FROM users");
    assert!(r.is_ok(), "CREATE VIEW parse failed: {:?}", r.err());
}

#[test]
fn test_create_view_execute() {
    let mut engine = make_engine();
    let _ = engine.execute("CREATE TABLE users (id INTEGER, name TEXT)").unwrap();
    let _ = engine.execute("INSERT INTO users VALUES (1, 'alice')").unwrap();

    let r = engine.execute("CREATE VIEW v AS SELECT id, name FROM users");
    assert!(r.is_ok(), "CREATE VIEW failed: {:?}", r.err());
}

#[test]
fn test_drop_view_execute() {
    let mut engine = make_engine();
    let _ = engine.execute("CREATE TABLE users (id INTEGER)").unwrap();
    let _ = engine.execute("CREATE VIEW v AS SELECT id FROM users").unwrap();

    let r = engine.execute("DROP VIEW v");
    assert!(r.is_ok(), "DROP VIEW failed: {:?}", r.err());
}

#[test]
fn test_drop_view_if_exists() {
    let mut engine = make_engine();
    let r = engine.execute("DROP VIEW IF EXISTS nonexistent");
    assert!(r.is_ok(), "DROP VIEW IF EXISTS should succeed");
}

// =============================================================================
// CREATE PROCEDURE / CALL
// =============================================================================

#[test]
fn test_create_procedure_parse() {
    use sqlrustgo_parser::parse;
    let r = parse("CREATE PROCEDURE myproc() BEGIN SELECT 1 END");
    assert!(r.is_ok(), "CREATE PROCEDURE parse failed: {:?}", r.err());
}

#[test]
fn test_create_procedure_with_catalog() {
    let mut engine = make_engine_with_catalog();
    let r = engine.execute("CREATE PROCEDURE myproc() BEGIN SELECT 1 END");
    assert!(r.is_ok(), "CREATE PROCEDURE failed: {:?}", r.err());
}

#[test]
fn test_call_procedure_parse() {
    use sqlrustgo_parser::parse;
    let r = parse("CALL myproc()");
    assert!(r.is_ok(), "CALL parse failed: {:?}", r.err());
}

// =============================================================================
// TRIGGER
// =============================================================================

#[test]
fn test_create_trigger_parse() {
    use sqlrustgo_parser::parse;
    let sql = "CREATE TRIGGER test_trig BEFORE INSERT ON t FOR EACH ROW BEGIN SET NEW.name = 'triggered'; END";
    let r = parse(sql);
    assert!(r.is_ok(), "CREATE TRIGGER parse failed: {:?}", r.err());
}

#[test]
fn test_create_trigger_execute() {
    let mut engine = make_engine();
    let _ = engine.execute("CREATE TABLE t (id INTEGER, name TEXT)").unwrap();
    let r = engine.execute("CREATE TRIGGER before_insert_t BEFORE INSERT ON t FOR EACH ROW BEGIN SET NEW.name = 'triggered'; END");
    assert!(r.is_ok(), "CREATE TRIGGER failed: {:?}", r.err());
}

#[test]
fn test_trigger_after_insert_create() {
    let mut engine = make_engine();
    let _ = engine.execute("CREATE TABLE t (id INTEGER, val TEXT)").unwrap();
    let r = engine.execute("CREATE TRIGGER after_insert_t AFTER INSERT ON t FOR EACH ROW BEGIN UPDATE t SET val = 'triggered'; END");
    assert!(r.is_ok(), "CREATE AFTER INSERT TRIGGER failed: {:?}", r.err());
}
