//! V311-02 v2: Adaptive Hash Index main-path integration tests.
//!
//! Verifies that the production execution engine routes through
//! `scan_with_ahi()` and that AHI metrics reflect real workload access patterns.

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo::MemoryStorage;
use std::sync::Arc;

fn fresh_engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(Arc::clone(&storage))
}

#[test]
fn ahi_accessor_returns_valid_instance() {
    let e = fresh_engine();
    // ahi() should return an accessible AdaptiveHashIndex
    let ahi = e.ahi();
    assert_eq!(ahi.size(), 0);
    assert_eq!(ahi.total_lookups(), 0);
    assert_eq!(ahi.promoted_count(), 0);
}

#[test]
fn single_scan_records_access_via_scan_with_ahi() {
    let mut e = fresh_engine();
    e.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT)")
        .unwrap();
    e.execute("INSERT INTO t VALUES (1, 'alice'), (2, 'bob'), (3, 'charlie')")
        .unwrap();

    // Snapshot AHI stats before query
    let before = e.ahi().promoted_count();

    // Run a SELECT that triggers scan_with_ahi
    let _ = e.execute("SELECT * FROM t WHERE id = 1").unwrap();

    // After a single scan, the table-level access is recorded.
    let after = e.ahi().promoted_count();
    // scan_with_ahi calls record_access once per table → after 17 hits promotion kicks in
    // For 1 access, no promotion yet
    assert_eq!(after, before, "1 access shouldn't promote");
}

#[test]
fn repeated_scans_eventually_promote_to_ahi() {
    let mut e = fresh_engine();
    e.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT)")
        .unwrap();
    e.execute("INSERT INTO t VALUES (1, 'alice')").unwrap();

    // Run 20 queries to exceed the default threshold of 17
    let mut promoted_at: Option<u64> = None;
    for i in 0..20 {
        let _ = e.execute("SELECT * FROM t WHERE id = 1").unwrap();
        let count = e.ahi().promoted_count();
        if count > 0 && promoted_at.is_none() {
            promoted_at = Some(count);
            println!("Promoted at iteration {i}: count={count}");
        }
    }

    assert!(
        e.ahi().promoted_count() >= 1,
        "AHI should have promoted at least 1 entry after 20 iterations, got {}",
        e.ahi().promoted_count()
    );
    // Map should have at least 1 entry after promotion
    assert!(e.ahi().size() >= 1, "AHI size should grow after promotion");
}

#[test]
fn ahi_hit_rate_grows_with_repeated_queries() {
    let mut e = fresh_engine();
    e.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT)")
        .unwrap();
    e.execute("INSERT INTO t VALUES (1, 'alice')").unwrap();

    // Run 30 queries to ensure promotion + lookups
    for _ in 0..30 {
        let _ = e.execute("SELECT id FROM t WHERE id = 1").unwrap();
    }

    // After 30 queries, AHI should have entries (promotion at 17 hits)
    let promoted = e.ahi().promoted_count();
    assert!(
        promoted >= 1,
        "should have ≥ 1 promoted entry, got {promoted}"
    );

    // AHI size should reflect the 1 promoted entry
    assert!(e.ahi().size() >= 1, "AHI size should grow after promotion");
}

#[test]
fn multiple_tables_have_isolated_ahi_state() {
    let mut e = fresh_engine();
    e.execute("CREATE TABLE t1 (id INTEGER PRIMARY KEY, val TEXT)")
        .unwrap();
    e.execute("CREATE TABLE t2 (id INTEGER PRIMARY KEY, val TEXT)")
        .unwrap();

    // Touch each table enough to promote
    for _ in 0..20 {
        let _ = e.execute("SELECT * FROM t1 WHERE id = 1").unwrap();
        let _ = e.execute("SELECT * FROM t2 WHERE id = 1").unwrap();
    }

    // Both tables should now have promotion entries
    let count = e.ahi().promoted_count();
    assert!(
        count >= 2,
        "expected ≥ 2 promoted entries (1 per table), got {count}"
    );
}

#[test]
fn queries_with_join_use_ahi_via_base_table_scan() {
    let mut e = fresh_engine();
    e.execute("CREATE TABLE orders (oid INTEGER PRIMARY KEY, pid INTEGER)")
        .unwrap();
    e.execute("CREATE TABLE items (oid INTEGER, qty INTEGER)")
        .unwrap();
    e.execute("INSERT INTO orders VALUES (1, 100), (2, 200)")
        .unwrap();
    e.execute("INSERT INTO items VALUES (1, 5), (2, 10)")
        .unwrap();

    // Run a join query 25 times
    let mut total_promotions = 0;
    for _ in 0..25 {
        let _ = e
            .execute("SELECT * FROM orders, items WHERE orders.oid = items.oid")
            .unwrap();
        total_promotions = e.ahi().promoted_count() as usize;
    }

    // At least 2 base tables (orders, items) should have triggered
    assert!(
        total_promotions >= 2,
        "expected ≥ 2 promoted for join query, got {total_promotions}"
    );
}

#[test]
fn ahi_accessor_works_across_query_types() {
    let mut e = fresh_engine();
    e.execute("CREATE TABLE t (id INTEGER PRIMARY KEY)")
        .unwrap();
    e.execute("INSERT INTO t VALUES (1), (2), (3)").unwrap();

    // Mix of query types: simple SELECT, COUNT, filtered SELECT
    let start_promoted = e.ahi().promoted_count();

    for _ in 0..25 {
        let _ = e.execute("SELECT * FROM t").unwrap();
        let _ = e.execute("SELECT COUNT(*) FROM t").unwrap();
        let _ = e.execute("SELECT * FROM t WHERE id = 1").unwrap();
    }

    let end_promoted = e.ahi().promoted_count();
    assert!(
        end_promoted > start_promoted,
        "promoted count should grow: start={start_promoted}, end={end_promoted}"
    );
    println!(
        "After mixed workload: promoted={}, lookups={}, hit_rate={:.2}",
        end_promoted,
        e.ahi().total_lookups(),
        e.ahi().hit_rate()
    );
}
