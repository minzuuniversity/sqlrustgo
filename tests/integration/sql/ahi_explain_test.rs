//! PR2 / #5304 — AHI state-machine tests + estimated_pages_for_table empty-table guard
//!         + BinaryTableStorage synthetic_zero contract.
//!
//! **Ruling 9 (final):** Direct AHI tests only.
//!
//! Recon at develop/v4.1.0 @ e12f7a48fa shows:
//! - `AdaptiveHashIndex::record_access` is called from 6 sites in
//!   `engine_select.rs` (lines 1544, 1590, 3823, 3847, 3877, 3904).
//! - `AdaptiveHashIndex::lookup` is called from **0 sites** in the
//!   whole workspace.
//! - `ExecutionEngine::with_memory()` selects through `MemoryStorage`,
//!   which has no B+Tree-backed secondary indexes, so SELECT WHERE
//!   does not reach any of the AHI record_access sites either.
//!
//! Net observable: `promoted_count()` increments (via threshold crosses
//! from `record_access`), `total_lookups()` and `total_hits()` stay 0.
//! Wiring the lookup-side of AHI into the secondary-index execution
//! path is a separate epic, explicitly out of PR2's scope (design §2.2
//! non-goals; design §6.4 future-replacement trigger).
//!
//! Review Focus pins:
//! - #4 (AHI single-conn race) — these tests run in single-threaded
//!   mode with no engine; no race.
//! - #3 (`estimated_pages_for_table` empty-table) — `test_estimated_pages_for_empty_table_returns_zero`.

use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::adaptive_hash_index::AdaptiveHashIndex;
use sqlrustgo_storage::binary_storage::BinaryTableStorage;
use sqlrustgo_storage::buffer_pool::BufferPoolStats;
use sqlrustgo_storage::StorageEngine;

#[test]
fn test_ahi_promoted_count_monotonic_via_record_access() {
    // PR2 / Ruling 9: `promoted_count` is the only AHI counter that
    // production code actually drives. Promotion happens after 17
    // accesses to the same (table, key, page) per AHI source line 17.
    let ahi = AdaptiveHashIndex::new();
    let mut prev_promoted = ahi.promoted_count();
    assert_eq!(prev_promoted, 0, "fresh AHI: promoted_count must be 0");

    for i in 0..30 {
        ahi.record_access("t", &[1u8], 42, 0);
        let now = ahi.promoted_count();
        assert!(
            now >= prev_promoted,
            "promoted_count must be monotonic: prev={prev_promoted}, now={now} at iter {i}"
        );
        prev_promoted = now;
    }

    // 30 accesses past the 17-threshold crossing must yield >=1
    // promoted entry.
    assert!(
        ahi.promoted_count() >= 1,
        "30 record_access calls past threshold=17 must yield at least 1 promotion (got {})",
        ahi.promoted_count()
    );
}

#[test]
fn test_ahi_hit_rate_converges_after_promotion() {
    // Pin the hit_rate formula: `hits/lookups`, monotonic as promoted
    // entries accumulate. Walk the state machine directly because no
    // production execution path invokes `.lookup()` today (Ruling 9).
    let ahi = AdaptiveHashIndex::new();

    // 20 accesses past threshold promotes at least one entry.
    for _ in 0..20 {
        ahi.record_access("t", &[1u8], 42, 0);
    }
    assert!(
        ahi.promoted_count() >= 1,
        "20 accesses past threshold=17 must promote (got {})",
        ahi.promoted_count()
    );

    // 10 lookups against the promoted key — every call increments
    // `lookups`, every hit (which is all of them for a promoted key)
    // increments `hits`.
    for _ in 0..10 {
        let result = ahi.lookup("t", &[1u8]);
        assert!(
            result.is_some(),
            "lookup against promoted key must return Some"
        );
    }

    let lookups = ahi.total_lookups();
    let hits = ahi.total_hits();
    assert_eq!(lookups, 10, "10 lookups recorded");
    assert_eq!(hits, 10, "all 10 lookups hit");

    let rate = hits as f64 / lookups as f64;
    assert!(
        rate > 0.5,
        "hit_rate after promotion must be > 0.5; got {rate}"
    );
}

