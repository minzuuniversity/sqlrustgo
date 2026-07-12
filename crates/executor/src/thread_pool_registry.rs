//! ThreadPoolRegistry - explicit per-query thread pool management
//!
//! Provides cached thread pools to avoid the overhead of creating/destroying
//! rayon pools for each query. Pools are keyed by parallelism degree.
//!
//! Design principles (per整合指南):
//! - Per-query explicit ThreadPool (NOT global pool) to avoid blocking ServerThreadPool workers
//! - Thread pools are cached and reused across queries with same parallelism
//! - Integration point for CBO-driven parallelism (Phase 3)

use crate::task_scheduler::RayonTaskScheduler;
use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use tracing::instrument;

/// Maximum number of cached pool configurations
const MAX_CACHED_POOLS: usize = 16;

/// ThreadPoolRegistry - manages a cache of rayon thread pools keyed by parallelism degree.
///
/// Key insight from整合指南: Use per-query explicit ThreadPool, NOT global pool.
/// This prevents long-running parallel tasks from blocking ServerThreadPool workers.
pub struct ThreadPoolRegistry {
    pools: parking_lot::Mutex<HashMap<usize, Arc<RayonTaskScheduler>>>,
    default_parallelism: usize,
}

impl ThreadPoolRegistry {
    /// Create a new registry with default parallelism based on available CPU cores
    pub fn new() -> Self {
        let default_parallelism = thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        Self::with_default(default_parallelism)
    }

    /// Create a registry with a specific default parallelism
    #[must_use]
    pub fn with_default(default_parallelism: usize) -> Self {
        Self {
            pools: parking_lot::Mutex::new(HashMap::with_capacity(4)),
            default_parallelism: default_parallelism.max(1),
        }
    }

    /// Get or create a thread pool for the given parallelism degree
    ///
    /// This is the main entry point. CBO (Phase 3) will call this with
    /// `should_parallelize()` result to decide which pool to use.
    #[must_use]
    #[instrument(skip(self), fields(parallelism = degree))]
    pub fn get_pool(&self, degree: usize) -> Arc<RayonTaskScheduler> {
        let degree = degree.max(1);

        // Check cache first (read-only lock scope)
        {
            let pools = self.pools.lock();
            if let Some(pool) = pools.get(&degree) {
                return Arc::clone(pool);
            }
        }

        // Cache miss - need to create new pool
        // Use write lock only for insertion
        let mut pools = self.pools.lock();

        // Double-check after acquiring write lock
        if let Some(pool) = pools.get(&degree) {
            return Arc::clone(pool);
        }

        // Evict oldest if cache is full
        if pools.len() >= MAX_CACHED_POOLS {
            // Remove the smallest key (typically degree=1 is most common)
            if let Some(min_key) = pools.keys().min().copied() {
                pools.remove(&min_key);
            }
        }

        // Create new pool
        let pool = Arc::new(RayonTaskScheduler::new(degree));
        pools.insert(degree, Arc::clone(&pool));
        pool
    }

    /// Get pool with default parallelism (from available CPU cores or configured default)
    #[must_use]
    pub fn get_default_pool(&self) -> Arc<RayonTaskScheduler> {
        self.get_pool(self.default_parallelism)
    }

    /// Pre-warm the cache with common parallelism degrees
    ///
    /// Call this during server startup to avoid first-query latency.
    pub fn pre_warm(&self) {
        // Warm common degrees: 1, 2, 4, 8 (most typical configurations)
        for degree in [1, 2, 4, 8] {
            let _ = self.get_pool(degree);
        }
    }

    /// Get the default parallelism degree
    #[must_use]
    pub fn default_parallelism(&self) -> usize {
        self.default_parallelism
    }

    /// Returns the number of cached pools
    #[allow(dead_code)]
    fn cached_pool_count(&self) -> usize {
        self.pools.lock().len()
    }
}

impl Default for ThreadPoolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Global registry instance for server-wide use
static GLOBAL_REGISTRY: std::sync::LazyLock<ThreadPoolRegistry> =
    std::sync::LazyLock::new(ThreadPoolRegistry::new);

impl ThreadPoolRegistry {
    /// Get the global registry instance
    ///
    /// Server-level singleton. Per-query parallelism is still managed by
    /// getting pools from this registry - each query gets its own pool
    /// reference that can be used independently.
    #[must_use]
    pub fn global() -> &'static ThreadPoolRegistry {
        &GLOBAL_REGISTRY
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task_scheduler::TaskScheduler;

    #[test]
    fn test_registry_creates_pools() {
        let registry = ThreadPoolRegistry::new();
        let pool = registry.get_pool(4);
        assert_eq!(pool.current_parallelism(), 4);
    }

    #[test]
    fn test_registry_caches_pools() {
        let registry = ThreadPoolRegistry::new();
        let pool1 = registry.get_pool(4);
        let pool2 = registry.get_pool(4);
        // Should return the same Arc
        assert!(Arc::ptr_eq(&pool1, &pool2));
    }

    #[test]
    fn test_registry_different_degrees() {
        let registry = ThreadPoolRegistry::new();
        let pool4 = registry.get_pool(4);
        let pool8 = registry.get_pool(8);
        assert!(!Arc::ptr_eq(&pool4, &pool8));
        assert_eq!(pool4.current_parallelism(), 4);
        assert_eq!(pool8.current_parallelism(), 8);
    }

    #[test]
    fn test_registry_degree_one() {
        let registry = ThreadPoolRegistry::new();
        let pool = registry.get_pool(1);
        assert_eq!(pool.current_parallelism(), 1);
    }

    #[test]
    fn test_registry_zero_degree_becomes_one() {
        let registry = ThreadPoolRegistry::new();
        let pool = registry.get_pool(0);
        assert_eq!(pool.current_parallelism(), 1);
    }

    #[test]
    fn test_pre_warm() {
        let registry = ThreadPoolRegistry::with_default(8);
        registry.pre_warm();

        // After pre_warm, these should already be cached
        let pool1 = registry.get_pool(1);
        let pool2 = registry.get_pool(2);
        let pool4 = registry.get_pool(4);
        let pool8 = registry.get_pool(8);

        assert_eq!(pool1.current_parallelism(), 1);
        assert_eq!(pool2.current_parallelism(), 2);
        assert_eq!(pool4.current_parallelism(), 4);
        assert_eq!(pool8.current_parallelism(), 8);
    }

    #[test]
    fn test_default_parallelism() {
        let registry = ThreadPoolRegistry::with_default(6);
        assert_eq!(registry.default_parallelism(), 6);

        let default_pool = registry.get_default_pool();
        assert_eq!(default_pool.current_parallelism(), 6);
    }

    #[test]
    fn test_cached_pool_count() {
        let registry = ThreadPoolRegistry::new();
        assert_eq!(registry.cached_pool_count(), 0);

        let _pool4 = registry.get_pool(4);
        assert_eq!(registry.cached_pool_count(), 1);

        let _pool8 = registry.get_pool(8);
        assert_eq!(registry.cached_pool_count(), 2);

        // Getting same pool doesn't increase count
        let _pool4_again = registry.get_pool(4);
        assert_eq!(registry.cached_pool_count(), 2);
    }
}
