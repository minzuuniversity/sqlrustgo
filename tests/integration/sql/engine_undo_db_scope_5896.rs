//! #5896: the engine layer's undo log must name the **database** the
//! write happened in — not just the bare table name.
//!
//! Before this change, `sqlrustgo_transaction::savepoint::UndoRecord`
//! carried only a table name, and the replayer had to call the
//! database-less `storage.delete` / `storage.insert`, which resolve a
//! bare name against `DEFAULT_DATABASE`. Two consequences:
//!
//! * **A ROLLBACK in a named database undid nothing** — the row to
//!   remove lived in `d1\x01t`, the undo looked in `default\x01t`.
//! * **A ROLLBACK could reach into another database** — the undo's
//!   delete/insert landed in `default`, so the named database kept the
//!   row while `default` could gain a row it never had.
//!
//! The database now travels with the record as an explicit field, and
//! the replayer addresses the exact `(db, table)` the write touched.
//!
//! # The scope of this file
//!
//! This exercises the **engine layer** (`UndoRecord` →
//! `rollback_with_undo` → `storage.*_in_db`). The storage layer's own
//! undo log is #5891 (a separate, not-yet-merged PR) and the
//! `MemoryStorage` transaction log is #5897 (also separate). So this
//! file pins the engine layer's behaviour on its own: the assertions
//! below hold **without** those two fixes, because the engine layer
//! addresses the database explicitly and the storage backend it drives
//! here (`MemoryStorage`) honours `*_in_db` directly.
//!
//! The merge-state end-to-end case (all three paths fixed at once) is
//! tracked separately; see the note in the issue.
//!
//! # Two details the test had to be written around
//!
//! * **`default` is a SQL keyword.** The lexer reads `USE default` as
//!   `Token::Default`, which the `USE` parser rejects
//!   ("Expected database name, got Default"). The backquoted form
//!   (`USE \`default\``) parses — measured, not assumed.
//! * **The UPDATE cases change a non-key column.** The engine's undo
//!   deletes by the *original* key before re-inserting the prior row,
//!   so an UPDATE that changes the **primary key** is not undone by
//!   that path (the delete finds nothing, because the row's key has
//!   already changed). That is a separate, pre-existing gap — it
//!   reproduces in the `default` database too, so it is not a
//!   database-scope problem — and is **not** what this file is about.
//!   The cases below therefore update a plain column `v` and leave the
//!   key alone.

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::{MemoryStorage, StorageEngine};
use std::sync::Arc;

/// A storage per call: these tests run in parallel by default and every
/// one of them would otherwise share (and mutate) the same global
/// state. The per-call counter is the same pattern the sibling
/// `insert_db_isolation_5057` / `session_db_isolation_5057` suites use.
fn engine() -> ExecutionEngine<MemoryStorage> {
    static SEQ: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let _ = n;
    ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())))
}

/// An engine **plus** a handle to its storage.
///
/// The idempotence case below has to reproduce the storage backend's own
/// half of a ROLLBACK, and the only way to do that from the outside is to
/// put the row back through the storage API itself.
fn engine_with_storage() -> (ExecutionEngine<MemoryStorage>, Arc<RwLock<MemoryStorage>>) {
    static SEQ: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let _ = n;
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    (ExecutionEngine::new(storage.clone()), storage)
}

/// `d1` and `default` each hold a table `t` with one row.
/// The row's `id` says which database it came from (so a row that
/// shows up in the wrong database is visible by `id` alone), and its
/// `v` is the column the UPDATE cases touch.
fn seeded(e: &mut ExecutionEngine<MemoryStorage>) {
    e.execute("CREATE DATABASE d1").unwrap();
    e.execute("USE `d1`").unwrap();
    e.execute("CREATE TABLE t (id INT PRIMARY KEY, v INT)").unwrap();
    e.execute("INSERT INTO t (id, v) VALUES (10, 1)").unwrap();
    e.execute("USE `default`").unwrap();
    e.execute("CREATE TABLE t (id INT PRIMARY KEY, v INT)").unwrap();
    e.execute("INSERT INTO t (id, v) VALUES (30, 1)").unwrap();
}

