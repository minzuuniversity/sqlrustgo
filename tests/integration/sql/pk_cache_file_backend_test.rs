//! #5412 — the INSERT primary-key cache must work on the **file** backend,
//! and it must never answer about a different database.
//!
//! # What #5412 was
//!
//! `ExecutionEngine` keeps a cached primary-key index per table so that an
//! INSERT's duplicate-key check does not need a full `storage.scan()`. The
//! cache is only usable while the storage's **change stamp** for that table
//! still matches the stamp the index was built at.
//!
//! `StorageEngine::table_change_stamp` had a default implementation that
//! returned `0`, and **only `MemoryStorage` overrode it**. Every other
//! backend — `FileStorage`, and therefore the whole production stack
//! (`FileStorage → MvccStorage → WalStorage → BoxStorageEngine`) — answered
//! `0`, which the engine reads as "this engine does not track changes, never
//! trust a cached copy".
//!
//! The consequence: N single-row INSERTs into a table with a PRIMARY KEY cost
//! O(N²) on the production backend, while the in-memory backend was already
//! O(N). Correct, but quadratic — which is what the issue is about.
//!
//! # What this suite pins
//!
//! 1. **The file backend builds the index once, not once per row.** The
//!    number of full index builds must not grow with the number of rows.
//! 2. **The duplicate-key check still rejects duplicates.**
//! 3. **A write that bypasses the engine invalidates the cache.** The
//!    storage is shared with writers the engine does not control; a row
//!    written behind the engine's back must still be seen as a duplicate.
//! 4. **The cache never answers across databases.** A key that exists in
//!    `d1.t` must be insertable into `d2.t`, and a key that exists in
//!    `d2.t` must still be rejected when inserted into `d2.t` — even when
//!    the cache slot was last filled from the other database.
//! 5. **DDL and transactions move the stamp.** DROP/CREATE of a same-named
//!    table, and a commit or rollback that changes a table's visible row
//!    set, must invalidate what the cache holds.
//!
//! # The cross-database hazard, stated precisely
//!
//! The change stamp is keyed by the **bare table name** (this is the
//! storage layer's contract, shared with `MemoryStorage`, which keys its
//! counter by the default database's table name). The engine's cache is
//! keyed by the bare table name too. So `d1.t` and `d2.t` share one stamp
//! and one cache slot.
//!
//! The stamp alone therefore cannot tell the two databases apart: it says
//! "something about a table named `t` changed", not "these rows changed".
//! The cache entry records **which database** it was built from, and a hit
//! requires that database to match. Without that field the two databases'
//! indexes are interchangeable, and the duplicate-key check answers from the
//! wrong key set — rejecting a key that is free in the target database, or
//! accepting one that is already taken.
//!
//! Both directions are asserted below, because both are real: the failure
//! mode depends on which database's index was cached last.
//!
//! The tests are parameterised over `MemoryStorage` and `FileStorage` so the
//! two backends cannot drift: the cross-database hazard exists in
//! `MemoryStorage` today, and this fix must not leave the two backends
//! behaving differently.
//!
//! ---------------------------------------------------------------------
//! STATUS: live. Run with
//!   cargo test --test pk_cache_file_backend_test -- --nocapture
//! ---------------------------------------------------------------------

use parking_lot::RwLock;
use sqlrustgo::{ExecutionEngine, MemoryStorage, Value};
use sqlrustgo_storage::{FileStorage, MvccStorage, StorageEngine};
use std::path::PathBuf;
use std::sync::Arc;

/// Removes a test's data directory when the test ends, so a crashed run
/// does not leave state behind for the next one.
struct DirGuard(Option<PathBuf>);

impl Drop for DirGuard {
    fn drop(&mut self) {
        if let Some(p) = self.0.take() {
            let _ = std::fs::remove_dir_all(p);
        }
    }
}

/// A per-call counter: naming every directory after the process id alone
/// makes parallel tests delete each other's data.
fn next_seq() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    SEQ.fetch_add(1, Ordering::SeqCst)
}

fn unique_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "pk_cache_5412_{}_{}_{}",
        tag,
        std::process::id(),
        next_seq()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Assert that `sql` fails with a duplicate-key error.
