//! Gap Lock Manager for MVCC-based database systems
//!
//! Provides gap locking for REPEATABLE-READ isolation level to prevent
//! phantom reads during index scans.
//!
//! ## Design
//!
//! - **Gap Lock**: Locks the space between index entries, not the entries themselves
//! - **Insert Intention Gap Lock**: Allows concurrent inserts into different gap regions
//! - **Compatibility Matrix**: Gap locks conflict with write operations but not
//!   with other gap locks in non-overlapping ranges

use parking_lot::Mutex;
use std::collections::HashMap;

/// Isolation level for gap locking decisions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolationLevel {
    /// Read committed - no gap locks needed
    ReadCommitted,
    /// Read committed with write intent detection
    ReadCommittedWriteIntent,
    /// Read uncommitted - no gap locks
    ReadUncommitted,
    /// Repeatable read with gap locks (default)
    RepeatableRead,
    /// Serializable - full gap locks
    Serializable,
}

/// Represents a gap lock on a range of values
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GapLock {
    /// Transaction ID that holds the lock
    pub tx_id: u64,
    /// Table/index name
    pub table: String,
    /// Start of the gap range (inclusive)
    pub range_start: Option<String>,
    /// End of the gap range (exclusive)
    pub range_end: Option<String>,
    /// Lock type
    pub lock_type: GapLockType,
    /// Whether this is an insert intention lock
    pub is_insert_intention: bool,
}

impl GapLock {
    /// Create a new gap lock
    pub fn new(
        tx_id: u64,
        table: String,
        range_start: Option<String>,
        range_end: Option<String>,
        lock_type: GapLockType,
        is_insert_intention: bool,
    ) -> Self {
        Self {
            tx_id,
            table,
            range_start,
            range_end,
            lock_type,
            is_insert_intention,
        }
    }
}

/// Type of gap lock
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GapLockType {
    /// Shared gap lock (for reads)
    Shared,
    /// Exclusive gap lock (for writes)
    Exclusive,
    /// Insert intention gap lock
    InsertIntention,
    /// Gap lock on a specific index record (locks the gap around a key)
    Record,
}

/// Gap lock compatibility matrix
/// (holder, requester) -> true if compatible
fn is_gap_lock_compatible(holder: &GapLockType, requester: &GapLockType) -> bool {
    match (holder, requester) {
        // Shared and shared are compatible
        (GapLockType::Shared, GapLockType::Shared) => true,
        // Anything with InsertIntention conflicts except when holder is also InsertIntention
        // and they don't overlap
        (GapLockType::InsertIntention, GapLockType::InsertIntention) => true,
        // InsertIntention conflicts with other types
        (_, GapLockType::InsertIntention) => false,
        (GapLockType::InsertIntention, _) => false,
        // All other combinations conflict
        _ => false,
    }
}

/// Check if two gap ranges overlap
fn ranges_overlap(
    start1: &Option<String>,
    end1: &Option<String>,
    start2: &Option<String>,
    end2: &Option<String>,
) -> bool {
    // If either range is unbounded, they overlap if the bounded range is within the unbounded
    let start1 = start1.as_ref();
    let end1 = end1.as_ref();
    let start2 = start2.as_ref();
    let end2 = end2.as_ref();

    // Check if ranges don't overlap
    if let (Some(s1), Some(e2)) = (start1, end2) {
        if s1 >= e2 {
            return false; // s1 >= e2 means no overlap
        }
    }
    if let (Some(s2), Some(e1)) = (start2, end1) {
        if s2 >= e1 {
            return false; // s2 >= e1 means no overlap
        }
    }

    true
}

/// Gap Lock Manager
///
/// Manages gap locks for all tables in a storage engine.
/// Thread-safe using parking_lot Mutex.
pub struct GapLockManager {
    /// Map of table -> gap locks held
    locks: Mutex<HashMap<String, Vec<GapLock>>>,
    /// Map of transaction ID -> isolation level
    isolation: Mutex<HashMap<u64, IsolationLevel>>,
    /// Counter for blocked transactions
    block_count: Mutex<u64>,
}

impl Default for GapLockManager {
    fn default() -> Self {
        Self::new()
    }
}

impl GapLockManager {
    /// Create a new GapLockManager
    pub fn new() -> Self {
        Self {
            locks: Mutex::new(HashMap::new()),
            isolation: Mutex::new(HashMap::new()),
            block_count: Mutex::new(0),
        }
    }

    /// Set the isolation level for a transaction
    pub fn set_isolation(&self, tx_id: u64, level: IsolationLevel) {
        let mut isolation = self.isolation.lock();
        isolation.insert(tx_id, level);
    }

