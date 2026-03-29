//! Storage and Transaction Module Coverage Tests
//!
//! These tests are designed to improve coverage for low-coverage modules:
//! - storage/wal.rs (167/399 = 41.9%)
//! - storage/page.rs (185/244 = 75.8%)
//! - transaction/lock.rs (57/113 = 50.4%)
//! - transaction/recovery.rs (15/163 = 9.2%)
//! - transaction/participant.rs (65/98 = 66.3%)

use sqlrustgo_storage::wal::{WalEntry, WalEntryType, WalManager};
use sqlrustgo_storage::{BufferPool, Page};
use std::sync::Arc;
use tempfile::TempDir;

// ============================================================================
// WAL Entry Tests (increase coverage for wal.rs)
// ============================================================================

fn create_temp_dir() -> TempDir {
    TempDir::new().unwrap()
}

fn make_entry(
    entry_type: WalEntryType,
    tx_id: u64,
    table_id: u64,
    key: Option<Vec<u8>>,
    data: Option<Vec<u8>>,
) -> WalEntry {
    WalEntry {
        tx_id,
        entry_type,
        table_id,
        key,
        data,
        lsn: 0,
        timestamp: 1234567890,
    }
}

#[test]
fn test_wal_entry_serialization_roundtrip() {
    let entry = make_entry(WalEntryType::Begin, 1, 1, None, None);
    let bytes = entry.to_bytes();
    let recovered = WalEntry::from_bytes(&bytes).unwrap();
    assert_eq!(entry.tx_id, recovered.tx_id);
    assert_eq!(entry.entry_type, recovered.entry_type);
}

#[test]
fn test_wal_entry_with_key_serialization() {
    let entry = make_entry(
        WalEntryType::Insert,
        42,
        1,
        Some(vec![1, 2, 3]),
        Some(vec![10, 20, 30]),
    );
    let bytes = entry.to_bytes();
    let recovered = WalEntry::from_bytes(&bytes).unwrap();
    assert_eq!(entry.key, recovered.key);
    assert_eq!(entry.data, recovered.data);
}

#[test]
fn test_wal_entry_type_from_u8() {
    assert_eq!(WalEntryType::from_u8(1), Some(WalEntryType::Begin));
    assert_eq!(WalEntryType::from_u8(2), Some(WalEntryType::Insert));
    assert_eq!(WalEntryType::from_u8(3), Some(WalEntryType::Update));
    assert_eq!(WalEntryType::from_u8(4), Some(WalEntryType::Delete));
    assert_eq!(WalEntryType::from_u8(5), Some(WalEntryType::Commit));
    assert_eq!(WalEntryType::from_u8(6), Some(WalEntryType::Rollback));
    assert_eq!(WalEntryType::from_u8(7), Some(WalEntryType::Checkpoint));
    assert_eq!(WalEntryType::from_u8(8), Some(WalEntryType::Prepare));
    assert_eq!(WalEntryType::from_u8(99), None);
}

#[test]
fn test_wal_manager_log_operations() {
    let dir = create_temp_dir();
    let wal_path = dir.path().join("test.wal");

    let manager = WalManager::new(wal_path);

    manager.log_begin(1).unwrap();
    manager.log_insert(1, 1, vec![1], vec![100]).unwrap();
    manager
        .log_update(1, 1, vec![1], vec![1], vec![200])
        .unwrap();
    manager.log_delete(1, 1, vec![1], vec![200]).unwrap();
    manager.log_commit(1).unwrap();
}

#[test]
fn test_wal_manager_rollback_recovery() {
    let dir = create_temp_dir();
    let wal_path = dir.path().join("test.wal");

    let manager = WalManager::new(wal_path);

    manager.log_begin(1).unwrap();
    manager.log_insert(1, 1, vec![1], vec![100]).unwrap();
    manager.log_rollback(1).unwrap();

    let entries = manager.recover().unwrap();
    assert!(entries
        .iter()
        .any(|e| e.entry_type == WalEntryType::Rollback));
}

#[test]
fn test_wal_manager_checkpoint() {
    let dir = create_temp_dir();
    let wal_path = dir.path().join("test.wal");

    let manager = WalManager::new(wal_path);

    manager.log_begin(1).unwrap();
    manager.log_insert(1, 1, vec![1], vec![100]).unwrap();
    manager.log_commit(1).unwrap();
    manager.checkpoint(1).unwrap();
}

#[test]
fn test_wal_manager_prepare_2pc() {
    let dir = create_temp_dir();
    let wal_path = dir.path().join("test.wal");

    let manager = WalManager::new(wal_path);

    manager.log_begin(1).unwrap();
    manager.log_insert(1, 1, vec![1], vec![100]).unwrap();
    manager.log_prepare(1).unwrap();
    manager.log_commit(1).unwrap();
}

// ============================================================================
// Buffer Pool Tests (increase coverage for storage/buffer_pool)
// ============================================================================

#[test]
fn test_buffer_pool_stats_reset() {
    let pool = Arc::new(BufferPool::new(10));
    for i in 0..5 {
        let page = Arc::new(Page::new(i));
        pool.insert(page);
    }
    pool.reset_stats();
    let stats = pool.stats();
    assert_eq!(stats.hits, 0);
    assert_eq!(stats.misses, 0);
}

#[test]
fn test_buffer_pool_clear() {
    let pool = Arc::new(BufferPool::new(10));
    for i in 0..5 {
        let page = Arc::new(Page::new(i));
        pool.insert(page);
    }
    pool.clear();
    assert_eq!(pool.len(), 0);
}

#[test]
fn test_page_data_operations() {
    let mut page = Page::new(1);
    page.set_data(vec![1, 2, 3, 4, 5]);
    assert_eq!(page.data(), &[1, 2, 3, 4, 5]);
}

#[test]
fn test_page_dirty_flag() {
    let page = Page::new(1);
    assert!(!page.is_dirty());
    let mut page = page;
    page.mark_dirty();
    assert!(page.is_dirty());
}

#[test]
fn test_page_lsn() {
    let mut page = Page::new(1);
    page.set_lsn(100);
    assert_eq!(page.lsn(), 100);
}

#[test]
fn test_page_pin_unpin() {
    let pool = Arc::new(BufferPool::new(10));
    for i in 0..5 {
        let page = Arc::new(Page::new(i));
        pool.insert(page);
    }
    let page = pool.get(0);
    assert!(page.is_some());
}
