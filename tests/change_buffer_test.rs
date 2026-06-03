//! F-25: Change Buffer (deferred secondary index updates)
//!
//! **Issue**: #2826
//! **Source**: v3.0.0 COMPLETE_LEGACY_TRACKING_REPORT.md (F-25)
//! **Change**: openspec/changes/f-25-f-26-change-buffer-doublewrite
//!
//! In-memory mock of InnoDB-style Change Buffer. Real storage integration
//! in v3.9.0.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq)]
pub enum ChangeOp {
    Insert { key: Vec<u8>, value: Vec<u8> },
    Delete { key: Vec<u8> },
    Update { key: Vec<u8>, new_value: Vec<u8> },
}

#[derive(Debug, Clone)]
pub struct ChangeEntry {
    pub page_id: u64,
    pub op: ChangeOp,
}

pub struct ChangeBuffer {
    pending: Arc<Mutex<VecDeque<ChangeEntry>>>,
    by_page: Arc<Mutex<HashMap<u64, Vec<ChangeEntry>>>>,
    max_entries: usize,
    flushes: Arc<Mutex<u64>>,
    merges_on_read: Arc<Mutex<u64>>,
}

impl ChangeBuffer {
    pub fn new() -> Self {
        Self::with_capacity(100)
    }

    pub fn with_capacity(max_entries: usize) -> Self {
        Self {
            pending: Arc::new(Mutex::new(VecDeque::new())),
            by_page: Arc::new(Mutex::new(HashMap::new())),
            max_entries,
            flushes: Arc::new(Mutex::new(0)),
            merges_on_read: Arc::new(Mutex::new(0)),
        }
    }

    pub fn defer_update(&self, page_id: u64, op: ChangeOp) {
        let entry = ChangeEntry { page_id, op };
        self.pending.lock().unwrap().push_back(entry.clone());
        self.by_page
            .lock()
            .unwrap()
            .entry(page_id)
            .or_default()
            .push(entry);
    }

    pub fn pending_count(&self) -> usize {
        self.pending.lock().unwrap().len()
    }

    pub fn should_flush(&self) -> bool {
        self.pending_count() >= self.max_entries
    }

    pub fn flush(&self) -> Vec<ChangeEntry> {
        let mut pending = self.pending.lock().unwrap();
        let drained: Vec<ChangeEntry> = pending.drain(..).collect();
        self.by_page.lock().unwrap().clear();
        *self.flushes.lock().unwrap() += 1;
        drained
    }

    pub fn merge_on_read(&self, page_id: u64) -> Vec<ChangeEntry> {
        let mut by_page = self.by_page.lock().unwrap();
        let entries = by_page.remove(&page_id).unwrap_or_default();
        if !entries.is_empty() {
            *self.merges_on_read.lock().unwrap() += 1;
        }
        // Also remove from pending (those pointing to this page)
        let mut pending = self.pending.lock().unwrap();
        pending.retain(|e| e.page_id != page_id);
        entries
    }

    pub fn flush_count(&self) -> u64 {
        *self.flushes.lock().unwrap()
    }

    pub fn merge_count(&self) -> u64 {
        *self.merges_on_read.lock().unwrap()
    }
}

impl Default for ChangeBuffer {
    fn default() -> Self {
        Self::new()
    }
}

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
    let merged = cb.merge_on_read(1);
    assert!(merged.is_empty());
    let merged2 = cb.merge_on_read(2);
    assert!(merged2.is_empty());
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
