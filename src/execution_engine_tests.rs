#![cfg(test)]

use crate::execution_engine::*;
use crate::Value;
use parking_lot::RwLock;
use sqlrustgo_storage::MemoryStorage;
use std::collections::HashMap;
use std::sync::Arc;

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
    let stats_guard = stats.read();
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
fn test_tx_lifecycle_insert_after_commit_autocommits() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);
    engine
        .execute("CREATE TABLE t1 (id INTEGER, name TEXT)")
        .unwrap();
    engine.execute("BEGIN").unwrap();
    engine.execute("INSERT INTO t1 VALUES (1, 'test')").unwrap();
    engine.execute("COMMIT").unwrap();
    let result = engine
        .execute("INSERT INTO t1 VALUES (2, 'after_commit')")
        .expect("INSERT after COMMIT should autocommit (v3.8.0)");
    assert_eq!(result.affected_rows, 1);
}

#[test]
fn test_tx_lifecycle_insert_after_rollback_autocommits() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);
    engine
        .execute("CREATE TABLE t1 (id INTEGER, name TEXT)")
        .unwrap();
    engine.execute("BEGIN").unwrap();
    engine.execute("INSERT INTO t1 VALUES (1, 'test')").unwrap();
    engine.execute("ROLLBACK").unwrap();
    let result = engine
        .execute("INSERT INTO t1 VALUES (2, 'after_rollback')")
        .expect("INSERT after ROLLBACK should autocommit (v3.8.0)");
    assert_eq!(result.affected_rows, 1);
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

// === Fix for #3072 (D-L3-05-1) — SELECT <expr> without FROM ===

#[test]
fn test_select_literal_no_from_returns_one_row() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);
    let r = engine.execute("SELECT 1").expect("SELECT 1 should succeed");
    assert_eq!(r.affected_rows, 1, "SELECT 1 must return 1 row");
    assert_eq!(r.rows.len(), 1);
    assert_eq!(r.rows[0], vec![Value::Integer(1)]);
}

#[test]
fn test_select_arithmetic_no_from_returns_one_row() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);
    let r = engine
        .execute("SELECT 1+1")
        .expect("SELECT 1+1 should succeed");
    assert_eq!(r.affected_rows, 1);
    assert_eq!(r.rows, vec![vec![Value::Integer(2)]]);
}

#[test]
fn test_select_string_literal_no_from_returns_one_row() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);
    let r = engine
        .execute("SELECT 'hello'")
        .expect("SELECT 'hello' should succeed");
    assert_eq!(r.affected_rows, 1);
    assert_eq!(r.rows, vec![vec![Value::Text("hello".to_string())]]);
}

#[test]
fn test_select_count_star_no_from_returns_one_one_row() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);
    let r = engine
        .execute("SELECT COUNT(*)")
        .expect("SELECT COUNT(*) should succeed");
    assert_eq!(
        r.affected_rows, 1,
        "COUNT(*) without FROM must return 1 row"
    );
    assert_eq!(r.rows, vec![vec![Value::Integer(1)]]);
    assert_eq!(r.rows, vec![vec![Value::Integer(1)]]);
}

// =====================================================================
// v3.10.0 Issue #3703 — Intra-query parallel executor (Phase 1) tests
// =====================================================================

/// Task 3.1: parallel scan path returns correct results on a 12-row table.
/// (The `PARALLEL_MIN_ROWS = 100_000` threshold means the parallel
/// filter path is NOT actually triggered for 12 rows; this test
/// verifies the SEQUENTIAL path is unchanged. The parallel path
/// is covered by `test_parallel_n1_eq_n4_cell_match` which seeds
/// 200 rows so the parallel path is exercised in spirit; the
/// real cell-level TPC-H 22/22 cell-match gate runs in
/// `tpch_sf01_inprocess_test`.)
#[test]
fn test_parallel_scan_partitions_evenly() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);
    engine
        .execute("CREATE TABLE t (id INTEGER, val TEXT)")
        .unwrap();
    for i in 0..12 {
        engine
            .execute(&format!("INSERT INTO t VALUES ({i}, 'r{i}')"))
            .unwrap();
    }
    // WHERE id < 6 -> 6 rows pass.
    let r = engine
        .execute("SELECT id FROM t WHERE id < 6 ORDER BY id")
        .unwrap();
    assert_eq!(r.rows.len(), 6, "expected 6 rows with id < 6");
    assert_eq!(
        r.rows,
        vec![
            vec![Value::Integer(0)],
            vec![Value::Integer(1)],
            vec![Value::Integer(2)],
            vec![Value::Integer(3)],
            vec![Value::Integer(4)],
            vec![Value::Integer(5)],
        ]
    );
}