///
/// The engine reports the conflict by column name, so match on "duplicate"
/// rather than an exact string.
fn assert_duplicate<S: StorageEngine + 'static>(
    engine: &mut ExecutionEngine<S>,
    sql: &str,
    what: &str,
) {
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

/// The number of rows a `SELECT COUNT(*)` reports.
///
/// This is the suite's **oracle**: it reads through the storage's own scan
/// path, not through the primary-key cache, so it can contradict the cache
/// instead of agreeing with it by construction.
fn count<S: StorageEngine + 'static>(engine: &mut ExecutionEngine<S>, table: &str) -> i64 {
    let r = engine.execute(&format!("SELECT COUNT(*) FROM {table}")).unwrap();
    r.rows[0][0].as_integer().expect("COUNT is an integer")
}

/// Emit the same suite for every backend, so the two cannot drift.
macro_rules! backend_suite {
    ($mod_name:ident, $storage:ty, $make_storage:expr) => {
        mod $mod_name {
            use super::*;

            type S = $storage;

            fn setup() -> (ExecutionEngine<S>, Arc<RwLock<S>>, DirGuard) {
                let (storage, guard) = $make_storage;
                let shared = Arc::new(RwLock::new(storage));
                (ExecutionEngine::new(Arc::clone(&shared)), shared, guard)
            }

            // -------------------------------------------------------------
            // 1. The point of #5412: one index build, not one per row.
            // -------------------------------------------------------------

            /// N single-row INSERTs must not cost N full table scans.
            ///
            /// The count of index builds is what pins the claim: before the
            /// fix the file backend answered `table_change_stamp` with 0,
            /// so the cache was never usable and every statement scanned
            /// the table. The build count therefore grew with the number of
            /// rows (O(N) scans of an O(N) table — O(N²) work).
            #[test]
            fn bulk_insert_builds_the_index_once_not_once_per_row() {
                let (mut e, _storage, _guard) = setup();
                e.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)")
                    .unwrap();

                for i in 1..=200 {
                    e.execute(&format!("INSERT INTO t VALUES ({i}, 'v{i}')"))
                        .unwrap();
                }
                let builds_after_200 = e.pk_cache_build_count();
                assert!(
                    builds_after_200 <= 2,
                    "200 inserts caused {builds_after_200} full index builds; \
                     the cache is not being reused"
                );

                // Doubling the row count must not double the scan count:
                // the second half of the table is inserted into the index
                // that already exists.
                for i in 201..=400 {
                    e.execute(&format!("INSERT INTO t VALUES ({i}, 'v{i}')"))
                        .unwrap();
                }
                let builds_after_400 = e.pk_cache_build_count();
                assert_eq!(
                    builds_after_400, builds_after_200,
                    "the number of full scans grew with the row count \
                     ({builds_after_200} -> {builds_after_400}); \
                     the INSERT path is quadratic again"
                );
                assert_eq!(count(&mut e, "t"), 400, "all rows must be present");
            }

            // -------------------------------------------------------------
            // 2. The cache must not change what INSERT accepts.
            // -------------------------------------------------------------

            /// A repeated primary key is still a duplicate.
            #[test]
            fn duplicate_key_is_still_rejected() {
                let (mut e, _storage, _guard) = setup();
                e.execute("CREATE TABLE t (id INTEGER PRIMARY KEY)").unwrap();
                e.execute("INSERT INTO t VALUES (1)").unwrap();
                assert_duplicate(&mut e, "INSERT INTO t VALUES (1)", "repeated key");
                assert_eq!(count(&mut e, "t"), 1, "the rejected insert must not land");
            }

            /// A key that was never used is still accepted.
            #[test]
            fn new_key_is_still_accepted() {
                let (mut e, _storage, _guard) = setup();
                e.execute("CREATE TABLE t (id INTEGER PRIMARY KEY)").unwrap();
                for i in 1..=20 {
                    e.execute(&format!("INSERT INTO t VALUES ({i})")).unwrap();
                }
                assert_eq!(count(&mut e, "t"), 20);
            }

            // -------------------------------------------------------------
            // 3. A write the engine did not perform invalidates the cache.
            // -------------------------------------------------------------

            /// A row written straight into the storage must be seen as a
            /// duplicate by the next INSERT.
            ///
            /// The storage is shared with writers the engine does not
            /// control — a trigger body, a wire endpoint, another process
            /// over the same directory. If such a write does not move the
            /// change stamp, the engine keeps answering from a stale index
            /// and accepts a duplicate key.
            #[test]
            fn write_that_bypasses_the_engine_invalidates_the_cache() {
                let (mut e, storage, _guard) = setup();
                e.execute("CREATE TABLE t (id INTEGER PRIMARY KEY)").unwrap();
                e.execute("INSERT INTO t VALUES (1)").unwrap();
                e.execute("INSERT INTO t VALUES (2)").unwrap();

                // The cache now holds {1, 2}. Write key 3 behind the
                // engine's back, exactly as an outside writer would.
                storage
                    .write()
                    .insert("t", vec![vec![Value::Integer(3)]])
                    .unwrap();

                assert_duplicate(
                    &mut e,
                    "INSERT INTO t VALUES (3)",
                    "key written behind the engine's back",
                );
                assert_eq!(count(&mut e, "t"), 3, "only the three real rows may exist");
            }

            // -------------------------------------------------------------
            // 4. The cache must never answer about another database.
            // -------------------------------------------------------------

            /// A key that exists in `d1.t` but not in `d2.t` must be
            /// insertable into `d2.t`.
            ///
            /// This is the "false rejection" direction: the cache slot was
            /// filled from `d1.t`, whose key set contains the key, so an
            /// engine that trusts the slot without checking which database
            /// the slot belongs to rejects a perfectly legal insert.
            #[test]
            fn cross_database_new_key_is_not_rejected() {
                let (mut e, _storage, _guard) = setup();
                e.execute("CREATE DATABASE d1").unwrap();
                e.execute("CREATE DATABASE d2").unwrap();

                // Both tables are created **before** any insert. A CREATE
                // TABLE between the cache build and the decisive INSERT
                // would move the change stamp on the file backend and hide
                // the failure this case is about; the case must isolate the
                // cache's behaviour, not the DDL's.
                e.execute("USE d1").unwrap();
                e.execute("CREATE TABLE t (id INTEGER PRIMARY KEY)").unwrap();
                e.execute("USE d2").unwrap();
                e.execute("CREATE TABLE t (id INTEGER PRIMARY KEY)").unwrap();

                e.execute("USE d1").unwrap();
                e.execute("INSERT INTO d1.t VALUES (7)").unwrap();
                e.execute("INSERT INTO d1.t VALUES (9)").unwrap();

                e.execute("USE d2").unwrap();
                // `d2.t` is empty. Key 7 is free there.
                e.execute("INSERT INTO d2.t VALUES (7)").unwrap();

                assert_eq!(count(&mut e, "d2.t"), 1, "d2.t must hold its own row");
                assert_eq!(count(&mut e, "d1.t"), 2, "d1.t must be untouched");
            }

            /// A key that exists in `d2.t` must still be rejected when it is
            /// inserted into `d2.t` — even when the cache slot was last
            /// filled from `d1.t`.
            ///
            /// This is the "false acceptance" direction, and it is the more
            /// dangerous one: the duplicate key is silently accepted and the
            /// table ends up with two rows for one primary key.
            ///
            /// The order matters. The first INSERT happens while the change
            /// stamp is still 0, so no index is built and no slot is
            /// published; the second INSERT builds the slot from `d1.t`,
            /// whose key set does not contain the key that `d2.t` already
            /// holds. Only then does the third INSERT ask the cache.
            #[test]
            fn cross_database_duplicate_is_still_rejected() {
                let (mut e, _storage, _guard) = setup();
                e.execute("CREATE DATABASE d1").unwrap();
                e.execute("CREATE DATABASE d2").unwrap();

                // Both tables exist before the first insert, for the same
                // reason as in `cross_database_new_key_is_not_rejected`.
                e.execute("USE d2").unwrap();
                e.execute("CREATE TABLE t (id INTEGER PRIMARY KEY)").unwrap();
                e.execute("USE d1").unwrap();
                e.execute("CREATE TABLE t (id INTEGER PRIMARY KEY)").unwrap();

                e.execute("USE d2").unwrap();
                e.execute("INSERT INTO d2.t VALUES (7)").unwrap();

                e.execute("USE d1").unwrap();
                e.execute("INSERT INTO d1.t VALUES (9)").unwrap();

                // `d2.t` already holds key 7. The cache slot currently
                // holds `d1.t`'s key set, which does not contain 7.
                e.execute("USE d2").unwrap();
                assert_duplicate(
                    &mut e,
                    "INSERT INTO d2.t VALUES (7)",
                    "duplicate in d2.t while the cache holds d1.t's keys",
                );
                assert_eq!(count(&mut e, "d2.t"), 1, "the duplicate must not land");
            }

            /// The same key set in two databases, written in an interleaved
            /// order, must stay independent.
            ///
            /// The oracle here is the test's own `HashSet` of the keys it
            /// inserted, compared against what `SELECT` reports — never
            /// against the cache, which is the thing under test.
            #[test]
            fn interleaved_writes_across_databases_keep_their_keys_apart() {
                use std::collections::BTreeSet;

                let (mut e, _storage, _guard) = setup();
                e.execute("CREATE DATABASE d1").unwrap();
                e.execute("CREATE DATABASE d2").unwrap();
                for db in ["d1", "d2"] {
                    e.execute(&format!("USE {db}")).unwrap();
                    e.execute("CREATE TABLE t (id INTEGER PRIMARY KEY)").unwrap();
                }

                let mut expected: BTreeSet<(String, i64)> = BTreeSet::new();
                // Alternate the databases, and give each its own key range
                // so a cross-database mix-up shows up as a wrong key set.
                for round in 0..25i64 {
                    let db = if round % 2 == 0 { "d1" } else { "d2" };
                    let key = round;
                    e.execute(&format!("INSERT INTO {db}.t VALUES ({key})"))
                        .unwrap();
                    expected.insert((db.to_string(), key));
                }

                for db in ["d1", "d2"] {
                    let rows = e
                        .execute(&format!("SELECT id FROM {db}.t ORDER BY id"))
                        .unwrap()
                        .rows;
                    let got: BTreeSet<i64> = rows
                        .iter()
                        .map(|r| r[0].as_integer().expect("id is an integer"))
                        .collect();
                    let want: BTreeSet<i64> = expected
                        .iter()
                        .filter(|(d, _)| d == db)
                        .map(|(_, k)| *k)
                        .collect();
                    assert_eq!(got, want, "{db}.t holds the wrong key set");
                }
            }

            /// A database name spelled in a different case is the same
            /// database, so it must share the cache instead of rebuilding it.
            #[test]
            fn database_name_case_variants_share_one_cache_entry() {
                let (mut e, _storage, _guard) = setup();
                e.execute("CREATE DATABASE d1").unwrap();
                e.execute("USE d1").unwrap();
                e.execute("CREATE TABLE t (id INTEGER PRIMARY KEY)").unwrap();

                e.execute("INSERT INTO d1.t VALUES (1)").unwrap();
                e.execute("INSERT INTO D1.t VALUES (2)").unwrap();
                e.execute("INSERT INTO d1.t VALUES (3)").unwrap();

                assert_eq!(
                    e.pk_cache_build_count(),
                    1,
                    "the same database spelled two ways must not rebuild the index"
                );
                assert_eq!(count(&mut e, "d1.t"), 3);
            }

            // -------------------------------------------------------------
            // 5. DDL and transactions move the stamp.
            // -------------------------------------------------------------

            /// A table that is dropped and recreated under the same name
            /// starts empty, so its old keys are free again.
            #[test]
            fn drop_and_recreate_table_starts_empty() {
                let (mut e, _storage, _guard) = setup();
                e.execute("CREATE TABLE t (id INTEGER PRIMARY KEY)").unwrap();
                e.execute("INSERT INTO t VALUES (1)").unwrap();
                e.execute("INSERT INTO t VALUES (2)").unwrap();

                e.execute("DROP TABLE t").unwrap();
                e.execute("CREATE TABLE t (id INTEGER PRIMARY KEY)").unwrap();

                // Key 1 is free in the new table.
                e.execute("INSERT INTO t VALUES (1)").unwrap();
                assert_eq!(count(&mut e, "t"), 1, "the new table must hold one row");
            }

            /// An UPDATE moves the stamp even though the key set is
            /// unchanged — the cached index holds values, not just keys.
            #[test]
            fn update_invalidates_the_cached_key_values() {
                let (mut e, _storage, _guard) = setup();
                e.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)")
                    .unwrap();
                e.execute("INSERT INTO t VALUES (1, 'a')").unwrap();
                e.execute("INSERT INTO t VALUES (2, 'b')").unwrap();

                e.execute("UPDATE t SET v = 'c' WHERE id = 1").unwrap();
                // The cache was rebuilt by the UPDATE's invalidation; the
                // key set is unchanged, so the duplicate check still works.
                assert_duplicate(&mut e, "INSERT INTO t VALUES (1)", "updated key");
                assert_eq!(count(&mut e, "t"), 2);
            }

            /// A DELETE frees the key again.
            #[test]
            fn delete_frees_the_key() {
                let (mut e, _storage, _guard) = setup();
                e.execute("CREATE TABLE t (id INTEGER PRIMARY KEY)").unwrap();
                e.execute("INSERT INTO t VALUES (1)").unwrap();
                e.execute("INSERT INTO t VALUES (2)").unwrap();

                e.execute("DELETE FROM t WHERE id = 1").unwrap();
                e.execute("INSERT INTO t VALUES (1)").unwrap();
                assert_eq!(count(&mut e, "t"), 2);
            }
        }
    };
}

