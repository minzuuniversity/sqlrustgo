//! Multi-Column ON Tests
//!
//! TPC-H Q9: `JOIN lineitem ON partsupp.ps_partkey = lineitem.l_partkey
//! AND partsupp.ps_suppkey = lineitem.l_suppkey`. The join key is the
//! composite (ps_partkey, ps_suppkey) pair, not a single column.
//! Currently `find_join_key_index` only accepts a single equality
//! (Pair); the AND of two equalities triggers
//! "Join condition must reference one column from each side".
//!
//! These tests cover the simplest non-trivial multi-column ON cases.

use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use sqlrustgo_types::Value;
use std::sync::{Arc, RwLock};

fn create_engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

#[test]
fn test_two_column_and_join_matches_both() {
    // Two-column AND join: row matches only if BOTH keys match.
    let mut engine = create_engine();
    engine
        .execute("CREATE TABLE a (id INTEGER, sub_id INTEGER)")
        .unwrap();
    engine
        .execute("CREATE TABLE b (a_id INTEGER, a_sub INTEGER)")
        .unwrap();
    engine
        .execute("INSERT INTO a VALUES (1, 10), (1, 11), (2, 10)")
        .unwrap();
    engine
        .execute("INSERT INTO b VALUES (1, 10), (1, 11), (2, 99)")
        .unwrap();

    // b rows that match a on (id, sub_id): (1,10) and (1,11).
    // (2, 99) does NOT match (2, 10) — different sub_id.
    let result = engine
        .execute(
            "SELECT a.id, a.sub_id \
             FROM a JOIN b ON a.id = b.a_id AND a.sub_id = b.a_sub",
        )
        .unwrap();

    assert_eq!(
        result.rows.len(),
        2,
        "two-column AND join should yield 2 rows, got {:?}",
        result.rows
    );
}

#[test]
fn test_two_column_and_join_no_match() {
    // Same key on the first column but different second column — no
    // rows should match. Verifies the AND-of-equalities is treated as a
    // single composite key, not a single-column match.
    let mut engine = create_engine();
    engine
        .execute("CREATE TABLE a (id INTEGER, sub_id INTEGER)")
        .unwrap();
    engine
        .execute("CREATE TABLE b (a_id INTEGER, a_sub INTEGER)")
        .unwrap();
    engine.execute("INSERT INTO a VALUES (1, 10)").unwrap();
    engine.execute("INSERT INTO b VALUES (1, 99)").unwrap();

    let result = engine
        .execute("SELECT a.id FROM a JOIN b ON a.id = b.a_id AND a.sub_id = b.a_sub")
        .unwrap();

    assert_eq!(
        result.rows.len(),
        0,
        "different sub_id should produce no matches, got {:?}",
        result.rows
    );
}