/// The rows of `db.t`, as `(id, v)` pairs, ordered by id.
fn rows(e: &mut ExecutionEngine<MemoryStorage>, db: &str) -> Vec<(i64, i64)> {
    e.execute(&format!("USE `{db}`")).unwrap();
    e.execute("SELECT id, v FROM t ORDER BY id")
        .unwrap()
        .rows
        .into_iter()
        .map(|r| {
            let id = match r.first() {
                Some(sqlrustgo::Value::Integer(v)) => *v,
                other => panic!("expected an integer id, got {other:?}"),
            };
            let v = match r.get(1) {
                Some(sqlrustgo::Value::Integer(v)) => *v,
                other => panic!("expected an integer v, got {other:?}"),
            };
            (id, v)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// ① the undo names the database of the write
// ---------------------------------------------------------------------------

/// A ROLLBACK of an INSERT made in `d1` removes the row from `d1.t`.
///
/// Before the fix the undo looked in `default\x01t`, so the row in
/// `d1\x01t` survived the ROLLBACK.
#[test]
fn rollback_removes_an_inserted_row_from_the_named_database_5896() {
    let mut e = engine();
    seeded(&mut e);

    e.execute("USE `d1`").unwrap();
    e.execute("BEGIN").unwrap();
    e.execute("INSERT INTO t (id, v) VALUES (11, 1)").unwrap();
    assert_eq!(
        rows(&mut e, "d1"),
        vec![(10, 1), (11, 1)],
        "visible in-transaction"
    );

    e.execute("ROLLBACK").unwrap();

    assert_eq!(
        rows(&mut e, "d1"),
        vec![(10, 1)],
        "ROLLBACK must remove the row from the database the INSERT was made in"
    );
    assert_eq!(
        rows(&mut e, "default"),
        vec![(30, 1)],
        "and must not touch the same table name in `default`"
    );
}

/// A ROLLBACK of a DELETE made in `d1` restores the row into `d1.t`.
#[test]
fn rollback_restores_a_deleted_row_in_the_named_database_5896() {
    let mut e = engine();
    seeded(&mut e);

    e.execute("USE `d1`").unwrap();
    e.execute("BEGIN").unwrap();
    e.execute("DELETE FROM t WHERE id = 10").unwrap();
    assert!(rows(&mut e, "d1").is_empty(), "gone inside the transaction");

    e.execute("ROLLBACK").unwrap();

    assert_eq!(
        rows(&mut e, "d1"),
        vec![(10, 1)],
        "ROLLBACK must restore the row in d1, not in default"
    );
    assert_eq!(rows(&mut e, "default"), vec![(30, 1)], "default untouched");
}

/// The engine's replay must be **idempotent** with respect to the storage
/// backend's own undo.
///
/// A ROLLBACK consults **two** undo logs: the engine's `UndoRecord` and
/// the storage backend's own (`FileStorage` keeps a per-transaction
/// `UndoOp` log and replays it from `rollback_transaction_lockfree_for`).
/// Once both know which database the write happened in (#5891 / #5896 /
/// #5897), **both** restore a deleted row — and a replay that merely
/// *appends* leaves **two** copies of it behind.
///
/// The DELETE path is the only one where that shows. The INSERT path's
/// engine undo *deletes* a row, and the UPDATE path's storage-side undo
/// *assigns* by index — both are idempotent — but a DELETE is restored by
/// an insert on both sides.
///
/// This case reproduces the backend's half of that interaction directly:
/// it puts the deleted row back through the storage API (exactly what the
/// backend's own undo does) and then lets the engine's ROLLBACK replay its
/// records. The row must end up there **exactly once** — before the fix
/// the replay appended a second copy.
#[test]
fn rollback_restores_a_deleted_row_exactly_once_when_the_backend_already_restored_it_5896() {
    let (mut e, storage) = engine_with_storage();
    seeded(&mut e);

    e.execute("USE `d1`").unwrap();
    e.execute("BEGIN").unwrap();
    e.execute("DELETE FROM t WHERE id = 10").unwrap();
    assert!(rows(&mut e, "d1").is_empty(), "gone inside the transaction");

    // The storage backend's own undo puts the row back — this is the
    // state the engine's replay then finds.
    {
        let mut s = storage.write();
        s.insert_in_db(
            "d1",
            "t",
            vec![vec![
                sqlrustgo_storage::Value::Integer(10),
                sqlrustgo_storage::Value::Integer(1),
            ]],
        )
        .unwrap();
    }
    assert_eq!(
        rows(&mut e, "d1").len(),
        1,
        "the backend restored exactly one copy before the engine replayed"
    );

    e.execute("ROLLBACK").unwrap();

    assert_eq!(
        rows(&mut e, "d1"),
        vec![(10, 1)],
        "the engine's replay must leave exactly one copy of the restored row"
    );
    assert_eq!(rows(&mut e, "default"), vec![(30, 1)], "default untouched");
}

/// A ROLLBACK of an UPDATE made in `d1` restores the prior row in `d1.t`.
#[test]
fn rollback_restores_an_updated_row_in_the_named_database_5896() {
    let mut e = engine();
    seeded(&mut e);

    e.execute("USE `d1`").unwrap();
    e.execute("BEGIN").unwrap();
    e.execute("UPDATE t SET v = 99 WHERE id = 10").unwrap();
    assert_eq!(
        rows(&mut e, "d1"),
        vec![(10, 99)],
        "updated inside the transaction"
    );

    e.execute("ROLLBACK").unwrap();

    assert_eq!(
        rows(&mut e, "d1"),
        vec![(10, 1)],
        "ROLLBACK must restore the prior row in d1, not in default"
    );
    assert_eq!(rows(&mut e, "default"), vec![(30, 1)], "default untouched");
}

// ---------------------------------------------------------------------------
// ② no ghost rows in the default database
// ---------------------------------------------------------------------------

/// The replayer must not push anything into `default`.
///
/// The undo's delete/insert used to target the default database (the
/// bare-name API), so a ROLLBACK in `d1` could *delete* a row from
/// `default.t` — a row it was never supposed to touch — while leaving
/// the `d1` row in place. This asserts the default database survives
/// the ROLLBACK row for row.
#[test]
fn rollback_in_a_named_database_leaves_the_default_database_intact_5896() {
    let mut e = engine();
    seeded(&mut e);

    // A default-database row the undo must not touch.
    e.execute("USE `default`").unwrap();
    e.execute("INSERT INTO t (id, v) VALUES (31, 1)").unwrap();
    assert_eq!(
        rows(&mut e, "default"),
        vec![(30, 1), (31, 1)],
        "fixture"
    );

    e.execute("USE `d1`").unwrap();
    e.execute("BEGIN").unwrap();
    e.execute("INSERT INTO t (id, v) VALUES (11, 1)").unwrap();
    e.execute("ROLLBACK").unwrap();

    assert_eq!(
        rows(&mut e, "default"),
        vec![(30, 1), (31, 1)],
        "the default database must be untouched by a ROLLBACK in d1 — \
         no ghost row, and no wrongly-deleted row"
    );
    assert_eq!(
        rows(&mut e, "d1"),
        vec![(10, 1)],
        "and d1 keeps only its own row"
    );
}

// ---------------------------------------------------------------------------
// ③ the default database's behaviour is unchanged
// ---------------------------------------------------------------------------

/// A ROLLBACK in the default database still undoes what it did.
#[test]
fn rollback_in_the_default_database_still_undoes_5896() {
    let mut e = engine();
    seeded(&mut e);

    e.execute("USE `default`").unwrap();
    e.execute("BEGIN").unwrap();
    e.execute("INSERT INTO t (id, v) VALUES (32, 1)").unwrap();
    assert_eq!(
        rows(&mut e, "default"),
        vec![(30, 1), (32, 1)],
        "visible in-transaction"
    );

    e.execute("ROLLBACK").unwrap();

    assert_eq!(
        rows(&mut e, "default"),
        vec![(30, 1)],
        "ROLLBACK must remove the in-transaction INSERT in `default` too"
    );
}

/// The default database's DELETE and UPDATE undo keeps working.
#[test]
fn rollback_in_the_default_database_restores_deleted_and_updated_rows_5896() {
    let mut e = engine();
    seeded(&mut e);

    e.execute("USE `default`").unwrap();
    e.execute("BEGIN").unwrap();
    e.execute("DELETE FROM t WHERE id = 30").unwrap();
    e.execute("ROLLBACK").unwrap();
    assert_eq!(
        rows(&mut e, "default"),
        vec![(30, 1)],
        "ROLLBACK must restore the deleted row in `default`"
    );

    e.execute("USE `default`").unwrap();
    e.execute("BEGIN").unwrap();
    e.execute("UPDATE t SET v = 33 WHERE id = 30").unwrap();
    e.execute("ROLLBACK").unwrap();
    assert_eq!(
        rows(&mut e, "default"),
        vec![(30, 1)],
        "ROLLBACK must restore the updated row in `default`"
    );
}

// ---------------------------------------------------------------------------
// ④ ROLLBACK TO SAVEPOINT goes through the same replayer
// ---------------------------------------------------------------------------

/// The engine's `UndoRecord` replay is the mechanism `ROLLBACK TO
/// SAVEPOINT` drives, and it is **the only** path that exercises the
/// engine layer alone: a plain `ROLLBACK` also goes through the
/// storage backend's own undo, which can mask an engine-layer defect.
///
/// The cases below therefore use SAVEPOINT, so the assertion is about
/// the engine layer and nothing else.
///
/// An INSERT undone by `ROLLBACK TO SAVEPOINT` must vanish from the
/// database it was written in.
#[test]
fn rollback_to_savepoint_removes_the_insert_from_the_named_database_5896() {
    let mut e = engine();
    seeded(&mut e);

    e.execute("USE `d1`").unwrap();
    e.execute("BEGIN").unwrap();
    e.execute("SAVEPOINT s1").unwrap();
    e.execute("INSERT INTO t (id, v) VALUES (11, 1)").unwrap();
    assert_eq!(
        rows(&mut e, "d1"),
        vec![(10, 1), (11, 1)],
        "visible in-transaction"
    );

    e.execute("ROLLBACK TO SAVEPOINT s1").unwrap();

    assert_eq!(
        rows(&mut e, "d1"),
        vec![(10, 1)],
        "ROLLBACK TO SAVEPOINT must remove the row from the database the \
         INSERT was made in"
    );
    assert_eq!(
        rows(&mut e, "default"),
        vec![(30, 1)],
        "and must not touch the same table name in `default`"
    );
}

/// A DELETE undone by `ROLLBACK TO SAVEPOINT` must be restored into the
/// database it was deleted from.
#[test]
fn rollback_to_savepoint_restores_the_deleted_row_in_the_named_database_5896() {
    let mut e = engine();
    seeded(&mut e);

    e.execute("USE `d1`").unwrap();
    e.execute("BEGIN").unwrap();
    e.execute("SAVEPOINT s1").unwrap();
    e.execute("DELETE FROM t WHERE id = 10").unwrap();
    assert!(rows(&mut e, "d1").is_empty(), "gone inside the transaction");

    e.execute("ROLLBACK TO SAVEPOINT s1").unwrap();

    assert_eq!(
        rows(&mut e, "d1"),
        vec![(10, 1)],
        "ROLLBACK TO SAVEPOINT must restore the row in d1, not in default"
    );
    assert_eq!(
        rows(&mut e, "default"),
        vec![(30, 1)],
        "default untouched"
    );
}

/// An UPDATE undone by `ROLLBACK TO SAVEPOINT` must be restored in the
/// database it was applied in.
#[test]
fn rollback_to_savepoint_restores_the_updated_row_in_the_named_database_5896() {
    let mut e = engine();
    seeded(&mut e);

    e.execute("USE `d1`").unwrap();
    e.execute("BEGIN").unwrap();
    e.execute("SAVEPOINT s1").unwrap();
    e.execute("UPDATE t SET v = 99 WHERE id = 10").unwrap();
    assert_eq!(rows(&mut e, "d1"), vec![(10, 99)], "updated in-transaction");

    e.execute("ROLLBACK TO SAVEPOINT s1").unwrap();

    assert_eq!(
        rows(&mut e, "d1"),
        vec![(10, 1)],
        "ROLLBACK TO SAVEPOINT must restore the prior row in d1, not in default"
    );
    assert_eq!(
        rows(&mut e, "default"),
        vec![(30, 1)],
        "default untouched"
    );
}

/// `ROLLBACK TO SAVEPOINT` uses the same `UndoRecord` replay, so it must
/// address the database too.
#[test]
fn rollback_to_savepoint_undoes_in_the_named_database_5896() {
    let mut e = engine();
    seeded(&mut e);

    e.execute("USE `d1`").unwrap();
    e.execute("BEGIN").unwrap();
    e.execute("INSERT INTO t (id, v) VALUES (11, 1)").unwrap();
    e.execute("SAVEPOINT s1").unwrap();
    e.execute("INSERT INTO t (id, v) VALUES (12, 1)").unwrap();
    assert_eq!(
        rows(&mut e, "d1"),
        vec![(10, 1), (11, 1), (12, 1)],
        "visible in-transaction"
    );

    e.execute("ROLLBACK TO SAVEPOINT s1").unwrap();

    assert_eq!(
        rows(&mut e, "d1"),
        vec![(10, 1), (11, 1)],
        "ROLLBACK TO SAVEPOINT must undo only what came after the savepoint, \
         in the database the writes happened in"
    );
    assert_eq!(rows(&mut e, "default"), vec![(30, 1)], "default untouched");

    e.execute("ROLLBACK").unwrap();
    assert_eq!(
        rows(&mut e, "d1"),
        vec![(10, 1)],
        "the outer ROLLBACK must then undo the rest, still in d1"
    );
}
