//! WHERE × Multi-Join Tests
//!
//! Verifies that WHERE predicates resolve correctly against the accumulated
//! schema produced by chained JOINs. The accumulated schema prefixes columns
//! (e.g. `a_join_b.col`, then `a_join_b_join_c.col` for a third join), so
//! the column lookup must match by suffix when the user references an
//! intermediate table by its original name.

use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::MemoryStorage;
use sqlrustgo_types::Value;
use std::sync::{Arc, RwLock};

fn create_engine() -> ExecutionEngine<MemoryStorage> {
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    ExecutionEngine::new(storage)
}

#[test]
fn test_multi_join_where_filters_by_first_table() {
    // 3-table join with WHERE on a column of the first table.
    // WHERE must resolve `a.tag` against the accumulated schema.
    let mut engine = create_engine();
    engine
        .execute("CREATE TABLE a (id INTEGER, tag TEXT)")
        .unwrap();
    engine
        .execute("CREATE TABLE b (aid INTEGER, val INTEGER)")
        .unwrap();
    engine
        .execute("CREATE TABLE c (vid INTEGER, note TEXT)")
        .unwrap();
    engine
        .execute("INSERT INTO a VALUES (1, 'x'), (2, 'y')")
        .unwrap();
    engine
        .execute("INSERT INTO b VALUES (1, 10), (2, 20)")
        .unwrap();
    engine.execute("INSERT INTO c VALUES (10, 'hit')").unwrap();

    let result = engine
        .execute(
            "SELECT * FROM a \
             JOIN b ON a.id = b.aid \
             JOIN c ON b.val = c.vid \
             WHERE a.tag = 'x'",
        )
        .unwrap();

    assert_eq!(
        result.rows.len(),
        1,
        "WHERE a.tag='x' should keep 1 row, got {:?}",
        result.rows
    );
}

#[test]
fn test_multi_join_where_filters_by_intermediate_table() {
    // 3-table join with WHERE on the middle (intermediate) table.
    // This is the case that breaks with prefixed accumulated column names:
    // `b.val` in WHERE must still resolve to the middle table's column.
    let mut engine = create_engine();
    engine
        .execute("CREATE TABLE a (id INTEGER, tag TEXT)")
        .unwrap();
    engine
        .execute("CREATE TABLE b (aid INTEGER, val INTEGER)")
        .unwrap();
    engine
        .execute("CREATE TABLE c (vid INTEGER, note TEXT)")
        .unwrap();
    engine
        .execute("INSERT INTO a VALUES (1, 'x'), (2, 'y'), (3, 'z')")
        .unwrap();
    engine
        .execute("INSERT INTO b VALUES (1, 10), (2, 20), (3, 30)")
        .unwrap();
    engine
        .execute("INSERT INTO c VALUES (10, 'ten'), (20, 'twenty')")
        .unwrap();

    let result = engine
        .execute(
            "SELECT * FROM a \
             JOIN b ON a.id = b.aid \
             JOIN c ON b.val = c.vid \
             WHERE b.val > 15",
        )
        .unwrap();

    // b.val > 15 → keep rows where b.val=20 (aid=2) and b.val=30 (aid=3).
    // c matches only vid=10 and vid=20, so a.id=2 (b.val=20, c.vid=20) survives.
    assert_eq!(
        result.rows.len(),
        1,
        "WHERE b.val>15 should keep 1 row (a.id=2), got {:?}",
        result.rows
    );
}

#[test]
fn test_multi_join_where_no_match() {
    // WHERE clause that excludes everything — verify the filter still runs.
    let mut engine = create_engine();
    engine.execute("CREATE TABLE a (id INTEGER)").unwrap();
    engine.execute("CREATE TABLE b (aid INTEGER)").unwrap();
    engine.execute("INSERT INTO a VALUES (1), (2)").unwrap();
    engine.execute("INSERT INTO b VALUES (1), (2)").unwrap();

    let result = engine
        .execute("SELECT * FROM a JOIN b ON a.id = b.aid WHERE a.id = 999")
        .unwrap();

    assert_eq!(result.rows.len(), 0);
}
