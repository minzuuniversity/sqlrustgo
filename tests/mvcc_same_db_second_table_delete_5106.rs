//! #5156 (a #5106 follow-up): with a SECOND table in the same database, a
//! committed DELETE silently reverted.
//!
//! **Fixed.** `MvccStorage::promote_pending_for` stamped promoted versions
//! with a timestamp read from a lazily-created `__commit_probe__` table.
//! `snapshot_counter` is per-`VersionedTable`, so that value had no relation
//! to any real table's `visible_from_ts` sequence — and `commit_tx`
//! OVERWRITES that field. A lagging stamp put the tombstone behind versions
//! that were already newer; `find_visible` walks the chain newest-first, so
//! it returned the older `put` and the deleted row came back.
//!
//! Each table now stamps its own commit from its own counter
//! (`VersionedTable::commit_tx_auto`), which keeps the ordering
//! self-consistent no matter what other tables have done.
//!
//! ## Symptom (before the fix)
//!
//! Production stack (`MvccStorage<FileStorage>`), named database `d1`:
//!
//! ```text
//! USE d1; CREATE TABLE t1(k INT PRIMARY KEY); INSERT INTO t1 VALUES (1);
//! USE d1; CREATE TABLE t2(k INT PRIMARY KEY); INSERT INTO t2 VALUES (1);  <-- the trigger
//! BEGIN;  DELETE FROM t1 WHERE k = 1;   -- affected_rows = 1
//! COMMIT;
//! SELECT COUNT(*) FROM t1;              -- 1  <-- expected 0
//! ```
//!
//! Everything before COMMIT is correct, which is what hides it:
//!
//! | point | observed | expected |
//! |---|---|---|
//! | `DELETE` affected rows | 1 | 1 |
//! | writer's read inside the tx | 0 | 0 |
//! | peer connection inside the tx | 1 | 1 |
//! | **both connections after COMMIT** | **1** | 0 |
//!
//! The delete applies, is correctly invisible while uncommitted, and then
//! **comes back at COMMIT** — no error, and the write connection sees it
//! too, so it is not a cross-connection visibility problem.
//!
//! ## The trigger is narrow and reproducible
//!
//! Everything else held constant, only the presence of `t2` varies:
//!
//! | second table | writer after COMMIT | correct? |
//! |---|---|---|
//! | none | 0 | yes |
//! | `t2` in the **same** database `d1` | **1** | **no — the row is back** |
//! | `t2` in a **different** database `d2` | 0 | yes |
//!
//! So the trigger is a second table in the *same* database, not a second
//! table anywhere.
//!
//! ## Not a storage-layer defect
//!
//! The same sequence driven straight at the storage layer — no engine — is
//! correct in every arrangement above, including two tables in one named
//! database:
//!
//! ```text
//! s.create_database("d1"); s.set_current_db("d1");
//! s.create_table("t1"); s.insert("t1", [1]);
//! s.create_table("t2"); s.insert("t2", [1]);
//! s.begin_transaction_for(10);
//! s.delete("t1", [1]);          // removed = 1; scan_in(0)=1  scan_in(10)=0
//! s.commit_transaction_for(10);
//!                              // scan(0)=0    scan_in(0)=0     <-- correct
//! ```
//!
//! `VersionedTable::delete`, `commit_tx` / `promote_pending_for` and
//! `find_visible` therefore all behave. The divergence appears only through
//! the engine, which makes the commit timestamp's provenance the prime
//! suspect: `MvccStorage::promote_pending_for` stamps promoted versions with
//! `self.mvcc_table("__commit_probe__").next_snapshot_ts()`, i.e. from a
//! throwaway table, while the read path samples the snapshot off the real
//! table. If those two counters can diverge, the tombstone can be stamped
//! above the reader's snapshot and `find_visible` falls through to the older
//! `put` version — the row reappearing. That matches the trigger: a second
//! table in the same database is exactly what would perturb the counters.
//!
//! Status: **root cause not yet confirmed** — the storage-level equivalence
//! rules the storage layer out but does not by itself prove the counter
//! mechanism. All three permutations are live now; the two negative controls
//! still guard against the OPPOSITE defect (a read path that simply stopped
//! reporting deleted rows would also make the positive case pass).

use parking_lot::RwLock;
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::{FileStorage, MvccStorage};
use std::path::PathBuf;
use std::sync::Arc;

type Conn = ExecutionEngine<MvccStorage<FileStorage>>;

fn next_seq() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    SEQ.fetch_add(1, Ordering::SeqCst)
}

