#![cfg(test)]

use crate::execution_engine::*;
use crate::Value;
use sqlrustgo_storage::MemoryStorage;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[test]
fn test_analyze_table_stats() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);

    engine
        .execute("CREATE TABLE users (id INTEGER, name TEXT, age INTEGER)")
        .unwrap();
    engine.execute("BEGIN").unwrap();
    engine
        .execute("INSERT INTO users VALUES (1, 'Alice', 30)")
        .unwrap();
    engine
        .execute("INSERT INTO users VALUES (2, 'Bob', 25)")
        .unwrap();
    engine
        .execute("INSERT INTO users VALUES (3, 'Charlie', 30)")
        .unwrap();
    engine.execute("COMMIT").unwrap();

    let result = engine.execute("ANALYZE users").unwrap();
    assert_eq!(result.affected_rows, 1);
    assert_eq!(result.rows[0][0], Value::Integer(3));

    let stats = engine.get_table_stats();
    let stats_guard = stats.read().unwrap();
    let table_stats = stats_guard.table_stats.get("users").unwrap();
    assert_eq!(table_stats.row_count, 3);
}

#[test]
fn test_execution_stats_default() {
    let stats = ExecutionStats::default();
    assert!(stats.table_stats.is_empty());
}

#[test]
fn test_table_statistics() {
    let mut stats = HashMap::new();
    stats.insert(
        "users".to_string(),
        TableStatistics {
            row_count: 100,
            column_stats: HashMap::new(),
        },
    );

    let exec_stats = ExecutionStats { table_stats: stats };
    assert_eq!(exec_stats.table_stats.get("users").unwrap().row_count, 100);
}

#[test]
fn test_estimate_row_count() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);

    engine
        .execute("CREATE TABLE users (id INTEGER, name TEXT)")
        .unwrap();
    engine.execute("BEGIN").unwrap();
    engine
        .execute("INSERT INTO users VALUES (1, 'Alice')")
        .unwrap();
    engine
        .execute("INSERT INTO users VALUES (2, 'Bob')")
        .unwrap();
    engine
        .execute("INSERT INTO users VALUES (3, 'Charlie')")
        .unwrap();
    engine.execute("COMMIT").unwrap();

    // Before ANALYZE, should return default estimate
    assert_eq!(engine.estimate_row_count("users"), 1000);

    // After ANALYZE, should return actual count
    engine.execute("ANALYZE users").unwrap();
    assert_eq!(engine.estimate_row_count("users"), 3);
}

#[test]
fn test_estimate_selectivity() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);

    engine
        .execute("CREATE TABLE users (id INTEGER, name TEXT)")
        .unwrap();
    engine.execute("BEGIN").unwrap();
    for i in 0..100 {
        engine
            .execute(&format!("INSERT INTO users VALUES ({}, 'User{}')", i, i))
            .unwrap();
    }
    engine.execute("COMMIT").unwrap();

    // Before ANALYZE, should return default selectivity
    let selectivity = engine.estimate_selectivity("users", "id");
    assert_eq!(selectivity, 0.1); // Default 10%

    // After ANALYZE with distinct_count, should return better estimate
    engine.execute("ANALYZE users").unwrap();
    let selectivity = engine.estimate_selectivity("users", "id");
    assert_eq!(selectivity, 0.01); // 1/100 distinct values
}

#[test]
fn test_optimize_join_order() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);

    // Create tables of different sizes
    engine.execute("CREATE TABLE large (id INTEGER)").unwrap();
    engine.execute("CREATE TABLE medium (id INTEGER)").unwrap();
    engine.execute("CREATE TABLE small (id INTEGER)").unwrap();

    engine.execute("BEGIN").unwrap();
    for i in 0..1000 {
        engine
            .execute(&format!("INSERT INTO large VALUES ({})", i))
            .unwrap();
    }
    for i in 0..100 {
        engine
            .execute(&format!("INSERT INTO medium VALUES ({})", i))
            .unwrap();
    }
    for i in 0..10 {
        engine
            .execute(&format!("INSERT INTO small VALUES ({})", i))
            .unwrap();
    }
    engine.execute("COMMIT").unwrap();

    // Analyze to get accurate row counts
    engine.execute("ANALYZE large").unwrap();
    engine.execute("ANALYZE medium").unwrap();
    engine.execute("ANALYZE small").unwrap();

    let tables = vec!["large", "medium", "small"];
    let optimal = engine.optimize_join_order(&tables);

    // Smallest table should be first after ANALYZE
    assert_eq!(optimal[0], "small");
    // Should have all tables
    assert_eq!(optimal.len(), 3);
}

#[test]
fn test_cbo_disable() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::with_cbo(storage, false);
    assert!(!engine.is_cbo_enabled());
    engine.set_cbo_enabled(true);
    assert!(engine.is_cbo_enabled());
}

#[test]
fn test_memory_engine_with_cbo() {
    let engine = ExecutionEngine::with_memory();
    assert!(engine.is_cbo_enabled());

    let engine_disabled = ExecutionEngine::with_memory_and_cbo(false);
    assert!(!engine_disabled.is_cbo_enabled());
}

