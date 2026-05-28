//! Buffer Pool Integration Tests
//!
//! Tests for the BufferPool implementation including LRU eviction,
//! page allocation, fetching, and statistics tracking.

use sqlrustgo_storage::buffer_pool::BufferPool;
use sqlrustgo_storage::page::Page;
use std::sync::Arc;

// ============================================================================
// Test Setup Helpers
// ============================================================================

/// Create a simple page loader for testing
fn create_page_loader() -> impl Fn(u32) -> Arc<Page> {
    move |page_id| Arc::new(Page::new(page_id))
}

// ============================================================================
// Basic Operations Tests
// ============================================================================

#[test]
fn test_buffer_pool_new() {
    let pool = BufferPool::new(10);
    assert_eq!(pool.capacity(), 10);
    assert!(pool.is_empty());
    assert_eq!(pool.len(), 0);
}

#[test]
fn test_buffer_pool_with_capacity() {
    let pool = BufferPool::new(100);
    assert_eq!(pool.capacity(), 100);
    assert!(pool.is_empty());
}

#[test]
fn test_buffer_pool_with_prefetch() {
    let pool = BufferPool::with_prefetch(50, 5);
    assert_eq!(pool.capacity(), 50);
    assert!(pool.is_empty());
}

// ============================================================================
// Page Insertion and Retrieval Tests
// ============================================================================

#[test]
fn test_insert_and_get_single_page() {
    let pool = BufferPool::new(10);

    let page = pool.allocate(1);
    assert_eq!(page.page_id(), 1);

    let retrieved = pool.get(1);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().page_id(), 1);
}

#[test]
fn test_insert_multiple_pages() {
    let pool = BufferPool::new(10);

    // Insert pages 1-5
    for i in 1..=5 {
        pool.allocate(i);
    }

    assert_eq!(pool.len(), 5);
    assert!(!pool.is_empty());

    // Verify all pages are retrievable
    for i in 1..=5 {
        let page = pool.get(i);
        assert!(page.is_some());
        assert_eq!(page.unwrap().page_id(), i);
    }
}

#[test]
fn test_get_nonexistent_page() {
    let pool = BufferPool::new(10);

    let page = pool.get(999);
    assert!(page.is_none());
}

#[test]
fn test_insert_duplicate_page() {
    let pool = BufferPool::new(10);

    let _page1 = pool.allocate(1);
    let _page2 = pool.allocate(1); // Same page_id

    // Should still only have one page
    assert_eq!(pool.len(), 1);

    // The retrieved page should have page_id 1
    let retrieved = pool.get(1);
    assert!(retrieved.is_some());
}

// ============================================================================
// LRU Eviction Tests
// ============================================================================

#[test]
fn test_lru_eviction_on_capacity() {
    let pool = BufferPool::new(3); // Small capacity for testing

    // Fill up the pool
    pool.allocate(1);
    pool.allocate(2);
    pool.allocate(3);
    assert_eq!(pool.len(), 3);

    // Adding a 4th page should evict page 1 (least recently used)
    pool.allocate(4);
    assert_eq!(pool.len(), 3);

    // Page 1 should be evicted
    assert!(pool.get(1).is_none());

    // Pages 2, 3, 4 should still be there
    assert!(pool.get(2).is_some());
    assert!(pool.get(3).is_some());
    assert!(pool.get(4).is_some());
}

#[test]
fn test_lru_order_after_get() {
    let pool = BufferPool::new(3);

    // Insert pages 1, 2, 3
    pool.allocate(1);
    pool.allocate(2);
    pool.allocate(3);

    // Access page 1 (makes it most recently used)
    let _ = pool.get(1);

    // Add page 4 - should evict page 2 (not page 1)
    pool.allocate(4);

    // Page 1 should still be accessible
    assert!(pool.get(1).is_some());
    // Page 2 should be evicted
    assert!(pool.get(2).is_none());
}

#[test]
fn test_lru_order_after_insert() {
    let pool = BufferPool::new(3);

    pool.allocate(1);
    pool.allocate(2);
    pool.allocate(3);

    // Insert page 1 again (refreshes its LRU position)
    pool.allocate(1);

    // Add page 4 - should evict page 2 (page 1 was refreshed)
    pool.allocate(4);

    assert!(pool.get(1).is_some());
    assert!(pool.get(2).is_none());
    assert!(pool.get(3).is_some());
    assert!(pool.get(4).is_some());
}

