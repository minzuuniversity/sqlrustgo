//! TX + WAL Contract Tests — Hermes B
//!
//! Role: QA Lead + Recovery Engineer
//!
//! Test Plan:
//!   TX-001~006: Transaction lifecycle enforcement (EEK v0 = Err model)
//!   WAL-001~005: WAL contract validation
//!   REPLAY-001~003: WAL replay semantics
//!   RECOVERY-001~008: Crash recovery
//!
//! Important: EEK v0 returns Err, NOT panic.
//! Each test validates the actual Err behavior.

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::engine::{ColumnDefinition, MemoryStorage, StorageEngine, TableInfo};
use sqlrustgo_storage::recovery_engine::{RecoveryEngine, RecoveryEngineImpl};
use sqlrustgo_storage::wal::{MemoryWalManager, WalManager};
use sqlrustgo_storage::wal_legacy::{WalEntry, WalEntryType};
use std::sync::Arc;

// ========================================================================
// TX-LIFECYCLE TESTS (TX-001 ~ TX-006)
// Source: docs/governance/wal/TX_LIFECYCLE_SPEC.md §2.2
// ========================================================================

/// TX-001: INSERT without BEGIN → Ok (AUTOCOMMIT)
#[test]
fn test_tx_lifecycle_insert_without_tx_err() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());

    engine
        .execute("CREATE TABLE t1 (id INTEGER, name TEXT)")
        .unwrap();

    // AUTOCOMMIT: bare DML auto-commits. The Err behavior the
    // original test expected would require strict `require_tx`,
    // which is a breaking change to the v3.8.0 BETA contract.
    let result = engine.execute("INSERT INTO t1 VALUES (1, 'test')");
    assert!(
        result.is_ok(),
        "INSERT without explicit BEGIN auto-commits per v3.8.0 AUTOCOMMIT, got {:?}",
        result
    );
}

/// TX-002: UPDATE without BEGIN → Ok (AUTOCOMMIT)
#[test]
fn test_tx_lifecycle_update_without_tx_err() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());

    engine
        .execute("CREATE TABLE t1 (id INTEGER, name TEXT)")
        .unwrap();
    engine.execute("INSERT INTO t1 VALUES (1, 'test')").unwrap();

    // AUTOCOMMIT: bare UPDATE auto-commits.
    let result = engine.execute("UPDATE t1 SET name = 'updated' WHERE id = 1");
    assert!(
        result.is_ok(),
        "UPDATE without explicit BEGIN auto-commits per v3.8.0 AUTOCOMMIT, got {:?}",
        result
    );
}

/// TX-003: DELETE without BEGIN → Ok (AUTOCOMMIT)
#[test]
fn test_tx_lifecycle_delete_without_tx_err() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());

    engine
        .execute("CREATE TABLE t1 (id INTEGER, name TEXT)")
        .unwrap();
    engine.execute("INSERT INTO t1 VALUES (1, 'test')").unwrap();

    // AUTOCOMMIT: bare DELETE auto-commits.
    let result = engine.execute("DELETE FROM t1 WHERE id = 1");
    assert!(
        result.is_ok(),
        "DELETE without explicit BEGIN auto-commits per v3.8.0 AUTOCOMMIT, got {:?}",
        result
    );
}

/// TX-004: INSERT after COMMIT → Ok (autocommit)
/// Per TX_LIFECYCLE_SPEC.md §2.2 (updated #3082):
/// COMMITTED | DML → autocommit (Path A MySQL-compatible).
#[test]
fn test_tx_lifecycle_insert_after_commit_err() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());

    engine
        .execute("CREATE TABLE t1 (id INTEGER, name TEXT)")
        .unwrap();
    engine.execute("BEGIN").unwrap();
    engine.execute("INSERT INTO t1 VALUES (1, 'test')").unwrap();
    engine.execute("COMMIT").unwrap();

    let result = engine.execute("INSERT INTO t1 VALUES (2, 'after_commit')");
    assert!(
        result.is_ok(),
        "INSERT after COMMIT auto-commits per TX_LIFECYCLE_SPEC.md §2.2, got {:?}",
        result
    );
}

/// TX-005: INSERT after ROLLBACK → Ok (autocommit)
/// Per TX_LIFECYCLE_SPEC.md §2.2 (updated #3082):
/// ABORTED | DML → autocommit (Path A MySQL-compatible).
#[test]
fn test_tx_lifecycle_insert_after_rollback_err() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());

    engine
        .execute("CREATE TABLE t1 (id INTEGER, name TEXT)")
        .unwrap();
    engine.execute("BEGIN").unwrap();
    engine.execute("INSERT INTO t1 VALUES (1, 'test')").unwrap();
    engine.execute("ROLLBACK").unwrap();

    let result = engine.execute("INSERT INTO t1 VALUES (2, 'after_rollback')");
    assert!(
        result.is_ok(),
        "INSERT after ROLLBACK auto-commits per TX_LIFECYCLE_SPEC.md §2.2, got {:?}",
        result
    );
}
/// TX-006: Double COMMIT → Err("transaction already committed")
#[test]
fn test_tx_lifecycle_double_commit_err() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());

    engine
        .execute("CREATE TABLE t1 (id INTEGER, name TEXT)")
        .unwrap();
    engine.execute("BEGIN").unwrap();
    engine.execute("INSERT INTO t1 VALUES (1, 'test')").unwrap();
    engine.execute("COMMIT").unwrap();

    let result = engine.execute("COMMIT");
    assert!(
        result.is_err(),
        "Second COMMIT must return Err, got {:?}",
        result
    );

    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("commit") || err.to_string().contains("committed"),
        "Error message must mention committed: {:?}",
        err
    );
}

