//! #5891 — a ROLLBACK must undo the writes of the **database** they
//! happened in.
//!
//! # The defect
//!
//! `FileStorage`'s per-transaction undo log ([`UndoOp`],
//! `crates/storage/src/file_storage.rs`) recorded only the **bare table
//! name** of the table a write touched:
//!
//! ```text
//! UndoOp::BufferedInsert { table: table.to_string(), row }
//! ```
//!
//! The replay side ([`undo_transaction_in_place`]) then looked that table up
//! with a hard-coded `DEFAULT_DATABASE`:
//!
//! ```text
//! s.tables.get_mut(&crate::engine::scoped_key(
//!     crate::engine::DEFAULT_DATABASE,
//!     &table,
//! ))
//! ```
//!
//! A write into any database other than `default` was therefore logged as
//! if it had happened in `default`, and the ROLLBACK went looking for it in
//! `default`. It never found it, so it undid **nothing**: the row stayed
//! visible and kept occupying its primary key, so the next INSERT of that
//! key was rejected as a duplicate.
//!
//! # What this suite pins
//!
//! 1. A rolled-back INSERT in a named database leaves the table empty.
//! 2. The rolled-back row's primary key is free again.
//! 3. A rollback in one database does not touch another database's
//!    same-named table — even when both hold the same primary key.
//! 4. A rolled-back UPDATE restores the pre-image.
//! 5. A rolled-back DELETE restores the deleted row.
//! 6. The **default** database's rollback still works (regression guard).
//!
//! ---------------------------------------------------------------------
//! STATUS: live. Run with
//!   cargo test --test undo_db_scope_test -- --nocapture
//! ---------------------------------------------------------------------

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
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
        "undo_db_scope_5891_{}_{}_{}",
        tag,
        std::process::id(),
        next_seq()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// The number of rows a `SELECT COUNT(*)` reports.
///
/// This is the suite's **oracle**: it reads through the storage's own scan
/// path, so it can contradict the undo log instead of agreeing with it by
/// construction.
fn count<S: StorageEngine + 'static>(engine: &mut ExecutionEngine<S>, table: &str) -> i64 {
    let r = engine
        .execute(&format!("SELECT COUNT(*) FROM {table}"))
        .unwrap();
    r.rows[0][0].as_integer().expect("COUNT is an integer")
}

/// The single value of a one-column table, or `None` when the table is
/// empty.
fn value<S: StorageEngine + 'static>(
    engine: &mut ExecutionEngine<S>,
    table: &str,
    column: &str,
) -> Option<String> {
    let r = engine
        .execute(&format!("SELECT {column} FROM {table} ORDER BY 1"))
        .unwrap();
    r.rows
        .first()
        .and_then(|row| row.first())
        .map(|v| v.to_string())
}