// ============================================================================
// Page Removal Tests
// ============================================================================

#[test]
fn test_remove_page() {
    let pool = BufferPool::new(10);

    pool.allocate(1);
    pool.allocate(2);
    assert_eq!(pool.len(), 2);

    let removed = pool.remove(1);
    assert!(removed);
    assert_eq!(pool.len(), 1);
    assert!(pool.get(1).is_none());
    assert!(pool.get(2).is_some());
}

#[test]
fn test_remove_nonexistent_page() {
    let pool = BufferPool::new(10);

    pool.allocate(1);
    let removed = pool.remove(999);
    assert!(!removed);
    assert_eq!(pool.len(), 1);
}

#[test]
fn test_clear_pool() {
    let pool = BufferPool::new(10);

    pool.allocate(1);
    pool.allocate(2);
    pool.allocate(3);
    assert_eq!(pool.len(), 3);

    pool.clear();
    assert!(pool.is_empty());
    assert_eq!(pool.len(), 0);
}

// ============================================================================
// Statistics Tests
// ============================================================================

#[test]
fn test_stats_initial() {
    let pool = BufferPool::new(10);
    let stats = pool.stats();

    assert_eq!(stats.hits, 0);
    assert_eq!(stats.misses, 0);
    assert_eq!(stats.evictions, 0);
    assert_eq!(stats.prefetch_hits, 0);
}

#[test]
fn test_stats_hits_and_misses() {
    let pool = BufferPool::new(10);

    pool.allocate(1);

    // Miss (not in pool)
    let _ = pool.get(999);

    // Hit
    let _ = pool.get(1);

    let stats = pool.stats();
    assert_eq!(stats.hits, 1);
    assert_eq!(stats.misses, 1);
}

#[test]
fn test_stats_evictions() {
    let pool = BufferPool::new(2);

    pool.allocate(1);
    pool.allocate(2);
    // This should trigger one eviction
    pool.allocate(3);

    let stats = pool.stats();
    assert_eq!(stats.evictions, 1);
}

#[test]
fn test_hit_rate_calculation() {
    let pool = BufferPool::new(10);

    pool.allocate(1);

    // 3 hits, 1 miss
    let _ = pool.get(1);
    let _ = pool.get(1);
    let _ = pool.get(1);
    let _ = pool.get(999);

    let stats = pool.stats();
    assert_eq!(stats.hits, 3);
    assert_eq!(stats.misses, 1);
    assert!((stats.hit_rate() - 0.75).abs() < 0.001);
}

// ============================================================================
// Get or Load Tests
// ============================================================================

#[test]
fn test_get_or_load_existing() {
    let pool = BufferPool::new(10);
    let loader = create_page_loader();

    pool.allocate(1);

    // Should get existing page, not call loader
    let page = pool.get_or_load(1, |_id| {
        panic!("Loader should not be called for existing page")
    });

    assert_eq!(page.page_id(), 1);
}

#[test]
fn test_get_or_load_missing() {
    let pool = BufferPool::new(10);
    let loader = create_page_loader();

    // Should load page 1 using the loader
    let page = pool.get_or_load(1, &loader);

    assert_eq!(page.page_id(), 1);
    assert!(pool.get(1).is_some());
}

// ============================================================================
// Prefetch Tests
// ============================================================================

#[test]
fn test_prefetch() {
    let pool = BufferPool::new(10);
    let loader = create_page_loader();

    pool.prefetch(&[1, 2, 3], &loader);

    // Pages may or may not be in pool depending on implementation
    // The important thing is that prefetch doesn't panic
}

#[test]
fn test_prefetch_range() {
    let pool = BufferPool::new(100);
    let loader = create_page_loader();

    pool.prefetch_range(1, 10, &loader);

    // Prefetch behavior may vary by implementation
    // At minimum verify it doesn't crash
}

// ============================================================================
// Capacity and Size Tests
// ============================================================================

#[test]
fn test_capacity_unchanged_after_operations() {
    let pool = BufferPool::new(5);

    pool.allocate(1);
    pool.allocate(2);
    pool.remove(1);

    assert_eq!(pool.capacity(), 5);
}

#[test]
fn test_len_after_operations() {
    let pool = BufferPool::new(10);

    assert_eq!(pool.len(), 0);

    pool.allocate(1);
    assert_eq!(pool.len(), 1);

    pool.allocate(2);
    assert_eq!(pool.len(), 2);

    pool.remove(1);
    assert_eq!(pool.len(), 1);

    pool.clear();
    assert_eq!(pool.len(), 0);
}