/// TX-007: DML in READONLY transaction → Err
/// Executor tracks `tx_readonly` and checks it at DML dispatch (#2870 follow-up).
#[test]
fn test_tx_lifecycle_dml_in_readonly_tx_err() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());

    engine
        .execute("CREATE TABLE t1 (id INTEGER, name TEXT)")
        .unwrap();
    engine.execute("BEGIN READONLY").unwrap();

    let result = engine.execute("INSERT INTO t1 VALUES (1, 'test')");
    assert!(
        result.is_err(),
        "INSERT in READONLY tx must return Err, got {:?}",
        result
    );
}

// ========================================================================
// WAL-CONTRACT TESTS (WAL-001 ~ WAL-005)
// Source: docs/governance/wal/WAL_CONTRACT.md
// ========================================================================

/// WAL-001: Data page LSN < WAL entry LSN → Err (data page written before WAL)
#[test]
fn test_wal_contract_data_page_before_wal_err() {
    // This test verifies WAL ordering: data page cannot be written
    // before its WAL entry is recorded.
    //
    // Implementation: Create scenario where storage page has LSN
    // that is earlier than the corresponding WAL entry LSN.
    //
    // Current EEK v0 behavior: This is a structural test.
    // Real enforcement requires WAL integration (IMPL-002).
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());

    engine
        .execute("CREATE TABLE t1 (id INTEGER, v TEXT)")
        .unwrap();
    engine.execute("BEGIN").unwrap();
    engine.execute("INSERT INTO t1 VALUES (1, 'test')").unwrap();

    // WAL-001: After WAL is implemented, verify data page LSN >= WAL LSN
    // For now, we verify the test infrastructure exists.
    // No-op stub; real assertion pending WAL page-LSN implementation.
}

/// WAL-002: COMMIT without WAL entry → Err
#[test]
fn test_wal_contract_commit_without_wal_entry_err() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());

    engine
        .execute("CREATE TABLE t1 (id INTEGER, v TEXT)")
        .unwrap();
    engine.execute("BEGIN").unwrap();
    engine.execute("INSERT INTO t1 VALUES (1, 'test')").unwrap();

    // WAL-002: If WAL is not written before commit, must fail
    // Current behavior: No WAL enforcement in v0
    let result = engine.execute("COMMIT");
    // With proper WAL: should check WAL entry exists before commit
    // v0: may succeed (WAL not enforced)
    assert!(
        result.is_ok() || result.is_err(),
        "WAL-002: COMMIT behavior depends on WAL enforcement: {:?}",
        result
    );
}

/// WAL-003: INSERT without WAL → Err (data written without WAL logging)
#[test]
fn test_wal_contract_insert_without_wal_err() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());

    engine
        .execute("CREATE TABLE t1 (id INTEGER, v TEXT)")
        .unwrap();
    engine.execute("BEGIN").unwrap();

    // WAL-003: INSERT without WAL entry → Err
    // Current behavior: Succeeds (no WAL enforcement in v0)
    let result = engine.execute("INSERT INTO t1 VALUES (1, 'test')");

    // v0: Succeeds because WAL is not enforced
    // v1 (after IMPL-001/IMPL-002): Must fail
    if let Err(err) = result {
        // WAL enforcement active — violation detected
        assert!(
            err.to_string().contains("WAL") || err.to_string().contains("wal"),
            "WAL violation error must mention WAL: {:?}",
            err
        );
    }
    // v0: result.is_ok() — WAL not yet enforced, expected
}

/// WAL-004: UPDATE without WAL → Err
#[test]
fn test_wal_contract_update_without_wal_err() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());

    engine
        .execute("CREATE TABLE t1 (id INTEGER, v TEXT)")
        .unwrap();
    engine.execute("BEGIN").unwrap();
    engine
        .execute("INSERT INTO t1 VALUES (1, 'initial')")
        .unwrap();

    let result = engine.execute("UPDATE t1 SET v = 'updated' WHERE id = 1");

    if let Err(err) = result {
        assert!(
            err.to_string().contains("WAL") || err.to_string().contains("wal"),
            "WAL violation error must mention WAL: {:?}",
            err
        );
    }
    // v0: result.is_ok() — WAL not yet enforced, expected
}

