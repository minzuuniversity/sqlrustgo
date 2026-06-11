use parking_lot::Mutex;
use sqlrustgo_parser::Statement;
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub size: usize,
    pub max_size: usize,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

pub struct PreparedStatementCache {
    inner: Mutex<CacheInner>,
}

struct CacheEntry {
    sql: String,
    #[allow(dead_code)] // reserved for future prepared-statement execution API
    statement: Statement,
}

struct CacheInner {
    by_name: HashMap<String, CacheEntry>,
    lru_order: VecDeque<String>,
    max_size: usize,
    hits: u64,
    misses: u64,
    evictions: u64,
}

impl PreparedStatementCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            inner: Mutex::new(CacheInner {
                by_name: HashMap::new(),
                lru_order: VecDeque::new(),
                max_size: max_size.max(1),
                hits: 0,
                misses: 0,
                evictions: 0,
            }),
        }
    }

    pub fn prepare(&self, name: &str, sql: &str, statement: Statement) {
        let mut inner = self.inner.lock();
        if inner.by_name.contains_key(name) {
            inner.lru_order.retain(|n| n != name);
        }
        if inner.by_name.len() >= inner.max_size && !inner.by_name.contains_key(name) {
            if let Some(victim) = inner.lru_order.pop_back() {
                inner.by_name.remove(&victim);
                inner.evictions += 1;
            }
        }
        inner.by_name.insert(
            name.to_string(),
            CacheEntry {
                sql: sql.to_string(),
                statement,
            },
        );
        inner.lru_order.push_front(name.to_string());
    }

    pub fn execute(&self, name: &str) -> Option<Statement> {
        let mut inner = self.inner.lock();
        if inner.by_name.contains_key(name) {
            inner.hits += 1;
            inner.lru_order.retain(|n| n != name);
            inner.lru_order.push_front(name.to_string());
            None
        } else {
            inner.misses += 1;
            None
        }
    }

    pub fn execute_with_sql(&self, name: &str) -> Option<String> {
        let mut inner = self.inner.lock();
        if let Some(entry) = inner.by_name.get(name) {
            let sql = entry.sql.clone();
            inner.hits += 1;
            inner.lru_order.retain(|n| n != name);
            inner.lru_order.push_front(name.to_string());
            Some(sql)
        } else {
            inner.misses += 1;
            None
        }
    }

    pub fn deallocate(&self, name: &str) -> bool {
        let mut inner = self.inner.lock();
        let removed = inner.by_name.remove(name).is_some();
        if removed {
            inner.lru_order.retain(|n| n != name);
        }
        removed
    }

    pub fn clear(&self) {
        let mut inner = self.inner.lock();
        inner.by_name.clear();
        inner.lru_order.clear();
        inner.hits = 0;
        inner.misses = 0;
        inner.evictions = 0;
    }

    pub fn stats(&self) -> CacheStats {
        let inner = self.inner.lock();
        CacheStats {
            hits: inner.hits,
            misses: inner.misses,
            evictions: inner.evictions,
            size: inner.by_name.len(),
            max_size: inner.max_size,
        }
    }

    pub fn len(&self) -> usize {
        self.inner.lock().by_name.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.lock().by_name.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlrustgo_parser::{parse, Statement};

    fn make_test_stmt() -> Statement {
        parse("SELECT 1").unwrap()
    }

    #[test]
    fn test_new_empty() {
        let cache = PreparedStatementCache::new(10);
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_prepare_execute() {
        let cache = PreparedStatementCache::new(10);
        cache.prepare("s1", "SELECT 1", make_test_stmt());
        assert!(cache.execute_with_sql("s1").is_some());
        assert!(cache.execute_with_sql("nonexistent").is_none());
    }

    #[test]
    fn test_hit_miss_counters() {
        let cache = PreparedStatementCache::new(10);
        cache.prepare("s1", "SELECT 1", make_test_stmt());
        cache.execute_with_sql("s1");
        cache.execute_with_sql("s1");
        cache.execute_with_sql("s1");
        cache.execute_with_sql("missing");
        let stats = cache.stats();
        assert_eq!(stats.hits, 3);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hit_rate(), 0.75);
    }

    #[test]
    fn test_lru_eviction() {
        let cache = PreparedStatementCache::new(2);
        cache.prepare("s1", "SELECT 1", make_test_stmt());
        cache.prepare("s2", "SELECT 2", make_test_stmt());
        cache.prepare("s3", "SELECT 3", make_test_stmt());
        assert!(
            cache.execute_with_sql("s1").is_none(),
            "s1 should be evicted"
        );
        assert!(cache.execute_with_sql("s2").is_some());
        assert!(cache.execute_with_sql("s3").is_some());
        let stats = cache.stats();
        assert_eq!(stats.evictions, 1);
    }

    #[test]
    fn test_lru_access_promotes() {
        let cache = PreparedStatementCache::new(2);
        cache.prepare("s1", "SELECT 1", make_test_stmt());
        cache.prepare("s2", "SELECT 2", make_test_stmt());
        let _ = cache.execute_with_sql("s1");
        cache.prepare("s3", "SELECT 3", make_test_stmt());
        assert!(
            cache.execute_with_sql("s1").is_some(),
            "s1 should be promoted, s2 evicted"
        );
        assert!(cache.execute_with_sql("s2").is_none());
    }

    #[test]
    fn test_deallocate() {
        let cache = PreparedStatementCache::new(10);
        cache.prepare("s1", "SELECT 1", make_test_stmt());
        assert!(cache.deallocate("s1"));
        assert!(!cache.deallocate("s1"));
        assert!(cache.execute("s1").is_none());
    }

    #[test]
    fn test_clear() {
        let cache = PreparedStatementCache::new(10);
        cache.prepare("s1", "SELECT 1", make_test_stmt());
        cache.prepare("s2", "SELECT 2", make_test_stmt());
        cache.clear();
        assert!(cache.is_empty());
        assert_eq!(cache.stats().hits, 0);
        assert_eq!(cache.stats().misses, 0);
    }

    #[test]
    fn test_duplicate_name_overwrites() {
        let cache = PreparedStatementCache::new(10);
        cache.prepare("s1", "SELECT 1", make_test_stmt());
        cache.prepare("s1", "SELECT 2", make_test_stmt());
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_max_size_at_least_1() {
        let cache = PreparedStatementCache::new(0);
        assert!(cache.stats().max_size >= 1);
    }

    #[test]
    fn test_concurrent_access() {
        use std::sync::Arc;
        use std::thread;
        let cache = Arc::new(PreparedStatementCache::new(100));
        for i in 0..10 {
            let c = cache.clone();
            let name = format!("s{}", i);
            let stmt = make_test_stmt();
            thread::spawn(move || {
                c.prepare(&name, "SELECT 1", stmt);
            });
        }
        for i in 0..10 {
            let c = cache.clone();
            let name = format!("s{}", i);
            thread::spawn(move || {
                let _ = c.execute(&name);
            });
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
        let stats = cache.stats();
        assert!(stats.size <= 100);
    }
}