/// Emit the same suite for every backend, so the backends cannot drift.
macro_rules! backend_suite {
    ($mod_name:ident, $storage:ty, $make_storage:expr) => {
        mod $mod_name {
            use super::*;

            type S = $storage;

            fn setup() -> (ExecutionEngine<S>, DirGuard) {
                let (storage, guard) = $make_storage;
                let shared = Arc::new(RwLock::new(storage));
                (ExecutionEngine::new(shared), guard)
            }

            // -------------------------------------------------------------
            // 1. A rolled-back INSERT must not survive in a named database.
            // -------------------------------------------------------------

            /// The row an INSERT wrote into `db_a` must be gone after the
            /// ROLLBACK that cancelled it.
            ///
            /// Before the fix the undo log carried only the bare table
            /// name, so the replay looked for `default.t`, found nothing,
            /// and left `db_a.t` holding the row.
            #[test]
            fn rolled_back_insert_leaves_a_named_database_empty() {
                let (mut e, _guard) = setup();
                e.execute("CREATE DATABASE db_a").unwrap();
                e.execute("CREATE TABLE db_a.t (id INTEGER PRIMARY KEY)")
                    .unwrap();

                e.execute("BEGIN").unwrap();
                e.execute("INSERT INTO db_a.t VALUES (1)").unwrap();
                assert_eq!(
                    count(&mut e, "db_a.t"),
                    1,
                    "the row must be visible to its own transaction before the rollback"
                );

                e.execute("ROLLBACK").unwrap();
                assert_eq!(
                    count(&mut e, "db_a.t"),
                    0,
                    "the ROLLBACK must remove the row it cancelled"
                );
            }

            // -------------------------------------------------------------
            // 2. The rolled-back row's primary key must be free again.
            // -------------------------------------------------------------

            /// The key a rolled-back INSERT occupied must be insertable
            /// again.
            ///
            /// This is the symptom the issue was filed for: the row stayed,
            /// so the key stayed taken, so the second INSERT was rejected
            /// as a duplicate.
            #[test]
            fn rolled_back_insert_frees_the_primary_key() {
                let (mut e, _guard) = setup();
                e.execute("CREATE DATABASE db_a").unwrap();
                e.execute("CREATE TABLE db_a.t (id INTEGER PRIMARY KEY)")
                    .unwrap();

                e.execute("BEGIN").unwrap();
                e.execute("INSERT INTO db_a.t VALUES (1)").unwrap();
                e.execute("ROLLBACK").unwrap();

                e.execute("INSERT INTO db_a.t VALUES (1)")
                    .expect("the key freed by the ROLLBACK must be insertable again");
                assert_eq!(count(&mut e, "db_a.t"), 1);
            }

            // -------------------------------------------------------------
            // 3. A rollback must not reach into another database.
            // -------------------------------------------------------------

            /// `db_b` holds a same-named table with the **same primary
            /// key**; a rollback in `db_a` must not touch it.
            ///
            /// The undo log's table name is not enough to tell the two
            /// apart — the database has to travel with it.
            #[test]
            fn rollback_in_one_database_does_not_touch_another() {
                let (mut e, _guard) = setup();
                e.execute("CREATE DATABASE db_a").unwrap();
                e.execute("CREATE DATABASE db_b").unwrap();
                e.execute("CREATE TABLE db_a.t (id INTEGER PRIMARY KEY, v TEXT)")
                    .unwrap();
                e.execute("CREATE TABLE db_b.t (id INTEGER PRIMARY KEY, v TEXT)")
                    .unwrap();

                e.execute("INSERT INTO db_b.t VALUES (1, 'b')").unwrap();

                e.execute("BEGIN").unwrap();
                e.execute("INSERT INTO db_a.t VALUES (1, 'a')").unwrap();
                e.execute("ROLLBACK").unwrap();

                assert_eq!(
                    count(&mut e, "db_a.t"),
                    0,
                    "db_a must not hold the rolled-back row"
                );
                assert_eq!(
                    count(&mut e, "db_b.t"),
                    1,
                    "db_b must still hold its own row"
                );
                assert_eq!(
                    value(&mut e, "db_b.t", "v").as_deref(),
                    Some("b"),
                    "db_b's row must be untouched by db_a's rollback"
                );
            }

            // -------------------------------------------------------------
            // 4. A rolled-back UPDATE must restore the pre-image.
            // -------------------------------------------------------------

            /// The UPDATE ran inside `db_a`; the ROLLBACK must put the
            /// old value back **in `db_a`**.
            #[test]
            fn rolled_back_update_restores_the_pre_image() {
                let (mut e, _guard) = setup();
                e.execute("CREATE DATABASE db_a").unwrap();
                e.execute("USE db_a").unwrap();
                e.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)")
                    .unwrap();
                e.execute("INSERT INTO t VALUES (1, 'before')").unwrap();

                e.execute("BEGIN").unwrap();
                e.execute("UPDATE t SET v = 'after' WHERE id = 1").unwrap();
                assert_eq!(
                    value(&mut e, "t", "v").as_deref(),
                    Some("after"),
                    "the UPDATE must be visible inside its transaction"
                );

                e.execute("ROLLBACK").unwrap();
                assert_eq!(count(&mut e, "t"), 1, "the ROLLBACK must not lose the row");
                assert_eq!(
                    value(&mut e, "t", "v").as_deref(),
                    Some("before"),
                    "the ROLLBACK must restore the pre-image"
                );
            }

            // -------------------------------------------------------------
            // 5. A rolled-back DELETE must restore the deleted row.
            // -------------------------------------------------------------

            /// The DELETE ran inside `db_a`; the ROLLBACK must put the row
            /// back **in `db_a`**.
            #[test]
            fn rolled_back_delete_restores_the_deleted_row() {
                let (mut e, _guard) = setup();
                e.execute("CREATE DATABASE db_a").unwrap();
                e.execute("USE db_a").unwrap();
                e.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)")
                    .unwrap();
                e.execute("INSERT INTO t VALUES (1, 'x')").unwrap();

                e.execute("BEGIN").unwrap();
                e.execute("DELETE FROM t WHERE id = 1").unwrap();
                assert_eq!(
                    count(&mut e, "t"),
                    0,
                    "the DELETE must be visible inside its transaction"
                );

                e.execute("ROLLBACK").unwrap();
                assert_eq!(
                    count(&mut e, "t"),
                    1,
                    "the ROLLBACK must restore the deleted row"
                );
                assert_eq!(
                    value(&mut e, "t", "v").as_deref(),
                    Some("x"),
                    "the restored row must carry its original value"
                );
            }

            // -------------------------------------------------------------
            // 6. The default database must keep working exactly as before.
            // -------------------------------------------------------------

            /// The regression guard for the fix: the default database's
            /// rollback must still undo its writes.
            ///
            /// The fix carries the database in the undo log. For the
            /// default database that value is `default`, so the behaviour
            /// has to be byte-for-byte the old one.
            #[test]
            fn rollback_in_the_default_database_still_undoes() {
                let (mut e, _guard) = setup();
                e.execute("CREATE TABLE t (id INTEGER PRIMARY KEY, v TEXT)")
                    .unwrap();
                e.execute("INSERT INTO t VALUES (1, 'before')").unwrap();

                e.execute("BEGIN").unwrap();
                e.execute("INSERT INTO t VALUES (2, 'new')").unwrap();
                e.execute("UPDATE t SET v = 'after' WHERE id = 1").unwrap();
                e.execute("DELETE FROM t WHERE id = 2").unwrap();
                e.execute("ROLLBACK").unwrap();

                assert_eq!(count(&mut e, "t"), 1, "only the original row must remain");
                assert_eq!(
                    value(&mut e, "t", "v").as_deref(),
                    Some("before"),
                    "the rolled-back UPDATE must not have changed the row"
                );
            }
        }
    };
}

backend_suite!(file_backend, FileStorage, {
    let dir = unique_dir("file");
    (FileStorage::new(dir.clone()).unwrap(), DirGuard(Some(dir)))
});

backend_suite!(mvcc_backend, MvccStorage<FileStorage>, {
    let dir = unique_dir("mvcc");
    let inner = MvccStorage::new(FileStorage::new(dir.clone()).unwrap());
    inner.rebuild_from_inner().expect("rebuild MVCC chain");
    (inner, DirGuard(Some(dir)))
});
