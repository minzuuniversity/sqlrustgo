//! White-box tests for WAL module (wal/mod.rs)
//!
//! These tests exercise the WalManager trait and helper functions.

use sqlrustgo_storage::wal::{
    make_begin_entry, make_commit_entry, make_delete_entry, make_insert_entry, make_rollback_entry,
    make_update_entry, FileBackedWalManager, MemoryWalManager, WalEntry, WalEntryType, WalManager,
};
use tempfile::TempDir;

// =============================================================================
// Helper Functions Tests
// =============================================================================

#[test]
fn test_make_begin_entry() {
    let entry = make_begin_entry(42);

    assert_eq!(entry.tx_id, 42);
    assert_eq!(entry.entry_type, WalEntryType::Begin);
    assert_eq!(entry.table_id, 0);
    assert!(entry.key.is_none());
    assert!(entry.data.is_none());
    assert_eq!(entry.lsn, 0);
    assert!(entry.timestamp > 0);
}

#[test]
fn test_make_commit_entry() {
    let entry = make_commit_entry(99, 12345);

    assert_eq!(entry.tx_id, 99);
    assert_eq!(entry.entry_type, WalEntryType::Commit);
    assert_eq!(entry.table_id, 0);
    assert!(entry.key.is_none());
    assert!(entry.data.is_none());
    assert_eq!(entry.lsn, 12345);
    assert!(entry.timestamp > 0);
}

#[test]
fn test_make_rollback_entry() {
    let entry = make_rollback_entry(77, 999);

    assert_eq!(entry.tx_id, 77);
    assert_eq!(entry.entry_type, WalEntryType::Rollback);
    assert_eq!(entry.table_id, 0);
    assert!(entry.key.is_none());
    assert!(entry.data.is_none());
    assert_eq!(entry.lsn, 999);
    assert!(entry.timestamp > 0);
}

#[test]
fn test_make_insert_entry() {
    let entry = make_insert_entry(
        1,             // tx_id
        100,           // table_id
        vec![1, 2, 3], // key
        vec![4, 5, 6], // data
        42,            // lsn
    );

    assert_eq!(entry.tx_id, 1);
    assert_eq!(entry.entry_type, WalEntryType::Insert);
    assert_eq!(entry.table_id, 100);
    assert_eq!(entry.key, Some(vec![1, 2, 3]));
    assert_eq!(entry.data, Some(vec![4, 5, 6]));
    assert_eq!(entry.lsn, 42);
    assert!(entry.timestamp > 0);
}

#[test]
fn test_make_update_entry() {
    let entry = make_update_entry(
        2,            // tx_id
        200,          // table_id
        vec![10],     // key
        vec![11, 12], // data
        50,           // lsn
    );

    assert_eq!(entry.tx_id, 2);
    assert_eq!(entry.entry_type, WalEntryType::Update);
    assert_eq!(entry.table_id, 200);
    assert_eq!(entry.key, Some(vec![10]));
    assert_eq!(entry.data, Some(vec![11, 12]));
    assert_eq!(entry.lsn, 50);
    assert!(entry.timestamp > 0);
}

#[test]
fn test_make_delete_entry() {
    let entry = make_delete_entry(3, 300, vec![1, 2, 3], 60);

    assert_eq!(entry.tx_id, 3);
    assert_eq!(entry.entry_type, WalEntryType::Delete);
    assert_eq!(entry.table_id, 300);
    assert_eq!(entry.key, Some(vec![1, 2, 3]));
    assert!(entry.data.is_none());
    assert_eq!(entry.lsn, 60);
    assert!(entry.timestamp > 0);
}

// =============================================================================
// WalManager Trait - MemoryWalManager Tests
// =============================================================================

#[test]
fn test_memory_wal_manager_new() {
    let wal = MemoryWalManager::new();
    // Should be able to create without error
    let mut w = wal;
    let entries = w.recover().unwrap();
    assert!(entries.is_empty());
}

#[test]
fn test_memory_wal_manager_append() {
    let mut wal = MemoryWalManager::new();
    let entry = make_insert_entry(1, 100, vec![1], vec![2], 0);
    wal.append(entry).unwrap();

    let entries = wal.recover().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].tx_id, 1);
    assert_eq!(entries[0].entry_type, WalEntryType::Insert);
}