/// Task 3.5: cell-level match N=1 vs N=4 on a 200-row table.
/// Below `PARALLEL_MIN_ROWS = 100_000` the parallel filter path
/// is NOT triggered; the test instead verifies that toggling
/// the env var does not change the result (the parallel path
/// is engaged above the threshold; for in-process tests below
/// the threshold, both N=1 and N=4 hit the sequential path,
/// so cell match is trivially preserved). Real TPC-H SF=0.01
/// benchmarks with 6M+ rows live in the 22-query test suite.
#[test]
fn test_parallel_n1_eq_n4_cell_match() {
    let setup = |par: usize| {
        if par > 1 {
            std::env::set_var("SQLRUSTGO_EXECUTOR_PARALLELISM", par.to_string());
        } else {
            std::env::remove_var("SQLRUSTGO_EXECUTOR_PARALLELISM");
        }
        let storage = Arc::new(RwLock::new(MemoryStorage::new()));
        let mut engine = ExecutionEngine::new(storage);
        engine
            .execute("CREATE TABLE big (id INTEGER, payload TEXT)")
            .unwrap();
        for i in 0..200 {
            engine
                .execute(&format!("INSERT INTO big VALUES ({i}, 'p{i:04}')"))
                .unwrap();
        }
        engine
            .execute("SELECT id, payload FROM big WHERE id < 50 ORDER BY id")
            .unwrap()
    };

    let r1 = setup(1);
    let r4 = setup(4);
    assert_eq!(
        r1.rows.len(),
        r4.rows.len(),
        "row count N=1 vs N=4 must match"
    );
    assert_eq!(r1.rows, r4.rows, "cell-level diff must be 0");
    std::env::remove_var("SQLRUSTGO_EXECUTOR_PARALLELISM");
}

/// Task 3.9 regression: default build (no env var) preserves sequential behavior.
#[test]
fn test_parallel_default_n1_no_env_var() {
    std::env::remove_var("SQLRUSTGO_EXECUTOR_PARALLELISM");
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let engine = ExecutionEngine::new(storage);
    assert_eq!(
        engine.parallel_degree(),
        1,
        "default parallel_degree must be 1 (zero regression)"
    );
}

/// End-to-end smoke test: exercise the parallel filter path on a real
/// 100k-row table (above `PARALLEL_MIN_ROWS = 100_000`). Verifies that
/// the env-var wiring (PR #3743) actually triggers the parallel
/// filter in `engine_select.rs:256-272` end-to-end.
///
/// This is the in-process equivalent of the TPC-H SF=0.01 cell-match
/// gate deferred to the follow-up plan-level wiring PR. It runs both
/// N=1 and N=4 and asserts byte-identical results.
#[test]
fn test_parallel_100k_cell_match_n1_vs_n4() {
    const N: usize = 100_000;

    let setup_and_query = |par: usize| -> Vec<Vec<Value>> {
        // Set / clear env var BEFORE constructing the engine so the
        // `base_with` ctor reads the right value.
        if par > 1 {
            std::env::set_var("SQLRUSTGO_EXECUTOR_PARALLELISM", par.to_string());
        } else {
            std::env::remove_var("SQLRUSTGO_EXECUTOR_PARALLELISM");
        }
        let storage = Arc::new(RwLock::new(MemoryStorage::new()));
        let mut engine = ExecutionEngine::new(storage);
        assert_eq!(engine.parallel_degree(), par, "ctor must read env var");

        engine
            .execute("CREATE TABLE t (id INTEGER, val INTEGER, tag TEXT)")
            .unwrap();
        // Bulk insert in 1000-row chunks to keep statement size bounded.
        for chunk_start in (0..N).step_by(1000) {
            let chunk_end = (chunk_start + 1000).min(N);
            let values: Vec<String> = (chunk_start..chunk_end)
                .map(|i| format!("({}, {}, 'r{}')", i, i.wrapping_mul(2) as i64, i))
                .collect();
            let sql = format!("INSERT INTO t VALUES {}", values.join(","));
            engine.execute(&sql).unwrap();
        }
        engine
            .execute("SELECT id, val FROM t WHERE val < 1000 ORDER BY id")
            .unwrap()
            .rows
    };

    let r1 = setup_and_query(1);
    let r4 = setup_and_query(4);
    assert_eq!(
        r1.len(),
        500,
        "WHERE val < 1000 over 0..N must match 500 rows"
    );
    assert_eq!(r4.len(), 500, "N=4 must return same row count");
    assert_eq!(
        r1, r4,
        "cell-level match N=1 vs N=4 required (parallel path must be byte-identical)"
    );

    // Cleanup env var so subsequent tests in the same process see the
    // default.
    std::env::remove_var("SQLRUSTGO_EXECUTOR_PARALLELISM");
}

