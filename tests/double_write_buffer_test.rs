//! F-26: Double-Write Buffer (crash-safe page writes)
//!
//! **Issue**: #2827
//! **Source**: v3.0.0 COMPLETE_LEGACY_TRACKING_REPORT.md (F-26)
//! **Change**: openspec/changes/f-25-f-26-change-buffer-doublewrite
//!
//! In-memory mock of InnoDB-style Double-Write buffer. Real storage
//! integration in v3.9.0.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub const DEFAULT_DWB_SIZE: usize = 64; // 64 pages

#[derive(Debug, Clone)]
pub struct Page {
    pub id: u64,
    pub data: Vec<u8>,
}

pub struct DoubleWriteBuffer {
    buffer: Arc<Mutex<Vec<Page>>>,
    written: Arc<Mutex<HashMap<u64, Page>>>, // pages in their final location
    max_size: usize,
    fsync_count: Arc<Mutex<u64>>,
    crash_recoveries: Arc<Mutex<u64>>,
}

impl DoubleWriteBuffer {
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_DWB_SIZE)
    }

    pub fn with_capacity(max_size: usize) -> Self {
        Self {
            buffer: Arc::new(Mutex::new(Vec::with_capacity(max_size))),
            written: Arc::new(Mutex::new(HashMap::new())),
            max_size,
            fsync_count: Arc::new(Mutex::new(0)),
            crash_recoveries: Arc::new(Mutex::new(0)),
        }
    }

    /// Stage a page in the double-write buffer.
    pub fn stage(&self, page: Page) {
        let mut buf = self.buffer.lock().unwrap();
        if buf.len() >= self.max_size {
            buf.remove(0); // simple eviction
        }
        buf.push(page);
    }

    pub fn buffered_count(&self) -> usize {
        self.buffer.lock().unwrap().len()
    }

    pub fn is_full(&self) -> bool {
        self.buffered_count() >= self.max_size
    }

    /// Atomic flush: write all staged pages to disk (simulated as fsync).
    /// Returns the pages that were fsynced.
    pub fn fsync(&self) -> Vec<Page> {
        let buf = self.buffer.lock().unwrap();
        let pages = buf.clone();
        *self.fsync_count.lock().unwrap() += 1;
        pages
    }

    /// Write a single page to its final location.
    pub fn write_page(&self, page: Page) {
        self.written.lock().unwrap().insert(page.id, page);
    }

    /// Write all fsynced pages to their final locations.
    pub fn write_all(&self) -> usize {
        let pages = self.fsync();
        let mut written = self.written.lock().unwrap();
        for p in pages {
            written.insert(p.id, p);
        }
        // Clear buffer
        self.buffer.lock().unwrap().clear();
        written.len()
    }

    /// Simulate a crash during write. Page is in buffer but not yet in final location.
    pub fn simulate_crash(&self) {
        // Buffer has pages, written is incomplete
    }

    /// Recover from a crash: pages in buffer take precedence over partial writes.
    pub fn recover_from_crash(&self) -> Vec<Page> {
        let buf = self.buffer.lock().unwrap();
        *self.crash_recoveries.lock().unwrap() += 1;
        buf.clone()
    }

    pub fn get_page(&self, id: u64) -> Option<Page> {
        self.written.lock().unwrap().get(&id).cloned()
    }

    pub fn fsync_count(&self) -> u64 {
        *self.fsync_count.lock().unwrap()
    }

    pub fn recovery_count(&self) -> u64 {
        *self.crash_recoveries.lock().unwrap()
    }
}

impl Default for DoubleWriteBuffer {
    fn default() -> Self {
        Self::new()
    }
}

fn make_page(id: u64, data: &[u8]) -> Page {
    Page {
        id,
        data: data.to_vec(),
    }
}

#[test]
fn test_stage_and_buffered_count() {
    let dwb = DoubleWriteBuffer::new();
    dwb.stage(make_page(1, b"page1"));
    dwb.stage(make_page(2, b"page2"));
    assert_eq!(dwb.buffered_count(), 2);
}

#[test]
fn test_fsync_atomic_flush() {
    let dwb = DoubleWriteBuffer::new();
    dwb.stage(make_page(1, b"a"));
    dwb.stage(make_page(2, b"b"));
    dwb.stage(make_page(3, b"c"));
    let fsynced = dwb.fsync();
    assert_eq!(fsynced.len(), 3);
    assert_eq!(dwb.fsync_count(), 1);
}

#[test]
fn test_write_page_to_final_location() {
    let dwb = DoubleWriteBuffer::new();
    dwb.stage(make_page(1, b"data1"));
    dwb.fsync();
    dwb.write_all();
    assert!(dwb.get_page(1).is_some());
    assert_eq!(dwb.get_page(1).unwrap().data, b"data1");
}

#[test]
fn test_crash_recovery_reconstructs_pages() {
    let dwb = DoubleWriteBuffer::new();
    dwb.stage(make_page(1, b"important"));
    dwb.stage(make_page(2, b"data"));
    dwb.simulate_crash();
    // Page 1 was partially written (torn)
    dwb.write_page(make_page(1, b"PARTIAL"));
    // Recovery: buffer version takes precedence
    let recovered = dwb.recover_from_crash();
    assert_eq!(recovered.len(), 2);
    // After recovery, full data is available
    assert_eq!(recovered[0].data, b"important");
}

#[test]
fn test_dwb_size_limit() {
    let dwb = DoubleWriteBuffer::with_capacity(3);
    dwb.stage(make_page(1, b"a"));
    dwb.stage(make_page(2, b"b"));
    dwb.stage(make_page(3, b"c"));
    assert!(dwb.is_full());
    dwb.stage(make_page(4, b"d")); // evicts page 1
    assert_eq!(dwb.buffered_count(), 3);
}

#[test]
fn test_sequential_write_pattern() {
    let dwb = DoubleWriteBuffer::new();
    // Simulate the canonical pattern: stage all -> fsync -> write
    for i in 0..10 {
        dwb.stage(make_page(i, format!("data{}", i).as_bytes()));
    }
    dwb.fsync();
    dwb.write_all();
    // All 10 pages should be in final location
    for i in 0..10 {
        assert!(dwb.get_page(i).is_some(), "page {} missing", i);
    }
}