#[test]
fn test_memory_wal_manager_append_multiple() {
    let mut wal = MemoryWalManager::new();

    wal.append(make_begin_entry(1)).unwrap();
    wal.append(make_insert_entry(1, 100, vec![1], vec![2], 1))
        .unwrap();
    wal.append(make_commit_entry(1, 2)).unwrap();

    let entries = wal.recover().unwrap();
    assert_eq!(entries.len(), 3);
}

#[test]
fn test_memory_wal_manager_flush() {
    let mut wal = MemoryWalManager::new();
    wal.append(make_insert_entry(1, 100, vec![1], vec![2], 0))
        .unwrap();

    // flush should succeed without error
    wal.flush().unwrap();

    // Entries should still be there (recover hasn't been called yet)
    let entries = wal.recover().unwrap();
    assert_eq!(entries.len(), 1);
}

#[test]
fn test_memory_wal_manager_sync() {
    let mut wal = MemoryWalManager::new();
    wal.append(make_insert_entry(1, 100, vec![1], vec![2], 0))
        .unwrap();

    // sync should succeed without error
    wal.sync().unwrap();

    // Entries should still be there
    let entries = wal.recover().unwrap();
    assert_eq!(entries.len(), 1);
}

#[test]
fn test_memory_wal_manager_recover_clears_entries() {
    let mut wal = MemoryWalManager::new();
    wal.append(make_begin_entry(1)).unwrap();
    wal.append(make_commit_entry(1, 1)).unwrap();

    let entries = wal.recover().unwrap();
    assert_eq!(entries.len(), 2);

    // Second recover should return empty (entries were moved)
    let entries2 = wal.recover().unwrap();
    assert_eq!(entries2.len(), 0);
}

#[test]
fn test_memory_wal_manager_default() {
    let wal = MemoryWalManager::default();
    let mut w = wal;
    let entries = w.recover().unwrap();
    assert!(entries.is_empty());
}

// =============================================================================
// FileBackedWalManager Tests
// =============================================================================

#[test]
fn test_file_backed_wal_manager_new() {
    let temp_dir = TempDir::new().unwrap();
    let wal_path = temp_dir.path().join("test.wal");

    let wal = FileBackedWalManager::new(wal_path.clone()).unwrap();
    assert_eq!(wal.path(), &wal_path);
}

#[test]
fn test_file_backed_wal_manager_size() {
    let temp_dir = TempDir::new().unwrap();
    let wal_path = temp_dir.path().join("test.wal");

    let mut wal = FileBackedWalManager::new(wal_path.clone()).unwrap();

    // Empty WAL should have size 0
    let size = wal.size().unwrap();
    assert_eq!(size, 0);

    // Append an entry
    wal.append(make_insert_entry(1, 100, vec![1], vec![2], 0))
        .unwrap();
    wal.flush().unwrap();

    // Size should be > 0 after appending
    let size = wal.size().unwrap();
    assert!(size > 0);
}

#[test]
fn test_file_backed_wal_manager_append_and_recover() {
    let temp_dir = TempDir::new().unwrap();
    let wal_path = temp_dir.path().join("test.wal");

    let mut wal = FileBackedWalManager::new(wal_path.clone()).unwrap();

    // Append entries
    wal.append(make_begin_entry(1)).unwrap();
    wal.append(make_insert_entry(1, 100, vec![1], vec![2], 1))
        .unwrap();
    wal.append(make_commit_entry(1, 2)).unwrap();

    // Recover should return the entries
    let entries = wal.recover().unwrap();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].entry_type, WalEntryType::Begin);
    assert_eq!(entries[1].entry_type, WalEntryType::Insert);
    assert_eq!(entries[2].entry_type, WalEntryType::Commit);
}

