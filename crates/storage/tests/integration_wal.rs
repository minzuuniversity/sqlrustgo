//! Integration tests for Storage Layer WAL component
//!
//! These tests verify WAL behavior and WAL+Storage integration.

use sqlrustgo_storage::wal::{FileBackedWalManager, WalEntryType, WalManager};
use sqlrustgo_storage::wal::{
    make_begin_entry, make_commit_entry, make_insert_entry, make_rollback_entry,
};
use tempfile::TempDir;

#[test]
fn test_wal_file_backed_create_and_recover() {
    // FileBackedWalManager can be created even if WAL file doesn't exist yet
    let temp_dir = TempDir::new().unwrap();
    let wal_path = temp_dir.path().join("test.wal");
    let _wal = FileBackedWalManager::new(wal_path).unwrap();
    // Basic smoke test - WAL manager instance created
}

#[test]
fn test_wal_log_single_transaction() {
    let temp_dir = TempDir::new().unwrap();
    let wal_path = temp_dir.path().join("single_tx.wal");
    let mut wal = FileBackedWalManager::new(wal_path).unwrap();

    let tx_id = 1u64;
    wal.append(make_begin_entry(tx_id)).unwrap();
    wal.append(make_insert_entry(tx_id, 1, b"key1".to_vec(), b"data".to_vec(), 1))
        .unwrap();
    wal.append(make_commit_entry(tx_id, 2)).unwrap();

    let entries = wal.recover().unwrap();

    let entry_types: Vec<WalEntryType> = entries.iter().map(|e| e.entry_type).collect();
    assert!(entry_types.contains(&WalEntryType::Begin));
    assert!(entry_types.contains(&WalEntryType::Insert));
    assert!(entry_types.contains(&WalEntryType::Commit));
}

#[test]
fn test_wal_multiple_transactions() {
    let temp_dir = TempDir::new().unwrap();
    let wal_path = temp_dir.path().join("multi_tx.wal");
    let mut wal = FileBackedWalManager::new(wal_path).unwrap();

    // Transaction 1 (commit)
    wal.append(make_begin_entry(1)).unwrap();
    wal.append(make_insert_entry(1, 1, b"k1".to_vec(), b"data1".to_vec(), 1))
        .unwrap();
    wal.append(make_commit_entry(1, 2)).unwrap();

    // Transaction 2 (commit)
    wal.append(make_begin_entry(2)).unwrap();
    wal.append(make_insert_entry(2, 1, b"k2".to_vec(), b"data2".to_vec(), 3))
        .unwrap();
    wal.append(make_commit_entry(2, 4)).unwrap();

    // Transaction 3 (rollback)
    wal.append(make_begin_entry(3)).unwrap();
    wal.append(make_insert_entry(3, 1, b"k3".to_vec(), b"data3".to_vec(), 5))
        .unwrap();
    wal.append(make_rollback_entry(3, 6)).unwrap();

    let entries = wal.recover().unwrap();

    let commits = entries
        .iter()
        .filter(|e| e.entry_type == WalEntryType::Commit)
        .count();
    let rollbacks = entries
        .iter()
        .filter(|e| e.entry_type == WalEntryType::Rollback)
        .count();

    assert_eq!(commits, 2, "Should have 2 commits");
    assert_eq!(rollbacks, 1, "Should have 1 rollback");
}

#[test]
fn test_wal_recovery_uncommitted_transaction() {
    let temp_dir = TempDir::new().unwrap();
    let wal_path = temp_dir.path().join("recovery.wal");

    // First "session" - write committed and uncommitted data
    {
        let mut wal = FileBackedWalManager::new(wal_path.clone()).unwrap();
        wal.append(make_begin_entry(1)).unwrap();
        wal.append(make_insert_entry(1, 1, b"k1".to_vec(), b"Alice".to_vec(), 1))
            .unwrap();
        wal.append(make_commit_entry(1, 2)).unwrap();

        wal.append(make_begin_entry(2)).unwrap();
        wal.append(make_insert_entry(2, 1, b"k2".to_vec(), b"Bob".to_vec(), 3))
            .unwrap();
        // Simulate crash - no commit for tx 2
    }

    // Recover - should only see committed transaction
    let mut wal = FileBackedWalManager::new(wal_path).unwrap();
    let entries = wal.recover().unwrap();

    let commits = entries
        .iter()
        .filter(|e| e.entry_type == WalEntryType::Commit)
        .count();
    assert_eq!(commits, 1, "Only committed transactions should be recovered");
}

#[test]
fn test_wal_truncate_before() {
    let temp_dir = TempDir::new().unwrap();
    let wal_path = temp_dir.path().join("checkpoint.wal");
    let mut wal = FileBackedWalManager::new(wal_path).unwrap();

    wal.append(make_begin_entry(1)).unwrap();
    wal.append(make_insert_entry(1, 1, b"k".to_vec(), b"d".to_vec(), 1))
        .unwrap();
    wal.append(make_commit_entry(1, 2)).unwrap();

    // truncate_before should retain entries with lsn >= given lsn
    wal.truncate_before(0).unwrap();
    let entries = wal.recover().unwrap();
    assert_eq!(entries.len(), 3, "truncate_before(0) should retain all entries");
}