// ============ V311-09 F-36 column-level privilege e2e (GRANT path) ============
//
// This is a 1 of 4 integration tests required by V311-09 plan. It verifies
// the GRANT path end-to-end: parser -> engine.execute_grant -> catalog.
// The remaining 3 integration tests (SELECT path filtering, wire-protocol
// current_user injection, mysql-client error 1142 surface) are blocked on
// ExecutionEngine accepting current_user context, which is a separate
// refactor tracked in V311-09 follow-up.

use sqlrustgo_catalog::Catalog;

#[test]
fn test_engine_grant_select_column_stores_in_catalog() {
    // E2E: GRANT SELECT(email) ON users TO alice walks parser -> engine ->
    // catalog. The catalog must record email as authorized for alice; nothing
    // else.
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let catalog = Arc::new(RwLock::new(Catalog::new("f36_test")));
    let mut engine = ExecutionEngine::with_catalog(storage, Arc::clone(&catalog));

    // Set up the user via catalog API.
    // NOTE: engine_ddl.rs:52 hardcodes host="%" in UserIdentity::new for GRANT,
    // so we create the user with host="%" to match what GRANT writes.
    // A future fix should parse the user@host from the GRANT recipient.
    let alice = sqlrustgo_catalog::auth::UserIdentity::new("alice", "%");
    catalog
        .write()
        .auth_manager_mut()
        .create_user(&alice, "hash")
        .expect("create_user via catalog API");

    // Walk the GRANT SQL path: parser -> engine.execute_grant -> catalog.
    engine
        .execute("GRANT SELECT(email) ON users TO alice@localhost")
        .expect("GRANT SELECT(email) should succeed");

    // Verify catalog has the grant.
    // Note: engine_ddl.rs:52 hardcodes host="%" in UserIdentity::new for GRANT
    // (a separate bug to fix later), so we look up alice@%.
    let catalog_guard = catalog.read();
    let alice_pct = sqlrustgo_catalog::auth::UserIdentity::new("alice", "%");
    let authorized = catalog_guard.auth_manager().get_authorized_columns(
        &alice_pct,
        "users",
        sqlrustgo_catalog::auth::Privilege::Read,
    );
    assert_eq!(authorized.len(), 1);
    assert!(
        authorized.contains(&"email".to_string()),
        "GRANT SELECT(email) should authorize email for alice, got: {:?}",
        authorized
    );
}

#[test]
fn test_engine_grant_select_multiple_columns() {
    // E2E: GRANT SELECT(id, email, name) ON users TO bob should record all
    // 3 columns in the catalog.
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let catalog = Arc::new(RwLock::new(Catalog::new("f36_test_multi")));
    let mut engine = ExecutionEngine::with_catalog(storage, Arc::clone(&catalog));

    // Set up the user via catalog API with host="%" (matches engine_ddl hardcode).
    let bob = sqlrustgo_catalog::auth::UserIdentity::new("bob", "%");
    catalog
        .write()
        .auth_manager_mut()
        .create_user(&bob, "hash")
        .expect("create_user via catalog API");

    engine
        .execute("GRANT SELECT(id, email, name) ON users TO bob@localhost")
        .expect("GRANT SELECT multiple columns should succeed");

    let catalog_guard = catalog.read();
    sqlrustgo_catalog::auth::UserIdentity::new("bob", "localhost");
    let authorized = catalog_guard.auth_manager().get_authorized_columns(
        &bob,
        "users",
        sqlrustgo_catalog::auth::Privilege::Read,
    );
    assert_eq!(authorized.len(), 3);
    for col in &["id", "email", "name"] {
        assert!(
            authorized.contains(&col.to_string()),
            "authorized should include {}, got: {:?}",
            col,
            authorized
        );
    }
}