#[test]
fn test_file_backed_wal_manager_truncate() {
    let temp_dir = TempDir::new().unwrap();
    let wal_path = temp_dir.path().join("test.wal");

    let mut wal = FileBackedWalManager::new(wal_path.clone()).unwrap();

    // Append some entries
    wal.append(make_insert_entry(1, 100, vec![1], vec![2], 0))
        .unwrap();
    wal.flush().unwrap();

    // Verify there are entries
    let entries = wal.recover().unwrap();
    assert_eq!(entries.len(), 1);

    // Truncate the WAL
    wal.truncate().unwrap();

    // Recover should return empty
    let entries = wal.recover().unwrap();
    assert_eq!(entries.len(), 0);

    // Should still be able to append after truncate
    wal.append(make_insert_entry(2, 100, vec![3], vec![4], 0))
        .unwrap();
    let entries = wal.recover().unwrap();
    assert_eq!(entries.len(), 1);
}

#[test]
fn test_file_backed_wal_manager_flush() {
    let temp_dir = TempDir::new().unwrap();
    let wal_path = temp_dir.path().join("test.wal");

    let mut wal = FileBackedWalManager::new(wal_path.clone()).unwrap();

    wal.append(make_insert_entry(1, 100, vec![1], vec![2], 0))
        .unwrap();

    // flush should succeed
    wal.flush().unwrap();

    // Entries should be recoverable
    let entries = wal.recover().unwrap();
    assert_eq!(entries.len(), 1);
}

#[test]
fn test_file_backed_wal_manager_sync() {
    let temp_dir = TempDir::new().unwrap();
    let wal_path = temp_dir.path().join("test.wal");

    let mut wal = FileBackedWalManager::new(wal_path.clone()).unwrap();

    wal.append(make_insert_entry(1, 100, vec![1], vec![2], 0))
        .unwrap();

    // sync should succeed
    wal.sync().unwrap();

    // Entries should be recoverable
    let entries = wal.recover().unwrap();
    assert_eq!(entries.len(), 1);
}

#[test]
fn test_file_backed_wal_manager_recover_nonexistent_file() {
    let temp_dir = TempDir::new().unwrap();
    let wal_path = temp_dir.path().join("nonexistent.wal");

    let mut wal = FileBackedWalManager::new(wal_path.clone()).unwrap();

    // Recovering from non-existent file should return empty Vec
    let entries = wal.recover().unwrap();
    assert_eq!(entries.len(), 0);
}

#[test]
fn test_file_backed_wal_manager_multiple_append_after_truncate() {
    let temp_dir = TempDir::new().unwrap();
    let wal_path = temp_dir.path().join("test.wal");

    let mut wal = FileBackedWalManager::new(wal_path.clone()).unwrap();

    // First batch
    wal.append(make_begin_entry(1)).unwrap();
    wal.append(make_commit_entry(1, 1)).unwrap();

    // Truncate
    wal.truncate().unwrap();

    // Second batch
    wal.append(make_begin_entry(2)).unwrap();
    wal.append(make_insert_entry(2, 100, vec![1], vec![2], 1))
        .unwrap();
    wal.append(make_commit_entry(2, 2)).unwrap();

    let entries = wal.recover().unwrap();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].tx_id, 2);
    assert_eq!(entries[0].entry_type, WalEntryType::Begin);
}

#[test]
fn test_file_backed_wal_manager_recover_with_reopened_manager() {
    let temp_dir = TempDir::new().unwrap();
    let wal_path = temp_dir.path().join("test.wal");

    // First manager - write entries
    {
        let mut wal = FileBackedWalManager::new(wal_path.clone()).unwrap();
        wal.append(make_begin_entry(1)).unwrap();
        wal.append(make_insert_entry(1, 100, vec![1], vec![2], 1))
            .unwrap();
        wal.append(make_commit_entry(1, 2)).unwrap();
    }

    // Second manager - read entries (opened fresh)
    {
        let mut wal = FileBackedWalManager::new(wal_path.clone()).unwrap();
        let entries = wal.recover().unwrap();
        assert_eq!(entries.len(), 3);
    }
}

// =============================================================================
// WalEntry Serde Tests
// =============================================================================

