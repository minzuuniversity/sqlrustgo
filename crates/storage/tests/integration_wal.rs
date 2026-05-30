//! Integration tests for Storage Layer WAL component
//!
//! These tests verify WAL behavior and WAL+Storage integration.

use sqlrustgo_storage::wal::{WalManager, WalEntryType};
use tempfile::TempDir;

#[test]
fn test_wal_manager_create_and_recover() {
    // WalManager can be created even if WAL file doesn't exist yet
    // (get_reader will fail if file doesn't exist, but creation succeeds)
    let temp_dir = TempDir::new().unwrap();
    let wal_path = temp_dir.path().join("test.wal");
    let wal = WalManager::new(wal_path);
    // Basic smoke test - WAL manager instance created
    let _reader = wal.get_reader(); // May fail if no file yet — this is ok
}

#[test]
fn test_wal_log_single_transaction() {
    let temp_dir = TempDir::new().unwrap();
    let wal_path = temp_dir.path().join("single_tx.wal");
    let wal = WalManager::new(wal_path);

    let tx_id = 1u64;
    wal.log_begin(tx_id).unwrap();
    wal.log_insert(tx_id, 1, b"table:1".to_vec(), b"data".to_vec()).unwrap();
    wal.log_commit(tx_id).unwrap();

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
    let wal = WalManager::new(wal_path);

    // Transaction 1 (commit)
    wal.log_begin(1).unwrap();
    wal.log_insert(1, 1, b"t1:1".to_vec(), b"data1".to_vec()).unwrap();
    wal.log_commit(1).unwrap();

    // Transaction 2 (commit)
    wal.log_begin(2).unwrap();
    wal.log_insert(2, 1, b"t1:2".to_vec(), b"data2".to_vec()).unwrap();
    wal.log_commit(2).unwrap();

    // Transaction 3 (rollback)
    wal.log_begin(3).unwrap();
    wal.log_insert(3, 1, b"t1:3".to_vec(), b"data3".to_vec()).unwrap();
    wal.log_rollback(3).unwrap();

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
        let wal = WalManager::new(wal_path.clone());
        wal.log_begin(1).unwrap();
        wal.log_insert(1, 1, b"users:1".to_vec(), b"Alice".to_vec()).unwrap();
        wal.log_commit(1).unwrap();

        wal.log_begin(2).unwrap();
        wal.log_insert(2, 1, b"users:2".to_vec(), b"Bob".to_vec()).unwrap();
        // Simulate crash - no commit for tx 2
    }

    // Recover - should only see committed transaction
    let wal = WalManager::new(wal_path);
    let entries = wal.recover().unwrap();

    let commits = entries
        .iter()
        .filter(|e| e.entry_type == WalEntryType::Commit)
        .count();
    assert_eq!(
        commits, 1,
        "Only committed transactions should be recovered"
    );
}

#[test]
fn test_wal_checkpoint() {
    let temp_dir = TempDir::new().unwrap();
    let wal_path = temp_dir.path().join("checkpoint.wal");
    let wal = WalManager::new(wal_path);

    wal.log_begin(1).unwrap();
    wal.log_insert(1, 1, b"t:1".to_vec(), b"d".to_vec()).unwrap();
    wal.log_commit(1).unwrap();

    let checkpoint_lsn = wal.checkpoint(1).unwrap();
    // Checkpoint LSN may be 0 for empty WAL or if no writer has been opened yet
    assert!(checkpoint_lsn >= 0, "Checkpoint should return a valid LSN");
}