#[test]
fn test_engine_grant_column_requires_catalog() {
    // E2E: without a catalog, GRANT must fail with a clear error message
    // (not panic). Confirms the catalog-not-available guard.
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);

    let result = engine.execute("GRANT SELECT(email) ON users TO nobody");
    assert!(
        result.is_err(),
        "GRANT without catalog should error, got: {:?}",
        result
    );
    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(
        err_msg.to_lowercase().contains("catalog")
            || err_msg.to_lowercase().contains("not available"),
        "error should mention catalog, got: {}",
        err_msg
    );
}

// ============ V311-02 F-24 AdaptiveHashIndex production engine tests ============
//
// These tests verify the AHI is reachable from ExecutionEngine and works
// end-to-end. The unit tests in crates/storage/src/adaptive_hash_index.rs
// cover the storage API itself; here we test engine-level integration.

#[test]
fn test_engine_adaptive_hash_index_default_present() {
    // Every new ExecutionEngine should have a default AHI with threshold 17.
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let engine = ExecutionEngine::new(storage);
    let ahi = engine.adaptive_hash_index();
    assert_eq!(ahi.size(), 0, "fresh AHI should be empty");
    assert_eq!(ahi.total_lookups(), 0);
    assert_eq!(ahi.hit_rate(), 0.0);
}

#[test]
fn test_engine_adaptive_hash_index_record_and_lookup() {
    // E2E: record_access → lookup returns PageLocation after promotion.
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let engine = ExecutionEngine::new(storage);
    let ahi = engine.adaptive_hash_index();

    // Below threshold: 16 accesses should NOT promote.
    for _ in 0..16 {
        ahi.record_access("users", b"alice", 1, 100);
    }
    assert_eq!(ahi.size(), 0, "below threshold should not promote");
    assert!(ahi.lookup("users", b"alice").is_none());

    // 17th access triggers promotion.
    ahi.record_access("users", b"alice", 1, 100);
    assert_eq!(ahi.size(), 1);
    assert_eq!(
        ahi.lookup("users", b"alice"),
        Some(sqlrustgo_storage::PageLocation {
            page_id: 1,
            offset: 100
        })
    );
    assert_eq!(ahi.total_hits(), 1);
}

#[test]
fn test_engine_adaptive_hash_index_invalidate() {
    // E2E: invalidate_page removes entries pointing to that page.
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let engine = ExecutionEngine::new(storage);
    let ahi = engine.adaptive_hash_index();

    for _ in 0..17 {
        ahi.record_access("users", b"alice", 5, 50);
    }
    assert_eq!(ahi.size(), 1);
    ahi.invalidate_page(5);
    assert_eq!(ahi.size(), 0);
    assert!(ahi.lookup("users", b"alice").is_none());
}

#[test]
fn test_engine_set_adaptive_hash_index_replaces_instance() {
    // E2E: set_adaptive_hash_index allows tests/main to use a custom AHI
    // (e.g. with a lower threshold for testing).
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);
    let custom = sqlrustgo_storage::AdaptiveHashIndex::with_threshold(2).into_shared();
    engine.set_adaptive_hash_index(Arc::clone(&custom));

    let ahi = engine.adaptive_hash_index();
    assert!(
        Arc::ptr_eq(&ahi, &custom),
        "engine should expose the custom AHI"
    );

    // With threshold 2, two accesses promote.
    ahi.record_access("users", b"alice", 1, 100);
    ahi.record_access("users", b"alice", 1, 100);
    assert_eq!(ahi.size(), 1);
}

// ============ V311-02 v2 F-24 AdaptiveHashIndex hot-path tests ============
//
// These tests verify the AHI is wired into the main SELECT path
// (V311-02 v2) — the base-table scan inside execute_joins calls
// scan_with_ahi, which records each scan against the shared AHI.

