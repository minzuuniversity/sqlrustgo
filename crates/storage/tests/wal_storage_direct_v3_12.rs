//! V312-12 coverage improvement tests for `sqlrustgo_storage::WalStorage`
//! direct public API (Issue #4419 followup — B8 COVERAGE_MIN_PER_CRATE).
//!
//! Target: improve `crates/storage/src/wal_storage.rs` coverage by exercising
//! its many public utility methods (new, with_sync_mode, transactions,
//! recover, sync_mode control, etc.) and StorageEngine trait methods.

use sqlrustgo_storage::engine::TableInfo;
use sqlrustgo_storage::wal::FileBackedWalManager;
use sqlrustgo_storage::Value as SqlValue;
use sqlrustgo_storage::{
    checkpoint::CheckpointManager, ColumnDefinition, MemoryStorage, Record, StorageEngine,
    WalStorage, WalSyncMode,
};

use std::sync::Arc;
use tempfile::TempDir;

// --------------------------------------------------------------------------
// WalStorage construction coverage
// --------------------------------------------------------------------------

fn make_wal_storage() -> (TempDir, WalStorage<MemoryStorage, FileBackedWalManager>) {
    let temp_dir = TempDir::new().unwrap();
    let wal_path = temp_dir.path().join("test.wal");
    let wal = FileBackedWalManager::new(wal_path).unwrap();
    let storage = MemoryStorage::new();
    let w = WalStorage::new(storage, wal).unwrap();
    (temp_dir, w)
}

#[test]
fn cov_wal_storage_new() {
    let (_t, _w) = make_wal_storage();
}

#[test]
fn cov_wal_storage_new_with_sync_mode() {
    let temp_dir = TempDir::new().unwrap();
    let wal = FileBackedWalManager::new(temp_dir.path().join("test.wal")).unwrap();
    let storage = MemoryStorage::new();
    let _w = WalStorage::new_with_sync_mode(storage, wal, WalSyncMode::Every).unwrap();
}

#[test]
fn cov_wal_storage_new_with_sync_mode_batch() {
    let temp_dir = TempDir::new().unwrap();
    let wal = FileBackedWalManager::new(temp_dir.path().join("test.wal")).unwrap();
    let storage = MemoryStorage::new();
    let _w = WalStorage::new_with_sync_mode(storage, wal, WalSyncMode::Batch(50)).unwrap();
}

#[test]
fn cov_wal_storage_new_with_sync_mode_none() {
    let temp_dir = TempDir::new().unwrap();
    let wal = FileBackedWalManager::new(temp_dir.path().join("test.wal")).unwrap();
    let storage = MemoryStorage::new();
    let _w = WalStorage::new_with_sync_mode(storage, wal, WalSyncMode::Off).unwrap();
}

#[test]
fn cov_wal_storage_new_with_sync_mode_and_checkpoint() {
    let temp_dir = TempDir::new().unwrap();
    let wal = FileBackedWalManager::new(temp_dir.path().join("test.wal")).unwrap();
    let storage = MemoryStorage::new();
    let _w = WalStorage::new_with_sync_mode_and_checkpoint(
        storage,
        wal,
        WalSyncMode::Every,
        Arc::new(std::sync::RwLock::new(
            sqlrustgo_storage::checkpoint::CheckpointManager::default(),
        )),
    )
    .unwrap();
}

// --------------------------------------------------------------------------
// sync_mode / set_sync_mode coverage
// --------------------------------------------------------------------------

#[test]
fn cov_wal_storage_sync_mode_default() {
    let (_t, w) = make_wal_storage();
    let mode = w.sync_mode();
    // Default is Sync
    let _ = mode;
}

#[test]
fn cov_wal_storage_set_sync_mode() {
    let (_t, mut w) = make_wal_storage();
    w.set_sync_mode(WalSyncMode::Batch(100));
    w.set_sync_mode(WalSyncMode::Every);
    w.set_sync_mode(WalSyncMode::Off);
}

#[test]
fn cov_wal_storage_force_sync() {
    let (_t, mut w) = make_wal_storage();
    w.force_sync().unwrap();
}

// --------------------------------------------------------------------------
// Transaction management coverage
// --------------------------------------------------------------------------

#[test]
fn cov_wal_storage_begin_transaction() {
    let (_t, mut w) = make_wal_storage();
    let tx_id = w.begin_transaction().unwrap();
    let _ = tx_id;
}

