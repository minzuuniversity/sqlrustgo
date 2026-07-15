// F-30 CREATE SEQUENCE / DROP SEQUENCE tests
// Issue: #3496 V311-10 CREATE SEQUENCE

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_parser::{parse_statements, Statement};
use sqlrustgo_storage::MemoryStorage;
use std::sync::Arc;

fn create_engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

// === Parser tests ===

#[test]
fn test_create_sequence_basic() {
    let sql = "CREATE SEQUENCE my_seq";
    let result = parse_statements(sql);
    assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    let stmts = result.unwrap();
    assert_eq!(stmts.len(), 1);
    match &stmts[0] {
        Statement::CreateSequence(seq) => {
            assert_eq!(seq.name, "my_seq");
            assert!(!seq.if_not_exists);
        }
        _ => panic!("Expected CreateSequence, got {:?}", stmts[0]),
    }
}

#[test]
fn test_create_sequence_if_not_exists() {
    let sql = "CREATE SEQUENCE IF NOT EXISTS my_seq";
    let result = parse_statements(sql);
    assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    let stmts = result.unwrap();
    assert_eq!(stmts.len(), 1);
    match &stmts[0] {
        Statement::CreateSequence(seq) => {
            assert_eq!(seq.name, "my_seq");
            assert!(seq.if_not_exists);
        }
        _ => panic!("Expected CreateSequence, got {:?}", stmts[0]),
    }
}

#[test]
fn test_create_sequence_with_options() {
    let sql = "CREATE SEQUENCE my_seq START WITH 100 INCREMENT BY 1 MINVALUE 1 MAXVALUE 1000 CACHE 20 CYCLE";
    let result = parse_statements(sql);
    assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    let stmts = result.unwrap();
    assert_eq!(stmts.len(), 1);
    match &stmts[0] {
        Statement::CreateSequence(seq) => {
            assert_eq!(seq.name, "my_seq");
            assert_eq!(seq.start_with.as_ref().unwrap(), "100");
            assert_eq!(seq.increment_by.as_ref().unwrap(), "1");
            assert_eq!(seq.minvalue.as_ref().unwrap(), "1");
            assert_eq!(seq.maxvalue.as_ref().unwrap(), "1000");
            assert_eq!(seq.cache.as_ref().unwrap(), "20");
            assert_eq!(seq.cycle, Some(true));
        }
        _ => panic!("Expected CreateSequence, got {:?}", stmts[0]),
    }
}

#[test]
fn test_create_sequence_negative_values() {
    let sql = "CREATE SEQUENCE my_seq START WITH -100 INCREMENT BY -1 MINVALUE -1000 MAXVALUE 1000";
    let result = parse_statements(sql);
    assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    let stmts = result.unwrap();
    match &stmts[0] {
        Statement::CreateSequence(seq) => {
            assert_eq!(seq.start_with.as_ref().unwrap(), "-100");
            assert_eq!(seq.increment_by.as_ref().unwrap(), "-1");
            assert_eq!(seq.minvalue.as_ref().unwrap(), "-1000");
        }
        _ => panic!("Expected CreateSequence"),
    }
}

#[test]
fn test_drop_sequence_basic() {
    let sql = "DROP SEQUENCE my_seq";
    let result = parse_statements(sql);
    assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    let stmts = result.unwrap();
    assert_eq!(stmts.len(), 1);
    match &stmts[0] {
        Statement::DropSequence(seq) => {
            assert_eq!(seq.name, "my_seq");
            assert!(!seq.if_exists);
        }
        _ => panic!("Expected DropSequence, got {:?}", stmts[0]),
    }
}

#[test]
fn test_drop_sequence_if_exists() {
    let sql = "DROP SEQUENCE IF EXISTS my_seq";
    let result = parse_statements(sql);
    assert!(result.is_ok(), "Failed to parse: {:?}", result.err());
    let stmts = result.unwrap();
    match &stmts[0] {
        Statement::DropSequence(seq) => {
            assert_eq!(seq.name, "my_seq");
            assert!(seq.if_exists);
        }
        _ => panic!("Expected DropSequence"),
    }
}

// === Execution tests ===

#[test]
fn test_sequence_execution_create() {
    let mut engine = create_engine();
    
    // Create a sequence - should succeed
    let result = engine.execute("CREATE SEQUENCE seq1 START WITH 1 INCREMENT BY 1");
    assert!(result.is_ok(), "CREATE SEQUENCE failed: {:?}", result.err());
}

#[test]
fn test_sequence_execution_drop() {
    let mut engine = create_engine();
    
    // Create a sequence
    engine.execute("CREATE SEQUENCE seq1").unwrap();
    
    // Drop the sequence
    let result = engine.execute("DROP SEQUENCE seq1");
    assert!(result.is_ok(), "DROP SEQUENCE failed: {:?}", result.err());
}

#[test]
fn test_sequence_execution_if_not_exists() {
    let mut engine = create_engine();
    
    // Create sequence first time
    let result = engine.execute("CREATE SEQUENCE seq3");
    assert!(result.is_ok());
    
    // Try to create again without IF NOT EXISTS - should fail
    let result = engine.execute("CREATE SEQUENCE seq3");
    assert!(result.is_err());
    
    // Create with IF NOT EXISTS - should succeed (no-op)
    let result = engine.execute("CREATE SEQUENCE IF NOT EXISTS seq3");
    assert!(result.is_ok());
}

#[test]
fn test_sequence_execution_drop_if_exists() {
    let mut engine = create_engine();
    
    // Create sequence
    engine.execute("CREATE SEQUENCE seq4").unwrap();
    
    // Drop with IF EXISTS - should succeed
    let result = engine.execute("DROP SEQUENCE IF EXISTS seq4");
    assert!(result.is_ok());
    
    // Drop again with IF EXISTS - should succeed (no-op)
    let result = engine.execute("DROP SEQUENCE IF EXISTS seq4");
    assert!(result.is_ok());
    
    // Drop without IF EXISTS - should fail
    let result = engine.execute("DROP SEQUENCE seq4");
    assert!(result.is_err());
}