#[test]
fn test_wal_entry_to_bytes_and_from_bytes() {
    let entry = WalEntry {
        tx_id: 42,
        entry_type: WalEntryType::Insert,
        table_id: 100,
        key: Some(vec![1, 2, 3]),
        data: Some(vec![4, 5, 6]),
        lsn: 7,
        timestamp: 1000,
    };

    let bytes = entry.to_bytes();
    let recovered = WalEntry::from_bytes(&bytes).unwrap();

    assert_eq!(recovered.tx_id, entry.tx_id);
    assert_eq!(recovered.entry_type, entry.entry_type);
    assert_eq!(recovered.table_id, entry.table_id);
    assert_eq!(recovered.key, entry.key);
    assert_eq!(recovered.data, entry.data);
    assert_eq!(recovered.lsn, entry.lsn);
    assert_eq!(recovered.timestamp, entry.timestamp);
}

#[test]
fn test_wal_entry_to_bytes_and_from_bytes_no_key_no_data() {
    let entry = WalEntry {
        tx_id: 1,
        entry_type: WalEntryType::Begin,
        table_id: 0,
        key: None,
        data: None,
        lsn: 0,
        timestamp: 1234,
    };

    let bytes = entry.to_bytes();
    let recovered = WalEntry::from_bytes(&bytes).unwrap();

    assert_eq!(recovered.tx_id, entry.tx_id);
    assert_eq!(recovered.entry_type, entry.entry_type);
    assert_eq!(recovered.key, None);
    assert_eq!(recovered.data, None);
}

#[test]
fn test_wal_entry_from_bytes_truncated() {
    // Too short to be valid
    let bytes = vec![1, 2, 3];
    let recovered = WalEntry::from_bytes(&bytes);
    assert!(recovered.is_none());
}

#[test]
fn test_wal_entry_from_bytes_invalid_entry_type() {
    // Valid header but invalid entry type byte (255)
    let mut bytes = vec![0; 34];
    bytes[24] = 255; // entry_type at offset 24
    let recovered = WalEntry::from_bytes(&bytes);
    assert!(recovered.is_none());
}

// =============================================================================
// Integration-style tests for FileBackedWalManager
// =============================================================================

#[test]
fn test_file_backed_wal_manager_all_entry_types() {
    let temp_dir = TempDir::new().unwrap();
    let wal_path = temp_dir.path().join("test.wal");

    let mut wal = FileBackedWalManager::new(wal_path.clone()).unwrap();

    wal.append(make_begin_entry(1)).unwrap();
    wal.append(make_insert_entry(1, 100, vec![1], vec![2], 1))
        .unwrap();
    wal.append(make_update_entry(1, 100, vec![1], vec![3], 2))
        .unwrap();
    wal.append(make_delete_entry(1, 100, vec![1], 3)).unwrap();
    wal.append(make_commit_entry(1, 4)).unwrap();

    let entries = wal.recover().unwrap();
    assert_eq!(entries.len(), 5);
    assert_eq!(entries[0].entry_type, WalEntryType::Begin);
    assert_eq!(entries[1].entry_type, WalEntryType::Insert);
    assert_eq!(entries[2].entry_type, WalEntryType::Update);
    assert_eq!(entries[3].entry_type, WalEntryType::Delete);
    assert_eq!(entries[4].entry_type, WalEntryType::Commit);
}

#[test]
fn test_file_backed_wal_manager_rollback_recovery() {
    let temp_dir = TempDir::new().unwrap();
    let wal_path = temp_dir.path().join("test.wal");

    let mut wal = FileBackedWalManager::new(wal_path.clone()).unwrap();

    // Transaction 1 - committed
    wal.append(make_begin_entry(1)).unwrap();
    wal.append(make_insert_entry(1, 100, vec![1], vec![2], 1))
        .unwrap();
    wal.append(make_commit_entry(1, 2)).unwrap();

    // Transaction 2 - rolled back
    wal.append(make_begin_entry(2)).unwrap();
    wal.append(make_insert_entry(2, 100, vec![3], vec![4], 3))
        .unwrap();
    wal.append(make_rollback_entry(2, 4)).unwrap();

    let entries = wal.recover().unwrap();
    // All entries including rollback are recoverable
    assert_eq!(entries.len(), 6);
}

// =============================================================================
// WalEntry equality and debug
// =============================================================================

