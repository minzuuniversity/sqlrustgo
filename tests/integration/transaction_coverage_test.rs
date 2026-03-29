//! Transaction Module Coverage Tests
//!
//! These tests are designed to improve coverage for low-coverage modules:
//! - transaction/lock.rs (57/113 = 50.4%)
//! - transaction/recovery.rs (15/163 = 9.2%)
//! - transaction/participant.rs (65/98 = 66.3%)
//! - transaction/mvcc.rs (69/84 = 82.1%)
//! - transaction/deadlock.rs (29/32 = 90.6%)

use sqlrustgo_transaction::deadlock::DeadlockDetector;
use sqlrustgo_transaction::lock::{LockInfo, LockManager, LockMode, LockRequest};
use sqlrustgo_transaction::mvcc::{MvccSnapshot, TxId};
use std::collections::HashSet;

// ============================================================================
// Lock Manager Tests (increase coverage for transaction/lock.rs)
// ============================================================================

#[test]
fn test_lock_request_creation() {
    let request = LockRequest::new(1, vec![1, 2, 3], LockMode::Exclusive);
    assert_eq!(request.tx_id, 1);
    assert_eq!(request.key, vec![1, 2, 3]);
    assert_eq!(request.mode, LockMode::Exclusive);
    assert!(!request.granted);
}

#[test]
fn test_lock_info_creation() {
    let info = LockInfo::new(vec![1, 2, 3], LockMode::Shared);
    assert_eq!(info.key, vec![1, 2, 3]);
    assert_eq!(info.mode, LockMode::Shared);
    assert!(info.holders.is_empty());
    assert!(info.waiters.is_empty());
}

#[test]
fn test_lock_info_add_holder() {
    let mut info = LockInfo::new(vec![1], LockMode::Exclusive);
    info.add_holder(42);
    assert!(info.holders.contains(&42));
}

#[test]
fn test_lock_info_remove_holder() {
    let mut info = LockInfo::new(vec![1], LockMode::Exclusive);
    info.add_holder(42);
    info.remove_holder(42);
    assert!(!info.holders.contains(&42));
}

#[test]
fn test_lock_info_add_waiter() {
    let mut info = LockInfo::new(vec![1], LockMode::Shared);
    info.add_waiter(42, LockMode::Exclusive);
    assert_eq!(info.waiters.len(), 1);
    assert_eq!(info.waiters[0], (42, LockMode::Exclusive));
}

#[test]
fn test_lock_manager_shared_lock() {
    let manager = LockManager::new();
    let result = manager.acquire_lock(1, vec![1], LockMode::Shared);
    assert!(result.granted);
}

#[test]
fn test_lock_manager_exclusive_lock() {
    let manager = LockManager::new();
    let result = manager.acquire_lock(1, vec![1], LockMode::Exclusive);
    assert!(result.granted);
}

#[test]
fn test_lock_manager_multiple_shared_locks() {
    let manager = LockManager::new();
    let result1 = manager.acquire_lock(1, vec![1], LockMode::Shared);
    let result2 = manager.acquire_lock(2, vec![1], LockMode::Shared);
    assert!(result1.granted);
    assert!(result2.granted);
}

#[test]
fn test_lock_manager_conflicting_locks() {
    let manager = LockManager::new();
    let _result1 = manager.acquire_lock(1, vec![1], LockMode::Exclusive);
    let result2 = manager.acquire_lock(2, vec![1], LockMode::Exclusive);
    assert!(!result2.granted);
}

#[test]
fn test_lock_manager_release_lock() {
    let manager = LockManager::new();
    let _result = manager.acquire_lock(1, vec![1], LockMode::Exclusive);
    let released = manager.release_lock(1, vec![1]);
    assert!(released);
}

#[test]
fn test_lock_manager_release_nonexistent() {
    let manager = LockManager::new();
    let released = manager.release_lock(999, vec![1]);
    assert!(!released);
}

#[test]
fn test_lock_manager_upgrade_lock() {
    let manager = LockManager::new();
    let _result1 = manager.acquire_lock(1, vec![1], LockMode::Shared);
    let result2 = manager.upgrade_lock(1, vec![1]);
    assert!(result2.granted);
}

#[test]
fn test_lock_mode_eq() {
    assert_eq!(LockMode::Shared, LockMode::Shared);
    assert_eq!(LockMode::Exclusive, LockMode::Exclusive);
    assert_ne!(LockMode::Shared, LockMode::Exclusive);
}

// ============================================================================
// Deadlock Detector Tests (increase coverage for transaction/deadlock.rs)
// ============================================================================

#[test]
fn test_deadlock_detector_creation() {
    let detector = DeadlockDetector::new(5);
    assert_eq!(detector.max_depth(), 5);
}

#[test]
fn test_deadlock_detector_no_deadlock_simple() {
    let detector = DeadlockDetector::new(5);
    letwaits = HashSet::new();
    let result = detector.detect_deadlock(&waits);
    assert!(result.is_none());
}

// ============================================================================
// MVCC Tests (increase coverage for transaction/mvcc.rs)
// ============================================================================

#[test]
fn test_mvcc_snapshot_creation() {
    let snapshot = MvccSnapshot::new(10);
    assert_eq!(snapshot.tx_id(), 10);
    assert!(snapshot.is_committed(5));
    assert!(!snapshot.is_committed(15));
}

#[test]
fn test_mvcc_snapshot_commit_order() {
    let mut snapshot = MvccSnapshot::new(10);
    snapshot.add_commit(5, 1);
    snapshot.add_commit(6, 2);
    assert!(snapshot.is_committed(5));
    assert!(snapshot.is_committed(6));
    assert!(!snapshot.is_committed(7));
}

#[test]
fn test_tx_id_creation() {
    let tx_id = TxId::new(1, 100);
    assert_eq!(tx_id.session_id(), 1);
    assert_eq!(tx_id.local_tx_num(), 100);
}

#[test]
fn test_tx_id_ordering() {
    let tx_id1 = TxId::new(1, 100);
    let tx_id2 = TxId::new(1, 101);
    assert!(tx_id1 < tx_id2);
}
