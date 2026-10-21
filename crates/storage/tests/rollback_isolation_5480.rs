//! #5480 — a ROLLBACK must be scoped to **its own** transaction.
//!
//! `crates/mysql-server/tests/verify_4957_rollback_buffer_test.rs` pins this
//! contract at the wire level:
//!
//! > `one_connections_rollback_must_not_discard_anothers_buffered_rows`
//! > — "A's ROLLBACK must not discard B's pending rows (100..110)"
//!
//! That contract is what `WalStorage`'s rollback path has to preserve while
//! it is being repaired (#5480). Two facts about `FileStorage` make a
//! *blanket* buffer wipe wrong:
//!
//! 1. `insert_buffer` is **instance-level**, not connection-level (#5060) —
//!    it holds the rows of every open transaction.
//! 2. `FileStorage::insert` **always buffers** ("Now: always buffer
//!    inserts, caller explicitly calls flush() to persist"), so the buffer
//!    also holds rows of transactions that already **committed** but have
//!    not been flushed yet.
//!
//! ⇒ A blanket `insert_buffer.clear()` during one connection's ROLLBACK
//! destroys other connections' pending rows **and** committed-but-unflushed
//! rows. The per-transaction undo (`FileStorage::rollback_transaction_for`,
//! which removes exactly this transaction's own `UndoOp::BufferedInsert`
//! entries by value — #5055 / #5060) is the mechanism that is correct.
//!
//! These tests drive the **stack** the server actually uses —
//! `WalStorage<FileStorage, _>` — because that is where the blanket wipe
//! lived. Driving `FileStorage` directly would pass even on the broken
//! code, and a test that cannot fail on the broken code proves nothing.

use sqlrustgo_storage::engine::{ColumnDefinition, StorageEngine, TableInfo, Value};
use sqlrustgo_storage::file_storage::FileStorage;
use sqlrustgo_storage::wal::FileBackedWalManager;
use sqlrustgo_storage::WalStorage;
use std::path::PathBuf;

type Store = WalStorage<FileStorage, FileBackedWalManager>;

fn table_info(name: &str) -> TableInfo {
    let mut id = ColumnDefinition::new("id", "INTEGER");
    id.primary_key = true;
    TableInfo {
        name: name.to_string(),
        columns: vec![id, ColumnDefinition::new("v", "INTEGER")],
        ..Default::default()
    }
}