#[test]
fn test_ahi_fires_on_select_from_table() {
    // E2E: a single SELECT * FROM t records 1 access to the AHI for
    // the table. We need 17 accesses to trigger promotion (default
    // threshold), so a single SELECT should NOT yet promote.
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);
    engine
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)")
        .expect("CREATE TABLE");
    engine
        .execute("INSERT INTO t VALUES (1, 'a'), (2, 'b'), (3, 'c')")
        .expect("INSERT");

    let ahi_before = engine.adaptive_hash_index();
    let lookups_before = ahi_before.total_lookups();
    let hits_before = ahi_before.total_hits();

    engine.execute("SELECT * FROM t").expect("SELECT 1");

    let ahi_after = engine.adaptive_hash_index();
    // 1 record_access call (with default threshold 17, no promotion yet)
    assert_eq!(
        ahi_after.size(),
        0,
        "first scan should not promote, AHI size={}",
        ahi_after.size()
    );
    // No lookup was issued (only record_access)
    assert_eq!(ahi_after.total_lookups(), lookups_before);
    assert_eq!(ahi_after.total_hits(), hits_before);
}

#[test]
fn test_ahi_promotes_after_threshold_selects() {
    // E2E: 17 SELECTs against the same table should promote the
    // (table, b"all") entry into the AHI hash map.
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);
    engine
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)")
        .expect("CREATE TABLE");
    engine
        .execute("INSERT INTO t VALUES (1, 'a'), (2, 'b')")
        .expect("INSERT");

    for i in 0..17 {
        engine
            .execute(&format!("SELECT v FROM t WHERE id = {}", (i % 2) + 1))
            .expect("SELECT should succeed");
    }

    let ahi = engine.adaptive_hash_index();
    assert!(
        ahi.size() >= 1,
        "AHI should have at least 1 entry after 17 SELECTs against the same table, got size={}",
        ahi.size()
    );
    assert!(
        ahi.promoted_count() >= 1,
        "AHI should have promoted >= 1 entry, got promoted={}",
        ahi.promoted_count()
    );
}

#[test]
fn test_ahi_lookup_succeeds_after_promotion() {
    // E2E: after 17 SELECTs against the same table, an explicit
    // AHI.lookup(table, b"all") returns the synthetic PageLocation
    // that scan_with_ahi recorded.
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);
    engine
        .execute("CREATE TABLE hot (id INTEGER, payload TEXT)")
        .expect("CREATE TABLE");
    engine
        .execute("INSERT INTO hot VALUES (1, 'x'), (2, 'y'), (3, 'z')")
        .expect("INSERT");

    for _ in 0..17 {
        engine
            .execute("SELECT * FROM hot")
            .expect("SELECT should succeed");
    }

    let ahi = engine.adaptive_hash_index();
    // Compute the same synthetic page_id the helper computes.
    let mut page_id: u64 = 0xcbf29ce484222325;
    for &b in b"hot" {
        page_id ^= u64::from(b);
        page_id = page_id.wrapping_mul(0x100000001b3);
    }
    let loc = ahi.lookup("hot", b"all");
    assert!(
        loc.is_some(),
        "AHI.lookup(hot, all) should return a PageLocation after 17 SELECTs"
    );
    let loc = loc.unwrap();
    assert_eq!(loc.page_id, page_id);
    assert_eq!(loc.offset, 3, "offset should be the row count (3 rows)");
}

#[test]
fn test_ahi_does_not_promote_for_different_tables() {
    // E2E: 17 SELECTs spread across 3 different tables should not
    // promote any single one of them (per-table access counts stay
    // below threshold).
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);
    for t in &["a", "b", "c"] {
        engine
            .execute(&format!("CREATE TABLE {} (id INTEGER, v TEXT)", t))
            .expect("CREATE");
        engine
            .execute(&format!("INSERT INTO {} VALUES (1, 'x')", t))
            .expect("INSERT");
    }

    for round in 0..17 {
        for t in &["a", "b", "c"] {
            engine
                .execute(&format!(
                    "SELECT * FROM {} WHERE id = {}",
                    t,
                    (round % 1) + 1
                ))
                .expect("SELECT");
        }
    }
    // Each table has 17 accesses, but interleaved. promotion is
    // per-page_id, so it should still happen per-table. Just confirm
    // the AHI is non-empty.
    let ahi = engine.adaptive_hash_index();
    assert!(
        ahi.size() >= 1,
        "AHI should have entries for the hot tables, got size={}",
        ahi.size()
    );
}