#[test]
fn test_estimate_index_benefit() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);

    engine
        .execute("CREATE TABLE users (id INTEGER, name TEXT)")
        .unwrap();
    engine.execute("BEGIN").unwrap();
    for i in 0..1000 {
        engine
            .execute(&format!("INSERT INTO users VALUES ({}, 'User{}')", i, i))
            .unwrap();
    }
    engine.execute("COMMIT").unwrap();

    // High selectivity (1/1000) - index should be very beneficial
    let high_sel = engine.estimate_selectivity("users", "id");
    let benefit = engine.estimate_index_benefit("users", high_sel);
    assert!(benefit > 0.0); // Index should be beneficial

    // With ANALYZE, we get actual stats
    engine.execute("ANALYZE users").unwrap();
    let benefit_after_analyze = engine.estimate_index_benefit("users", high_sel);
    assert!(benefit_after_analyze > 0.0);
}

#[test]
fn test_should_use_index() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);

    engine
        .execute("CREATE TABLE users (id INTEGER, name TEXT)")
        .unwrap();
    engine.execute("BEGIN").unwrap();
    for i in 0..10000 {
        engine
            .execute(&format!("INSERT INTO users VALUES ({}, 'User{}')", i, i))
            .unwrap();
    }
    engine.execute("COMMIT").unwrap();

    // With low selectivity (high cardinality), index is beneficial
    let use_index = engine.should_use_index("users", "id");
    assert!(use_index);

    // After ANALYZE, should still recommend index for high cardinality
    engine.execute("ANALYZE users").unwrap();
    let use_index_after = engine.should_use_index("users", "id");
    assert!(use_index_after);
}

#[test]
fn test_estimate_join_cost() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);

    engine
        .execute("CREATE TABLE orders (id INTEGER, user_id INTEGER)")
        .unwrap();
    engine
        .execute("CREATE TABLE users (id INTEGER, name TEXT)")
        .unwrap();

    engine.execute("BEGIN").unwrap();
    for i in 0..100 {
        engine
            .execute(&format!("INSERT INTO orders VALUES ({}, {})", i, i % 10))
            .unwrap();
    }
    for i in 0..10 {
        engine
            .execute(&format!("INSERT INTO users VALUES ({}, 'User{}')", i, i))
            .unwrap();
    }
    engine.execute("COMMIT").unwrap();

    let hash_cost = engine.estimate_join_cost("orders", "users", "hash");
    let nl_cost = engine.estimate_join_cost("orders", "users", "nested_loop");
    let merge_cost = engine.estimate_join_cost("orders", "users", "merge");

    // Hash join should be reasonable
    assert!(hash_cost > 0.0);
    assert!(nl_cost > 0.0);
    assert!(merge_cost > 0.0);
}

#[test]
fn test_optimize_join_order_after_analyze() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);

    engine.execute("CREATE TABLE t1 (id INTEGER)").unwrap();
    engine.execute("CREATE TABLE t2 (id INTEGER)").unwrap();
    engine.execute("CREATE TABLE t3 (id INTEGER)").unwrap();

    engine.execute("BEGIN").unwrap();
    for i in 0..500 {
        engine
            .execute(&format!("INSERT INTO t1 VALUES ({})", i))
            .unwrap();
    }
    for i in 0..50 {
        engine
            .execute(&format!("INSERT INTO t2 VALUES ({})", i))
            .unwrap();
    }
    for i in 0..5 {
        engine
            .execute(&format!("INSERT INTO t3 VALUES ({})", i))
            .unwrap();
    }
    engine.execute("COMMIT").unwrap();

    // Analyze to get accurate stats
    engine.execute("ANALYZE t1").unwrap();
    engine.execute("ANALYZE t2").unwrap();
    engine.execute("ANALYZE t3").unwrap();

    let tables = vec!["t1", "t2", "t3"];
    let optimal = engine.optimize_join_order(&tables);

    // Smallest (t3 with 5 rows) should be first after ANALYZE
    assert_eq!(optimal[0], "t3");
}

// ========================================================================
// TX-LIFECYCLE TESTS (TASK_REGISTRY: TX-001 ~ TX-006)
// Hermes B: Shadow Tester — Contract Validation
// Purpose: Verify EEK (Execution Enforcement Kernel) panics on TX violations
// Source: docs/governance/wal/TX_LIFECYCLE_SPEC.md §2.2
// ========================================================================

#[test]
fn test_tx_lifecycle_insert_without_tx_autocommits() {
    // TX-001: INSERT without BEGIN → autocommit (valid in v3.8.0 AUTOCOMMIT mode)
    // Source: v3.8.0 ARCH DECISION — AUTOCOMMIT semantics adopted
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);
    engine
        .execute("CREATE TABLE t1 (id INTEGER, name TEXT)")
        .unwrap();
    // DML without explicit BEGIN → must succeed via autocommit
    let result = engine.execute("INSERT INTO t1 VALUES (1, 'test')");
    assert!(
        result.is_ok(),
        "INSERT without explicit TX should autocommit in v3.8.0"
    );
}

