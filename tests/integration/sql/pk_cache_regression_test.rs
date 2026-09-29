//! Regression suite for the INSERT duplicate-key snapshot cache
//! (`ExecutionEngine::pk_lookup_cache`).
//!
//! The cache exists because `execute_insert` used to call `storage.scan()`
//! on every statement, making N single-row INSERTs into a table with a
//! PRIMARY KEY cost O(N^2). These tests pin the behaviour the optimisation
//! must not change: duplicates are still rejected, new keys still insert,
//! and every mutation that removes or renames a key invalidates the cache.
//!
//! They also assert the property that makes the cache safe — after any
//! sequence of INSERT / UPDATE / DELETE / TRUNCATE, a re-used primary key is
//! still detected as a duplicate.

use sqlrustgo::{ExecutionEngine, MemoryStorage};
use std::sync::Arc;

/// Assert that `sql` fails and that the message names a duplicate-key
/// violation. The engine reports the conflict by column name, so match on
/// "duplicate" rather than an exact string.
fn assert_duplicate(engine: &mut ExecutionEngine<MemoryStorage>, sql: &str, what: &str) {
    match engine.execute(sql) {
        Ok(_) => panic!("{what}: expected a duplicate-key error, insert succeeded"),
        Err(e) => {
            let msg = e.to_string().to_lowercase();
            assert!(
                msg.contains("duplicate"),
                "{what}: expected a duplicate-key error, got: {e}"
            );
        }
    }
}

fn row_count(engine: &mut ExecutionEngine<MemoryStorage>) -> i64 {
    let r = engine.execute("SELECT COUNT(*) FROM t").unwrap();
    r.rows[0][0].as_integer().expect("COUNT is an integer")
}

fn engine() -> ExecutionEngine<MemoryStorage> {
    ExecutionEngine::new(Arc::new(parking_lot::RwLock::new(MemoryStorage::new())))
}

#[test]
fn insert_pk_rows_then_reject_duplicate() {
    let mut e = engine();
    e.execute("CREATE TABLE t (k INTEGER PRIMARY KEY, v TEXT)")
        .unwrap();
    for i in 1..=50 {
        e.execute(&format!("INSERT INTO t VALUES ({i}, 'v{i}')"))
            .unwrap();
    }
    assert_eq!(row_count(&mut e), 50);
    assert_duplicate(&mut e, "INSERT INTO t VALUES (7, 'dup')", "existing key");
    assert_duplicate(&mut e, "INSERT INTO t VALUES (1, 'dup')", "first key");
    // The rejected inserts must not have been applied.
    assert_eq!(row_count(&mut e), 50);
}

#[test]
fn delete_frees_the_key_and_cache_does_not_mask_it() {
    let mut e = engine();
    e.execute("CREATE TABLE t (k INTEGER PRIMARY KEY, v TEXT)")
        .unwrap();
    for i in 1..=20 {
        e.execute(&format!("INSERT INTO t VALUES ({i}, 'v{i}')"))
            .unwrap();
    }
    e.execute("DELETE FROM t WHERE k = 5").unwrap();
    assert_eq!(row_count(&mut e), 19);
    // The freed key must be insertable again, and a still-present key must
    // still collide — i.e. the cache was invalidated, not blindly reused.
    e.execute("INSERT INTO t VALUES (5, 'again')").unwrap();
    assert_duplicate(&mut e, "INSERT INTO t VALUES (6, 'dup')", "surviving key");
    assert_eq!(row_count(&mut e), 20);
}

#[test]
fn truncate_clears_cache_so_first_key_is_free_again() {
    let mut e = engine();
    e.execute("CREATE TABLE t (k INTEGER PRIMARY KEY, v TEXT)")
        .unwrap();
    for i in 1..=20 {
        e.execute(&format!("INSERT INTO t VALUES ({i}, 'v{i}')"))
            .unwrap();
    }
    e.execute("TRUNCATE TABLE t").unwrap();
    assert_eq!(row_count(&mut e), 0);
    // Every key was freed; re-inserting key 1 must not be reported as a
    // duplicate from the pre-truncate snapshot.
    e.execute("INSERT INTO t VALUES (1, 'fresh')").unwrap();
    assert_duplicate(&mut e, "INSERT INTO t VALUES (1, 'dup')", "after truncate");
    assert_eq!(row_count(&mut e), 1);
}