backend_suite!(memory_backend, MemoryStorage, (MemoryStorage::new(), DirGuard(None)));

backend_suite!(
    file_backend,
    FileStorage,
    {
        let dir = unique_dir("file");
        (FileStorage::new(dir.clone()).unwrap(), DirGuard(Some(dir)))
    }
);

// ---------------------------------------------------------------------
// The MVCC stack — the production shape for the visibility question
// ---------------------------------------------------------------------

/// Two engines over one `MvccStorage<FileStorage>` — the production stack
/// (`FileStorage → MvccStorage → WalStorage → BoxStorageEngine`, minus the
/// WAL layer, which does not change row visibility).
fn mvcc_two_connections() -> (
    ExecutionEngine<MvccStorage<FileStorage>>,
    ExecutionEngine<MvccStorage<FileStorage>>,
    DirGuard,
) {
    let dir = unique_dir("mvcc");
    let inner = MvccStorage::new(FileStorage::new(dir.clone()).unwrap());
    inner.rebuild_from_inner().expect("rebuild MVCC chain");
    let storage = Arc::new(RwLock::new(inner));
    (
        ExecutionEngine::new(Arc::clone(&storage)),
        ExecutionEngine::new(storage),
        DirGuard(Some(dir)),
    )
}

/// A key that another connection commits must become a duplicate.
///
/// The commit changes the table's visible row set, so it has to move the
/// change stamp — otherwise the other connection keeps answering from an
/// index that predates the commit, and the key is accepted a second time.
///
/// The keys are deliberately different: the connection that checks for the
/// duplicate already holds a row of its own, so the only thing that can
/// reject the insert is the other connection's committed row.
#[test]
fn committed_row_in_another_connection_becomes_a_duplicate() {
    let (mut a, mut b, _guard) = mvcc_two_connections();

    a.execute("CREATE DATABASE d1").unwrap();
    a.execute("USE d1").unwrap();
    a.execute("CREATE TABLE t (id INTEGER PRIMARY KEY)").unwrap();
    b.execute("USE d1").unwrap();

    // A writes but does not commit: B cannot see the row.
    a.execute("BEGIN").unwrap();
    a.execute("INSERT INTO t VALUES (2)").unwrap();
    b.execute("INSERT INTO t VALUES (1)").unwrap();

    // A commits. Now key 2 exists for real.
    a.execute("COMMIT").unwrap();

    assert_duplicate(
        &mut b,
        "INSERT INTO t VALUES (2)",
        "key committed by the other connection",
    );

    let rows = b.execute("SELECT id FROM t ORDER BY id").unwrap().rows;
    assert_eq!(rows.len(), 2, "the table must hold exactly two rows");
}