#[test]
fn test_is_empty() {
    let pool = BufferPool::new(10);

    assert!(pool.is_empty());

    pool.allocate(1);
    assert!(!pool.is_empty());

    pool.remove(1);
    assert!(pool.is_empty());
}

// ============================================================================
// Concurrent-Like Access Patterns (Sequential Access)
// ============================================================================

#[test]
fn test_sequential_page_access() {
    let pool = BufferPool::new(5);

    // Sequential access pattern
    for i in 0..100 {
        let page_id = i % 10;
        let page = pool.get_or_load(page_id, &|id| Arc::new(Page::new(id)));

        // Touch the page to simulate use
        assert_eq!(page.page_id(), page_id);
    }

    // Pool should have some pages cached
    assert!(pool.len() > 0);
}

#[test]
fn test_random_page_access() {
    let pool = BufferPool::new(10);

    let page_ids = [5, 2, 8, 1, 9, 3, 7, 4, 6, 0];

    for &page_id in &page_ids {
        pool.get_or_load(page_id, &|id| Arc::new(Page::new(id)));
    }

    // All pages should be in the pool
    for &page_id in &page_ids {
        assert!(pool.get(page_id).is_some());
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_zero_capacity() {
    let pool = BufferPool::new(0);
    assert_eq!(pool.capacity(), 0);
    assert!(pool.is_empty());

    // Allocating with zero capacity - behavior depends on implementation
    pool.allocate(1);
    // Pool may or may not hold the page depending on eviction logic
}

#[test]
fn test_very_large_capacity() {
    let pool = BufferPool::new(1_000_000);
    assert_eq!(pool.capacity(), 1_000_000);
    assert!(pool.is_empty());
}

#[test]
fn test_page_id_zero() {
    let pool = BufferPool::new(10);

    pool.allocate(0);
    assert!(pool.get(0).is_some());

    pool.remove(0);
    assert!(pool.get(0).is_none());
}

#[test]
fn test_large_page_id() {
    let pool = BufferPool::new(10);

    pool.allocate(u32::MAX);
    assert!(pool.get(u32::MAX).is_some());
}

// ============================================================================
// Performance-Related Tests
// ============================================================================

#[test]
fn test_many_small_allocations() {
    let pool = BufferPool::new(100);

    for i in 0..1000 {
        pool.allocate(i);
    }

    // Pool should be at capacity
    assert_eq!(pool.len(), 100);
}

#[test]
fn test_many_allocations_with_eviction() {
    let pool = BufferPool::new(50);

    for i in 0..100 {
        pool.allocate(i);
    }

    // Should have evicted pages
    let stats = pool.stats();
    assert!(stats.evictions > 0);

    // Pool should be at or near capacity
    assert!(pool.len() <= 50);
}

#[test]
fn test_repeated_get_same_page() {
    let pool = BufferPool::new(10);

    pool.allocate(1);

    // Repeatedly get the same page
    for _ in 0..1000 {
        let page = pool.get(1);
        assert!(page.is_some());
    }

    // Should have many hits
    let stats = pool.stats();
    assert_eq!(stats.hits, 1000);
}

// ============================================================================
// Stress Tests
// ============================================================================

#[test]
fn test_stress_many_pages() {
    let pool = BufferPool::new(1000);

    // Allocate many pages
    for i in 0..500 {
        pool.allocate(i);
    }

    // Access many pages
    for i in 0..500 {
        let _ = pool.get(i);
    }

    // Add more pages to trigger eviction
    for i in 500..1500 {
        pool.allocate(i);
    }

    // Stats should show significant activity
    let stats = pool.stats();
    assert!(stats.evictions > 0 || stats.hits > 0);
}

#[test]
fn test_alternating_access_pattern() {
    let pool = BufferPool::new(3);

    // Alternating access pattern: 1, 2, 3, 1, 2, 3, ...
    for _ in 0..100 {
        for &page_id in &[1, 2, 3] {
            pool.get_or_load(page_id, &|id| Arc::new(Page::new(id)));
        }
    }

    // All pages should still be in pool due to repeated access
    assert!(pool.get(1).is_some());
    assert!(pool.get(2).is_some());
    assert!(pool.get(3).is_some());

    // Many hits should be recorded
    let stats = pool.stats();
    assert!(stats.hits > 100);
}
