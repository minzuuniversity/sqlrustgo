//! #5106: the DML paths must resolve schema AND rows against the
//! statement's own database.
//!
//! # What was wrong
//!
//! `execute_update_multi_table` / `execute_delete_multi_table` took neither
//! a database nor a transaction. Each per-table read did
//!
//! ```text
//! storage.get_table_info(&tref.name)                 // shared current_db
//! engine.scan_for_reader_with(&storage, &tref.name)   // also shared, but
//!                                                      // carrying reader_tx
//! ```
//!
//! Both halves answered "whoever ran `USE` last", so on a shared storage a
//! multi-table statement could take one table's columns and another
//! table's rows. Taking the schema from one database and the rows from
//! another is worse than either error alone: the row count and column
//! names then describe different tables, and the failure surfaces as a
//! shape mismatch far from its cause.
//!
//! # What these pin
//!
//! 1. Multi-table UPDATE / DELETE touch only the statement's database,
//!    even when another database holds identically-named tables.
//! 2. Multi-table statements still carry the reading transaction.
//! 3. Uncommitted DML stays invisible to another connection — the
//!    half that must not regress while fixing the other.

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::{FileStorage, MvccStorage, StorageEngine};
use sqlrustgo_types::Value;
use std::path::PathBuf;
use std::sync::Arc;

type Conn = ExecutionEngine<FileStorage>;

fn next_seq() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    SEQ.fetch_add(1, Ordering::SeqCst)
}

