//! F-25: Change Buffer — deferred secondary index updates
//!
//! InnoDB-style change buffer: secondary index updates are deferred
//! (not written to disk immediately) and merged on read.
//!
//! This reduces random I/O for high-write workloads where secondary
//! indexes are not immediately needed.
//!
//! **V311-03**: Main-path integration from ISOLATED test.
//! **Issue**: #3491

use parking_lot::Mutex;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

/// Operation types for deferred secondary index updates
#[derive(Debug, Clone, PartialEq)]
pub enum ChangeOp {
    Insert { key: Vec<u8>, value: Vec<u8> },
    Delete { key: Vec<u8> },
    Update { key: Vec<u8>, new_value: Vec<u8> },
}

/// A deferred change entry targeting a specific page
#[derive(Debug, Clone)]
pub struct ChangeEntry {
    pub page_id: u64,
    pub op: ChangeOp,
}

/// InnoDB-style Change Buffer for deferred secondary index updates
///
/// # Behavior
/// - `defer_update()` — queue a secondary index change for later application
/// - `merge_on_read()` — apply all deferred changes for a page when it's read
/// - `flush()` — force-apply all deferred changes immediately
/// - `should_flush()` — true when internal capacity threshold is reached
///
/// # Integration
/// Integrated into `BufferPool::read_page()` — when a page is read,
/// any pending changes for that page are merged before returning the page.
pub struct ChangeBuffer {
    pending: Arc<Mutex<VecDeque<ChangeEntry>>>,
    by_page: Arc<Mutex<HashMap<u64, Vec<ChangeEntry>>>>,
    max_entries: usize,
    flushes: Arc<Mutex<u64>>,
    merges_on_read: Arc<Mutex<u64>>,
}

impl ChangeBuffer {
    /// Create a new ChangeBuffer with default capacity (100 entries)
    pub fn new() -> Self {
        Self::with_capacity(100)
    }

    /// Create a new ChangeBuffer with specified capacity
    pub fn with_capacity(max_entries: usize) -> Self {
        Self {
            pending: Arc::new(Mutex::new(VecDeque::new())),
            by_page: Arc::new(Mutex::new(HashMap::new())),
            max_entries,
            flushes: Arc::new(Mutex::new(0)),
            merges_on_read: Arc::new(Mutex::new(0)),
        }
    }

    /// Defer a secondary index update for a page
    ///
    /// The update is queued in memory and will be applied either:
    /// - On read: when `merge_on_read(page_id)` is called
    /// - On flush: when `flush()` is called
    pub fn defer_update(&self, page_id: u64, op: ChangeOp) {
        let entry = ChangeEntry { page_id, op };

        // Add to pending queue
        self.pending.lock().push_back(entry.clone());

        // Add to by_page index
        self.by_page.lock().entry(page_id).or_default().push(entry);
    }

    /// Number of pending change entries
    pub fn pending_count(&self) -> usize {
        self.pending.lock().len()
    }

    /// Returns true when the change buffer has reached its capacity threshold
    pub fn should_flush(&self) -> bool {
        self.pending_count() >= self.max_entries
    }

    /// Flush all pending changes (force-apply)
    ///
    /// Returns all flushed `ChangeEntry` values and clears internal state.
    /// Callers are responsible for applying the entries to the storage layer.
    pub fn flush(&self) -> Vec<ChangeEntry> {
        let mut pending = self.pending.lock();
        let drained: Vec<ChangeEntry> = pending.drain(..).collect();
        self.by_page.lock().clear();
        *self.flushes.lock() += 1;
        drained
    }

    /// Merge all deferred changes for a page on read
    ///
    /// Returns all `ChangeEntry` values for the given `page_id` and removes
    /// them from internal state. Callers should apply these entries to the
    /// page before returning it to the caller.
    ///
    /// This is called from `BufferPool::read_page()` when a page is accessed.
    pub fn merge_on_read(&self, page_id: u64) -> Vec<ChangeEntry> {
        let mut by_page = self.by_page.lock();
        let entries = by_page.remove(&page_id).unwrap_or_default();
        if !entries.is_empty() {
            *self.merges_on_read.lock() += 1;
        }
        // Also remove from pending (those pointing to this page)
        let mut pending = self.pending.lock();
        pending.retain(|e| e.page_id != page_id);
        entries
    }

    /// Number of times flush() has been called
    pub fn flush_count(&self) -> u64 {
        *self.flushes.lock()
    }

    /// Number of times merge_on_read() successfully merged entries
    pub fn merge_count(&self) -> u64 {
        *self.merges_on_read.lock()
    }
}

impl Default for ChangeBuffer {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defer_secondary_index_update() {
        let cb = ChangeBuffer::new();
        cb.defer_update(
            1,
            ChangeOp::Insert {
                key: b"k1".to_vec(),
                value: b"v1".to_vec(),
            },
        );
        assert_eq!(cb.pending_count(), 1);
    }

    #[test]
    fn test_batch_flush_at_threshold() {
        let cb = ChangeBuffer::with_capacity(5);
        for i in 0..5 {
            cb.defer_update(
                i,
                ChangeOp::Delete {
                    key: format!("k{}", i).into_bytes(),
                },
            );
        }
        assert!(cb.should_flush());
        let flushed = cb.flush();
        assert_eq!(flushed.len(), 5);
        assert_eq!(cb.pending_count(), 0);
        assert_eq!(cb.flush_count(), 1);
    }

    #[test]
    fn test_merge_on_page_read() {
        let cb = ChangeBuffer::new();
        cb.defer_update(
            1,
            ChangeOp::Insert {
                key: b"a".to_vec(),
                value: b"1".to_vec(),
            },
        );
        cb.defer_update(
            1,
            ChangeOp::Update {
                key: b"a".to_vec(),
                new_value: b"2".to_vec(),
            },
        );
        cb.defer_update(2, ChangeOp::Delete { key: b"b".to_vec() });

        let merged = cb.merge_on_read(1);
        assert_eq!(merged.len(), 2);
        assert_eq!(cb.pending_count(), 1); // page 2 still pending
        assert_eq!(cb.merge_count(), 1);
    }

    #[test]
    fn test_eviction_on_flush() {
        let cb = ChangeBuffer::new();
        cb.defer_update(
            1,
            ChangeOp::Insert {
                key: b"a".to_vec(),
                value: b"1".to_vec(),
            },
        );
        cb.defer_update(2, ChangeOp::Delete { key: b"b".to_vec() });
        let _ = cb.flush();
        // After flush, by_page should be empty
        assert!(cb.merge_on_read(1).is_empty());
        assert!(cb.merge_on_read(2).is_empty());
    }

    #[test]
    fn test_change_buffer_capacity_limit() {
        let cb = ChangeBuffer::with_capacity(3);
        for i in 0..3 {
            cb.defer_update(
                i,
                ChangeOp::Insert {
                    key: vec![i as u8],
                    value: vec![i as u8 * 2],
                },
            );
        }
        assert!(cb.should_flush());
        // Add one more; should still be flushable
        cb.defer_update(3, ChangeOp::Delete { key: vec![3] });
        let flushed = cb.flush();
        assert!(flushed.len() >= 3);
    }
}
