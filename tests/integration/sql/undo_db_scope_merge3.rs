//! The merge-state end-to-end case for the three undo paths.
//!
//! Three separate defects shared one shape — the undo log named only the
//! **table**, never the **database** — and each had to be fixed on its
//! own layer:
//!
//! * **#5891** — the storage layer: `FileStorage`'s per-transaction
//!   undo log ([`UndoOp`]) resolved the bare table name against
//!   `DEFAULT_DATABASE`.
//! * **#5896** — the engine layer: `UndoRecord` (crates/transaction)
//!   replayed through the database-less `storage.delete` / `storage.insert`.
//! * **#5897** — `MemoryStorage`'s `TxLog`, replayed through the
//!   database active at replay time.
//!
//! Fixing any one of them is not enough: a ROLLBACK goes through the
//! engine, which drives the storage backend, and each layer's undo log
//! is consulted on its own. This file is the case that only passes when
//! **all three** are fixed — which is why it lives in its own file and
//! is run against the merged tree, not against any single PR.
//!
//! # This file is a MERGE-STATE gate — it is meant to run on the merged
//! tree, not on any single PR
//!
//! The three defects have to be fixed *together* to make a ROLLBACK in a
//! named database behave. Each layer keeps its own undo log, and a
//! ROLLBACK consults both:
//!
//! * the storage backend's own log (`FileStorage`'s per-transaction
//!   `UndoOp` log; `MemoryStorage`'s `TxLog`) — replayed by the backend;
//! * the engine's `UndoRecord` — replayed by the engine.
//!
//! Before the three fixes, **both** logs named only the table and
//! resolved it against the default database, so a ROLLBACK in `d1` undid
//! nothing there and could write into `default`. Fixing one log without
//! the other leaves the behaviour half-wrong — and fixing **both** used
//! to leave a deleted row restored **twice** (#5905), because each side
//! restored it on its own.
//!
//! ## Two independent judgements — there is no NOT-RUN state
//!
//! A suite that only holds on the merged tree must not be able to report
//! success on a tree that has not been merged. So the gate is split into
//! two halves that are **judged independently**:
//!
//! * the **static** half (`merge_state_fix_markers_present_...`) checks
//!   that the three fixes are present, by reading their source paths
//!   (`UndoRecord::Delete`, `UndoOp::DeleteRow`, `TxLog`'s tuple head);
//! * the **dynamic** half (the ten cases below) checks the resulting
//!   ROLLBACK behaviour, and each case verifies the marks first and
//!   **fails** if they are missing.
//!
//! Why the marker check is by source and not by behaviour: a behavioural
//! probe cannot tell "the fix is missing" apart from "the fix regressed"
//! — both look like "the rollback did not restore the row" — and the
//! second one must fail, not be skipped.
//!
//! Why there is no NOT-RUN: a tree without the three fixes genuinely has
//! the wrong ROLLBACK semantics, so the gate must not report success for
//! it. An earlier version reported NOT-RUN when the marks were absent;
//! that state could be read as a pass, which is exactly what GA evidence
//! must not allow. **The merge-state gate is satisfied only when it is
//! green** — the static check passing and every behavioural assertion
//! passing.
//!
//! On the develop tip — before #5899 / #5902 / #5903 are merged — this
//! gate is therefore **red on purpose**. That redness is the proof that
//! the fixes are not in yet.
//!
//! ## Merge order
//!
//! The three fixes live in separate PRs (#5899 storage layer, #5902
//! `MemoryStorage`, #5903 engine layer). This gate is the check on the
//! tree they produce, so it merges **last**.
//!
//! # What it asserts
//!
//! A ROLLBACK in a named database (`d1`) must:
//!
//! * actually undo the write **in `d1`** (not leave the row visible),
//! * leave `default` **byte for byte unchanged** (no ghost row written
//!   there, and no row of its own wrongly deleted),
//! * and behave identically in the `default` database (no regression).
//!
//! # Backends
//!
//! Both production backends are covered, because the two layers of the
//! undo path are split differently between them:
//!
//! * `FileStorage` — the storage layer's own undo log does the work
//!   (#5891), and the engine's `UndoRecord` path is exercised on top
//!   (#5896);
//! * `MemoryStorage` — the transaction log does the work (#5897).
//!
//! The engine drives both, so the engine's `UndoRecord` path is on the
//! critical path in both cases.

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::{FileStorage, MemoryStorage};
use std::path::PathBuf;
use std::sync::Arc;