#[test]
fn test_tx_lifecycle_update_without_tx_autocommits() {
    // TX-002: UPDATE without BEGIN → autocommit
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);
    engine
        .execute("CREATE TABLE t1 (id INTEGER, name TEXT)")
        .unwrap();
    engine.execute("INSERT INTO t1 VALUES (1, 'test')").unwrap();
    // DML without explicit BEGIN → must succeed via autocommit
    let result = engine.execute("UPDATE t1 SET name = 'updated' WHERE id = 1");
    assert!(
        result.is_ok(),
        "UPDATE without explicit TX should autocommit in v3.8.0"
    );
}

#[test]
fn test_tx_lifecycle_delete_without_tx_autocommits() {
    // TX-003: DELETE without BEGIN → autocommit
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);
    engine
        .execute("CREATE TABLE t1 (id INTEGER, name TEXT)")
        .unwrap();
    engine.execute("INSERT INTO t1 VALUES (1, 'test')").unwrap();
    // DML without explicit BEGIN → must succeed via autocommit
    let result = engine.execute("DELETE FROM t1 WHERE id = 1");
    assert!(
        result.is_ok(),
        "DELETE without explicit TX should autocommit in v3.8.0"
    );
}

#[test]
#[should_panic(expected = "transaction already committed")]
fn test_tx_lifecycle_insert_after_commit_panics() {
    // TX-004: INSERT after COMMIT → must panic
    // Source: TX_LIFECYCLE_SPEC.md §2.2 "COMMITTED | DML | panic"
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);
    engine
        .execute("CREATE TABLE t1 (id INTEGER, name TEXT)")
        .unwrap();
    engine.execute("BEGIN").unwrap();
    engine.execute("INSERT INTO t1 VALUES (1, 'test')").unwrap();
    engine.execute("COMMIT").unwrap();
    // INSERT after COMMIT → must panic
    engine
        .execute("INSERT INTO t1 VALUES (2, 'after_commit')")
        .unwrap();
}

#[test]
#[should_panic(expected = "transaction already aborted")]
fn test_tx_lifecycle_insert_after_rollback_panics() {
    // TX-005: INSERT after ROLLBACK → must panic
    // Source: TX_LIFECYCLE_SPEC.md §2.2 "ABORTED | DML | panic"
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);
    engine
        .execute("CREATE TABLE t1 (id INTEGER, name TEXT)")
        .unwrap();
    engine.execute("BEGIN").unwrap();
    engine.execute("INSERT INTO t1 VALUES (1, 'test')").unwrap();
    engine.execute("ROLLBACK").unwrap();
    // INSERT after ROLLBACK → must panic
    engine
        .execute("INSERT INTO t1 VALUES (2, 'after_rollback')")
        .unwrap();
}

#[test]
#[should_panic(expected = "transaction already committed")]
fn test_tx_lifecycle_double_commit_panics() {
    // TX-006: COMMIT twice → must panic
    // Source: TX_LIFECYCLE_SPEC.md §2.2 "COMMITTED | COMMIT | panic"
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);
    engine.execute("BEGIN").unwrap();
    engine.execute("COMMIT").unwrap();
    // Double COMMIT → must panic
    engine.execute("COMMIT").unwrap();
}

// ========================================================================
// WAL CONTRACT TESTS (TASK_REGISTRY: WAL-003 ~ WAL-005)
// Hermes B: Shadow Tester — WAL Ordering Validation
// Source: docs/governance/wal/WAL_CONTRACT.md §1.1
// Note: WAL not yet implemented — tests will PASS after IMPL-002
// ========================================================================

#[test]
#[should_panic(expected = "WAL")]
fn test_wal_contract_insert_without_wal_panics() {
    // WAL-003: INSERT without WAL entry → must panic
    // Source: WAL_CONTRACT.md "铁律 #WAL-001: DML must write WAL before data page"
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);
    engine.execute("CREATE TABLE t1 (id INTEGER)").unwrap();
    engine.execute("BEGIN").unwrap();
    let _ = engine.execute("INSERT INTO t1 VALUES (1)");
    panic!("INSERT without WAL did not panic — WAL not enforced");
}

#[test]
#[should_panic(expected = "WAL")]
fn test_wal_contract_update_without_wal_panics() {
    // WAL-004: UPDATE without WAL entry → must panic
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);
    engine.execute("CREATE TABLE t1 (id INTEGER)").unwrap();
    engine.execute("BEGIN").unwrap();
    engine.execute("INSERT INTO t1 VALUES (1)").unwrap();
    let _ = engine.execute("UPDATE t1 SET id = 2 WHERE id = 1");
    panic!("UPDATE without WAL did not panic — WAL not enforced");
}

#[test]
#[should_panic(expected = "WAL")]
fn test_wal_contract_delete_without_wal_panics() {
    // WAL-005: DELETE without WAL entry → must panic
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);
    engine.execute("CREATE TABLE t1 (id INTEGER)").unwrap();
    engine.execute("BEGIN").unwrap();
    engine.execute("INSERT INTO t1 VALUES (1)").unwrap();
    let _ = engine.execute("DELETE FROM t1 WHERE id = 1");
    panic!("DELETE without WAL did not panic — WAL not enforced");
}
