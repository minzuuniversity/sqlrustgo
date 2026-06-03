//! LIKE Pattern Tests
//!
//! TPC-H Q9: `WHERE p_name LIKE '%green%'`. The AST already has
//! `Expression::Like(expr, pattern, escape)` but `evaluate_expression`
//! in src/expr_utils.rs has no arm for it, so the predicate falls
//! through to Null. These tests verify the substring-match path.

use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use std::sync::{Arc, RwLock};

fn create_engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

#[test]
fn test_like_substring_matches() {
    // TPC-H Q9 pattern: `p_name LIKE '%green%'`. Should keep rows whose
    // name contains 'green' as a substring.
    let mut engine = create_engine();
    engine
        .execute("CREATE TABLE part (p_partkey INTEGER, p_name TEXT)")
        .unwrap();
    engine
        .execute(
            "INSERT INTO part VALUES \
             (1, 'green widget'), \
             (2, 'red widget'), \
             (3, 'forest green leaves'), \
             (4, 'plain widget')",
        )
        .unwrap();

    let result = engine
        .execute("SELECT p_partkey FROM part WHERE p_name LIKE '%green%'")
        .unwrap();

    // Rows 1 and 3 contain 'green' as a substring.
    assert_eq!(
        result.rows.len(),
        2,
        "LIKE '%green%' should keep 2 rows, got {:?}",
        result.rows
    );
}

#[test]
fn test_like_no_match() {
    let mut engine = create_engine();
    engine.execute("CREATE TABLE t (x TEXT)").unwrap();
    engine
        .execute("INSERT INTO t VALUES ('alpha'), ('beta'), ('gamma')")
        .unwrap();

    let result = engine
        .execute("SELECT x FROM t WHERE x LIKE '%zzz%'")
        .unwrap();

    assert_eq!(result.rows.len(), 0);
}

#[test]
fn test_like_exact_match() {
    // LIKE without wildcards is an exact (case-insensitive) match.
    let mut engine = create_engine();
    engine.execute("CREATE TABLE t (x TEXT)").unwrap();
    engine
        .execute("INSERT INTO t VALUES ('hello'), ('Hello'), ('world')")
        .unwrap();

    // Case-insensitive by default: 'hello' and 'Hello' both match.
    let result = engine
        .execute("SELECT x FROM t WHERE x LIKE 'hello'")
        .unwrap();

    assert_eq!(result.rows.len(), 2);
}
