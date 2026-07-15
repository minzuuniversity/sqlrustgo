//! Adaptive Hash Index (AHI) — InnoDB-style hot-page cache.
//!
//! V311-02 (F-24 Adaptive Hash Index main-path integration).
//!
//! ## What it does
//!
//! AHI automatically builds a small in-memory hash index over frequently
//! accessed (table, key) → (page_id, offset) pairs. For InnoDB-style
//! workloads, the same query often hits the same B+ tree page many times
//! (e.g. secondary-index lookups against a hot row). AHI turns these
//! repeated point lookups from O(log N) into O(1).
//!
//! ## Algorithm
//!
//! 1. **record_access(table, key, page_id, offset)** — called on every
//!    index lookup. The page's access_count is incremented. When the count
//!    reaches the promotion threshold (default 17), the (table, key) →
//!    (page_id, offset) entry is inserted into the hash map. We don't
//!    insert on every access because most pages are touched only once.
//!
//! 2. **lookup(table, key) → Option<PageLocation>** — the hot-path query.
//!    Returns the cached page location if the (table, key) was promoted.
//!    Increments `lookups` and (on hit) `hits` for hit-rate statistics.
//!
//! 3. **invalidate_page(page_id) / invalidate_table(table)** — called when
//!    pages are evicted or tables dropped. Removes stale entries so the
//!    AHI never returns a page location that has been recycled.
//!
//! ## Thread-safety
//!
//! All fields are protected by `parking_lot::RwLock`. Multiple readers
//! can do `lookup()` concurrently; writers (record_access on promotion,
//! invalidate_*) take exclusive access for short critical sections.
//!
//! ## Production status
//!
//! V311-02 v1: AHI is reachable from production code via
//! `sqlrustgo_storage::AdaptiveHashIndex` and via
//! `ExecutionEngine::adaptive_hash_index()`. V311-02 v2 (deferred) will
//! wire the AHI into the secondary-index lookup path so lookups go
//! through AHI before B+ Tree traversal. V311-02 v1 is the storage-API
//! surface + unit tests, matching the V311-01 v1 scope.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Default promotion threshold: a page must be touched 17 times before
/// its (table, key) is promoted into the AHI. This matches the
/// pre-existing `tests/integration/sql/adaptive_hash_index_test.rs`
/// constant so behavior is unchanged when migrating to the production API.
pub const DEFAULT_PROMOTION_THRESHOLD: u64 = 17;

/// Page location: a (page_id, byte_offset) pair pointing into the storage
/// engine's address space. For MemoryStorage this is conceptual (a single
/// in-memory page); for FileStorage it is (page_file_id, byte_offset_within_page).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PageLocation {
    pub page_id: u64,
    pub offset: u32,
}

/// Composite hash key: (table_name, serialized_key_bytes). Using
/// `Vec<u8>` for the key keeps AHI agnostic of the value type — callers
/// serialize their PK or secondary-index key to bytes before calling.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IndexKey {
    pub table: String,
    pub key: Vec<u8>,
}

/// Adaptive Hash Index. Internally holds:
/// - `map`: IndexKey → PageLocation (the actual hash)
/// - `access_count`: page_id → touches (used to decide when to promote)
/// - `lookups` / `hits` / `promoted_count`: stats counters
pub struct AdaptiveHashIndex {
    map: RwLock<HashMap<IndexKey, PageLocation>>,
    access_count: RwLock<HashMap<u64, u64>>,
    threshold: u64,
    promoted_count: RwLock<u64>,
    lookups: RwLock<u64>,
    hits: RwLock<u64>,
}

impl AdaptiveHashIndex {
    /// Create a new AHI with the default promotion threshold (17).
    pub fn new() -> Self {
        Self::with_threshold(DEFAULT_PROMOTION_THRESHOLD)
    }

    /// Create a new AHI with a custom promotion threshold. Lower values
    /// promote more aggressively (faster hot-page detection, more memory);
    /// higher values are more conservative.
    pub fn with_threshold(threshold: u64) -> Self {
        Self {
            map: RwLock::new(HashMap::new()),
            access_count: RwLock::new(HashMap::new()),
            threshold,
            promoted_count: RwLock::new(0),
            lookups: RwLock::new(0),
            hits: RwLock::new(0),
        }
    }

    /// Wrap this AHI in `Arc` for sharing with `ExecutionEngine`. Most
    /// callers will want this since the engine holds a single AHI
    /// instance and shares it across all queries.
    pub fn into_shared(self) -> Arc<Self> {
        Arc::new(self)
    }

    /// Record an access to (table, key) which resolved to (page_id, offset).
    /// After `threshold` accesses to the same page_id, the entry is
    /// promoted into the AHI hash map.
    pub fn record_access(&self, table: &str, key: &[u8], page_id: u64, offset: u32) {
        let mut counts = self.access_count.write();
        let count = counts.entry(page_id).or_insert(0);
        *count += 1;
        if *count >= self.threshold {
            let idx_key = IndexKey {
                table: table.to_string(),
                key: key.to_vec(),
            };
            self.map
                .write()
                .insert(idx_key, PageLocation { page_id, offset });
            *self.promoted_count.write() += 1;
        }
    }

    /// Look up a (table, key) pair in the AHI. Returns the cached
    /// PageLocation on hit, None on miss. Both cases update stats.
    pub fn lookup(&self, table: &str, key: &[u8]) -> Option<PageLocation> {
        *self.lookups.write() += 1;
        let idx_key = IndexKey {
            table: table.to_string(),
            key: key.to_vec(),
        };
        let result = self.map.read().get(&idx_key).cloned();
        if result.is_some() {
            *self.hits.write() += 1;
        }
        result
    }

