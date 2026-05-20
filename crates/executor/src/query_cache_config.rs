use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct QueryCacheConfig {
    pub max_entries: usize,
    pub max_memory_bytes: usize,
    pub ttl_seconds: u64,
    pub enabled: bool,
    pub benchmark_mode: bool,
}

impl Default for QueryCacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 1000,
            max_memory_bytes: 100 * 1024 * 1024, // 100MB
            ttl_seconds: 30,
            enabled: true,
            benchmark_mode: false,
        }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct CacheKey {
    pub normalized_sql: String,
    pub params_hash: u64,
}

#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub result: super::ExecutorResult,
    pub tables: Vec<String>,
    pub created_at: Instant,
    pub size_bytes: usize,
    pub last_access: u64,
}

impl CacheEntry {
    pub fn is_expired(&self, ttl: Duration) -> bool {
        self.created_at.elapsed() > ttl
    }

    pub fn estimate_size(&self) -> usize {
        let mut size = 0;
        for row in &self.result.rows {
            for val in row {
                size += val.estimate_memory_size();
            }
        }
        size
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlrustgo_types::Value;

    #[test]
    fn test_cache_key_equality() {
        let key1 = CacheKey {
            normalized_sql: "SELECT 1".to_string(),
            params_hash: 12345,
        };
        let key2 = CacheKey {
            normalized_sql: "SELECT 1".to_string(),
            params_hash: 12345,
        };
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_cache_key_inequality() {
        let key1 = CacheKey {
            normalized_sql: "SELECT 1".to_string(),
            params_hash: 12345,
        };
        let key2 = CacheKey {
            normalized_sql: "SELECT 2".to_string(),
            params_hash: 12345,
        };
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_cache_key_hash() {
        use std::collections::HashSet;
        let key1 = CacheKey {
            normalized_sql: "SELECT 1".to_string(),
            params_hash: 12345,
        };
        let key2 = CacheKey {
            normalized_sql: "SELECT 1".to_string(),
            params_hash: 12345,
        };
        let mut set = HashSet::new();
        set.insert(key1.clone());
        set.insert(key2.clone());
        // Same key should not add a second entry
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn test_cache_entry_is_expired_false() {
        let entry = CacheEntry {
            result: super::super::ExecutorResult {
                rows: vec![],
                affected_rows: 0,
                last_insert_id: None,
            },
            tables: vec![],
            created_at: Instant::now(),
            size_bytes: 100,
            last_access: 0,
        };
        // Fresh entry should not be expired with 1 hour TTL
        assert!(!entry.is_expired(Duration::from_secs(3600)));
    }

    #[test]
    fn test_cache_entry_is_expired_true() {
        let entry = CacheEntry {
            result: super::super::ExecutorResult {
                rows: vec![],
                affected_rows: 0,
                last_insert_id: None,
            },
            tables: vec![],
            created_at: Instant::now() - Duration::from_secs(120),
            size_bytes: 100,
            last_access: 0,
        };
        // Entry created 120 seconds ago should be expired with 60 second TTL
        assert!(entry.is_expired(Duration::from_secs(60)));
    }

    #[test]
    fn test_query_cache_config_default() {
        let config = QueryCacheConfig::default();
        assert_eq!(config.max_entries, 1000);
        assert_eq!(config.max_memory_bytes, 100 * 1024 * 1024);
        assert_eq!(config.ttl_seconds, 30);
        assert!(config.enabled);
        assert!(!config.benchmark_mode);
    }

    #[test]
    fn test_cache_entry_estimate_size() {
        // estimate_size() computes from result.rows content, not size_bytes field
        let entry = CacheEntry {
            result: super::super::ExecutorResult {
                rows: vec![],
                affected_rows: 0,
                last_insert_id: None,
            },
            tables: vec!["users".to_string()],
            created_at: Instant::now(),
            size_bytes: 256,
            last_access: 0,
        };
        // Empty rows => 0
        assert_eq!(entry.estimate_size(), 0);
    }
}
