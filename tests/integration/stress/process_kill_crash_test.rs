//! T-20 Process kill -9 mid-transaction crash recovery tests.
//!
//! These tests verify that the WAL recovery engine correctly handles
//! process-level crashes (simulated by dropping WalStorage without
//! clean shutdown) in the exact same recovery path used by the server.
//!
//! # Design
//!
//! Instead of spawning a real subprocess and sending SIGKILL (which
//! requires a compiled binary + free port + MySQL client), we simulate
//! the crash by dropping the WalStorage scope — the WAL file persists
//! on disk with all entries written up to the last fsync, exactly as
//! after a real kill -9. Recovery then runs through the same
//! `StatefulRecoveryEngine` path that the server uses at startup.
//!
//! # Usage
//!
//! ```bash
//! cargo test --test process_kill_crash_test -- --test-threads=1 --nocapture
//! ```
//!
//! # Reference
//!
//! - Issue #3769 (V310-15 T-20)
//! - src/engine_builder.rs:238-262 (recover_wal wiring)

use std::path::Path;

use sqlrustgo_storage::engine::{ColumnDefinition, StorageEngine, TableInfo, Value};
use sqlrustgo_storage::recovery_engine::{RecoveryEngine, RecoveryReport, StatefulRecoveryEngine};
use sqlrustgo_storage::wal::FileBackedWalManager;
use sqlrustgo_storage::WalStorage;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a minimal table schema.
fn make_table_info(name: &str, columns: usize) -> TableInfo {
    let cols: Vec<ColumnDefinition> = (0..columns)
        .map(|i| {
            let mut col = ColumnDefinition::new(&format!("c{}", i), "INTEGER");
            if i == 0 {
                col.primary_key = true;
            }
            col
        })
        .collect();
    TableInfo {
        name: name.to_string(),
        columns: cols,
        foreign_keys: vec![],
        unique_constraints: vec![],
        check_constraints: vec![],
        compression: None,
        collations: std::collections::HashMap::new(),
        partition_info: None,
        original_sql: String::new(),
    }
}

/// Build a test table with some initial data committed.
fn seed_table(
    storage: &mut WalStorage<sqlrustgo_storage::file_storage::FileStorage, FileBackedWalManager>,
    table: &str,
    rows: &[i64],
) {
    storage.create_table(&make_table_info(table, 1)).unwrap();
    for &v in rows {
        storage
            .insert(table, vec![vec![Value::Integer(v)]])
            .unwrap();
    }
}

/// Run recovery on a data directory, returning the report and the recovered storage.
fn run_recovery(data_dir: &Path) -> (RecoveryReport, sqlrustgo_storage::file_storage::FileStorage) {
    // Same sequence as src/engine_builder.rs recover_wal()
    let mut file_storage = sqlrustgo_storage::file_storage::FileStorage::new(data_dir.into())
        .expect("FileStorage::new must succeed for recovery");

    let wal_path = data_dir.join("sqlrustgo.wal");
    let mut wal_mgr = FileBackedWalManager::new(wal_path).expect("WAL manager for recovery");

    let mut recovery = StatefulRecoveryEngine::new();
    let report = RecoveryEngine::recover(&mut recovery, &mut file_storage, &mut wal_mgr)
        .expect("recovery must succeed");

    (report, file_storage)
}

/// Create a full WalStorage<FileStorage, FileBackedWalManager> stack.
fn create_wal_storage(
    data_dir: &Path,
) -> WalStorage<sqlrustgo_storage::file_storage::FileStorage, FileBackedWalManager> {
    let wal_path = data_dir.join("sqlrustgo.wal");
    let inner = sqlrustgo_storage::file_storage::FileStorage::new(data_dir.into())
        .expect("FileStorage::new");
    let wal = FileBackedWalManager::new(wal_path).expect("FileBackedWalManager::new");
    WalStorage::new(inner, wal).expect("WalStorage::new")
}

// ---------------------------------------------------------------------------
// T-20 Test Scenarios
// ---------------------------------------------------------------------------

/// Test: BEGIN → INSERT → UPDATE → (crash) → recovery → verify.
///
/// An uncommitted multi-statement transaction should be fully rolled back
/// after crash recovery — the WAL has Begin + Insert + Update entries but
/// no Commit, so the RecoveryEngine must discard them.
#[test]
fn test_kill_mid_insert_update_uncommitted() {
    let dir = TempDir::new().unwrap();
    let data_dir = dir.path().join("data");
    std::fs::create_dir_all(&data_dir).unwrap();

    // Phase 1: Open storage, create table & seed, then begin a multi-stmt tx.
    {
        let mut storage = create_wal_storage(&data_dir);
        seed_table(&mut storage, "t1", &[100]); // committed row

        // Uncommitted multi-statement transaction
        storage.begin_transaction().unwrap();
        storage
            .insert("t1", vec![vec![Value::Integer(200)]])
            .unwrap();
        storage
            .update("t1", &[Value::Integer(200)], &[(0, Value::Integer(201))])
            .unwrap();
        // No commit — simulate kill -9
    }
    // storage dropped here — WAL file persists with Begin + Insert + Update but NO Commit

    // Phase 2: Recovery
    let (report, recovered) = run_recovery(&data_dir);

    // Verify recovery stats
    assert_eq!(
        report.incomplete_txns, 1,
        "Should detect 1 incomplete (uncommitted) transaction"
    );
    assert_eq!(
        report.rows_inserted, 0,
        "Should replay 0 rows from uncommitted tx"
    );

    // Verify data: only the committed row (100) should exist
    let rows = recovered.scan("t1").unwrap();
    let vals: Vec<i64> = rows
        .iter()
        .map(|r| match &r[0] {
            Value::Integer(v) => *v,
            _ => panic!("expected Integer"),
        })
        .collect();
    assert_eq!(
        vals,
        vec![100],
        "Uncommitted insert+update must be rolled back"
    );
}