#[test]
fn test_wal_entry_clone() {
    let entry = make_insert_entry(1, 100, vec![1, 2], vec![3, 4], 5);
    let cloned = entry.clone();

    assert_eq!(entry.tx_id, cloned.tx_id);
    assert_eq!(entry.entry_type, cloned.entry_type);
    assert_eq!(entry.table_id, cloned.table_id);
    assert_eq!(entry.key, cloned.key);
    assert_eq!(entry.data, cloned.data);
    assert_eq!(entry.lsn, cloned.lsn);
}

#[test]
fn test_wal_entry_debug() {
    let entry = make_begin_entry(42);
    let debug_str = format!("{:?}", entry);
    assert!(debug_str.contains("Begin"));
}

#[test]
fn test_wal_entry_type_debug() {
    let debug_str = format!("{:?}", WalEntryType::Insert);
    assert!(debug_str.contains("Insert"));
}

// =============================================================================
// Additional tests for complete coverage
// =============================================================================

#[test]
fn test_file_backed_wal_manager_append_after_none_writer() {
    // Test the path where writer is None and gets recreated
    let temp_dir = TempDir::new().unwrap();
    let wal_path = temp_dir.path().join("test.wal");

    // Create manager and truncate to set writer to None
    let mut wal = FileBackedWalManager::new(wal_path.clone()).unwrap();
    wal.truncate().unwrap();

    // Now append should recreate the writer
    wal.append(make_insert_entry(1, 100, vec![1], vec![2], 0))
        .unwrap();

    let entries = wal.recover().unwrap();
    assert_eq!(entries.len(), 1);
}

#[test]
fn test_wal_entry_type_all_variants() {
    // Test all WalEntryType variants can be created and used
    let types = vec![
        (WalEntryType::Begin, 1u8),
        (WalEntryType::Insert, 2),
        (WalEntryType::Update, 3),
        (WalEntryType::Delete, 4),
        (WalEntryType::Commit, 5),
        (WalEntryType::Rollback, 6),
        (WalEntryType::Checkpoint, 7),
        (WalEntryType::Prepare, 8),
    ];

    for (entry_type, expected_discriminant) in types {
        let entry = WalEntry {
            tx_id: 1,
            entry_type,
            table_id: 0,
            key: None,
            data: None,
            lsn: 0,
            timestamp: 0,
        };

        let bytes = entry.to_bytes();
        let recovered = WalEntry::from_bytes(&bytes).unwrap();
        // Cast to u8 to compare discriminants since the enum has explicit values
        assert_eq!(recovered.entry_type as u8, expected_discriminant);
    }
}

#[test]
fn test_wal_entry_with_large_data() {
    // Test WAL entry with larger data
    let large_key: Vec<u8> = (0u8..200).collect();
    let large_data: Vec<u8> = (0u8..250).collect();

    let entry = WalEntry {
        tx_id: 1,
        entry_type: WalEntryType::Insert,
        table_id: 100,
        key: Some(large_key.clone()),
        data: Some(large_data.clone()),
        lsn: 42,
        timestamp: 1000,
    };

    let bytes = entry.to_bytes();
    let recovered = WalEntry::from_bytes(&bytes).unwrap();

    assert_eq!(recovered.key.unwrap(), large_key);
    assert_eq!(recovered.data.unwrap(), large_data);
}

#[test]
fn test_memory_wal_manager_multiple_transactions() {
    let mut wal = MemoryWalManager::new();

    // Transaction 1
    wal.append(make_begin_entry(1)).unwrap();
    wal.append(make_insert_entry(1, 100, vec![1], vec![2], 1))
        .unwrap();
    wal.append(make_commit_entry(1, 2)).unwrap();

    // Transaction 2
    wal.append(make_begin_entry(2)).unwrap();
    wal.append(make_insert_entry(2, 100, vec![3], vec![4], 3))
        .unwrap();
    wal.append(make_commit_entry(2, 4)).unwrap();

    // Transaction 3 (rolled back)
    wal.append(make_begin_entry(3)).unwrap();
    wal.append(make_insert_entry(3, 100, vec![5], vec![6], 5))
        .unwrap();
    wal.append(make_rollback_entry(3, 6)).unwrap();

    let entries = wal.recover().unwrap();
    // 3 entries for tx1 + 3 for tx2 + 3 for tx3 = 9
    assert_eq!(entries.len(), 9);
}