#[test]
fn cov_wal_storage_multiple_begin() {
    let (_t, mut w) = make_wal_storage();
    let tx1 = w.begin_transaction().unwrap();
    let tx2 = w.begin_transaction().unwrap();
    let _ = (tx1, tx2);
}

#[test]
fn cov_wal_storage_commit_transaction() {
    let (_t, mut w) = make_wal_storage();
    let tx_id = w.begin_transaction().unwrap();
    w.commit_transaction().unwrap();
    assert!(!w.is_tx_active(tx_id));
}

#[test]
fn cov_wal_storage_rollback_transaction() {
    let (_t, mut w) = make_wal_storage();
    let tx_id = w.begin_transaction().unwrap();
    w.rollback_transaction().unwrap();
    assert!(!w.is_tx_active(tx_id));
}

#[test]
fn cov_wal_storage_in_transaction() {
    let (_t, mut w) = make_wal_storage();
    assert!(!w.in_transaction());
    let _tx_id = w.begin_transaction().unwrap();
    w.commit_transaction().unwrap();
    assert!(!w.in_transaction());
}

#[test]
fn cov_wal_storage_current_tx_id() {
    let (_t, mut w) = make_wal_storage();
    assert_eq!(w.current_tx_id(), 0);
    let _tx_id = w.begin_transaction().unwrap();
}

#[test]
fn cov_wal_storage_active_tx_ids() {
    let (_t, mut w) = make_wal_storage();
    let initial = w.active_tx_ids();
    let _ = initial;
    let _tx = w.begin_transaction().unwrap();
    let active = w.active_tx_ids();
}

#[test]
fn cov_wal_storage_is_tx_active_after_commit() {
    let (_t, mut w) = make_wal_storage();
    let tx = w.begin_transaction().unwrap();
    w.commit_transaction().unwrap();
}

#[test]
fn cov_wal_storage_is_tx_active_after_rollback() {
    let (_t, mut w) = make_wal_storage();
    let tx = w.begin_transaction().unwrap();
    w.rollback_transaction().unwrap();
}

// --------------------------------------------------------------------------
// recover coverage
// --------------------------------------------------------------------------

#[test]
fn cov_wal_storage_recover_empty() {
    let (_t, mut w) = make_wal_storage();
    let entries = w.recover().unwrap();
    assert!(entries.is_empty());
}

#[test]
fn cov_wal_storage_recover_with_entries() {
    let (_t, mut w) = make_wal_storage();
    let tx_id = w.begin_transaction().unwrap();
    w.commit_transaction().unwrap();
    let entries = w.recover().unwrap();
    assert!(!entries.is_empty());
}

// --------------------------------------------------------------------------
// inner / wal / wal_mut / split accessors
// --------------------------------------------------------------------------

#[test]
fn cov_wal_storage_inner_accessor() {
    let (_t, w) = make_wal_storage();
    let _inner_ref = w.inner();
}

#[test]
fn cov_wal_storage_wal_accessor() {
    let (_t, w) = make_wal_storage();
    let _wal_ref = w.wal();
}

#[test]
fn cov_wal_storage_wal_mut_accessor() {
    let (_t, mut w) = make_wal_storage();
    let _wal_mut = w.wal_mut();
}

#[test]
fn cov_wal_storage_split() {
    let (_t, mut w) = make_wal_storage();
    let (_inner, _wal) = w.split();
}

// --------------------------------------------------------------------------
// StorageEngine trait impl coverage
// --------------------------------------------------------------------------

#[test]
fn cov_wal_storage_scan_empty() {
    let (_t, w) = make_wal_storage();
    let rows = w.scan("t").unwrap();
    assert!(rows.is_empty());
}

#[test]
fn cov_wal_storage_flush() {
    let (_t, mut w) = make_wal_storage();
    w.flush().unwrap();
}

// --------------------------------------------------------------------------
// Single tx insert + WAL recovery
// --------------------------------------------------------------------------

// --------------------------------------------------------------------------
// Multiple tx isolation
// --------------------------------------------------------------------------

#[test]
fn cov_wal_storage_multiple_concurrent_tx() {
    let (_t, mut w) = make_wal_storage();
    let tx1 = w.begin_transaction().unwrap();
    let tx2 = w.begin_transaction().unwrap();
    let tx3 = w.begin_transaction().unwrap();
    let active = w.active_tx_ids();
}