/// Test: BEGIN → DELETE → (crash) → recovery → verify.
///
/// An uncommitted DELETE should be rolled back — the row remains.
#[test]
fn test_kill_mid_delete_uncommitted() {
    let dir = TempDir::new().unwrap();
    let data_dir = dir.path().join("data");
    std::fs::create_dir_all(&data_dir).unwrap();

    // Phase 1: Seed with data, then begin a DELETE without committing.
    {
        let mut storage = create_wal_storage(&data_dir);
        seed_table(&mut storage, "t2", &[10, 20, 30]);

        storage.begin_transaction().unwrap();
        // Use RowFilter matching to delete
        storage.delete("t2", &[Value::Integer(20)]).unwrap();
        // No commit — simulate kill -9
    }

    // Phase 2: Recovery
    let (report, recovered) = run_recovery(&data_dir);

    assert_eq!(
        report.incomplete_txns, 1,
        "Should detect 1 incomplete transaction"
    );
    assert_eq!(
        report.rows_deleted, 0,
        "Should replay 0 deletes from uncommitted tx"
    );

    // All rows should still exist
    let rows = recovered.scan("t2").unwrap();
    let vals: Vec<i64> = rows
        .iter()
        .map(|r| match &r[0] {
            Value::Integer(v) => *v,
            _ => panic!("expected Integer"),
        })
        .collect();
    assert_eq!(
        vals,
        vec![10, 20, 30],
        "Uncommitted DELETE must be rolled back"
    );
}

/// Test: Committed transaction survives crash.
///
/// BEGIN → INSERT → COMMIT → (crash) → recovery → data present.
#[test]
fn test_committed_survives_crash() {
    let dir = TempDir::new().unwrap();
    let data_dir = dir.path().join("data");
    std::fs::create_dir_all(&data_dir).unwrap();

    // Phase 1: Commit a transaction, then crash.
    {
        let mut storage = create_wal_storage(&data_dir);
        seed_table(&mut storage, "t3", &[1]); // autocommit

        storage.begin_transaction().unwrap();
        storage.insert("t3", vec![vec![Value::Integer(2)]]).unwrap();
        storage.commit_transaction().unwrap();
        // Now crash
    }

    // Phase 2: Recovery
    let (report, recovered) = run_recovery(&data_dir);

    assert_eq!(
        report.committed_txns, 1,
        "Should detect 1 committed transaction"
    );

    // Both rows should exist
    let rows = recovered.scan("t3").unwrap();
    let vals: Vec<i64> = rows
        .iter()
        .map(|r| match &r[0] {
            Value::Integer(v) => *v,
            _ => panic!("expected Integer"),
        })
        .collect();
    assert!(vals.contains(&1), "Committed row id=1 must exist");
    assert!(vals.contains(&2), "Committed row id=2 must exist");
}

/// Test: Committed DELETE survives crash.
///
/// INSERT + COMMIT → BEGIN → DELETE → COMMIT → (crash) → row gone.
#[test]
fn test_committed_delete_survives_crash() {
    let dir = TempDir::new().unwrap();
    let data_dir = dir.path().join("data");
    std::fs::create_dir_all(&data_dir).unwrap();

    // Phase 1: Commit a DELETE, then crash.
    {
        let mut storage = create_wal_storage(&data_dir);
        seed_table(&mut storage, "t4", &[100, 200, 300]);

        storage.begin_transaction().unwrap();
        storage.delete("t4", &[Value::Integer(200)]).unwrap();
        storage.commit_transaction().unwrap();
        // Now crash
    }

    // Phase 2: Recovery
    let (_report, recovered) = run_recovery(&data_dir);

    let rows = recovered.scan("t4").unwrap();
    let vals: Vec<i64> = rows
        .iter()
        .map(|r| match &r[0] {
            Value::Integer(v) => *v,
            _ => panic!("expected Integer"),
        })
        .collect();
    assert_eq!(vals, vec![100, 300], "Committed DELETE must be replayed");
}

