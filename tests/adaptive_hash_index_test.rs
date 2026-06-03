//! F-24: Adaptive Hash Index (AHI)
//!
//! **Issue**: #2825
//! **Source**: v3.0.0 COMPLETE_LEGACY_TRACKING_REPORT.md (F-24)
//! **Change**: openspec/changes/f-24-adaptive-hash-index
//!
//! In-memory AHI with auto-promote/demote based on access frequency.
//! Real B+ tree integration in v3.9.0.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub const DEFAULT_PROMOTION_THRESHOLD: u64 = 17;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PageLocation {
    pub page_id: u64,
    pub offset: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IndexKey {
    pub table: String,
    pub key: Vec<u8>,
}

pub struct AdaptiveHashIndex {
    map: Arc<RwLock<HashMap<IndexKey, PageLocation>>>,
    access_count: Arc<RwLock<HashMap<u64, u64>>>, // page_id -> count
    threshold: u64,
    promoted_count: Arc<RwLock<u64>>,
    lookups: Arc<RwLock<u64>>,
    hits: Arc<RwLock<u64>>,
}

impl AdaptiveHashIndex {
    pub fn new() -> Self {
        Self::with_threshold(DEFAULT_PROMOTION_THRESHOLD)
    }

    pub fn with_threshold(threshold: u64) -> Self {
        Self {
            map: Arc::new(RwLock::new(HashMap::new())),
            access_count: Arc::new(RwLock::new(HashMap::new())),
            threshold,
            promoted_count: Arc::new(RwLock::new(0)),
            lookups: Arc::new(RwLock::new(0)),
            hits: Arc::new(RwLock::new(0)),
        }
    }

    pub fn record_access(&self, table: &str, key: &[u8], page_id: u64, offset: u32) {
        let mut counts = self.access_count.write().unwrap();
        let count = counts.entry(page_id).or_insert(0);
        *count += 1;
        if *count >= self.threshold {
            // Promote to AHI
            let idx_key = IndexKey {
                table: table.to_string(),
                key: key.to_vec(),
            };
            self.map
                .write()
                .unwrap()
                .insert(idx_key, PageLocation { page_id, offset });
            *self.promoted_count.write().unwrap() += 1;
        }
    }

    pub fn lookup(&self, table: &str, key: &[u8]) -> Option<PageLocation> {
        *self.lookups.write().unwrap() += 1;
        let idx_key = IndexKey {
            table: table.to_string(),
            key: key.to_vec(),
        };
        let result = self.map.read().unwrap().get(&idx_key).cloned();
        if result.is_some() {
            *self.hits.write().unwrap() += 1;
        }
        result
    }

    pub fn invalidate_page(&self, page_id: u64) {
        let mut map = self.map.write().unwrap();
        map.retain(|_, loc| loc.page_id != page_id);
        self.access_count.write().unwrap().remove(&page_id);
    }

    pub fn invalidate_table(&self, table: &str) {
        let mut map = self.map.write().unwrap();
        map.retain(|k, _| k.table != table);
    }

    pub fn size(&self) -> usize {
        self.map.read().unwrap().len()
    }

    pub fn hit_rate(&self) -> f64 {
        let lookups = *self.lookups.read().unwrap() as f64;
        let hits = *self.hits.read().unwrap() as f64;
        if lookups == 0.0 {
            0.0
        } else {
            hits / lookups
        }
    }
}

impl Default for AdaptiveHashIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[test]
fn test_basic_lookup() {
    let ahi = AdaptiveHashIndex::new();
    ahi.record_access("users", b"alice", 1, 100);
    let result = ahi.lookup("users", b"alice");
    // Below threshold, no promotion yet
    assert!(result.is_none());
}

#[test]
fn test_promotion_after_threshold() {
    let ahi = AdaptiveHashIndex::with_threshold(3);
    for _ in 0..5 {
        ahi.record_access("users", b"alice", 1, 100);
    }
    let result = ahi.lookup("users", b"alice");
    assert!(result.is_some(), "should be promoted after 5 accesses");
    assert_eq!(result.unwrap().page_id, 1);
}

#[test]
fn test_demote_on_table_drop() {
    let ahi = AdaptiveHashIndex::with_threshold(2);
    ahi.record_access("users", b"k1", 1, 0);
    ahi.record_access("users", b"k1", 1, 0);
    assert!(ahi.lookup("users", b"k1").is_some());

    ahi.invalidate_table("users");
    assert!(ahi.lookup("users", b"k1").is_none());
    assert_eq!(ahi.size(), 0);
}

#[test]
fn test_no_promote_below_threshold() {
    let ahi = AdaptiveHashIndex::with_threshold(10);
    for _ in 0..5 {
        ahi.record_access("t", b"k", 1, 0);
    }
    assert_eq!(ahi.size(), 0);
    assert_eq!(ahi.lookup("t", b"k"), None);
}

#[test]
fn test_invalidate_page() {
    let ahi = AdaptiveHashIndex::with_threshold(1);
    ahi.record_access("t", b"k1", 1, 0);
    ahi.record_access("t", b"k2", 1, 0);
    assert_eq!(ahi.size(), 2);
    ahi.invalidate_page(1);
    assert_eq!(ahi.size(), 0);
}

#[test]
fn test_hit_rate_tracking() {
    let ahi = AdaptiveHashIndex::with_threshold(1);
    ahi.record_access("t", b"k1", 1, 0);
    // 2 lookups: 1 hit, 1 miss (k2 not in AHI)
    ahi.lookup("t", b"k1");
    ahi.lookup("t", b"k2");
    let rate = ahi.hit_rate();
    assert!(
        (rate - 0.5).abs() < 0.01,
        "hit rate should be 0.5, got {}",
        rate
    );
}

#[test]
fn test_multiple_tables_isolation() {
    let ahi = AdaptiveHashIndex::with_threshold(1);
    ahi.record_access("t1", b"k", 1, 0);
    ahi.record_access("t2", b"k", 2, 0);
    assert!(ahi.lookup("t1", b"k").is_some());
    assert!(ahi.lookup("t2", b"k").is_some());
    ahi.invalidate_table("t1");
    assert!(ahi.lookup("t1", b"k").is_none());
    assert!(ahi.lookup("t2", b"k").is_some());
}
