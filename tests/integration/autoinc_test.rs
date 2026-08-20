// AUTO_INCREMENT Integration Tests (Issue #889)

use parking_lot::RwLock;
use sqlrustgo::{parse, ExecutionEngine, MemoryStorage};
use sqlrustgo_types::Value;
use std::sync::Arc;

#[test]
fn test_autoinc_parsing() {
    // Test AUTO_INCREMENT parsing
    let result = parse("CREATE TABLE orders (id INTEGER AUTO_INCREMENT PRIMARY KEY, name TEXT)");
    assert!(
        result.is_ok(),
        "AUTO_INCREMENT should parse: {:?}",
        result.err()
    );

    // Also test AUTOINCREMENT (SQLite variant)
    let result = parse("CREATE TABLE items (id INTEGER AUTOINCREMENT PRIMARY KEY)");
    assert!(result.is_ok(), "AUTOINCREMENT should parse");

    println!("✓ AUTO_INCREMENT parsing works");
}

#[test]
fn test_autoinc_insert() {
    // Test AUTO_INCREMENT behavior
    let mut engine = ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())));

    engine
        .execute("CREATE TABLE orders (id INTEGER AUTO_INCREMENT PRIMARY KEY, name TEXT)")
        .unwrap();

    // Insert without specifying id
    engine
        .execute("INSERT INTO orders (name) VALUES ('first')")
        .unwrap();

    // Insert another
    engine
        .execute("INSERT INTO orders (name) VALUES ('second')")
        .unwrap();

    // Should have 2 rows
    let count = engine.execute("SELECT COUNT(*) FROM orders").unwrap();
    assert_eq!(count.rows[0][0], Value::Integer(2));

    println!("✓ AUTO_INCREMENT insert test completed");
}

#[test]
fn test_autoinc_with_explicit_id() {
    // Test AUTO_INCREMENT with explicit id
    let mut engine = ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())));

    engine
        .execute("CREATE TABLE items (id INTEGER AUTO_INCREMENT PRIMARY KEY, name TEXT)")
        .unwrap();

    // Insert with explicit id
    engine
        .execute("INSERT INTO items VALUES (100, 'explicit')")
        .unwrap();

    // Insert without id (should get next auto value)
    engine
        .execute("INSERT INTO items (name) VALUES ('auto')")
        .unwrap();

    // Should have 2 rows
    let count = engine.execute("SELECT COUNT(*) FROM items").unwrap();
    assert_eq!(count.rows[0][0], Value::Integer(2));

    println!("✓ AUTO_INCREMENT with explicit ID works");
}

#[test]
fn test_autoinc_multiple_columns() {
    // Test AUTO_INCREMENT on non-PK column
    let result =
        parse("CREATE TABLE logs (id INTEGER PRIMARY KEY, seq INTEGER AUTO_INCREMENT, data TEXT)");
    assert!(result.is_ok(), "AUTO_INCREMENT on non-PK should parse");

    println!("✓ AUTO_INCREMENT on non-PK column parsing works");
}