    /// Invalidate all entries pointing to a specific page_id. Called when
    /// a page is evicted or its contents change (split, rewrite, etc).
    pub fn invalidate_page(&self, page_id: u64) {
        let mut map = self.map.write();
        map.retain(|_, loc| loc.page_id != page_id);
        self.access_count.write().remove(&page_id);
    }

    /// Invalidate all entries for a table. Called when a table is dropped
    /// or its schema changes in a way that invalidates indexes.
    pub fn invalidate_table(&self, table: &str) {
        let mut map = self.map.write();
        map.retain(|k, _| k.table != table);
    }

    /// Number of entries currently in the AHI hash map.
    pub fn size(&self) -> usize {
        self.map.read().len()
    }

    /// Hits / lookups as a fraction in [0.0, 1.0]. Returns 0.0 if no
    /// lookups have been performed yet.
    pub fn hit_rate(&self) -> f64 {
        let lookups = *self.lookups.read() as f64;
        let hits = *self.hits.read() as f64;
        if lookups == 0.0 {
            0.0
        } else {
            hits / lookups
        }
    }

    /// Total number of entries that have been promoted into the AHI
    /// over its lifetime. May be larger than `size()` if invalidations
    /// have removed entries.
    pub fn promoted_count(&self) -> u64 {
        *self.promoted_count.read()
    }

    /// Total number of lookup() calls over the AHI's lifetime.
    pub fn total_lookups(&self) -> u64 {
        *self.lookups.read()
    }

    /// Total number of hits over the AHI's lifetime.
    pub fn total_hits(&self) -> u64 {
        *self.hits.read()
    }
}

impl Default for AdaptiveHashIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_has_zero_size() {
        let ahi = AdaptiveHashIndex::new();
        assert_eq!(ahi.size(), 0);
        assert_eq!(ahi.hit_rate(), 0.0);
        assert_eq!(ahi.promoted_count(), 0);
    }

    #[test]
    fn test_lookup_miss_returns_none() {
        let ahi = AdaptiveHashIndex::new();
        assert!(ahi.lookup("users", b"alice").is_none());
        assert_eq!(ahi.total_lookups(), 1);
        assert_eq!(ahi.total_hits(), 0);
    }

    #[test]
    fn test_record_access_below_threshold_does_not_promote() {
        let ahi = AdaptiveHashIndex::new();
        for _ in 0..16 {
            ahi.record_access("users", b"alice", 1, 100);
        }
        assert_eq!(ahi.size(), 0, "below threshold should not promote");
        assert!(ahi.lookup("users", b"alice").is_none());
    }

    #[test]
    fn test_record_access_at_threshold_promotes() {
        let ahi = AdaptiveHashIndex::new();
        for _ in 0..17 {
            ahi.record_access("users", b"alice", 1, 100);
        }
        assert_eq!(ahi.size(), 1);
        assert_eq!(
            ahi.lookup("users", b"alice"),
            Some(PageLocation {
                page_id: 1,
                offset: 100
            })
        );
        assert_eq!(ahi.promoted_count(), 1);
    }

    #[test]
    fn test_custom_threshold() {
        let ahi = AdaptiveHashIndex::with_threshold(3);
        ahi.record_access("t", b"k", 7, 70);
        ahi.record_access("t", b"k", 7, 70);
        ahi.record_access("t", b"k", 7, 70);
        assert_eq!(ahi.size(), 1);
    }

    #[test]
    fn test_invalidate_page_removes_entries() {
        let ahi = AdaptiveHashIndex::new();
        for _ in 0..17 {
            ahi.record_access("users", b"alice", 1, 100);
        }
        assert_eq!(ahi.size(), 1);
        ahi.invalidate_page(1);
        assert_eq!(ahi.size(), 0);
        assert!(ahi.lookup("users", b"alice").is_none());
    }

    #[test]
    fn test_invalidate_table_removes_only_that_table() {
        let ahi = AdaptiveHashIndex::new();
        for _ in 0..17 {
            ahi.record_access("users", b"alice", 1, 100);
            ahi.record_access("orders", b"o1", 2, 200);
        }
        assert_eq!(ahi.size(), 2);
        ahi.invalidate_table("users");
        assert_eq!(ahi.size(), 1);
        assert!(ahi.lookup("users", b"alice").is_none());
        assert!(ahi.lookup("orders", b"o1").is_some());
    }

    #[test]
    fn test_hit_rate_calculation() {
        let ahi = AdaptiveHashIndex::new();
        for _ in 0..17 {
            ahi.record_access("users", b"alice", 1, 100);
        }
        // Now (users, alice) is hot. 3 lookups: 1 miss + 1 hit + 1 hit.
        let _ = ahi.lookup("users", b"bob"); // miss
        let _ = ahi.lookup("users", b"alice"); // hit
        let _ = ahi.lookup("users", b"alice"); // hit
                                               // 2 hits / 3 lookups = 0.6667
        let rate = ahi.hit_rate();
        assert!((rate - 2.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn test_into_shared() {
        let ahi = AdaptiveHashIndex::new().into_shared();
        // Two callers share the same AHI.
        let ahi2 = Arc::clone(&ahi);
        for _ in 0..17 {
            ahi.record_access("t", b"k", 1, 0);
        }
        assert_eq!(ahi2.size(), 1);
    }

    #[test]
    fn test_record_access_updates_promoted_count() {
        let ahi = AdaptiveHashIndex::new();
        // Same page accessed 17 times from different keys → 17 promotions? No:
        // promotions are per-page, and access_count is the trigger. With
        // threshold=17, the 17th access triggers exactly one promotion.
        for i in 0..17 {
            ahi.record_access("t", format!("k{}", i).as_bytes(), 1, 0);
        }
        assert_eq!(ahi.promoted_count(), 1);
        assert_eq!(ahi.size(), 1);
    }
}