/// A rolled-back row must not become a duplicate.
///
/// This runs in the **default** database on purpose. `FileStorage`'s
/// undo log (`undo_transaction_in_place`, `crates/storage/src/file_storage.rs`)
/// resolves the table it has to undo with a hard-coded `DEFAULT_DATABASE`,
/// so a ROLLBACK in any *other* database does not remove the row it
/// inserted. That is a pre-existing defect in the undo path, unrelated to
/// the change stamp — and asserting on it here would test that defect
/// rather than this one. It is recorded separately in the evidence
/// document; see the "newly found, pre-existing" section there.
#[test]
fn rolled_back_row_in_another_connection_is_not_a_duplicate() {
    let (mut a, mut b, _guard) = mvcc_two_connections();

    a.execute("CREATE TABLE t (id INTEGER PRIMARY KEY)").unwrap();

    a.execute("BEGIN").unwrap();
    a.execute("INSERT INTO t VALUES (2)").unwrap();
    a.execute("ROLLBACK").unwrap();

    // The row is gone; key 2 is free.
    b.execute("INSERT INTO t VALUES (2)").unwrap();
    let rows = b.execute("SELECT id FROM t ORDER BY id").unwrap().rows;
    assert_eq!(rows.len(), 1, "the table must hold exactly one row");
}
