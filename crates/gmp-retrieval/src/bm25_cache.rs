//! BM25 Query Cache for FTS channel
//!
//! Caches normalized BM25 query results to avoid repeated full-text scans.
//! Normalization: lowercase + trim + sort tokens to ensure query variants hit same cache key.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use gmp_core::ScoredChunk;

/// Cache entry with TTL
struct CacheEntry {
    results: Vec<ScoredChunk>,
    created_at: Instant,
}

/// LRU-style BM25 query cache with deterministic FIFO eviction
///
/// Normalizes queries before caching so that:
/// - "偏差 调查" and "偏差调查" produce the same cache key
/// - "偏差　调查" (fullwidth space) also matches
#[derive(Clone)]
pub struct Bm25Cache {
    inner: Arc<RwLock<CacheState>>,
    ttl_seconds: u64,
    max_capacity: usize,
}

struct CacheState {
    map: HashMap<String, CacheEntry>,
    order: VecDeque<String>, // insertion-order keys for FIFO eviction
}

impl Bm25Cache {
    /// Create a new cache with given TTL and capacity
    pub fn new(ttl_seconds: u64, max_capacity: usize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(CacheState {
                map: HashMap::new(),
                order: VecDeque::new(),
            })),
            ttl_seconds,
            max_capacity,
        }
    }

    /// Get cache key: normalized query string
    fn normalize(query: &str) -> String {
        let lower = query.trim().to_lowercase();
        let mut tokens: Vec<&str> = lower.split_whitespace().collect();
        tokens.sort();
        tokens.concat()
    }

    /// Look up cached results (returns None if expired or not found)
    pub fn get(&self, query: &str) -> Option<Vec<ScoredChunk>> {
        let key = Self::normalize(query);
        let mut state = self.inner.write().ok()?;

        if let Some(entry) = state.map.get(&key) {
            if entry.created_at.elapsed() < Duration::from_secs(self.ttl_seconds) {
                return Some(entry.results.clone());
            }
            state.map.remove(&key);
            if let Some(pos) = state.order.iter().position(|k| k == &key) {
                state.order.remove(pos);
            }
        }
        None
    }

    /// Insert results into cache
    pub fn insert(&self, query: &str, results: Vec<ScoredChunk>) {
        let key = Self::normalize(query);

        let mut state = match self.inner.write() {
            Ok(s) => s,
            Err(_) => return,
        };

        // Evict if at capacity (FIFO)
        if state.map.len() >= self.max_capacity {
            if let Some(oldest) = state.order.pop_front() {
                state.map.remove(&oldest);
            }
        }

        // Insert or update
        if state.map.contains_key(&key) {
            if let Some(pos) = state.order.iter().position(|k| k == &key) {
                state.order.remove(pos);
            }
        }

        state.order.push_back(key.clone());
        state.map.insert(
            key,
            CacheEntry {
                results,
                created_at: Instant::now(),
            },
        );
    }

    /// Clear the cache
    #[allow(dead_code)]
    pub fn clear(&self) {
        if let Ok(mut state) = self.inner.write() {
            state.map.clear();
            state.order.clear();
        }
    }

    /// Get current entry count
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.inner.read().map(|s| s.map.len()).unwrap_or(0)
    }

    /// Check if cache is empty
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::Bm25Cache;

    #[test]
    fn test_normalize_different_whitespace() {
        let cache = Bm25Cache::new(3600, 10);
        cache.insert("偏差调查", vec![]);
        assert!(cache.get("偏差  调查").is_some());
        assert!(cache.get("调查 偏差").is_some());
    }

    #[test]
    fn test_ttl_eviction() {
        let cache = Bm25Cache::new(0, 10);
        cache.insert("test", vec![]);
        assert!(cache.get("test").is_none());
    }

    #[test]
    fn test_capacity_eviction() {
        let cache = Bm25Cache::new(3600, 2);
        cache.insert("a", vec![]);
        cache.insert("b", vec![]);
        cache.insert("c", vec![]);
        assert!(cache.get("a").is_none());
        assert!(cache.get("b").is_some());
        assert!(cache.get("c").is_some());
    }

    #[test]
    fn test_clear() {
        let cache = Bm25Cache::new(3600, 10);
        cache.insert("test", vec![]);
        assert!(!cache.is_empty());
        cache.clear();
        assert!(cache.is_empty());
    }
}