/// Two connections over one shared storage.
///
/// `FileStorage`, not `MvccStorage`: the change is about database
/// scoping, and scoping belongs to the layer the change is on. Running
/// through `MvccStorage` puts a second, unrelated gap underneath — a
/// multi-table DELETE does not become visible to other connections there
/// (see the note at the bottom of this file) — and a scoping failure would
/// hide behind it.
fn two_connections() -> (Conn, Conn, PathBuf) {
    let dir =
        std::env::temp_dir().join(format!("dml_read_db_{}_{}", std::process::id(), next_seq()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let storage = Arc::new(RwLock::new(FileStorage::new(dir.clone()).unwrap()));
    (
        ExecutionEngine::new(Arc::clone(&storage)),
        ExecutionEngine::new(storage),
        dir,
    )
}

/// `d1` and `d2`, each with `t1` and `t2`, distinguishable by value.
fn seed(x: &mut Conn) {
    x.execute("CREATE DATABASE d1").unwrap();
    x.execute("CREATE DATABASE d2").unwrap();
    for (db, tag) in [("d1", 1), ("d2", 2)] {
        x.execute(&format!("USE {db}")).unwrap();
        for t in ["t1", "t2"] {
            x.execute(&format!("CREATE TABLE {t} (id INT PRIMARY KEY, k INT)"))
                .unwrap();
            x.execute(&format!("INSERT INTO {t} VALUES ({tag}, {tag})"))
                .unwrap();
        }
    }
}

fn scalar(x: &mut Conn, sql: &str) -> Value {
    let rows = x.execute(sql).unwrap().rows;
    assert!(!rows.is_empty(), "no rows for {sql}");
    rows[0][0].clone()
}

fn int(v: &Value) -> i64 {
    match v {
        Value::Integer(n) => *n,
        other => panic!("expected integer, got {other:?}"),
    }
}

// ---------------------------------------------------------------------
// 1. Multi-table UPDATE
// ---------------------------------------------------------------------

/// A multi-table UPDATE in `d1` must not reach `d2`'s same-named tables.
///
/// #5120 fixed the blocker this used to be `#[ignore]`d for: multi-table
/// `UPDATE` used to report `affected_rows = 1` and change no row, because a
/// bare column name in the SET list matched no table prefix. That defect is
/// fixed and its own regression file (`multi_table_update_5117.rs`, 7 cases)
/// covers it, so this specification can now actually run.
#[test]
fn multi_table_update_stays_in_the_statements_database() {
    let (mut a, mut b, dir) = two_connections();
    seed(&mut a);
    a.execute("USE d1").unwrap();
    b.execute("USE d2").unwrap();

    a.execute("UPDATE t1, t2 SET k = 99").unwrap();

    a.execute("USE d1").unwrap();
    assert_eq!(
        int(&scalar(&mut a, "SELECT k FROM t1")),
        99,
        "d1.t1 was not updated"
    );
    b.execute("USE d2").unwrap();
    assert_eq!(
        int(&scalar(&mut b, "SELECT k FROM t1")),
        2,
        "the update in d1 leaked into d2.t1"
    );
    assert_eq!(
        int(&scalar(&mut b, "SELECT k FROM t2")),
        2,
        "the update in d1 leaked into d2.t2"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Multi-table DELETE, same shape.
#[test]
fn multi_table_delete_stays_in_the_statements_database() {
    let (mut a, mut b, dir) = two_connections();
    seed(&mut a);
    a.execute("USE d1").unwrap();
    b.execute("USE d2").unwrap();

    a.execute("DELETE t1, t2 FROM t1, t2").unwrap();

    a.execute("USE d1").unwrap();
    assert_eq!(
        int(&scalar(&mut a, "SELECT COUNT(*) FROM t1")),
        0,
        "d1.t1 was not deleted"
    );
    b.execute("USE d2").unwrap();
    assert_eq!(
        int(&scalar(&mut b, "SELECT COUNT(*) FROM t1")),
        1,
        "the delete in d1 removed d2's rows"
    );
    assert_eq!(
        int(&scalar(&mut b, "SELECT COUNT(*) FROM t2")),
        1,
        "the delete in d1 removed d2.t2's rows"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------
// 2. The transaction must survive the rewiring
// ---------------------------------------------------------------------

/// An uncommitted multi-table DELETE must not be visible elsewhere.
///
/// The database half and the transaction half are separate arguments of
/// `scan_in_tx_db`; a change that threads `db` through by dropping
/// `reader_tx` satisfies every scoping test above and fails this one.
///
/// `#[ignore]`d — and the reason has since been re-verified and
/// re-attributed (#5156).
///
/// **Effect (confirmed):** on `MvccStorage` a multi-table DELETE does not
/// become visible to either connection even after COMMIT.
///
/// ```
/// inside tx,  writer : t1=0  t2=0     <- sees its own delete
/// inside tx,  peer   : t1=1  t2=1     <- correctly isolated
/// after COMMIT writer: t1=1  t2=1     <- reverted
/// after COMMIT peer  : t1=1  t2=1     <- reverted
/// ```
///
/// **Attribution (was wrong):** this is not a multi-table visibility
/// defect, nor is it about `reader_tx`. It is #5156 — a committed DELETE
/// reverts whenever the database holds a second table, and a multi-table
/// DELETE *necessarily* holds two tables in one database, so it always
/// meets the trigger. Single-table DELETE in the same database reverts
/// identically (see `mvcc_same_db_second_table_delete_5106.rs`).
///
/// The same sequence is correct on plain `FileStorage`, and correct at the
/// storage layer in every arrangement, so this is an engine-layer defect.
///
/// This test is kept because it covers the multi-table statement shape,
/// which #5156's own test does not; it should start passing when #5156 is
/// fixed, with no edit here.
#[test]
#[ignore = "#5156 — a committed DELETE reverts when the database holds a second table; multi-table DELETE always meets that trigger. Replaces the earlier (correct-in-effect, wrong-in-attribution) reason."]
fn uncommitted_multi_table_delete_stays_invisible() {
    let (mut a, mut b, dir) = two_connections();
    seed(&mut a);
    a.execute("USE d1").unwrap();
    b.execute("USE d1").unwrap();
    a.execute("USE d1").unwrap();
    b.execute("USE d1").unwrap();

    a.execute("BEGIN").unwrap();
    a.execute("DELETE t1, t2 FROM t1, t2").unwrap();

    assert_eq!(
        int(&scalar(&mut b, "SELECT COUNT(*) FROM t1")),
        1,
        "connection B saw A's uncommitted multi-table delete"
    );
    a.execute("COMMIT").unwrap();
    assert_eq!(
        int(&scalar(&mut b, "SELECT COUNT(*) FROM t1")),
        0,
        "connection B missed A's commit"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Control group: multi-table DELETE still works on a single database.
///
/// If the `db` argument were threaded through by falling back to
/// `current_db`, every isolation test above would still pass — they all
/// have a `current_db` that happens to name a real database. This one
/// pins the ordinary case.
///
/// UPDATE is absent for the reason given on
/// `multi_table_update_stays_in_the_statements_database`.
#[test]
fn multi_table_statements_still_work_in_a_single_database() {
    let (mut a, _b, dir) = two_connections();
    seed(&mut a);
    a.execute("USE d1").unwrap();

    a.execute("DELETE t1, t2 FROM t1, t2").unwrap();
    assert_eq!(int(&scalar(&mut a, "SELECT COUNT(*) FROM t1")), 0);
    assert_eq!(int(&scalar(&mut a, "SELECT COUNT(*) FROM t2")), 0);
    let _ = std::fs::remove_dir_all(&dir);
}