/// A `FileStorage`-backed engine on a fresh directory.
///
/// The directory name carries the process id **and** a per-call counter:
/// these tests run in parallel by default and every one of them wipes
/// and recreates its directory, so without the counter they deleted each
/// other's data mid-run. Same pattern as the sibling
/// `insert_db_isolation_5057` / `session_db_isolation_5057` suites.
fn file_engine() -> (ExecutionEngine<FileStorage>, PathBuf) {
    static SEQ: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("undo_merge3_file_{}_{}", std::process::id(), n));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let e = ExecutionEngine::new(Arc::new(RwLock::new(
        FileStorage::new(dir.clone()).unwrap(),
    )));
    (e, dir)
}

/// A `MemoryStorage`-backed engine. `MemoryStorage` is in-process, so
/// there is no directory to manage — but the engine is still fresh per
/// call, because the engine carries session state.
fn memory_engine() -> ExecutionEngine<MemoryStorage> {
    static SEQ: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let _ = n;
    ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())))
}

/// `d1` and `default` each hold a table `t` with one row.
///
/// The row's `id` says which database it came from (so a row that shows
/// up in the wrong database is visible by `id` alone), and `v` is a
/// plain column the UPDATE cases touch.
fn seed(e: &mut ExecutionEngine<FileStorage>) {
    e.execute("CREATE DATABASE d1").unwrap();
    e.execute("USE `d1`").unwrap();
    e.execute("CREATE TABLE t (id INT PRIMARY KEY, v INT)")
        .unwrap();
    e.execute("INSERT INTO t (id, v) VALUES (10, 1)").unwrap();
    e.execute("USE `default`").unwrap();
    e.execute("CREATE TABLE t (id INT PRIMARY KEY, v INT)")
        .unwrap();
    e.execute("INSERT INTO t (id, v) VALUES (30, 1)").unwrap();
}

fn seed_memory(e: &mut ExecutionEngine<MemoryStorage>) {
    e.execute("CREATE DATABASE d1").unwrap();
    e.execute("USE `d1`").unwrap();
    e.execute("CREATE TABLE t (id INT PRIMARY KEY, v INT)")
        .unwrap();
    e.execute("INSERT INTO t (id, v) VALUES (10, 1)").unwrap();
    e.execute("USE `default`").unwrap();
    e.execute("CREATE TABLE t (id INT PRIMARY KEY, v INT)")
        .unwrap();
    e.execute("INSERT INTO t (id, v) VALUES (30, 1)").unwrap();
}