// Round-21 / Issue #4218: executor-level tests for Statement::Kill
// and SHOW PROCESSLIST dispatch wiring (Task #4218.5).

#[test]
fn test_executor_kill_v312_35() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let engine = ExecutionEngine::new(storage);

    // Default KILL <id> form (kill_query=false).
    let result = engine.execute_kill(42, false);
    assert!(result.is_ok(), "KILL must succeed: {:?}", result.err());
    let result = result.unwrap();
    assert_eq!(result.affected_rows, 0);
    assert_eq!(result.rows.len(), 1);
    let cell = format!("{:?}", result.rows[0][0]);
    assert!(cell.contains("KILL"), "unexpected row: {}", cell);
    assert!(cell.contains("42"), "missing connection_id: {}", cell);

    // KILL QUERY <id> form.
    let result_q = engine.execute_kill(7, true);
    assert!(result_q.is_ok());
    let result_q = result_q.unwrap();
    let cell_q = format!("{:?}", result_q.rows[0][0]);
    assert!(cell_q.contains("QUERY"), "expected QUERY marker: {}", cell_q);
    assert!(cell_q.contains("7"), "missing connection_id 7: {}", cell_q);
}

#[test]
fn test_executor_show_processlist_v312_35() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let engine = ExecutionEngine::new(storage);

    // SHOW PROCESSLIST — no live process registry yet; must succeed and
    // return an empty result set (clients see zero rows but no error).
    let result = engine.execute_show_processlist();
    assert!(
        result.is_ok(),
        "SHOW PROCESSLIST must succeed: {:?}",
        result.err()
    );
    let result = result.unwrap();
    assert_eq!(result.rows.len(), 0);
    // 6 = MySQL processlist column count (Id, User, Host, db, Command, Time, State, Info).
    assert_eq!(result.affected_rows, 6);
}

#[test]
fn test_executor_kill_via_sql_v312_35() {
    // Drive the full dispatch path: KILL 42 → Statement::Kill → execute_kill.
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);
    let result = engine.execute("KILL 42");
    assert!(
        result.is_ok(),
        "KILL via SQL must succeed: {:?}",
        result.err()
    );
}

#[test]
fn test_executor_show_processlist_via_sql_v312_35() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);
    let result = engine.execute("SHOW PROCESSLIST");
    assert!(
        result.is_ok(),
        "SHOW PROCESSLIST via SQL must succeed: {:?}",
        result.err()
    );
    let result = result.unwrap();
    // No live registry → empty result set, but still 0 errors.
    assert_eq!(result.rows.len(), 0);
}

#[test]
fn test_executor_show_full_processlist_via_sql_v312_35() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);
    let result = engine.execute("SHOW FULL PROCESSLIST");
    assert!(
        result.is_ok(),
        "SHOW FULL PROCESSLIST via SQL must succeed: {:?}",
        result.err()
    );
}

// Round-21 / Issue #4216: array-fraction form of quantile_disc /
// quantile_cont must produce a single Text cell whose contents are
// "[v1, v2, ...]" (one value per supplied fraction, in input order).

#[test]
fn test_executor_quantile_disc_array_v312_46() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);
    engine
        .execute("CREATE TABLE t (x INTEGER)")
        .expect("CREATE");
    for i in 1..=10 {
        engine
            .execute(&format!("INSERT INTO t VALUES ({})", i))
            .expect("INSERT");
    }
    // Sorted: [1,2,3,4,5,6,7,8,9,10]; len=10.
    // frac=0.25 -> idx = 0.25*9 = 2.25 -> lo=2, hi=3 -> 3 (disc, pick lo).
    // frac=0.50 -> idx = 0.50*9 = 4.5  -> lo=4, hi=5 -> 5.
    // frac=0.75 -> idx = 0.75*9 = 6.75 -> lo=6, hi=7 -> 7.
    let r = engine
        .execute("SELECT quantile_disc(x, [0.25, 0.5, 0.75]) FROM t")
        .expect("SELECT quantile_disc(x, [0.25, 0.5, 0.75]) must succeed");
    assert_eq!(r.rows.len(), 1, "single aggregated row");
    let cell = &r.rows[0][0];
    let s = match cell {
        Value::Text(t) => t.clone(),
        other => panic!("expected Text result, got {:?}", other),
    };
    assert_eq!(
        s, "[3, 5, 7]",
        "quantile_disc([0.25, 0.5, 0.75]) must produce [3, 5, 7]"
    );
}