    /// Get the isolation level for a transaction
    pub fn get_isolation(&self, tx_id: u64) -> IsolationLevel {
        let isolation = self.isolation.lock();
        isolation.get(&tx_id).copied().unwrap_or(IsolationLevel::RepeatableRead)
    }

    /// Acquire a gap lock for a transaction
    ///
    /// Returns `true` if the lock was acquired, `false` if it conflicts
    /// with an existing lock held by another transaction.
    pub fn acquire_gap(
        &self,
        tx_id: u64,
        table: &str,
        range_start: Option<String>,
        range_end: Option<String>,
        lock_type: GapLockType,
        is_insert_intention: bool,
    ) -> bool {
        let mut locks = self.locks.lock();

        // Check if this transaction already holds a conflicting lock
        let table_locks = locks.get(table);

        if let Some(table_locks) = table_locks {
            for existing in table_locks {
                // Skip locks held by the same transaction
                if existing.tx_id == tx_id {
                    continue;
                }

                // Check if ranges overlap and locks conflict
                if ranges_overlap(
                    &range_start,
                    &range_end,
                    &existing.range_start,
                    &existing.range_end,
                ) && !is_gap_lock_compatible(&existing.lock_type, &lock_type)
                {
                    // Conflict - increment block count and return false
                    let mut block_count = self.block_count.lock();
                    *block_count += 1;
                    return false;
                }
            }
        }

        // No conflict - acquire the lock
        let table_locks = locks.entry(table.to_string()).or_insert_with(Vec::new);

        let new_lock = GapLock::new(
            tx_id,
            table.to_string(),
            range_start,
            range_end,
            lock_type,
            is_insert_intention,
        );

        table_locks.push(new_lock);
        true
    }

    /// Release all gap locks held by a transaction
    pub fn release_all(&self, tx_id: u64) {
        let mut locks = self.locks.lock();
        for table_locks in locks.values_mut() {
            table_locks.retain(|lock| lock.tx_id != tx_id);
        }
    }

    /// Release all gap locks on a specific table held by a transaction
    pub fn release_table(&self, tx_id: u64, table: &str) {
        let mut locks = self.locks.lock();
        if let Some(table_locks) = locks.get_mut(table) {
            table_locks.retain(|lock| lock.tx_id != tx_id);
        }
    }

    /// Check if a value can be inserted (no conflicting gap locks)
    ///
    /// For insert intention locks, we check if any existing gap lock
    /// would conflict with the insert.
    pub fn can_insert(&self, tx_id: u64, table: &str, value: &str) -> bool {
        let isolation = self.isolation.lock();
        let level = isolation.get(&tx_id).copied().unwrap_or(IsolationLevel::RepeatableRead);
        drop(isolation);

        // Only check gap locks for REPEATABLE_READ and above
        match level {
            IsolationLevel::ReadUncommitted | IsolationLevel::ReadCommitted => {
                return true;
            }
            _ => {}
        }

        let locks = self.locks.lock();
        let table_locks = locks.get(table);

        if let Some(table_locks) = table_locks {
            for existing in table_locks {
                // Skip locks held by the same transaction
                if existing.tx_id == tx_id {
                    continue;
                }

                // Check if the value falls within the locked range
                // For gap locks, we check if value is in [range_start, range_end)
                let in_range = match (&existing.range_start, &existing.range_end) {
                    (Some(start), Some(end)) => value >= start && value < end,
                    (Some(start), None) => value >= start,
                    (None, Some(end)) => value < end,
                    (None, None) => true, // Lock on entire index
                };

                if in_range {
                    // For insert intention, check compatibility
                    if existing.is_insert_intention {
                        // Insert intentions don't conflict with each other
                        continue;
                    }
                    // Gap lock conflicts with insert
                    return false;
                }
            }
        }

        true
    }

    /// Get the number of times a transaction was blocked waiting for a lock
    pub fn get_block_count(&self) -> u64 {
        let block_count = self.block_count.lock();
        *block_count
    }

