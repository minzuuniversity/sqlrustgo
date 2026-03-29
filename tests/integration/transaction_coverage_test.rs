//! Transaction Module Coverage Tests
//!
//! These tests are designed to improve coverage for low-coverage modules:
//! - transaction/lock.rs (57/113 = 50.4%)
//! - transaction/deadlock.rs (29/32 = 90.6%)

use sqlrustgo_transaction::deadlock::DeadlockDetector;
use sqlrustgo_transaction::lock::{LockInfo, LockManager, LockMode, LockRequest};
use sqlrustgo_transaction::mvcc::TxId;

// ============================================================================
// Lock Manager Tests
// ============================================================================

#[test]
fn test_lock_request_creation() {
    let request = LockRequest::new(TxId::new(1), vec![1, 2, 3], LockMode::Exclusive);
    assert_eq!(request.tx_id.as_u64(), 1);
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
    info.add_holder(TxId::new(42));
    assert!(info.holders.contains(&TxId::new(42)));
}

#[test]
fn test_lock_info_remove_holder() {
    let mut info = LockInfo::new(vec![1], LockMode::Exclusive);
    info.add_holder(TxId::new(42));
    info.remove_holder(TxId::new(42));
    assert!(!info.holders.contains(&TxId::new(42)));
}

#[test]
fn test_lock_info_add_waiter() {
    let mut info = LockInfo::new(vec![1], LockMode::Shared);
    info.add_waiter(TxId::new(42), LockMode::Exclusive);
    assert_eq!(info.waiters.len(), 1);
    assert_eq!(info.waiters[0], (TxId::new(42), LockMode::Exclusive));
}

#[test]
fn test_lock_manager_shared_lock() {
    let mut manager = LockManager::new();
    let result = manager.acquire_lock(TxId::new(1), vec![1], LockMode::Shared);
    assert!(result.is_ok());
}

#[test]
fn test_lock_manager_exclusive_lock() {
    let mut manager = LockManager::new();
    let result = manager.acquire_lock(TxId::new(1), vec![1], LockMode::Exclusive);
    assert!(result.is_ok());
}

#[test]
fn test_lock_manager_multiple_shared_locks() {
    let mut manager = LockManager::new();
    let result1 = manager.acquire_lock(TxId::new(1), vec![1], LockMode::Shared);
    let result2 = manager.acquire_lock(TxId::new(2), vec![1], LockMode::Shared);
    assert!(result1.is_ok());
    assert!(result2.is_ok());
}

#[test]
fn test_lock_manager_conflicting_locks() {
    let mut manager = LockManager::new();
    let _result1 = manager.acquire_lock(TxId::new(1), vec![1], LockMode::Exclusive);
    let result2 = manager.acquire_lock(TxId::new(2), vec![1], LockMode::Exclusive);
    assert!(result2.is_ok());
}

#[test]
fn test_lock_manager_release_lock() {
    let mut manager = LockManager::new();
    let _result = manager.acquire_lock(TxId::new(1), vec![1], LockMode::Exclusive);
    let released = manager.release_lock(TxId::new(1), &vec![1]);
    assert!(released.is_ok());
}

#[test]
fn test_lock_manager_release_nonexistent() {
    let mut manager = LockManager::new();
    let released = manager.release_lock(TxId::new(999), &vec![1]);
    assert!(released.is_ok());
}

#[test]
fn test_lock_mode_eq() {
    assert_eq!(LockMode::Shared, LockMode::Shared);
    assert_eq!(LockMode::Exclusive, LockMode::Exclusive);
    assert_ne!(LockMode::Shared, LockMode::Exclusive);
}

// ============================================================================
// Deadlock Detector Tests
// ============================================================================

#[test]
fn test_deadlock_detector_timeout() {
    let detector = DeadlockDetector::new();
    assert_eq!(detector.get_timeout().as_secs(), 5);
}