fn two_connections() -> (Conn, Conn, PathBuf) {
    let dir = std::env::temp_dir().join(format!(
        "same_db_two_tables_{}_{}",
        std::process::id(),
        next_seq()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let inner = MvccStorage::new(FileStorage::new(dir.clone()).unwrap());
    inner.rebuild_from_inner().expect("rebuild MVCC chain");
    let storage = Arc::new(RwLock::new(inner));
    (
        ExecutionEngine::new(Arc::clone(&storage)),
        ExecutionEngine::new(storage),
        dir,
    )
}

fn count(e: &mut Conn, table: &str) -> i64 {
    e.execute(&format!("SELECT COUNT(*) FROM {table}"))
        .unwrap()
        .rows
        .into_iter()
        .next()
        .unwrap()
        .into_iter()
        .next()
        .unwrap()
        .as_integer()
        .unwrap()
}

/// The P0 that #5156 fixed. Needs a peer connection reading the table while
/// the writer's transaction is open — with a single connection it does not
/// reproduce.
#[test]

fn committed_delete_reverts_when_the_database_holds_a_second_table() {
    let (mut a, mut b, dir) = two_connections();
    a.execute("CREATE DATABASE d1").unwrap();
    a.execute("USE d1").unwrap();
    a.execute("CREATE TABLE t1 (k INT PRIMARY KEY)").unwrap();
    a.execute("INSERT INTO t1 VALUES (1)").unwrap();
    // The trigger: a second table in the SAME database.
    a.execute("CREATE TABLE t2 (k INT PRIMARY KEY)").unwrap();
    a.execute("INSERT INTO t2 VALUES (1)").unwrap();
    b.execute("USE d1").unwrap();

    a.execute("BEGIN").unwrap();
    let deleted = a.execute("DELETE FROM t1 WHERE k = 1").unwrap();
    assert_eq!(deleted.affected_rows, 1, "the DELETE itself must apply");
    assert_eq!(count(&mut a, "t1"), 0, "the writer must see its own delete");
    assert_eq!(count(&mut b, "t1"), 1, "the peer must not see it yet");

    a.execute("COMMIT").unwrap();

    assert_eq!(
        count(&mut a, "t1"),
        0,
        "the writer's committed DELETE reverted: the row is back"
    );
    assert_eq!(
        count(&mut b, "t1"),
        0,
        "the peer still sees the deleted row"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Negative control: one table only. Correct today.
///
/// Without it, the failing test could be satisfied by a read path that
/// simply stopped reporting deleted rows whenever a database holds more
/// than one table — the opposite defect, which would look like a pass.
#[test]
fn committed_delete_sticks_when_the_database_holds_one_table() {
    let (mut a, mut b, dir) = two_connections();
    a.execute("CREATE DATABASE d1").unwrap();
    a.execute("USE d1").unwrap();
    a.execute("CREATE TABLE t1 (k INT PRIMARY KEY)").unwrap();
    a.execute("INSERT INTO t1 VALUES (1)").unwrap();
    b.execute("USE d1").unwrap();

    a.execute("BEGIN").unwrap();
    a.execute("DELETE FROM t1 WHERE k = 1").unwrap();
    assert_eq!(count(&mut b, "t1"), 1, "peer saw an uncommitted delete");
    a.execute("COMMIT").unwrap();
    assert_eq!(count(&mut a, "t1"), 0, "single-table db: delete reverted");
    assert_eq!(
        count(&mut b, "t1"),
        0,
        "single-table db: peer still sees it"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Negative control: the second table lives in a **different** database.
///
/// This is what pins the trigger to "same database" rather than "any extra
/// table", which is the difference between a counter shared per database and
/// one shared per storage.
#[test]
fn committed_delete_sticks_when_the_second_table_is_in_another_database() {
    let (mut a, mut b, dir) = two_connections();
    a.execute("CREATE DATABASE d1").unwrap();
    a.execute("CREATE DATABASE d2").unwrap();
    a.execute("USE d2").unwrap();
    a.execute("CREATE TABLE t2 (k INT PRIMARY KEY)").unwrap();
    a.execute("INSERT INTO t2 VALUES (1)").unwrap();
    a.execute("USE d1").unwrap();
    a.execute("CREATE TABLE t1 (k INT PRIMARY KEY)").unwrap();
    a.execute("INSERT INTO t1 VALUES (1)").unwrap();
    b.execute("USE d1").unwrap();

    a.execute("BEGIN").unwrap();
    a.execute("DELETE FROM t1 WHERE k = 1").unwrap();
    assert_eq!(count(&mut b, "t1"), 1, "peer saw an uncommitted delete");
    a.execute("COMMIT").unwrap();
    assert_eq!(count(&mut a, "t1"), 0, "cross-db second table: reverted");
    assert_eq!(
        count(&mut b, "t1"),
        0,
        "cross-db second table: peer sees it"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