/// The rows of `db.t` as `(id, v)` pairs, ordered by id.
fn file_rows(e: &mut ExecutionEngine<FileStorage>, db: &str) -> Vec<(i64, i64)> {
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

fn memory_rows(e: &mut ExecutionEngine<MemoryStorage>, db: &str) -> Vec<(i64, i64)> {
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
// The applicability probe
// ---------------------------------------------------------------------------

/// Whether this tree carries the three database-scope fixes.
///
/// The decision is made from **source markers**, not from behaviour.
///
/// A behavioural probe cannot tell "the fix is missing" apart from "the fix
/// regressed" — both look like "the rollback did not restore the row" — and
/// the second one must **fail**, not be skipped. So the markers decide
/// whether the suite applies at all, and once they are present the
/// assertions below run **unconditionally**: any behavioural difference is
/// a test failure.
///
/// A source file that cannot be read is a failure too, not a skip: a
/// missing source means the evidence would be meaningless.
fn tree_carries_the_db_scope_fixes() -> bool {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));

    // #5896 — the engine's `UndoRecord` carries the database.
    let savepoint = std::fs::read_to_string(root.join("crates/transaction/src/savepoint.rs"))
        .expect("crates/transaction/src/savepoint.rs must be readable");
    // #5891 — the storage layer's `UndoOp` carries the database.
    let file_storage = std::fs::read_to_string(root.join("crates/storage/src/file_storage.rs"))
        .expect("crates/storage/src/file_storage.rs must be readable");
    // #5897 — `MemoryStorage`'s `TxLog` carries the database.
    let engine = std::fs::read_to_string(root.join("crates/storage/src/engine.rs"))
        .expect("crates/storage/src/engine.rs must be readable");

    /// Whether the block that starts at `anchor` carries `needle`.
    ///
    /// The anchor is the **semantic path this gate exercises** — the
    /// DELETE variant of each undo log — not a loose keyword, so the
    /// check cannot be satisfied by an unrelated field elsewhere.
    fn block_carries(source: &str, anchor: &str, needle: &str) -> bool {
        let start = match source.find(anchor) {
            Some(i) => i,
            None => return false,
        };
        let rest = &source[start..];
        let end = rest.find('}').unwrap_or(rest.len());
        rest[..end].contains(needle)
    }

    // #5896 — the engine's `UndoRecord::Delete` carries the database.
    // #5891 — the storage layer's `UndoOp::DeleteRow` carries it.
    // #5897 — `MemoryStorage`'s `TxLog` carries it as the first element
    //         of each tuple: `(db, table, row)`.
    block_carries(&savepoint, "Delete {", "db: String")
        && block_carries(&file_storage, "DeleteRow {", "db: String")
        && block_carries(&engine, "pub struct TxLog", "String, String, Record")
}

/// Whether this tree carries the three database-scope fixes.
///
/// Probed once per process: the answer cannot change while the suite
/// runs, and reading the sources is not free.
fn suite_applies() -> bool {
    static STATE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *STATE.get_or_init(tree_carries_the_db_scope_fixes)
}

/// The **static** half of the gate, judged on its own.
///
/// A source marker proves only that the fix path EXISTS — it cannot prove
/// the behaviour is right. The dynamic half below proves the behaviour.
/// Both are judged independently, and a missing marker is a FAILURE, not a
/// skip: a tree without the three fixes genuinely has the wrong ROLLBACK
/// semantics, so the gate must not report success for it. (An earlier
/// version reported NOT-RUN here; that state could be mistaken for a pass,
/// which is exactly what the GA evidence must not allow.)
#[test]
fn merge_state_fix_markers_present_5891_5896_5897() {
    assert!(
        tree_carries_the_db_scope_fixes(),
        "合并态门禁：三条库身份修复（#5891 存储层 / #5896 引擎层 / #5897 MemoryStorage）\
         尚未全部合入 —— 本门禁要求它们同时到位（合并顺序：#5899 → #5902 → #5903 → 本 PR）。\
         当前树缺失修复路径，ROLLBACK 语义尚未正确，因此判红而非跳过。"
    );
}

// ---------------------------------------------------------------------------
// FileStorage — storage-layer undo log (#5891) + engine-layer UndoRecord (#5896)
// ---------------------------------------------------------------------------

