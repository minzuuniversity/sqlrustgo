// SAVEPOINT Integration Tests (Issue #892)
//
// Note: These tests verify parsing and basic transaction flow.

use parking_lot::RwLock;
use sqlrustgo::{parse, ExecutionEngine, MemoryStorage};
use sqlrustgo_types::Value;
use std::sync::Arc;

#[test]
fn test_savepoint_parsing() {
    // Test SAVEPOINT parsing
    let result = parse("SAVEPOINT sp1");
    assert!(result.is_ok(), "SAVEPOINT should parse: {:?}", result.err());

    let result = parse("ROLLBACK TO SAVEPOINT sp1");
    assert!(
        result.is_ok(),
        "ROLLBACK TO SAVEPOINT should parse: {:?}",
        result.err()
    );

    let result = parse("RELEASE SAVEPOINT sp1");
    assert!(
        result.is_ok(),
        "RELEASE SAVEPOINT should parse: {:?}",
        result.err()
    );

    println!("✓ SAVEPOINT parsing works");
}

#[test]
fn test_transaction_basic() {
    // Basic transaction test
    let mut engine = ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())));

    engine.execute("CREATE TABLE tx_test (id INTEGER)").unwrap();

    engine.execute("BEGIN").unwrap();
    engine.execute("INSERT INTO tx_test VALUES (1)").unwrap();

    // Commit
    engine.execute("COMMIT").unwrap();

    // Verify row was inserted
    let result = engine.execute("SELECT COUNT(*) FROM tx_test").unwrap();
    assert_eq!(result.rows[0][0], Value::Integer(1));

    println!("✓ Basic transaction works");
}

#[test]
fn test_multiple_savepoints_parsing() {
    // Test multiple savepoints can be parsed
    let result = parse("SAVEPOINT sp1");
    assert!(result.is_ok());

    let result = parse("SAVEPOINT sp2");
    assert!(result.is_ok());

    let result = parse("ROLLBACK TO SAVEPOINT sp1");
    assert!(result.is_ok());

    let result = parse("RELEASE SAVEPOINT sp2");
    assert!(result.is_ok());

    println!("✓ Multiple SAVEPOINTs parsing works");
}

#[test]
fn test_nested_savepoint_parsing() {
    // Test nested savepoint parsing. The lexer reserves `OUTER` for
    // `OUTER JOIN`, so savepoint names must avoid that token (and other
    // reserved keywords); use `outer_sp` / `inner_sp` here.
    let result = parse("SAVEPOINT outer_sp");
    assert!(
        result.is_ok(),
        "SAVEPOINT outer_sp should parse: {:?}",
        result.err()
    );

    let result = parse("SAVEPOINT inner_sp");
    assert!(
        result.is_ok(),
        "SAVEPOINT inner_sp should parse: {:?}",
        result.err()
    );

    let result = parse("ROLLBACK TO SAVEPOINT outer_sp");
    assert!(
        result.is_ok(),
        "ROLLBACK TO SAVEPOINT outer_sp should parse: {:?}",
        result.err()
    );

    println!("✓ Nested SAVEPOINTs parsing works");
}