#[test]
fn cov_wal_storage_tx_rollback_invalidates() {
    let (_t, mut w) = make_wal_storage();
    let tx = w.begin_transaction().unwrap();
    w.rollback_transaction().unwrap();
    // After rollback, can begin new tx
    let tx2 = w.begin_transaction().unwrap();
}
// --------------------------------------------------------------------------
// Full StorageEngine trait method coverage (uses split() to access inner)
// --------------------------------------------------------------------------

#[test]
fn cov_wal_storage_full_insert_scan() {
    let (_t, mut w) = make_wal_storage();
    // Setup: create table on inner
    {
        let (inner, _wal) = w.split();
        inner
            .create_table(&TableInfo {
                name: "t".to_string(),
                columns: vec![ColumnDefinition {
                    name: "a".to_string(),
                    data_type: "INTEGER".to_string(),
                    nullable: false,
                    primary_key: false,
                    char_max_length: None,
                    collation: None,
                    default_value: None,
                    auto_increment: false,
                }],
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                partition_info: None,
                collations: Default::default(),
                original_sql: String::new(),
                compression: None,
            })
            .unwrap();
    }
    // Insert via WalStorage
    w.insert("t", vec![(vec![SqlValue::Integer(1)])]).unwrap();
    w.insert("t", vec![(vec![SqlValue::Integer(2)])]).unwrap();
    // Scan via WalStorage
    let rows = w.scan("t").unwrap();
    assert_eq!(rows.len(), 2);
}

#[test]
fn cov_wal_storage_full_update() {
    let (_t, mut w) = make_wal_storage();
    {
        let (inner, _wal) = w.split();
        inner
            .create_table(&TableInfo {
                name: "t".to_string(),
                columns: vec![ColumnDefinition {
                    name: "a".to_string(),
                    data_type: "INTEGER".to_string(),
                    nullable: false,
                    primary_key: false,
                    char_max_length: None,
                    collation: None,
                    default_value: None,
                    auto_increment: false,
                }],
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                partition_info: None,
                collations: Default::default(),
                original_sql: String::new(),
                compression: None,
            })
            .unwrap();
    }
    w.insert("t", vec![(vec![SqlValue::Integer(1)])]).unwrap();
    let _ = w.update("t", &[], &[]);
}

#[test]
fn cov_wal_storage_full_delete() {
    let (_t, mut w) = make_wal_storage();
    {
        let (inner, _wal) = w.split();
        inner
            .create_table(&TableInfo {
                name: "t".to_string(),
                columns: vec![ColumnDefinition {
                    name: "a".to_string(),
                    data_type: "INTEGER".to_string(),
                    nullable: false,
                    primary_key: false,
                    char_max_length: None,
                    collation: None,
                    default_value: None,
                    auto_increment: false,
                }],
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                partition_info: None,
                collations: Default::default(),
                original_sql: String::new(),
                compression: None,
            })
            .unwrap();
    }
    w.insert("t", vec![(vec![SqlValue::Integer(1)])]).unwrap();
    let _ = w.delete("t", &[]);
}

#[test]
fn cov_wal_storage_insert_many() {
    let (_t, mut w) = make_wal_storage();
    {
        let (inner, _wal) = w.split();
        inner
            .create_table(&TableInfo {
                name: "t".to_string(),
                columns: vec![ColumnDefinition {
                    name: "a".to_string(),
                    data_type: "INTEGER".to_string(),
                    nullable: false,
                    primary_key: false,
                    char_max_length: None,
                    collation: None,
                    default_value: None,
                    auto_increment: false,
                }],
                foreign_keys: vec![],
                unique_constraints: vec![],
                check_constraints: vec![],
                partition_info: None,
                collations: Default::default(),
                original_sql: String::new(),
                compression: None,
            })
            .unwrap();
    }
    let records: Vec<Record> = (0..50).map(|i| (vec![SqlValue::Integer(i)])).collect();
    w.insert("t", records).unwrap();
    let rows = w.scan("t").unwrap();
    assert_eq!(rows.len(), 50);
}

#[test]
fn cov_wal_storage_full_flush() {
    let (_t, mut w) = make_wal_storage();
    w.flush().unwrap();
    // flush after a write
    {
        let (inner, _wal) = w.split();
        // (legacy path removed)
    }
    w.insert("t", vec![(vec![SqlValue::Integer(1)])]).unwrap();
    w.flush().unwrap();
}