fn fresh_store(tag: &str) -> (Store, PathBuf) {
    let dir = std::env::temp_dir().join(format!(
        "rollback_isolation_5480_{tag}_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    let file = FileStorage::new(dir.clone()).expect("FileStorage::new");
    let wal = FileBackedWalManager::new(dir.join("sqlrustgo.wal")).expect("wal manager");
    let mut store = WalStorage::new(file, wal).expect("WalStorage::new");
    store.create_table(&table_info("t")).expect("create table");
    (store, dir)
}

fn ids(store: &Store) -> Vec<i64> {
    let mut out: Vec<i64> = store
        .scan("t")
        .expect("scan")
        .iter()
        .map(|row| match row.first() {
            Some(Value::Integer(v)) => *v,
            other => panic!("expected an Integer id, got {other:?}"),
        })
        .collect();
    out.sort_unstable();
    out
}

/// A **bare** `FileStorage` (no `WalStorage` wrapper).
///
/// #5506 lives in `FileStorage`'s own lock-free rollback, and the
/// `WalStorage` wrapper masks it: through the wrapper the aborted row is
/// removed by a different mechanism, so a test that goes through the
/// wrapper cannot tell the two implementations apart.
fn fresh_file_storage(tag: &str) -> (FileStorage, PathBuf) {
    let dir = std::env::temp_dir().join(format!(
        "rollback_isolation_5480_fs_{tag}_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    let mut store = FileStorage::new(dir.clone()).expect("FileStorage::new");
    store.create_table(&table_info("t")).expect("create table");
    (store, dir)
}

/// The ids of the rows `store` currently reports.
fn ids_fs(store: &FileStorage) -> Vec<i64> {
    let mut out: Vec<i64> = store
        .scan("t")
        .expect("scan")
        .iter()
        .map(|row| match row.first() {
            Some(Value::Integer(v)) => *v,
            other => panic!("expected an Integer id, got {other:?}"),
        })
        .collect();
    out.sort_unstable();
    out
}

/// Two transactions on one store; the first one rolls back.
///
/// The second transaction's row must survive — it belongs to a transaction
/// that is still open.
#[test]
fn a_rollback_removes_only_its_own_transaction_rows() {
    let (mut store, _dir) = fresh_store("own-rows");

    store.set_current_tx_id(1);
    store
        .insert("t", vec![vec![Value::Integer(1), Value::Integer(1)]])
        .expect("A insert");

    store.set_current_tx_id(2);
    store
        .insert("t", vec![vec![Value::Integer(2), Value::Integer(2)]])
        .expect("B insert");

    // A rolls back.
    store.set_current_tx_id(1);
    store.rollback_transaction().expect("A rollback");

    assert_eq!(
        ids(&store),
        vec![2],
        "A's ROLLBACK must remove A's own row and leave B's pending row alone"
    );
}

/// A row that has already **committed** (but not been flushed — the buffer
/// is what makes it visible until `flush()` runs) must also survive another
/// transaction's ROLLBACK.
///
/// This is the case a blanket buffer wipe destroys: `FileStorage::insert`
/// always buffers, and `commit_transaction_for` deliberately does NOT
/// promote the buffered rows ("INSERTs buffered via insert_buffered are NOT
/// auto-flushed here; caller decides when to commit visibility"), so a
/// committed row lives in `insert_buffer` until the next flush.
#[test]
fn a_rollback_does_not_discard_another_transaction_committed_rows() {
    let (mut store, _dir) = fresh_store("committed-rows");

    // B commits first: its row is durable from the store's point of view.
    store.set_current_tx_id(2);
    store
        .insert("t", vec![vec![Value::Integer(5), Value::Integer(5)]])
        .expect("B insert");
    store.commit_transaction_for(2).expect("B commit");
    assert_eq!(ids(&store), vec![5], "B's committed row must be visible");

    // A opens a transaction and rolls it back.
    store.set_current_tx_id(1);
    store
        .insert("t", vec![vec![Value::Integer(1), Value::Integer(1)]])
        .expect("A insert");
    store.rollback_transaction().expect("A rollback");

    assert_eq!(
        ids(&store),
        vec![5],
        "A's ROLLBACK must not discard B's committed row"
    );
}

/// The **lock-free** rollback path, driven through the `WalStorage`
/// wrapper, must be scoped to its own transaction as well.
///
/// This is a different code path from the `&mut self` rollback above:
///
/// ```text
/// WalStorage::rollback_transaction_lockfree_for(tx_id)
///   ① WAL `Rollback` entry + sync
///   ② inner().rollback_transaction_lockfree_for(tx_id)   → per-transaction undo
///   ③ inner().discard_all_buffers_shared()               → shared discard
/// ```
///
/// Step ③ is the dangerous one. `FileStorage`'s write buffer is
/// **instance-level** (#5060), so a blanket `insert_buffer.clear()` there
/// would take B's pending rows down with A's. It is also *redundant*:
/// step ② already undid exactly A's rows.
///
/// The existing `&mut self` test above cannot catch this — it never goes
/// through step ③ — so the contract needs its own pin on this path.
#[test]
fn the_walstorage_lock_free_rollback_keeps_other_transactions_rows() {
    let (mut store, _dir) = fresh_store("lockfree-isolation");

    store.set_current_tx_id(1);
    store
        .insert("t", vec![vec![Value::Integer(1), Value::Integer(1)]])
        .expect("A insert");

    store.set_current_tx_id(2);
    store
        .insert("t", vec![vec![Value::Integer(2), Value::Integer(2)]])
        .expect("B insert");

    assert_eq!(ids(&store), vec![1, 2], "both rows must be visible first");

    // A rolls back through the lock-free path.
    store.set_current_tx_id(1);
    store
        .rollback_transaction_lockfree()
        .expect("A lock-free rollback");

    assert_eq!(
        ids(&store),
        vec![2],
        "A's lock-free ROLLBACK must remove A's own row and leave B's \
         pending row alone"
    );
}

/// The rolled-back row must not reach the disk either.
///
/// #3964's original bug was that ROLLBACK *persisted* the abandoned
/// transaction's rows (`flush()` on the rollback path). The repair must
/// discard them from the buffer **without** flushing them — and must not
/// flush the other rows either.
#[test]
fn a_rolled_back_row_is_not_persisted_to_disk() {
    let (mut store, dir) = fresh_store("not-persisted");

    store.set_current_tx_id(1);
    store
        .insert("t", vec![vec![Value::Integer(1), Value::Integer(1)]])
        .expect("A insert");
    store.rollback_transaction().expect("A rollback");

    // Re-open the data directory: whatever is on disk is what survived.
    let reopened = FileStorage::new(dir).expect("reopen");
    assert_eq!(
        reopened.scan("t").expect("scan").len(),
        0,
        "an aborted transaction's row must never reach the disk"
    );
}

/// The **lock-free** rollback path must undo the same way (#5506).
///
/// `FileStorage::rollback_transaction_lockfree_for` used to drop the
/// transaction's entries from the undo log and then address the buffer
/// key `""` — a key no table ever has. The aborted transaction's rows
/// therefore stayed in `insert_buffer`, and the next `flush()` persisted
/// rows the ROLLBACK was meant to cancel.
///
/// Measured on a bare `FileStorage` (no `WalStorage` wrapper, so nothing
/// else can be responsible for the outcome):
///
/// | | after the lock-free ROLLBACK | after `flush()` | on disk |
/// |---|---|---|---|
/// | before the fix | the row is **still visible** | still visible | **`t.json` contains it** |
/// | after the fix | gone | gone | `t.json` has `"rows": []` |
///
/// The test drives `FileStorage` directly for the same reason: the
/// `WalStorage` wrapper's own rollback path masks the defect, so a test
/// that goes through the wrapper cannot see it.
#[test]
fn the_lock_free_rollback_undoes_the_aborted_transaction_rows() {
    let (mut store, dir) = fresh_file_storage("lockfree");

    store.set_current_tx_id(1);
    store
        .insert("t", vec![vec![Value::Integer(1), Value::Integer(1)]])
        .expect("A insert");
    assert_eq!(
        ids_fs(&store),
        vec![1],
        "the row must be visible before the rollback"
    );

    store
        .rollback_transaction_lockfree()
        .expect("lock-free rollback");

    assert_eq!(
        ids_fs(&store),
        Vec::<i64>::new(),
        "the lock-free ROLLBACK must undo the aborted transaction's rows"
    );

    // And it must not have persisted them either. The flush is what makes
    // the difference observable: a row left in the write buffer is
    // invisible to `scan` once its transaction is closed, but a `flush()`
    // still persists it — that is how an aborted transaction's row reaches
    // the disk.
    store.flush().expect("flush");
    let reopened = FileStorage::new(dir).expect("reopen");
    assert_eq!(
        reopened.scan("t").expect("scan").len(),
        0,
        "an aborted transaction's row must never reach the disk"
    );
}