#[test]
fn update_of_primary_key_invalidates_cache() {
    let mut e = engine();
    e.execute("CREATE TABLE t (k INTEGER PRIMARY KEY, v TEXT)")
        .unwrap();
    for i in 1..=20 {
        e.execute(&format!("INSERT INTO t VALUES ({i}, 'v{i}')"))
            .unwrap();
    }
    e.execute("UPDATE t SET k = 500 WHERE k = 3").unwrap();
    // 3 was freed, 500 was taken.
    e.execute("INSERT INTO t VALUES (3, 'reuse')").unwrap();
    assert_duplicate(&mut e, "INSERT INTO t VALUES (500, 'dup')", "new pk");
    assert_duplicate(&mut e, "INSERT INTO t VALUES (4, 'dup')", "untouched key");
}

#[test]
fn drop_table_then_recreate_does_not_inherit_stale_cache() {
    let mut e = engine();
    e.execute("CREATE TABLE t (k INTEGER PRIMARY KEY, v TEXT)")
        .unwrap();
    for i in 1..=20 {
        e.execute(&format!("INSERT INTO t VALUES ({i}, 'v{i}')"))
            .unwrap();
    }
    e.execute("DROP TABLE t").unwrap();
    e.execute("CREATE TABLE t (k INTEGER PRIMARY KEY, v TEXT)")
        .unwrap();
    // The new table is empty, so key 1 must be free.
    e.execute("INSERT INTO t VALUES (1, 'reborn')").unwrap();
    assert_duplicate(&mut e, "INSERT INTO t VALUES (1, 'dup')", "recreated");
    assert_eq!(row_count(&mut e), 1);
}

#[test]
fn bulk_multi_row_insert_and_duplicates_within_the_statement() {
    let mut e = engine();
    e.execute("CREATE TABLE t (k INTEGER PRIMARY KEY, v TEXT)")
        .unwrap();
    e.execute("INSERT INTO t VALUES (1,'a'), (2,'b'), (3,'c')")
        .unwrap();
    assert_eq!(row_count(&mut e), 3);
    // A key already on disk must still collide.
    assert_duplicate(
        &mut e,
        "INSERT INTO t VALUES (2,'dup'), (4,'d')",
        "mixed bulk",
    );
}

#[test]
fn insert_ignore_and_replace_respect_the_cache() {
    let mut e = engine();
    e.execute("CREATE TABLE t (k INTEGER PRIMARY KEY, v TEXT)")
        .unwrap();
    for i in 1..=10 {
        e.execute(&format!("INSERT INTO t VALUES ({i}, 'v{i}')"))
            .unwrap();
    }
    // INSERT IGNORE silently skips the duplicate.
    e.execute("INSERT IGNORE INTO t VALUES (3, 'ignored')")
        .unwrap();
    assert_eq!(row_count(&mut e), 10);
    // The ignored row must not have replaced the original.
    let v = e.execute("SELECT v FROM t WHERE k = 3").unwrap();
    assert_eq!(v.rows[0][0].to_sql_string(), "v3");

    // REPLACE overwrites in place.
    e.execute("REPLACE INTO t VALUES (3, 'replaced')").unwrap();
    let v = e.execute("SELECT v FROM t WHERE k = 3").unwrap();
    assert_eq!(v.rows[0][0].to_sql_string(), "replaced");
    assert_eq!(row_count(&mut e), 10);
}