/// Test: Empty transaction crash — no data loss.
///
/// BEGIN → (no DML) → (crash) → recovery → no effect.
#[test]
fn test_empty_transaction_crash() {
    let dir = TempDir::new().unwrap();
    let data_dir = dir.path().join("data");
    std::fs::create_dir_all(&data_dir).unwrap();

    {
        let mut storage = create_wal_storage(&data_dir);
        seed_table(&mut storage, "t5", &[42]);

        storage.begin_transaction().unwrap();
        // No DML, no commit — crash on empty tx
    }

    let (report, recovered) = run_recovery(&data_dir);

    assert_eq!(
        report.incomplete_txns, 1,
        "Empty uncommitted tx is still incomplete"
    );

    let rows = recovered.scan("t5").unwrap();
    let vals: Vec<i64> = rows
        .iter()
        .map(|r| match &r[0] {
            Value::Integer(v) => *v,
            _ => panic!("expected Integer"),
        })
        .collect();
    assert_eq!(vals, vec![42], "Empty tx crash must not affect data");
}

/// Test: RecoveryReport invariants after mixed workload crash.
///
/// Mix of committed, rolled-back, and incomplete transactions.
#[test]
fn test_mixed_workload_recovery_report() {
    let dir = TempDir::new().unwrap();
    let data_dir = dir.path().join("data");
    std::fs::create_dir_all(&data_dir).unwrap();

    {
        let mut storage = create_wal_storage(&data_dir);
        seed_table(&mut storage, "t6", &[]);

        // TX1: committed insert
        storage.begin_transaction().unwrap();
        storage.insert("t6", vec![vec![Value::Integer(1)]]).unwrap();
        storage.commit_transaction().unwrap();

        // TX2: rolled back insert
        storage.begin_transaction().unwrap();
        storage.insert("t6", vec![vec![Value::Integer(2)]]).unwrap();
        storage.rollback_transaction().unwrap();

        // TX3: incomplete insert (simulate crash mid-tx)
        storage.begin_transaction().unwrap();
        storage.insert("t6", vec![vec![Value::Integer(3)]]).unwrap();
        // No commit, no rollback — crash
    }

    let (report, recovered) = run_recovery(&data_dir);

    assert_eq!(report.committed_txns, 1, "Exactly 1 committed transaction");
    assert_eq!(
        report.incomplete_txns, 1,
        "Exactly 1 incomplete transaction"
    );

    // Only TX1's row should exist
    let rows = recovered.scan("t6").unwrap();
    let vals: Vec<i64> = rows
        .iter()
        .map(|r| match &r[0] {
            Value::Integer(v) => *v,
            _ => panic!("expected Integer"),
        })
        .collect();
    assert!(vals.contains(&1), "Committed row id=1 must exist");
    assert!(!vals.contains(&2), "Rolled-back row id=2 must NOT exist");
    assert!(!vals.contains(&3), "Incomplete row id=3 must NOT exist");
}

/// Test: Large transaction crash — many inserts without commit.
#[test]
fn test_large_batch_crash() {
    let dir = TempDir::new().unwrap();
    let data_dir = dir.path().join("data");
    std::fs::create_dir_all(&data_dir).unwrap();

    const BATCH_SIZE: usize = 100;

    {
        let mut storage = create_wal_storage(&data_dir);
        seed_table(&mut storage, "t7", &[]);

        storage.begin_transaction().unwrap();
        for i in 0..BATCH_SIZE {
            storage
                .insert("t7", vec![vec![Value::Integer(i as i64)]])
                .unwrap();
        }
        // No commit — crash
    }

    let (report, _recovered) = run_recovery(&data_dir);

    assert_eq!(report.incomplete_txns, 1, "Large incomplete tx detected");
    assert_eq!(
        report.rows_inserted, 0,
        "No rows from incomplete tx should be replayed"
    );
}

/// Test: Multiple rounds of crash-recovery.
#[test]
fn test_multiple_crash_recovery_cycles() {
    let dir = TempDir::new().unwrap();
    let data_dir = dir.path().join("data");
    std::fs::create_dir_all(&data_dir).unwrap();

    // Round 1: seed data, crash
    {
        let mut storage = create_wal_storage(&data_dir);
        seed_table(&mut storage, "t8", &[1, 2]);
        // Normal shutdown (commit + drop)
        // Actually let's just let it drop cleanly
    }
    let (_r1, _s1) = run_recovery(&data_dir); // recovery sees committed data

    // Round 2: add uncommitted data, crash
    {
        let mut storage = create_wal_storage(&data_dir);
        storage.begin_transaction().unwrap();
        storage.insert("t8", vec![vec![Value::Integer(3)]]).unwrap();
        // crash — no commit
    }

    // Round 3: verify only committed data from round 1
    let (report, recovered) = run_recovery(&data_dir);

    assert_eq!(report.incomplete_txns, 1, "Round 2 incomplete tx detected");

    let rows = recovered.scan("t8").unwrap();
    let vals: Vec<i64> = rows
        .iter()
        .map(|r| match &r[0] {
            Value::Integer(v) => *v,
            _ => panic!("expected Integer"),
        })
        .collect();
    assert_eq!(vals, vec![1, 2], "Only originally committed data survives");
}