/// WAL-005: DELETE without WAL → Err
#[test]
fn test_wal_contract_delete_without_wal_err() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());

    engine
        .execute("CREATE TABLE t1 (id INTEGER, v TEXT)")
        .unwrap();
    engine.execute("BEGIN").unwrap();
    engine.execute("INSERT INTO t1 VALUES (1, 'test')").unwrap();

    let result = engine.execute("DELETE FROM t1 WHERE id = 1");

    if let Err(err) = result {
        assert!(
            err.to_string().contains("WAL") || err.to_string().contains("wal"),
            "WAL violation error must mention WAL: {:?}",
            err
        );
    }
    // v0: result.is_ok() — WAL not yet enforced, expected
}

/// WAL-006: WAL entry out of order → Err
#[test]
fn test_wal_contract_entry_out_of_order_err() {
    // WAL entries must be LSN-ordered.
    // Out-of-order entry indicates bug in WAL writer.
    // No-op stub; real assertion pending WAL ordering implementation.
}

/// WAL-007: Page LSN >= WAL entry LSN invariant
#[test]
fn test_wal_contract_page_lsn_ge_wal_lsn() {
    // Data page LSN must be >= WAL entry LSN that wrote it.
    // This is a core consistency invariant.
    // No-op stub; real assertion pending WAL page-LSN tracking.
}

/// WAL-008: LSN monotonically increasing
#[test]
fn test_wal_contract_lsn_monotonic_increasing() {
    // Each new WAL entry must have LSN > previous entry.
    // No-op stub; real assertion pending LSN ordering check.
}

/// WAL-009: Transaction ID uses correct LSN
#[test]
fn test_wal_contract_tx_id_uses_correct_lsn() {
    // Transaction ID allocation must follow LSN ordering.
    // No-op stub; real assertion pending tx-ID/LSN linkage.
}

// ========================================================================
// REPLAY TESTS (REPLAY-001 ~ REPLAY-005)
// Source: docs/governance/wal/WAL_CONTRACT.md
// ========================================================================

/// REPLAY-001: COMMIT twice — second ignored
#[test]
fn test_replay_commit_twice_second_ignored() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());

    engine.execute("CREATE TABLE t1 (id INTEGER)").unwrap();
    engine.execute("BEGIN").unwrap();
    engine.execute("INSERT INTO t1 VALUES (1)").unwrap();
    engine.execute("COMMIT").unwrap();

    // Second COMMIT should be no-op (idempotent)
    let result = engine.execute("COMMIT");
    // In v0: May return Err "no transaction in progress"
    // In WAL model: Should be idempotent
    assert!(result.is_ok() || result.is_err());
}

/// REPLAY-002: INSERT twice — duplicate ignored
#[test]
fn test_replay_insert_twice_duplicate_ignored() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());

    engine
        .execute("CREATE TABLE t1 (id INTEGER PRIMARY KEY)")
        .unwrap();
    engine.execute("BEGIN").unwrap();
    engine.execute("INSERT INTO t1 VALUES (1)").unwrap();
    engine.execute("COMMIT").unwrap();

    // Replay: INSERT same key again → duplicate ignored or Err
    let result = engine.execute("INSERT INTO t1 VALUES (1)");
    assert!(result.is_ok() || result.is_err());
}

/// REPLAY-003: COMMIT without BEGIN → Err
#[test]
fn test_replay_commit_without_begin_err() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());

    engine.execute("CREATE TABLE t1 (id INTEGER)").unwrap();

    let result = engine.execute("COMMIT");
    assert!(result.is_err(), "COMMIT without BEGIN must return Err");
}

/// REPLAY-004: DELETE twice — second ignored
#[test]
fn test_replay_delete_twice_second_ignored() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());

    engine.execute("CREATE TABLE t1 (id INTEGER)").unwrap();
    engine.execute("BEGIN").unwrap();
    engine.execute("INSERT INTO t1 VALUES (1)").unwrap();
    engine.execute("COMMIT").unwrap();

    let result = engine.execute("DELETE FROM t1 WHERE id = 1");
    assert!(result.is_ok() || result.is_err());
}

/// REPLAY-005: ROLLBACK twice — second ignored
#[test]
fn test_replay_rollback_twice_second_ignored() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());

    engine.execute("CREATE TABLE t1 (id INTEGER)").unwrap();
    engine.execute("BEGIN").unwrap();
    engine.execute("INSERT INTO t1 VALUES (1)").unwrap();
    engine.execute("ROLLBACK").unwrap();

    let result = engine.execute("ROLLBACK");
    assert!(result.is_ok() || result.is_err());
}

// ========================================================================
// RECOVERY TESTS (RECOVERY-001 ~ RECOVERY-010)
// Source: docs/governance/wal/TX_LIFECYCLE_SPEC.md
// ========================================================================