#[test]
fn composite_primary_key_duplicates_are_detected() {
    let mut e = engine();
    e.execute("CREATE TABLE t (a INTEGER, b INTEGER, v TEXT, PRIMARY KEY (a, b))")
        .unwrap();
    for i in 1..=20 {
        e.execute(&format!("INSERT INTO t VALUES ({i}, {i}, 'v{i}')"))
            .unwrap();
    }
    // Same `a`, same `b` -> collision.
    assert_duplicate(&mut e, "INSERT INTO t VALUES (5, 5, 'dup')", "composite");
    // Same `a`, different `b` -> fine.
    e.execute("INSERT INTO t VALUES (5, 999, 'ok')").unwrap();
    // Different `a`, same `b` -> fine.
    e.execute("INSERT INTO t VALUES (999, 5, 'ok')").unwrap();
    assert_eq!(row_count(&mut e), 22);
}

/// The property the whole optimisation rests on: inserting N rows row-at-a-time
/// must reject a re-used key that was inserted earlier in the same sequence.
/// A stale cache that missed the earlier rows would let this through.
#[test]
fn key_inserted_earlier_in_the_sequence_is_not_forgotten() {
    let mut e = engine();
    e.execute("CREATE TABLE t (k INTEGER PRIMARY KEY, v TEXT)")
        .unwrap();
    for i in 1..=300 {
        e.execute(&format!("INSERT INTO t VALUES ({i}, 'v{i}')"))
            .unwrap();
    }
    // Every one of the 300 keys must now be known to the engine.
    for i in (1..=300).step_by(37) {
        assert_duplicate(
            &mut e,
            &format!("INSERT INTO t VALUES ({i}, 'dup')"),
            "earlier key",
        );
    }
    // 301 was never inserted, so it must still be accepted.
    e.execute("INSERT INTO t VALUES (301, 'new')").unwrap();
    assert_eq!(row_count(&mut e), 301);
}

// ---------------------------------------------------------------------------
// Staleness after mutation paths that do not go through `execute_insert`.
//
// The index is invalidated by `StorageEngine::table_change_stamp`, so every
// row mutation must bump it. Each test below failed when invalidation was
// done by hand at a list of known call sites: a path missing from that list
// left the index describing a table state that no longer existed.
// ---------------------------------------------------------------------------

/// ROLLBACK removes the row, so its key must be insertable again. When the
/// index kept the rolled-back key, the key was permanently unusable.
#[test]
fn rollback_frees_the_key_for_reuse() {
    let mut e = engine();
    e.execute("CREATE TABLE t (k INTEGER PRIMARY KEY, v TEXT)")
        .unwrap();
    e.execute("INSERT INTO t VALUES (1, 'kept')").unwrap();
    e.execute("BEGIN").unwrap();
    e.execute("INSERT INTO t VALUES (77, 'doomed')").unwrap();
    e.execute("ROLLBACK").unwrap();
    assert_eq!(row_count(&mut e), 1, "rolled-back row must be gone");
    e.execute("INSERT INTO t VALUES (77, 'reused')")
        .expect("key freed by ROLLBACK must be insertable again");
    assert_duplicate(&mut e, "INSERT INTO t VALUES (77, 'dup')", "after rollback");
}

/// Same, for `ROLLBACK TO SAVEPOINT`, which replays undo records by writing
/// to storage behind the executor's back.
#[test]
fn rollback_to_savepoint_frees_the_key_for_reuse() {
    let mut e = engine();
    e.execute("CREATE TABLE t (k INTEGER PRIMARY KEY, v TEXT)")
        .unwrap();
    e.execute("INSERT INTO t VALUES (1, 'kept')").unwrap();
    e.execute("BEGIN").unwrap();
    e.execute("SAVEPOINT sp1").unwrap();
    e.execute("INSERT INTO t VALUES (42, 'rolled-back')")
        .unwrap();
    e.execute("ROLLBACK TO sp1").unwrap();
    e.execute("COMMIT").unwrap();
    assert_eq!(row_count(&mut e), 1);
    e.execute("INSERT INTO t VALUES (42, 'reused')")
        .expect("key freed by ROLLBACK TO SAVEPOINT must be insertable again");
}