    /// Get all gap locks held by a transaction on a table
    #[allow(dead_code)]
    pub fn get_locks(&self, tx_id: u64, table: &str) -> Vec<GapLock> {
        let locks = self.locks.lock();
        locks
            .get(table)
            .map(|table_locks| {
                table_locks
                    .iter()
                    .filter(|lock| lock.tx_id == tx_id)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Check if a transaction holds any gap locks on a table
    #[allow(dead_code)]
    pub fn has_locks(&self, tx_id: u64, table: &str) -> bool {
        let locks = self.locks.lock();
        locks
            .get(table)
            .map(|table_locks| table_locks.iter().any(|lock| lock.tx_id == tx_id))
            .unwrap_or(false)
    }

    /// Clear all locks (for testing or recovery)
    pub fn clear(&self) {
        let mut locks = self.locks.lock();
        locks.clear();
        let mut block_count = self.block_count.lock();
        *block_count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gap_lock_basic() {
        let manager = GapLockManager::new();

        // Tx1 acquires gap lock on range [10, 20)
        assert!(manager.acquire_gap(
            1,
            "t1",
            Some("10".to_string()),
            Some("20".to_string()),
            GapLockType::Shared,
            false,
        ));

        // Tx2 can also acquire shared gap lock on overlapping range
        // (Shared locks don't conflict with each other - only InsertIntention/Exclusive do)
        assert!(manager.acquire_gap(
            2,
            "t1",
            Some("15".to_string()),
            Some("25".to_string()),
            GapLockType::Shared,
            false,
        ));

        // Tx2 can acquire non-overlapping gap lock
        assert!(manager.acquire_gap(
            2,
            "t1",
            Some("20".to_string()),
            Some("30".to_string()),
            GapLockType::Shared,
            false,
        ));

        // But InsertIntention conflicts with existing Shared lock
        assert!(!manager.acquire_gap(
            3,
            "t1",
            Some("15".to_string()),
            Some("25".to_string()),
            GapLockType::InsertIntention,
            true,
        ));

        // Release all locks for tx1
        manager.release_all(1);

        // Tx2's lock should still be held
        assert!(manager.has_locks(2, "t1"));
        assert!(!manager.has_locks(1, "t1"));
    }

    #[test]
    fn test_insert_intention_locks() {
        let manager = GapLockManager::new();

        // Tx1 acquires insert intention lock on [10, 20)
        assert!(manager.acquire_gap(
            1,
            "t1",
            Some("10".to_string()),
            Some("20".to_string()),
            GapLockType::InsertIntention,
            true,
        ));

        // Tx2 can also acquire insert intention lock on overlapping range
        // (insert intentions don't conflict with each other)
        assert!(manager.acquire_gap(
            2,
            "t1",
            Some("12".to_string()),
            Some("18".to_string()),
            GapLockType::InsertIntention,
            true,
        ));

        // But shared gap lock conflicts with insert intention
        assert!(!manager.acquire_gap(
            3,
            "t1",
            Some("10".to_string()),
            Some("20".to_string()),
            GapLockType::Shared,
            false,
        ));
    }

    #[test]
    fn test_can_insert() {
        let manager = GapLockManager::new();

        // Set REPEATABLE_READ isolation
        manager.set_isolation(1, IsolationLevel::RepeatableRead);

        // Tx1 acquires gap lock on [10, 20)
        assert!(manager.acquire_gap(
            1,
            "t1",
            Some("10".to_string()),
            Some("20".to_string()),
            GapLockType::Shared,
            false,
        ));

        // Tx2 with READ_COMMITTED can still insert (no gap lock checking)
        manager.set_isolation(2, IsolationLevel::ReadCommitted);
        assert!(manager.can_insert(2, "t1", "15"));

        // Tx2 with REPEATABLE_READ cannot insert in locked range
        manager.set_isolation(2, IsolationLevel::RepeatableRead);
        assert!(!manager.can_insert(2, "t1", "15"));

        // But can insert outside the range
        assert!(manager.can_insert(2, "t1", "25"));
    }

    #[test]
    fn test_different_tables() {
        let manager = GapLockManager::new();

        // Tx1 locks gap on table A
        assert!(manager.acquire_gap(
            1,
            "table_a",
            Some("10".to_string()),
            Some("20".to_string()),
            GapLockType::Shared,
            false,
        ));

        // Tx2 can lock same range on table B (different table)
        assert!(manager.acquire_gap(
            2,
            "table_b",
            Some("10".to_string()),
            Some("20".to_string()),
            GapLockType::Shared,
            false,
        ));

        // Release and verify
        manager.release_table(1, "table_a");
        assert!(!manager.has_locks(1, "table_a"));
        assert!(manager.has_locks(2, "table_b"));
    }

    #[test]
    fn test_block_count() {
        let manager = GapLockManager::new();

        // Initial block count is 0
        assert_eq!(manager.get_block_count(), 0);

        // Acquire an exclusive gap lock
        assert!(manager.acquire_gap(
            1,
            "t1",
            Some("10".to_string()),
            Some("20".to_string()),
            GapLockType::Exclusive,
            false,
        ));

        // Conflicting exclusive lock is blocked
        assert!(!manager.acquire_gap(
            2,
            "t1",
            Some("15".to_string()),
            Some("25".to_string()),
            GapLockType::Exclusive,
            false,
        ));

        assert_eq!(manager.get_block_count(), 1);

        // Another blocked exclusive lock
        assert!(!manager.acquire_gap(
            3,
            "t1",
            Some("12".to_string()),
            Some("18".to_string()),
            GapLockType::Exclusive,
            false,
        ));

        assert_eq!(manager.get_block_count(), 2);
    }
}