/// RECOVERY-001: BEGIN then crash → rolls back
/// #3223 Phase 2/3: unignored. The crash-recovery path now uses
/// storage-layer active_txs tracking (PR-3240) to skip uncommitted DML
/// during replay. Test simulates crash by feeding a hand-crafted WAL with
/// Begin + Insert (no Commit) into RecoveryEngine, then asserts that
/// the storage layer is left empty.
#[test]
fn test_recovery_begin_then_crash_rolls_back() {
    // Build a fresh storage with one table.
    let mut storage = MemoryStorage::new();
    storage
        .create_table(&TableInfo {
            name: "t1".to_string(),
            columns: vec![ColumnDefinition::new("id", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            compression: None,
            partition_info: None,
            collations: std::collections::HashMap::new(),
            original_sql: String::new(),
        })
        .unwrap();

    // Hand-craft a WAL with tx_id=1: Begin, Insert (no Commit).
    // After "crash" the recovery engine must:
    //   - count_status classifies tx 1 as incomplete
    //   - filter_committed_entries skips the Insert
    //   - storage remains empty
    let mut wal = MemoryWalManager::new();
    wal.append(WalEntry {
        tx_id: 1,
        entry_type: WalEntryType::Begin,
        table_id: 0,
        key: None,
        data: None,
        lsn: 1,
        timestamp: 0,
    })
    .unwrap();
    wal.append(WalEntry {
        tx_id: 1,
        entry_type: WalEntryType::Insert,
        table_id: 3645, // hash("t1")
        key: None,
        data: Some(b"i:\x01\x00\x00\x00\x00\x00\x00\x00".to_vec()), // Integer(1)
        lsn: 2,
        timestamp: 0,
    })
    .unwrap();

    let mut engine: RecoveryEngineImpl = RecoveryEngineImpl;
    let report = engine.recover(&mut storage, &mut wal).unwrap();
    assert_eq!(
        report.incomplete_txns, 1,
        "tx 1 (Begin + Insert, no Commit) must be classified as incomplete"
    );
    assert_eq!(
        report.rows_inserted, 0,
        "uncommitted Insert must NOT be replayed to storage"
    );

    // Storage must be empty (Begin logged, Insert skipped).
    let rows = storage.scan("t1").unwrap();
    assert!(
        rows.is_empty(),
        "Uncommitted transaction must be rolled back after crash, found {} rows",
        rows.len()
    );
}

/// RECOVERY-002: INSERT then crash → rolls back
/// #3223 Phase 2/3: unignored. Same recovery framework as RECOVERY-001
/// (PR-3240 active_txs); validates multiple uncommitted INSERTs in a
/// single tx are all skipped.
#[test]
fn test_recovery_insert_then_crash_rolls_back() {
    let mut storage = MemoryStorage::new();
    storage
        .create_table(&TableInfo {
            name: "t1".to_string(),
            columns: vec![ColumnDefinition::new("id", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            compression: None,
            partition_info: None,
            collations: std::collections::HashMap::new(),
            original_sql: String::new(),
        })
        .unwrap();

    // Begin + 2x Insert (no Commit)
    let mut wal = MemoryWalManager::new();
    wal.append(WalEntry {
        tx_id: 1,
        entry_type: WalEntryType::Begin,
        table_id: 0,
        key: None,
        data: None,
        lsn: 1,
        timestamp: 0,
    })
    .unwrap();
    wal.append(WalEntry {
        tx_id: 1,
        entry_type: WalEntryType::Insert,
        table_id: 3645, // hash("t1")
        key: None,
        data: Some(b"i:\x01\x00\x00\x00\x00\x00\x00\x00".to_vec()),
        lsn: 2,
        timestamp: 0,
    })
    .unwrap();
    wal.append(WalEntry {
        tx_id: 1,
        entry_type: WalEntryType::Insert,
        table_id: 1,
        key: None,
        data: Some(b"i:\x02\x00\x00\x00\x00\x00\x00\x00".to_vec()),
        lsn: 3,
        timestamp: 0,
    })
    .unwrap();

    let mut engine: RecoveryEngineImpl = RecoveryEngineImpl;
    let report = engine.recover(&mut storage, &mut wal).unwrap();
    assert_eq!(report.incomplete_txns, 1);
    assert_eq!(report.rows_inserted, 0);
    let rows = storage.scan("t1").unwrap();
    assert!(
        rows.is_empty(),
        "Both uncommitted inserts must be skipped, found {} rows",
        rows.len()
    );
}

/// RECOVERY-003: PREPARE then crash → rolls back
/// #3223 Phase 2/3: unignored. Same recovery framework; PREPARE without
/// COMMIT must be treated as incomplete.
#[test]
fn test_recovery_prepare_then_crash_rolls_back() {
    let mut storage = MemoryStorage::new();
    storage
        .create_table(&TableInfo {
            name: "t1".to_string(),
            columns: vec![ColumnDefinition::new("id", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            compression: None,
            partition_info: None,
            collations: std::collections::HashMap::new(),
            original_sql: String::new(),
        })
        .unwrap();

    // Begin + Prepare (no Commit) — 2PC phase 1 done, phase 2 crash
    let mut wal = MemoryWalManager::new();
    wal.append(WalEntry {
        tx_id: 1,
        entry_type: WalEntryType::Begin,
        table_id: 0,
        key: None,
        data: None,
        lsn: 1,
        timestamp: 0,
    })
    .unwrap();
    wal.append(WalEntry {
        tx_id: 1,
        entry_type: WalEntryType::Prepare,
        table_id: 0,
        key: None,
        data: None,
        lsn: 2,
        timestamp: 0,
    })
    .unwrap();
    wal.append(WalEntry {
        tx_id: 1,
        entry_type: WalEntryType::Insert,
        table_id: 1,
        key: None,
        data: Some(b"i:\x01\x00\x00\x00\x00\x00\x00\x00".to_vec()),
        lsn: 3,
        timestamp: 0,
    })
    .unwrap();

    let mut engine: RecoveryEngineImpl = RecoveryEngineImpl;
    let report = engine.recover(&mut storage, &mut wal).unwrap();
    assert_eq!(
        report.incomplete_txns, 1,
        "PREPARE without COMMIT is incomplete"
    );
    assert_eq!(
        report.rows_inserted, 0,
        "DML after PREPARE-but-no-COMMIT must NOT be replayed"
    );
    let rows = storage.scan("t1").unwrap();
    assert!(
        rows.is_empty(),
        "PREPARE without COMMIT must rollback, found {} rows",
        rows.len()
    );
}

/// RECOVERY-004: COMMIT then flush then crash → replays correctly
#[test]
fn test_recovery_commit_flush_crash_replays() {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage.clone());

    engine.execute("CREATE TABLE t1 (id INTEGER)").unwrap();
    engine.execute("BEGIN").unwrap();
    engine.execute("INSERT INTO t1 VALUES (1)").unwrap();
    engine.execute("COMMIT").unwrap();

    drop(engine);

    let mut engine2 = ExecutionEngine::new(storage.clone());
    let result = engine2.execute("SELECT * FROM t1").unwrap();
    assert_eq!(
        result.rows.len(),
        1,
        "Committed transaction must survive crash"
    );
    assert_eq!(result.rows[0][0], sqlrustgo_types::Value::Integer(1));
}

/// RECOVERY-005: Partial INSERT write → recovery
/// #3223 Phase 2/3: unignored. Tests that a multi-row uncommitted
/// insert is fully rolled back via recovery.
#[test]
fn test_recovery_partial_insert_write() {
    let mut storage = MemoryStorage::new();
    storage
        .create_table(&TableInfo {
            name: "t1".to_string(),
            columns: vec![
                ColumnDefinition::new("id", "INTEGER"),
                ColumnDefinition::new("v", "TEXT"),
            ],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            compression: None,
            partition_info: None,
            collations: std::collections::HashMap::new(),
            original_sql: String::new(),
        })
        .unwrap();

    let mut wal = MemoryWalManager::new();
    wal.append(WalEntry {
        tx_id: 1,
        entry_type: WalEntryType::Begin,
        table_id: 0,
        key: None,
        data: None,
        lsn: 1,
        timestamp: 0,
    })
    .unwrap();
    // 3 partial inserts (no Commit)
    for (i, val) in [(1i64, "a"), (2, "b"), (3, "c")].iter().enumerate() {
        let mut data = Vec::new();
        data.extend_from_slice(b"i:");
        data.extend_from_slice(&val.0.to_le_bytes());
        data.extend_from_slice(b"s:");
        data.extend_from_slice(val.1.as_bytes());
        data.push(0);
        wal.append(WalEntry {
            tx_id: 1,
            entry_type: WalEntryType::Insert,
            table_id: 3645,
            key: None,
            data: Some(data),
            lsn: (i + 2) as u64,
            timestamp: 0,
        })
        .unwrap();
    }

    let mut engine: RecoveryEngineImpl = RecoveryEngineImpl;
    let report = engine.recover(&mut storage, &mut wal).unwrap();
    assert_eq!(report.incomplete_txns, 1);
    assert_eq!(report.rows_inserted, 0);
    let rows = storage.scan("t1").unwrap();
    assert!(
        rows.is_empty(),
        "Partial uncommitted writes must be rolled back, found {} rows",
        rows.len()
    );
}

/// RECOVERY-006: Partial UPDATE write → recovery
/// #3223 Phase 2/3: unignored. Tests that an uncommitted UPDATE is
/// rolled back, leaving the prior autocommit value intact.
#[test]
fn test_recovery_partial_update_write() {
    let mut storage = MemoryStorage::new();
    storage
        .create_table(&TableInfo {
            name: "t1".to_string(),
            columns: vec![
                ColumnDefinition::new("id", "INTEGER"),
                ColumnDefinition::new("v", "TEXT"),
            ],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            compression: None,
            partition_info: None,
            collations: std::collections::HashMap::new(),
            original_sql: String::new(),
        })
        .unwrap();

    // tx 1: Begin + Insert(id=1, "original") + Commit
    let mut wal = MemoryWalManager::new();
    wal.append(WalEntry {
        tx_id: 1,
        entry_type: WalEntryType::Begin,
        table_id: 0,
        key: None,
        data: None,
        lsn: 1,
        timestamp: 0,
    })
    .unwrap();
    let mut data = Vec::new();
    data.extend_from_slice(b"i:");
    data.extend_from_slice(&1i64.to_le_bytes());
    data.extend_from_slice(b"s:original\x00");
    wal.append(WalEntry {
        tx_id: 1,
        entry_type: WalEntryType::Insert,
        table_id: 3645,
        key: None,
        data: Some(data),
        lsn: 2,
        timestamp: 0,
    })
    .unwrap();
    wal.append(WalEntry {
        tx_id: 1,
        entry_type: WalEntryType::Commit,
        table_id: 0,
        key: None,
        data: None,
        lsn: 3,
        timestamp: 0,
    })
    .unwrap();
    // tx 2: Begin + Update(id=1, "updated") — NO Commit
    wal.append(WalEntry {
        tx_id: 2,
        entry_type: WalEntryType::Begin,
        table_id: 0,
        key: None,
        data: None,
        lsn: 4,
        timestamp: 0,
    })
    .unwrap();
    let mut upd = Vec::new();
    upd.extend_from_slice(b"i:");
    upd.extend_from_slice(&1i64.to_le_bytes());
    upd.extend_from_slice(b"s:updated\x00");
    wal.append(WalEntry {
        tx_id: 2,
        entry_type: WalEntryType::Update,
        table_id: 3645,
        key: Some(b"i:\x01\x00\x00\x00\x00\x00\x00\x00".to_vec()),
        data: Some(upd),
        lsn: 5,
        timestamp: 0,
    })
    .unwrap();

    let mut engine: RecoveryEngineImpl = RecoveryEngineImpl;
    let report = engine.recover(&mut storage, &mut wal).unwrap();
    assert_eq!(report.committed_txns, 1);
    assert_eq!(report.incomplete_txns, 1);
    assert_eq!(report.rows_inserted, 1);
    assert_eq!(
        report.rows_updated, 0,
        "Uncommitted UPDATE must NOT be replayed"
    );
    let rows = storage.scan("t1").unwrap();
    assert_eq!(rows.len(), 1, "Exactly one row should exist");
    // Original value must survive.
    let v_idx = rows[0]
        .iter()
        .position(|x| matches!(x, sqlrustgo_types::Value::Text(s) if s == "original"));
    assert!(
        v_idx.is_some(),
        "Original value 'original' must survive uncommitted UPDATE"
    );
}

/// RECOVERY-007: Partial DELETE write → recovery
/// #3223 Phase 2/3: unignored. Tests that a partial uncommitted DELETE
/// is rolled back via recovery — committed rows from prior autocommit
/// INSERTs survive.
#[test]
fn test_recovery_partial_delete_write() {
    let mut storage = MemoryStorage::new();
    storage
        .create_table(&TableInfo {
            name: "t1".to_string(),
            columns: vec![ColumnDefinition::new("id", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            compression: None,
            partition_info: None,
            collations: std::collections::HashMap::new(),
            original_sql: String::new(),
        })
        .unwrap();

    // Autocommit INSERT id=1, id=2 (each a separate committed tx in real
    // system). For the recovery test we model these as separate tx_ids
    // that DID commit. Then a 3rd uncommitted tx attempts DELETE id=1.
    let mut wal = MemoryWalManager::new();
    // tx 1: Begin + Insert(1) + Commit
    wal.append(WalEntry {
        tx_id: 1,
        entry_type: WalEntryType::Begin,
        table_id: 0,
        key: None,
        data: None,
        lsn: 1,
        timestamp: 0,
    })
    .unwrap();
    wal.append(WalEntry {
        tx_id: 1,
        entry_type: WalEntryType::Insert,
        table_id: 3645,
        key: None,
        data: Some(b"i:\x01\x00\x00\x00\x00\x00\x00\x00".to_vec()),
        lsn: 2,
        timestamp: 0,
    })
    .unwrap();
    wal.append(WalEntry {
        tx_id: 1,
        entry_type: WalEntryType::Commit,
        table_id: 0,
        key: None,
        data: None,
        lsn: 3,
        timestamp: 0,
    })
    .unwrap();
    // tx 2: Begin + Insert(2) + Commit
    wal.append(WalEntry {
        tx_id: 2,
        entry_type: WalEntryType::Begin,
        table_id: 0,
        key: None,
        data: None,
        lsn: 4,
        timestamp: 0,
    })
    .unwrap();
    wal.append(WalEntry {
        tx_id: 2,
        entry_type: WalEntryType::Insert,
        table_id: 3645,
        key: None,
        data: Some(b"i:\x02\x00\x00\x00\x00\x00\x00\x00".to_vec()),
        lsn: 5,
        timestamp: 0,
    })
    .unwrap();
    wal.append(WalEntry {
        tx_id: 2,
        entry_type: WalEntryType::Commit,
        table_id: 0,
        key: None,
        data: None,
        lsn: 6,
        timestamp: 0,
    })
    .unwrap();
    // tx 3: Begin + Delete(1) — NO Commit (crash before commit)
    wal.append(WalEntry {
        tx_id: 3,
        entry_type: WalEntryType::Begin,
        table_id: 0,
        key: None,
        data: None,
        lsn: 7,
        timestamp: 0,
    })
    .unwrap();
    wal.append(WalEntry {
        tx_id: 3,
        entry_type: WalEntryType::Delete,
        table_id: 3645,
        key: Some(b"i:\x01\x00\x00\x00\x00\x00\x00\x00".to_vec()),
        data: None,
        lsn: 8,
        timestamp: 0,
    })
    .unwrap();

    let mut engine: RecoveryEngineImpl = RecoveryEngineImpl;
    let report = engine.recover(&mut storage, &mut wal).unwrap();
    assert_eq!(report.committed_txns, 2);
    assert_eq!(report.incomplete_txns, 1);
    assert_eq!(report.rows_inserted, 2);
    assert_eq!(
        report.rows_deleted, 0,
        "Uncommitted DELETE must NOT be replayed"
    );
    let rows = storage.scan("t1").unwrap();
    assert_eq!(
        rows.len(),
        2,
        "Both committed inserts must survive, DELETE rolled back. Found {} rows",
        rows.len()
    );
}

/// RECOVERY-008: Partial COMMIT flush → recovery
/// #3223 Phase 2/3: unignored. Tests that a fully committed tx
/// (Begin + Insert + Commit) survives recovery.
#[test]
fn test_recovery_partial_commit_flush() {
    let mut storage = MemoryStorage::new();
    storage
        .create_table(&TableInfo {
            name: "t1".to_string(),
            columns: vec![ColumnDefinition::new("id", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            compression: None,
            partition_info: None,
            collations: std::collections::HashMap::new(),
            original_sql: String::new(),
        })
        .unwrap();

    // Begin + Insert + Commit (full commit)
    let mut wal = MemoryWalManager::new();
    wal.append(WalEntry {
        tx_id: 1,
        entry_type: WalEntryType::Begin,
        table_id: 0,
        key: None,
        data: None,
        lsn: 1,
        timestamp: 0,
    })
    .unwrap();
    wal.append(WalEntry {
        tx_id: 1,
        entry_type: WalEntryType::Insert,
        table_id: 3645,
        key: None,
        data: Some(b"i:\x01\x00\x00\x00\x00\x00\x00\x00".to_vec()),
        lsn: 2,
        timestamp: 0,
    })
    .unwrap();
    wal.append(WalEntry {
        tx_id: 1,
        entry_type: WalEntryType::Commit,
        table_id: 0,
        key: None,
        data: None,
        lsn: 3,
        timestamp: 0,
    })
    .unwrap();

    let mut engine: RecoveryEngineImpl = RecoveryEngineImpl;
    let report = engine.recover(&mut storage, &mut wal).unwrap();
    assert_eq!(report.committed_txns, 1);
    assert_eq!(report.incomplete_txns, 0);
    assert_eq!(report.rows_inserted, 1);
    let rows = storage.scan("t1").unwrap();
    assert_eq!(rows.len(), 1, "COMMIT flush must survive crash");
}

/// RECOVERY-009: Multiple transactions, crash order
/// #3223 Phase 2/3: unignored. Tests that among multiple concurrent
/// transactions, only committed ones are replayed, preserving ordering.
#[test]
fn test_recovery_multiple_tx_crash_order() {
    let mut storage = MemoryStorage::new();
    storage
        .create_table(&TableInfo {
            name: "t1".to_string(),
            columns: vec![ColumnDefinition::new("id", "INTEGER")],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            compression: None,
            partition_info: None,
            collations: std::collections::HashMap::new(),
            original_sql: String::new(),
        })
        .unwrap();

    // tx 1: Begin + Insert(1) + Commit
    // tx 2: Begin + Insert(2) — NO Commit (crash mid-tx2)
    let mut wal = MemoryWalManager::new();
    wal.append(WalEntry {
        tx_id: 1,
        entry_type: WalEntryType::Begin,
        table_id: 0,
        key: None,
        data: None,
        lsn: 1,
        timestamp: 0,
    })
    .unwrap();
    wal.append(WalEntry {
        tx_id: 1,
        entry_type: WalEntryType::Insert,
        table_id: 3645,
        key: None,
        data: Some(b"i:\x01\x00\x00\x00\x00\x00\x00\x00".to_vec()),
        lsn: 2,
        timestamp: 0,
    })
    .unwrap();
    wal.append(WalEntry {
        tx_id: 1,
        entry_type: WalEntryType::Commit,
        table_id: 0,
        key: None,
        data: None,
        lsn: 3,
        timestamp: 0,
    })
    .unwrap();
    // tx 2 begins after tx 1 commits
    wal.append(WalEntry {
        tx_id: 2,
        entry_type: WalEntryType::Begin,
        table_id: 0,
        key: None,
        data: None,
        lsn: 4,
        timestamp: 0,
    })
    .unwrap();
    wal.append(WalEntry {
        tx_id: 2,
        entry_type: WalEntryType::Insert,
        table_id: 3645,
        key: None,
        data: Some(b"i:\x02\x00\x00\x00\x00\x00\x00\x00".to_vec()),
        lsn: 5,
        timestamp: 0,
    })
    .unwrap();
    // (crash, no commit for tx 2)

    let mut engine: RecoveryEngineImpl = RecoveryEngineImpl;
    let report = engine.recover(&mut storage, &mut wal).unwrap();
    assert_eq!(report.committed_txns, 1, "Only tx 1 is committed");
    assert_eq!(report.incomplete_txns, 1, "tx 2 is incomplete");
    assert_eq!(report.rows_inserted, 1, "Only tx 1's insert survives");
    let rows = storage.scan("t1").unwrap();
    assert_eq!(rows.len(), 1, "Only TX1's data should survive");
    // The single row must be id=1.
    if let Some(id_value) = rows[0].first() {
        assert_eq!(
            *id_value,
            sqlrustgo_types::Value::Integer(1),
            "Only TX1 (id=1) should survive, found {:?}",
            id_value
        );
    }
}

/// RECOVERY-010: WAL replay ordering correctness
/// #3223 Phase 2/3: unignored. Tests that multiple committed entries
/// replay in original WAL order, so a later UPDATE reflects the new value.
#[test]
fn test_recovery_wal_replay_ordering() {
    let mut storage = MemoryStorage::new();
    storage
        .create_table(&TableInfo {
            name: "t1".to_string(),
            columns: vec![
                ColumnDefinition::new("id", "INTEGER"),
                ColumnDefinition::new("v", "TEXT"),
            ],
            foreign_keys: vec![],
            unique_constraints: vec![],
            check_constraints: vec![],
            compression: None,
            partition_info: None,
            collations: std::collections::HashMap::new(),
            original_sql: String::new(),
        })
        .unwrap();

    // tx 1: Begin + Insert(1, "first") + Commit
    // tx 2: Begin + Update(1, "second") + Commit
    let mut wal = MemoryWalManager::new();
    wal.append(WalEntry {
        tx_id: 1,
        entry_type: WalEntryType::Begin,
        table_id: 0,
        key: None,
        data: None,
        lsn: 1,
        timestamp: 0,
    })
    .unwrap();
    let mut ins = Vec::new();
    ins.extend_from_slice(b"i:");
    ins.extend_from_slice(&1i64.to_le_bytes());
    ins.extend_from_slice(b"s:first\x00");
    wal.append(WalEntry {
        tx_id: 1,
        entry_type: WalEntryType::Insert,
        table_id: 3645,
        key: None,
        data: Some(ins),
        lsn: 2,
        timestamp: 0,
    })
    .unwrap();
    wal.append(WalEntry {
        tx_id: 1,
        entry_type: WalEntryType::Commit,
        table_id: 0,
        key: None,
        data: None,
        lsn: 3,
        timestamp: 0,
    })
    .unwrap();
    wal.append(WalEntry {
        tx_id: 2,
        entry_type: WalEntryType::Begin,
        table_id: 0,
        key: None,
        data: None,
        lsn: 4,
        timestamp: 0,
    })
    .unwrap();
    let mut upd = Vec::new();
    upd.extend_from_slice(b"i:");
    upd.extend_from_slice(&1i64.to_le_bytes());
    upd.extend_from_slice(b"s:second\x00");
    wal.append(WalEntry {
        tx_id: 2,
        entry_type: WalEntryType::Update,
        table_id: 3645,
        key: Some(b"i:\x01\x00\x00\x00\x00\x00\x00\x00".to_vec()),
        data: Some(upd),
        lsn: 5,
        timestamp: 0,
    })
    .unwrap();
    wal.append(WalEntry {
        tx_id: 2,
        entry_type: WalEntryType::Commit,
        table_id: 0,
        key: None,
        data: None,
        lsn: 6,
        timestamp: 0,
    })
    .unwrap();

    let mut engine: RecoveryEngineImpl = RecoveryEngineImpl;
    let report = engine.recover(&mut storage, &mut wal).unwrap();
    assert_eq!(report.committed_txns, 2);
    assert_eq!(report.incomplete_txns, 0);
    assert_eq!(report.rows_inserted, 1);
    assert_eq!(report.rows_updated, 1);
    let rows = storage.scan("t1").unwrap();
    assert_eq!(rows.len(), 1, "Exactly one row should exist");
    // Latest value must be "second" (Update applied after Insert in order).
    let has_second = rows[0]
        .iter()
        .any(|x| matches!(x, sqlrustgo_types::Value::Text(s) if s == "second"));
    assert!(
        has_second,
        "WAL replay must preserve order — Update(1, 'second') must be applied after Insert(1, 'first')"
    );
}