/// A multi-table UPDATE that changes the PRIMARY KEY must not leave two rows
/// holding the new key, and must free the old one.
#[test]
fn multi_table_update_changing_primary_key_is_not_a_duplicate() {
    let mut e = engine();
    e.execute("CREATE TABLE t (k INTEGER PRIMARY KEY, v TEXT)")
        .unwrap();
    e.execute("CREATE TABLE u (k INTEGER PRIMARY KEY, w TEXT)")
        .unwrap();
    for i in 1..=5 {
        e.execute(&format!("INSERT INTO t VALUES ({i}, 'v{i}')"))
            .unwrap();
        e.execute(&format!("INSERT INTO u VALUES ({i}, 'w{i}')"))
            .unwrap();
    }
    e.execute("UPDATE t, u SET t.k = 500 WHERE t.k = u.k AND t.k = 2")
        .unwrap();

    let rows = e.execute("SELECT k FROM t ORDER BY k").unwrap();
    let keys: Vec<i64> = rows
        .rows
        .iter()
        .map(|r| r[0].as_integer().unwrap())
        .collect();
    assert_eq!(
        keys.iter().filter(|&&k| k == 500).count(),
        1,
        "exactly one row may hold the new key, got {keys:?}"
    );
    assert_duplicate(&mut e, "INSERT INTO t VALUES (500, 'dup')", "new pk");
    e.execute("INSERT INTO t VALUES (2, 'free')")
        .expect("old pk must be free after the update");
}

/// Multi-table DELETE removes rows without passing through `execute_delete`.
#[test]
fn multi_table_delete_frees_the_key_for_reuse() {
    let mut e = engine();
    e.execute("CREATE TABLE t (k INTEGER PRIMARY KEY, v TEXT)")
        .unwrap();
    e.execute("CREATE TABLE u (k INTEGER PRIMARY KEY, w TEXT)")
        .unwrap();
    for i in 1..=5 {
        e.execute(&format!("INSERT INTO t VALUES ({i}, 'v{i}')"))
            .unwrap();
        e.execute(&format!("INSERT INTO u VALUES ({i}, 'w{i}')"))
            .unwrap();
    }
    e.execute("DELETE t FROM t, u WHERE t.k = u.k AND t.k = 3")
        .unwrap();
    assert_eq!(row_count(&mut e), 4, "one row deleted from t");
    e.execute("INSERT INTO t VALUES (3, 'reused')")
        .expect("key freed by multi-table DELETE must be insertable again");
    assert_duplicate(&mut e, "INSERT INTO t VALUES (4, 'dup')", "surviving key");
}

/// A trigger writes to a table through the trigger executor, not the DML
/// path, so the index must be invalidated by the storage write itself.
#[test]
fn trigger_side_write_invalidates_the_index() {
    let mut e = engine();
    e.execute("CREATE TABLE t (k INTEGER PRIMARY KEY, v TEXT)")
        .unwrap();
    e.execute("CREATE TABLE log (k INTEGER PRIMARY KEY, note TEXT)")
        .unwrap();
    for i in 1..=5 {
        e.execute(&format!("INSERT INTO t VALUES ({i}, 'v{i}')"))
            .unwrap();
    }
    // Warm the index for `log`, then mutate it from a trigger body.
    e.execute("INSERT INTO log VALUES (1, 'a')").unwrap();
    e.execute(
        "CREATE TRIGGER trg AFTER INSERT ON t FOR EACH ROW \
         INSERT INTO log VALUES (900, 'fired')",
    )
    .unwrap();
    e.execute("INSERT INTO t VALUES (6, 'v6')").unwrap();

    // Key 900 now exists in `log` and must be reported as a duplicate.
    assert_duplicate(
        &mut e,
        "INSERT INTO log VALUES (900, 'dup')",
        "trigger-written key",
    );
}