/// An INSERT in `d1` undone by ROLLBACK: gone from `d1`, `default` untouched.
#[test]
fn file_backend_rollback_in_a_named_database_undoes_the_insert_5891_5896_5897() {
    if !suite_applies() {
        panic!(
            "合并态门禁：三条库身份修复尚未全部合入，ROLLBACK 语义未正确 —— 判红而非跳过\
             （合并顺序：#5899 → #5902 → #5903 → 本 PR）。"
        );
    }

    let (mut e, dir) = file_engine();
    seed(&mut e);

    e.execute("USE `d1`").unwrap();
    e.execute("BEGIN").unwrap();
    e.execute("INSERT INTO t (id, v) VALUES (11, 1)").unwrap();
    assert_eq!(
        file_rows(&mut e, "d1"),
        vec![(10, 1), (11, 1)],
        "visible in-transaction"
    );

    e.execute("ROLLBACK").unwrap();

    assert_eq!(
        file_rows(&mut e, "d1"),
        vec![(10, 1)],
        "ROLLBACK must remove the row from the database the INSERT was made in"
    );
    assert_eq!(
        file_rows(&mut e, "default"),
        vec![(30, 1)],
        "and must not touch the same table name in `default`"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// A DELETE in `d1` undone by ROLLBACK: restored into `d1`.
#[test]
fn file_backend_rollback_in_a_named_database_restores_the_deleted_row_5891_5896_5897() {
    if !suite_applies() {
        panic!(
            "合并态门禁：三条库身份修复尚未全部合入，ROLLBACK 语义未正确 —— 判红而非跳过\
             （合并顺序：#5899 → #5902 → #5903 → 本 PR）。"
        );
    }

    let (mut e, dir) = file_engine();
    seed(&mut e);

    e.execute("USE `d1`").unwrap();
    e.execute("BEGIN").unwrap();
    e.execute("DELETE FROM t WHERE id = 10").unwrap();
    assert!(
        file_rows(&mut e, "d1").is_empty(),
        "gone inside the transaction"
    );

    e.execute("ROLLBACK").unwrap();

    assert_eq!(
        file_rows(&mut e, "d1"),
        vec![(10, 1)],
        "ROLLBACK must restore the row in d1, not in default"
    );
    assert_eq!(
        file_rows(&mut e, "default"),
        vec![(30, 1)],
        "default untouched"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// An UPDATE in `d1` undone by ROLLBACK: restored in `d1`.
///
/// The UPDATE changes a **non-key** column: the engine's undo deletes by
/// the *original* key before re-inserting the prior row, so an UPDATE
/// that changes the **primary key** is not undone by that path. That is
/// a separate, pre-existing gap (it reproduces in the `default` database
/// too), not a database-scope problem.
#[test]
fn file_backend_rollback_in_a_named_database_restores_the_updated_row_5891_5896_5897() {
    if !suite_applies() {
        panic!(
            "合并态门禁：三条库身份修复尚未全部合入，ROLLBACK 语义未正确 —— 判红而非跳过\
             （合并顺序：#5899 → #5902 → #5903 → 本 PR）。"
        );
    }

    let (mut e, dir) = file_engine();
    seed(&mut e);

    e.execute("USE `d1`").unwrap();
    e.execute("BEGIN").unwrap();
    e.execute("UPDATE t SET v = 99 WHERE id = 10").unwrap();
    assert_eq!(
        file_rows(&mut e, "d1"),
        vec![(10, 99)],
        "updated inside the transaction"
    );

    e.execute("ROLLBACK").unwrap();

    assert_eq!(
        file_rows(&mut e, "d1"),
        vec![(10, 1)],
        "ROLLBACK must restore the prior row in d1, not in default"
    );
    assert_eq!(
        file_rows(&mut e, "default"),
        vec![(30, 1)],
        "default untouched"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// A ROLLBACK in `d1` must leave the `default` database exactly as it
/// was — no ghost row written there, and none of its own rows deleted.
#[test]
fn file_backend_rollback_in_a_named_database_leaves_default_intact_5891_5896_5897() {
    if !suite_applies() {
        panic!(
            "合并态门禁：三条库身份修复尚未全部合入，ROLLBACK 语义未正确 —— 判红而非跳过\
             （合并顺序：#5899 → #5902 → #5903 → 本 PR）。"
        );
    }

    let (mut e, dir) = file_engine();
    seed(&mut e);

    e.execute("USE `default`").unwrap();
    e.execute("INSERT INTO t (id, v) VALUES (31, 1)").unwrap();
    assert_eq!(
        file_rows(&mut e, "default"),
        vec![(30, 1), (31, 1)],
        "fixture"
    );

    e.execute("USE `d1`").unwrap();
    e.execute("BEGIN").unwrap();
    e.execute("INSERT INTO t (id, v) VALUES (11, 1)").unwrap();
    e.execute("ROLLBACK").unwrap();

    assert_eq!(
        file_rows(&mut e, "default"),
        vec![(30, 1), (31, 1)],
        "the default database must be untouched by a ROLLBACK in d1"
    );
    assert_eq!(
        file_rows(&mut e, "d1"),
        vec![(10, 1)],
        "and d1 keeps only its own row"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// `ROLLBACK TO SAVEPOINT` goes through the **engine**'s `UndoRecord`
/// replay — the one path where the storage backend's own undo does not
/// do the work — so it is the sharpest check of the engine layer.
#[test]
fn file_backend_rollback_to_savepoint_undoes_in_the_named_database_5891_5896_5897() {
    if !suite_applies() {
        panic!(
            "合并态门禁：三条库身份修复尚未全部合入，ROLLBACK 语义未正确 —— 判红而非跳过\
             （合并顺序：#5899 → #5902 → #5903 → 本 PR）。"
        );
    }

    let (mut e, dir) = file_engine();
    seed(&mut e);

    e.execute("USE `d1`").unwrap();
    e.execute("BEGIN").unwrap();
    e.execute("SAVEPOINT s1").unwrap();
    e.execute("INSERT INTO t (id, v) VALUES (11, 1)").unwrap();
    e.execute("UPDATE t SET v = 55 WHERE id = 10").unwrap();
    assert_eq!(
        file_rows(&mut e, "d1"),
        vec![(10, 55), (11, 1)],
        "visible in-transaction"
    );

    e.execute("ROLLBACK TO SAVEPOINT s1").unwrap();

    assert_eq!(
        file_rows(&mut e, "d1"),
        vec![(10, 1)],
        "ROLLBACK TO SAVEPOINT must undo both the INSERT and the UPDATE, \
         in the database the writes happened in"
    );
    assert_eq!(
        file_rows(&mut e, "default"),
        vec![(30, 1)],
        "default untouched"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// The `default` database keeps working — no regression on the path that
/// was already correct.
#[test]
fn file_backend_rollback_in_the_default_database_still_undoes_5891_5896_5897() {
    if !suite_applies() {
        panic!(
            "合并态门禁：三条库身份修复尚未全部合入，ROLLBACK 语义未正确 —— 判红而非跳过\
             （合并顺序：#5899 → #5902 → #5903 → 本 PR）。"
        );
    }

    let (mut e, dir) = file_engine();
    seed(&mut e);

    e.execute("USE `default`").unwrap();
    e.execute("BEGIN").unwrap();
    e.execute("INSERT INTO t (id, v) VALUES (32, 1)").unwrap();
    assert_eq!(
        file_rows(&mut e, "default"),
        vec![(30, 1), (32, 1)],
        "visible in-transaction"
    );

    e.execute("ROLLBACK").unwrap();

    assert_eq!(
        file_rows(&mut e, "default"),
        vec![(30, 1)],
        "ROLLBACK must remove the in-transaction INSERT in `default` too"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// MemoryStorage — transaction log (#5897)
// ---------------------------------------------------------------------------

/// The same three statements against `MemoryStorage`, whose undo lives
/// in the `TxLog` (#5897) rather than in a per-table undo log.
#[test]
fn memory_backend_rollback_in_a_named_database_undoes_the_insert_5891_5896_5897() {
    if !suite_applies() {
        panic!(
            "合并态门禁：三条库身份修复尚未全部合入，ROLLBACK 语义未正确 —— 判红而非跳过\
             （合并顺序：#5899 → #5902 → #5903 → 本 PR）。"
        );
    }

    let mut e = memory_engine();
    seed_memory(&mut e);

    e.execute("USE `d1`").unwrap();
    e.execute("BEGIN").unwrap();
    e.execute("INSERT INTO t (id, v) VALUES (11, 1)").unwrap();
    assert_eq!(
        memory_rows(&mut e, "d1"),
        vec![(10, 1), (11, 1)],
        "visible in-transaction"
    );

    e.execute("ROLLBACK").unwrap();

    assert_eq!(
        memory_rows(&mut e, "d1"),
        vec![(10, 1)],
        "ROLLBACK must remove the row from the database the INSERT was made in"
    );
    assert_eq!(
        memory_rows(&mut e, "default"),
        vec![(30, 1)],
        "and must not touch the same table name in `default`"
    );
}

/// A DELETE in `d1` undone by ROLLBACK, on `MemoryStorage`.
#[test]
fn memory_backend_rollback_in_a_named_database_restores_the_deleted_row_5891_5896_5897() {
    if !suite_applies() {
        panic!(
            "合并态门禁：三条库身份修复尚未全部合入，ROLLBACK 语义未正确 —— 判红而非跳过\
             （合并顺序：#5899 → #5902 → #5903 → 本 PR）。"
        );
    }

    let mut e = memory_engine();
    seed_memory(&mut e);

    e.execute("USE `d1`").unwrap();
    e.execute("BEGIN").unwrap();
    e.execute("DELETE FROM t WHERE id = 10").unwrap();
    assert!(
        memory_rows(&mut e, "d1").is_empty(),
        "gone inside the transaction"
    );

    e.execute("ROLLBACK").unwrap();

    assert_eq!(
        memory_rows(&mut e, "d1"),
        vec![(10, 1)],
        "ROLLBACK must restore the row in d1, not in default"
    );
    assert_eq!(
        memory_rows(&mut e, "default"),
        vec![(30, 1)],
        "default untouched"
    );
}

/// An UPDATE in `d1` undone by ROLLBACK, on `MemoryStorage`.
#[test]
fn memory_backend_rollback_in_a_named_database_restores_the_updated_row_5891_5896_5897() {
    if !suite_applies() {
        panic!(
            "合并态门禁：三条库身份修复尚未全部合入，ROLLBACK 语义未正确 —— 判红而非跳过\
             （合并顺序：#5899 → #5902 → #5903 → 本 PR）。"
        );
    }

    let mut e = memory_engine();
    seed_memory(&mut e);

    e.execute("USE `d1`").unwrap();
    e.execute("BEGIN").unwrap();
    e.execute("UPDATE t SET v = 99 WHERE id = 10").unwrap();
    assert_eq!(
        memory_rows(&mut e, "d1"),
        vec![(10, 99)],
        "updated inside the transaction"
    );

    e.execute("ROLLBACK").unwrap();

    assert_eq!(
        memory_rows(&mut e, "d1"),
        vec![(10, 1)],
        "ROLLBACK must restore the prior row in d1, not in default"
    );
    assert_eq!(
        memory_rows(&mut e, "default"),
        vec![(30, 1)],
        "default untouched"
    );
}

/// The `default` database keeps working on `MemoryStorage` too.
#[test]
fn memory_backend_rollback_in_the_default_database_still_undoes_5891_5896_5897() {
    if !suite_applies() {
        panic!(
            "合并态门禁：三条库身份修复尚未全部合入，ROLLBACK 语义未正确 —— 判红而非跳过\
             （合并顺序：#5899 → #5902 → #5903 → 本 PR）。"
        );
    }

    let mut e = memory_engine();
    seed_memory(&mut e);

    e.execute("USE `default`").unwrap();
    e.execute("BEGIN").unwrap();
    e.execute("INSERT INTO t (id, v) VALUES (32, 1)").unwrap();
    assert_eq!(
        memory_rows(&mut e, "default"),
        vec![(30, 1), (32, 1)],
        "visible in-transaction"
    );

    e.execute("ROLLBACK").unwrap();

    assert_eq!(
        memory_rows(&mut e, "default"),
        vec![(30, 1)],
        "ROLLBACK must remove the in-transaction INSERT in `default` too"
    );
}