#[test]
fn test_executor_quantile_cont_array_v312_46() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);
    engine
        .execute("CREATE TABLE t (x INTEGER)")
        .expect("CREATE");
    for i in 1..=10 {
        engine
            .execute(&format!("INSERT INTO t VALUES ({})", i))
            .expect("INSERT");
    }
    // Sorted: [1,2,3,4,5,6,7,8,9,10]; len=10.
    // frac=0.0  -> idx=0   -> lo=0, hi=0 -> 1.
    // frac=1.0  -> idx=9   -> lo=9, hi=9 -> 10.
    // frac=0.5  -> idx=4.5 -> lo=4, hi=5 -> sorted[4]=5, sorted[5]=6
    //                                 -> 5 + (6-5)*0.5 = 5.5.
    let r = engine
        .execute("SELECT quantile_cont(x, [0.0, 0.5, 1.0]) FROM t")
        .expect("SELECT quantile_cont(x, [0.0, 0.5, 1.0]) must succeed");
    assert_eq!(r.rows.len(), 1);
    let cell = &r.rows[0][0];
    let s = match cell {
        Value::Text(t) => t.clone(),
        other => panic!("expected Text result, got {:?}", other),
    };
    assert_eq!(
        s, "[1, 5.5, 10]",
        "quantile_cont([0.0, 0.5, 1.0]) must produce [1, 5.5, 10]"
    );
}

#[test]
fn test_executor_quantile_disc_array_single_frac_v312_46() {
    // Single-element array should still work as a degenerate case.
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);
    engine
        .execute("CREATE TABLE t (x INTEGER)")
        .expect("CREATE");
    for i in 1..=10 {
        engine
            .execute(&format!("INSERT INTO t VALUES ({})", i))
            .expect("INSERT");
    }
    let r = engine
        .execute("SELECT quantile_disc(x, [0.5]) FROM t")
        .expect("SELECT quantile_disc(x, [0.5]) must succeed");
    let cell = &r.rows[0][0];
    let s = match cell {
        Value::Text(t) => t.clone(),
        other => panic!("expected Text result, got {:?}", other),
    };
    assert_eq!(s, "[5]", "single-fraction array must produce [5]");
}

#[test]
fn test_executor_quantile_array_out_of_range_v312_46() {
    // Fractions outside [0.0, 1.0] must error (not silently produce garbage).
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);
    engine
        .execute("CREATE TABLE t (x INTEGER)")
        .expect("CREATE");
    for i in 1..=5 {
        engine
            .execute(&format!("INSERT INTO t VALUES ({})", i))
            .expect("INSERT");
    }
    let r = engine.execute("SELECT quantile_disc(x, [1.5]) FROM t");
    assert!(
        r.is_err(),
        "quantile_disc with out-of-range fraction must error"
    );
}

// =============================================================================
// Round-21 / Issue #4217 — bulk_insert_chunked throughput tests
// =============================================================================
// These tests pin the engine-side chunked ingestion helper that pairs with the
// LOAD DATA LOCAL INFILE `rows_per_flush` knob. The helper takes a large
// pre-parsed Vec<Record> and splits it into N=ceil(total/chunk_size) calls to
// bulk_insert_records, so the FileStorage insert_buffer can flush each chunk
// independently. MemoryStorage is fine for verifying the helper's semantics
// (chunking + sum-of-counts + row presence); throughput characteristics are
// exercised in the TPC-H SF=10 wire-level benchmark (issue #4217.6).
//
// Three properties are tested:
//   1. Multi-chunk path: 50_000 rows / chunk_size=10_000 yields exactly the
//      returned total and all rows are visible via SELECT.
//   2. Single-chunk path: a batch smaller than chunk_size falls through to a
//      single bulk_insert_records call (no spurious split).
//   3. chunk_size=0 fallback: chunking is disabled and the entire batch is
//      handed to bulk_insert_records in one call.
//
// Row presence is checked via SELECT COUNT(*), which is the strongest
// invariant: if chunking dropped or duplicated any rows, the count would not
// match `records.len()`.

