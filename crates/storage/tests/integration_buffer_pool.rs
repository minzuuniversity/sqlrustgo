//! Integration tests for Buffer Pool component
//!
//! These tests verify buffer pool behavior with different storage backends.

use sqlrustgo_storage::BufferPool;

#[test]
fn test_buffer_pool_basic_operations() {
    // BufferPool::new takes only capacity (no storage backend in constructor)
    let pool = BufferPool::new(100);
    assert_eq!(pool.capacity(), 100);
    assert!(pool.is_empty());
}

#[test]
fn test_buffer_pool_capacity_and_length() {
    let pool = BufferPool::new(50);
    assert_eq!(pool.capacity(), 50);
    assert_eq!(pool.len(), 0);
}

#[test]
fn test_buffer_pool_with_prefetch() {
    let pool = BufferPool::with_prefetch(100, 4);
    assert_eq!(pool.capacity(), 100);
    assert!(pool.is_empty());
}

#[test]
fn test_buffer_pool_stats_initial() {
    let pool = BufferPool::new(10);
    let stats = pool.stats();
    // Initially no hits or misses
    assert_eq!(stats.hits, 0);
    assert_eq!(stats.misses, 0);
}

#[test]
fn test_buffer_pool_get_not_present() {
    let pool = BufferPool::new(10);
    // Page not in pool should return None
    let result = pool.get(999);
    assert!(result.is_none());
}