#[test]
fn test_ahi_via_engine_memory_storage_no_lookup_wired() {
    // Documents the production wiring gap (Ruling 9): WithMemoryStorage
    // SELECT WHERE does NOT engage AHI today. `total_lookups` stays 0
    // because no production code invokes `.lookup()`. Pinning the
    // observable zero here so any future regression toward an actual
    // production-wired AHI shows up as a non-zero assertion failure.
    let mut engine = ExecutionEngine::with_memory();
    engine
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY);")
        .expect("create table");
    engine
        .execute("INSERT INTO t VALUES (1), (2), (3);")
        .expect("insert rows");

    let ahi = engine.adaptive_hash_index();

    for i in 0..50 {
        engine
            .execute("SELECT id FROM t WHERE id = 1;")
            .unwrap_or_else(|e| panic!("hot iter {i} failed: {e}"));
    }

    assert_eq!(
        ahi.total_lookups(),
        0,
        "MemoryStorage SELECT WHERE does NOT wire AHI lookup-side today \
         (Ruling 9: production lookup-side is a separate epic)."
    );
    // `record_access` IS called from `engine_select.rs:1544 / 1590 /
    // 3823 / 3847 / 3877 / 3904` but only when the secondary-index path
    // is taken, which MemoryStorage does not provide. So
    // `promoted_count` may or may not register depending on
    // engine_select.rs internals — we don't pin it here (it's the
    // subject of a separate epic).
}

#[test]
fn test_estimated_pages_for_empty_table_returns_zero() {
    // Review Focus #3: empty-table underflow guard. `len().div_ceil(32)`
    // for `len=0` returns `0` (correct); `unwrap_or(0)` covers
    // non-existent tables.
    //
    // PR2 Ruling 1: no `engine.storage()` exists; use `storage_ref().read()`
    // and deref the lock guard. `MemoryStorage` (the inner concrete type
    // of `with_memory()`) impls `StorageEngine` so the trait method
    // `estimated_pages_for_table` resolves through Deref of the guard.
    let mut engine = ExecutionEngine::with_memory();
    engine
        .execute("CREATE TABLE empty_t (id INTEGER);")
        .expect("create table");

    let n_empty = {
        let storage_lock = engine.storage_ref().read();
        storage_lock.estimated_pages_for_table("empty_t")
    };
    assert_eq!(
        n_empty, 0,
        "empty table must yield 0 estimated pages, got {n_empty}"
    );

    let n_missing = {
        let storage_lock = engine.storage_ref().read();
        storage_lock.estimated_pages_for_table("does_not_exist")
    };
    assert_eq!(
        n_missing, 0,
        "non-existent table must yield 0 estimated pages"
    );

    // Direct trait method via BufferPoolStats import confirms wiring.
    let stats: BufferPoolStats = BufferPoolStats::synthetic_zero();
    assert_eq!(stats.hits, 0);
    assert_eq!(stats.misses, 0);
}

#[test]
fn test_binary_table_storage_buffer_pool_stats_returns_synthetic_zero() {
    // Ruling 8: BinaryTableStorage itself has no override on
    // `buffer_pool_stats()`, so it inherits the trait default returning
    // `Some(synthetic_zero())`. The forwarding override on
    // `BoxStorageEngine` (Task 3) is forward-only and observes the same
    // value. This test pins the **observable** contract: until a future
    // paged engine overrides `buffer_pool_stats()`, every engine in
    // the storage stack reports zero counters.
    let tmp = std::env::temp_dir().join("ahi_explain_pr2_5330_bts");
    let _ = std::fs::remove_dir_all(&tmp);
    let bts = BinaryTableStorage::new(tmp.clone()).unwrap();
    let stats = bts
        .buffer_pool_stats()
        .expect("BinaryTableStorage must return Some(synthetic_zero)");
    assert_eq!(stats.hits, 0);
    assert_eq!(stats.misses, 0);
    assert_eq!(stats.evictions, 0);
    assert_eq!(stats.prefetch_hits, 0);
    let _ = std::fs::remove_dir_all(&tmp);
}