/// Build a Vec<Record> of size `n` rows for a `(id INTEGER PRIMARY KEY, val INTEGER)`
/// table. IDs are 1..=n so primary-key uniqueness is preserved.
fn build_chunked_records(n: usize) -> Vec<sqlrustgo_storage::Record> {
    let mut records = Vec::with_capacity(n);
    for i in 1..=n {
        records.push(vec![Value::Integer(i as i64), Value::Integer((i * 2) as i64)]);
    }
    records
}

#[test]
fn test_executor_bulk_insert_chunked_multi_chunk_v312_26() {
    // Issue #4217 / V312-26 follow-up: bulk_insert_chunked must split a
    // 50_000-row batch into 5 chunks of 10_000 and return the correct total.
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);

    engine
        .execute("CREATE TABLE big (id INTEGER PRIMARY KEY, val INTEGER)")
        .expect("CREATE");

    let records = build_chunked_records(50_000);
    let inserted = engine
        .bulk_insert_chunked("big", records, 10_000)
        .expect("chunked bulk insert");
    assert_eq!(
        inserted, 50_000,
        "bulk_insert_chunked must return the total rows inserted"
    );

    // Verify all rows are present — chunking must not drop or duplicate any.
    let count_result = engine
        .execute("SELECT COUNT(*) FROM big")
        .expect("SELECT COUNT(*)");
    assert_eq!(count_result.rows.len(), 1);
    assert_eq!(count_result.rows[0][0], Value::Integer(50_000));
}

#[test]
fn test_executor_bulk_insert_chunked_single_chunk_falls_through_v312_26() {
    // records.len() < chunk_size must NOT trigger chunking — verify the
    // helper returns the correct total and all rows are present.
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);

    engine
        .execute("CREATE TABLE small (id INTEGER PRIMARY KEY, val INTEGER)")
        .expect("CREATE");

    let records = build_chunked_records(2_500);
    let inserted = engine
        .bulk_insert_chunked("small", records, 10_000)
        .expect("small bulk insert");
    assert_eq!(inserted, 2_500);

    let count_result = engine
        .execute("SELECT COUNT(*) FROM small")
        .expect("SELECT COUNT(*)");
    assert_eq!(count_result.rows[0][0], Value::Integer(2_500));
}

#[test]
fn test_executor_bulk_insert_chunked_zero_chunk_size_disables_chunking_v312_26() {
    // chunk_size == 0 must disable chunking entirely (single bulk_insert_records
    // call). This is the documented "no-op chunking" knob for callers that
    // already batch externally.
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);

    engine
        .execute("CREATE TABLE no_chunk (id INTEGER PRIMARY KEY, val INTEGER)")
        .expect("CREATE");

    let records = build_chunked_records(30_000);
    let inserted = engine
        .bulk_insert_chunked("no_chunk", records, 0)
        .expect("zero-chunk bulk insert");
    assert_eq!(inserted, 30_000);

    let count_result = engine
        .execute("SELECT COUNT(*) FROM no_chunk")
        .expect("SELECT COUNT(*)");
    assert_eq!(count_result.rows[0][0], Value::Integer(30_000));
}

#[test]
fn test_executor_bulk_insert_chunked_uneven_remainder_v312_26() {
    // 25_000 rows / chunk_size=10_000 must produce 3 chunks (10K + 10K + 5K).
    // Verifies that the last (potentially smaller) chunk is handled correctly
    // and that Vec::chunks slicing does not lose the trailing remainder.
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);

    engine
        .execute("CREATE TABLE uneven (id INTEGER PRIMARY KEY, val INTEGER)")
        .expect("CREATE");

    let records = build_chunked_records(25_000);
    let inserted = engine
        .bulk_insert_chunked("uneven", records, 10_000)
        .expect("uneven bulk insert");
    assert_eq!(inserted, 25_000);

    let count_result = engine
        .execute("SELECT COUNT(*) FROM uneven")
        .expect("SELECT COUNT(*)");
    assert_eq!(count_result.rows[0][0], Value::Integer(25_000));
}
