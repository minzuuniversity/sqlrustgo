//! F-16: Gap Locking (phantom read prevention)
//!
//! **Issue**: #2822
//! **Source**: v3.0.0 COMPLETE_LEGACY_TRACKING_REPORT.md (F-16)
//! **Change**: openspec/changes/f-16-gap-locking
//!
//! In-memory gap lock infrastructure. Real storage integration in v3.9.0.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IsolationLevel {
    ReadUncommitted,
    ReadCommitted,
    RepeatableRead,
    Serializable,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GapRange {
    pub start: i64, // exclusive
    pub end: i64,   // exclusive
}

impl GapRange {
    pub fn contains(&self, val: i64) -> bool {
        val > self.start && val < self.end
    }
}

#[derive(Debug, Clone)]
pub struct GapLock {
    pub tx_id: u64,
    pub table: String,
    pub range: GapRange,
}

pub struct GapLockManager {
    locks: Arc<Mutex<HashMap<String, Vec<GapLock>>>>,
    isolation: Arc<Mutex<HashMap<u64, IsolationLevel>>>,
    block_count: Arc<Mutex<u64>>,
}

impl GapLockManager {
    pub fn new() -> Self {
        Self {
            locks: Arc::new(Mutex::new(HashMap::new())),
            isolation: Arc::new(Mutex::new(HashMap::new())),
            block_count: Arc::new(Mutex::new(0)),
        }
    }

    pub fn set_isolation(&self, tx_id: u64, level: IsolationLevel) {
        self.isolation.lock().unwrap().insert(tx_id, level);
    }

    pub fn acquire_gap(&self, tx_id: u64, table: &str, range: GapRange) -> Result<(), String> {
        // Check isolation level - skip gap locks in READ COMMITTED
        let level = self
            .isolation
            .lock()
            .unwrap()
            .get(&tx_id)
            .copied()
            .unwrap_or(IsolationLevel::ReadCommitted);
        if matches!(
            level,
            IsolationLevel::ReadCommitted | IsolationLevel::ReadUncommitted
        ) {
            return Ok(()); // no gap lock
        }

        // Check for conflicts
        let locks = self.locks.lock().unwrap();
        if let Some(table_locks) = locks.get(table) {
            for lock in table_locks {
                if lock.tx_id != tx_id && ranges_overlap(&lock.range, &range) {
                    *self.block_count.lock().unwrap() += 1;
                    return Err(format!(
                        "Gap lock conflict: tx {} holds {}-{}, tx {} wants {}-{}",
                        lock.tx_id, lock.range.start, lock.range.end, tx_id, range.start, range.end
                    ));
                }
            }
        }
        drop(locks);

        self.locks
            .lock()
            .unwrap()
            .entry(table.to_string())
            .or_default()
            .push(GapLock {
                tx_id,
                table: table.to_string(),
                range,
            });
        Ok(())
    }

    pub fn release_all(&self, tx_id: u64) -> usize {
        let mut locks = self.locks.lock().unwrap();
        let mut released = 0;
        for vec in locks.values_mut() {
            let before = vec.len();
            vec.retain(|l| l.tx_id != tx_id);
            released += before - vec.len();
        }
        self.isolation.lock().unwrap().remove(&tx_id);
        released
    }

    pub fn can_insert(&self, table: &str, value: i64, other_tx: u64) -> Result<(), String> {
        let locks = self.locks.lock().unwrap();
        if let Some(table_locks) = locks.get(table) {
            for lock in table_locks {
                if lock.tx_id != other_tx && lock.range.contains(value) {
                    return Err(format!(
                        "Insert {} blocked by gap lock from tx {} ({}-{})",
                        value, lock.tx_id, lock.range.start, lock.range.end
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn lock_count(&self, table: &str) -> usize {
        self.locks
            .lock()
            .unwrap()
            .get(table)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    pub fn total_blocks(&self) -> u64 {
        *self.block_count.lock().unwrap()
    }
}

impl Default for GapLockManager {
    fn default() -> Self {
        Self::new()
    }
}

fn ranges_overlap(a: &GapRange, b: &GapRange) -> bool {
    a.start < b.end && b.start < a.end
}

#[test]
fn test_gap_lock_acquire() {
    let mgr = GapLockManager::new();
    mgr.set_isolation(1, IsolationLevel::RepeatableRead);
    mgr.acquire_gap(1, "users", GapRange { start: 5, end: 10 })
        .unwrap();
    assert_eq!(mgr.lock_count("users"), 1);
}

#[test]
fn test_phantom_prevention() {
    let mgr = GapLockManager::new();
    mgr.set_isolation(1, IsolationLevel::RepeatableRead);
    // T1 acquires gap lock on (5, 10)
    mgr.acquire_gap(1, "users", GapRange { start: 5, end: 10 })
        .unwrap();
    // T2 attempts to insert id=7 (within gap)
    let result = mgr.can_insert("users", 7, 2);
    assert!(result.is_err(), "insert should be blocked by gap lock");
}

#[test]
fn test_gap_lock_release_on_commit() {
    let mgr = GapLockManager::new();
    mgr.set_isolation(1, IsolationLevel::RepeatableRead);
    mgr.acquire_gap(1, "users", GapRange { start: 5, end: 10 })
        .unwrap();
    assert_eq!(mgr.lock_count("users"), 1);
    let released = mgr.release_all(1);
    assert_eq!(released, 1);
    assert_eq!(mgr.lock_count("users"), 0);
    // After release, T2 can insert
    assert!(mgr.can_insert("users", 7, 2).is_ok());
}

#[test]
fn test_no_gap_lock_in_read_committed() {
    let mgr = GapLockManager::new();
    mgr.set_isolation(1, IsolationLevel::ReadCommitted);
    // Even though we try to acquire gap lock, RC skips it
    mgr.acquire_gap(1, "users", GapRange { start: 5, end: 10 })
        .unwrap();
    assert_eq!(mgr.lock_count("users"), 0); // no lock acquired
                                            // Concurrent insert proceeds immediately
    assert!(mgr.can_insert("users", 7, 2).is_ok());
}

#[test]
fn test_overlapping_gap_locks_conflict() {
    let mgr = GapLockManager::new();
    mgr.set_isolation(1, IsolationLevel::RepeatableRead);
    mgr.set_isolation(2, IsolationLevel::RepeatableRead);
    mgr.acquire_gap(1, "users", GapRange { start: 0, end: 10 })
        .unwrap();
    // T2's overlapping range should fail
    let result = mgr.acquire_gap(2, "users", GapRange { start: 5, end: 15 });
    assert!(result.is_err());
}

#[test]
fn test_non_overlapping_gap_locks_succeed() {
    let mgr = GapLockManager::new();
    mgr.set_isolation(1, IsolationLevel::RepeatableRead);
    mgr.set_isolation(2, IsolationLevel::RepeatableRead);
    mgr.acquire_gap(1, "users", GapRange { start: 0, end: 10 })
        .unwrap();
    let result = mgr.acquire_gap(2, "users", GapRange { start: 20, end: 30 });
    assert!(result.is_ok(), "non-overlapping should succeed");
}

#[test]
fn test_gap_range_contains() {
    let r = GapRange { start: 5, end: 10 };
    assert!(!r.contains(5)); // exclusive
    assert!(!r.contains(10)); // exclusive
    assert!(r.contains(6));
    assert!(r.contains(9));
    assert!(!r.contains(4));
    assert!(!r.contains(11));
}
