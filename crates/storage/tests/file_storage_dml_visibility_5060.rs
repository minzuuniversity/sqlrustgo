//! #5060 — `insert_buffer` is a second copy of table state, and only
//! `scan` knew about it.
//!
//! An autocommit `INSERT` parks the row in `insert_buffer`
//! (`insert()` routes there unless `enable_buffer` is off). `update`,
//! `update_if`, `delete` and `delete_if` read `tables` only, so:
//!
//! ```text
//! PROBE P1 update affected = 0     (expected 1)
//! PROBE P1 scan         = [[Integer(1), Text("a")]]   <- old value
//! PROBE P5 update_if affected = 0  (expected 1)
//! ```
//!
//! `delete` was worse than a no-op. It stripped the matching row from
//! the buffer but counted only the `tables` rows, so `removed` came back
//! 0, `dirty_tables` was never set, and the deletion was never
//! persisted — the row vanished from memory and came back on the next
//! open. That is a silent durability hole, not just a wrong count.
//!
//! #5059 was the same root cause seen from the other side: ROLLBACK
//! never removed buffered inserts, so 100 committed + 100 rolled-back
//! rows all survived. Its sweep iterated `s.tables.keys()` — already
//! scoped `db\x01table` keys — and scoped them a second time, addressing
//! `default\x01default\x01tx_t`, which matches no buffer.

use sqlrustgo_storage::engine::{
    ColumnDefinition, Record, RowFilter, RowMutation, StorageEngine, TableInfo,
};
use sqlrustgo_storage::file_storage::FileStorage;
use sqlrustgo_types::Value;

fn users_table() -> TableInfo {
    TableInfo {
        name: "users".into(),
        columns: vec![
            ColumnDefinition::new("id", "INTEGER"),
            ColumnDefinition::new("name", "VARCHAR(64)"),
        ],
        foreign_keys: vec![],
        unique_constraints: vec![],
        check_constraints: vec![],
        compression: None,
        collations: std::collections::HashMap::new(),
        partition_info: None,
        original_sql: String::new(),
    }
}

fn dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("fs_dml_5060_{tag}"));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Reopen from disk. A buffered row is not on disk, so "the in-memory
/// scan looks right" is not the same claim as "the data is there".
fn on_disk(dir: &std::path::Path) -> Vec<Vec<Value>> {
    FileStorage::new(dir.to_path_buf())
        .unwrap()
        .scan("users")
        .unwrap()
}

fn alice() -> Vec<Value> {
    vec![Value::Integer(1), Value::Text("Alice".into())]
}

fn updated() -> Vec<Value> {
    vec![Value::Integer(1), Value::Text("Alicia".into())]
}

// --- #5060: the DML paths must see buffered rows -------------------------

#[test]
fn update_after_autocommit_insert_affects_the_row() {
    let d = dir("update");
    let mut s = FileStorage::new(d.clone()).unwrap();
    s.create_table(&users_table()).unwrap();
    s.insert("users", vec![alice()]).unwrap();
    let n = s
        .update(
            "users",
            &[Value::Integer(1)],
            &[(1, Value::Text("Alicia".into()))],
        )
        .unwrap();
    assert_eq!(n, 1, "an UPDATE right after an INSERT must affect its row");
    s.flush_all_buffers().unwrap();
    assert_eq!(on_disk(&d), vec![updated()]);
}

#[test]
fn update_if_after_autocommit_insert_affects_the_row() {
    let d = dir("update_if");
    let mut s = FileStorage::new(d.clone()).unwrap();
    s.create_table(&users_table()).unwrap();
    s.insert("users", vec![alice()]).unwrap();
    let filter: RowFilter = Box::new(|r: &Record| r.get(0) == Some(&Value::Integer(1)));
    let mutation = RowMutation::new(vec![(1, Value::Text("Alicia".into()))], 0);
    let n = s.update_if("users", &filter, &mutation).unwrap();
    assert_eq!(n, 1);
    s.flush_all_buffers().unwrap();
    assert_eq!(on_disk(&d), vec![updated()]);
}

#[test]
fn delete_after_autocommit_insert_removes_the_row_from_disk() {
    let d = dir("delete");
    let mut s = FileStorage::new(d.clone()).unwrap();
    s.create_table(&users_table()).unwrap();
    s.insert("users", vec![alice()]).unwrap();
    let n = s.delete("users", &[Value::Integer(1)]).unwrap();
    assert_eq!(n, 1, "a DELETE right after an INSERT must affect its row");
    s.flush_all_buffers().unwrap();
    assert!(
        on_disk(&d).is_empty(),
        "the deletion must be persisted, not just dropped from memory — \
         this is the half of the bug that lost data on reopen"
    );
}

#[test]
fn delete_if_after_autocommit_insert_removes_the_row_from_disk() {
    let d = dir("delete_if");
    let mut s = FileStorage::new(d.clone()).unwrap();
    s.create_table(&users_table()).unwrap();
    s.insert("users", vec![alice()]).unwrap();
    let filter: RowFilter = Box::new(|r: &Record| r.get(0) == Some(&Value::Integer(1)));
    let n = s.delete_if("users", &filter).unwrap();
    assert_eq!(n, 1);
    s.flush_all_buffers().unwrap();
    assert!(on_disk(&d).is_empty());
}

/// The same, inside a transaction. A buffered row is an uncommitted
/// insert; it must still be visible to *this* transaction's own UPDATE,
/// or the transaction commits the old value.
#[test]
fn update_inside_a_transaction_reaches_the_uncommitted_row() {
    let d = dir("in_tx_update");
    let mut s = FileStorage::new(d.clone()).unwrap();
    s.create_table(&users_table()).unwrap();
    s.begin_transaction().unwrap();
    s.insert("users", vec![alice()]).unwrap();
    let n = s
        .update(
            "users",
            &[Value::Integer(1)],
            &[(1, Value::Text("Alicia".into()))],
        )
        .unwrap();
    assert_eq!(n, 1);
    assert_eq!(s.scan("users").unwrap(), vec![updated()]);
    s.commit_transaction().unwrap();
    s.flush_all_buffers().unwrap();
    assert_eq!(on_disk(&d), vec![updated()]);
}

/// A ROLLBACK has to undo the UPDATE as well as the INSERT.
#[test]
fn rollback_undoes_an_update_applied_to_an_uncommitted_row() {
    let d = dir("in_tx_rollback_update");
    let mut s = FileStorage::new(d.clone()).unwrap();
    s.create_table(&users_table()).unwrap();
    s.begin_transaction().unwrap();
    s.insert("users", vec![alice()]).unwrap();
    s.update(
        "users",
        &[Value::Integer(1)],
        &[(1, Value::Text("Alicia".into()))],
    )
    .unwrap();
    s.rollback_transaction().unwrap();
    s.flush_all_buffers().unwrap();
    assert!(
        s.scan("users").unwrap().is_empty(),
        "the whole transaction is rolled back, not just its last statement"
    );
}

// --- #5059: rollback must undo buffered inserts -------------------------

#[test]
fn rollback_removes_the_buffered_insert() {
    let d = dir("rollback_insert");
    let mut s = FileStorage::new(d.clone()).unwrap();
    s.create_table(&users_table()).unwrap();
    s.begin_transaction().unwrap();
    s.insert("users", vec![alice()]).unwrap();
    s.rollback_transaction().unwrap();
    s.flush_all_buffers().unwrap();
    assert!(
        s.scan("users").unwrap().is_empty(),
        "a rolled-back INSERT must not survive the flush"
    );
    drop(s);
    assert!(on_disk(&d).is_empty());
}

/// A committed row deleted inside a transaction comes back on ROLLBACK.
/// Exercises the `DeleteRow` undo, which addresses the row by index in
/// `tables`.
#[test]
fn rollback_restores_a_committed_row_deleted_in_the_transaction() {
    let d = dir("rollback_delete");
    let mut s = FileStorage::new(d.clone()).unwrap();
    s.create_table(&users_table()).unwrap();
    s.insert("users", vec![alice()]).unwrap();
    s.flush_all_buffers().unwrap();

    s.begin_transaction().unwrap();
    assert_eq!(s.delete("users", &[Value::Integer(1)]).unwrap(), 1);
    assert!(s.scan("users").unwrap().is_empty());
    s.rollback_transaction().unwrap();
    s.flush_all_buffers().unwrap();
    assert_eq!(
        s.scan("users").unwrap(),
        vec![alice()],
        "the delete is rolled back, so the row is back"
    );
}

/// Insert and delete inside one transaction: the two value-based undo
/// entries cancel, and ROLLBACK restores the pre-transaction state —
/// which is empty. Getting this wrong in either direction is a
/// correctness bug: leaving the row behind resurrects data the
/// transaction cancelled, and dropping it would lose a row that was
/// never there to begin with.
#[test]
fn rollback_of_insert_then_delete_restores_the_empty_table() {
    let d = dir("rollback_insert_delete");
    let mut s = FileStorage::new(d.clone()).unwrap();
    s.create_table(&users_table()).unwrap();
    s.begin_transaction().unwrap();
    s.insert("users", vec![alice()]).unwrap();
    assert_eq!(s.delete("users", &[Value::Integer(1)]).unwrap(), 1);
    s.rollback_transaction().unwrap();
    s.flush_all_buffers().unwrap();
    assert!(
        s.scan("users").unwrap().is_empty(),
        "insert-then-delete in one transaction nets to no change"
    );
    drop(s);
    assert!(on_disk(&d).is_empty());
}

/// A **committed but not yet flushed** row lives in `insert_buffer`, not
/// in `tables`. Deleting it inside a later transaction therefore has no
/// index to address it by, and `DeleteRow` cannot restore it — the
/// value-based `BufferedDelete` undo is the only thing that can.
///
/// This is the one path in the delete/rollback code where `BufferedDelete`
/// is load-bearing, and it is easy to miss: every other delete test in
/// this file flushes first, which silently moves the row into `tables`
/// and routes the rollback through `DeleteRow` instead. Mutation M6
/// (turn the `BufferedDelete` undo arm into a no-op) passes without this
/// test and fails with it.
#[test]
fn rollback_restores_a_committed_but_unflushed_row_deleted_in_the_transaction() {
    let d = dir("rollback_buffered_delete");
    let mut s = FileStorage::new(d.clone()).unwrap();
    s.create_table(&users_table()).unwrap();

    // Autocommit, and deliberately NO flush — so the row is still in the
    // deferred-insert buffer when the transaction below deletes it.
    s.insert("users", vec![alice()]).unwrap();

    s.begin_transaction().unwrap();
    assert_eq!(s.delete("users", &[Value::Integer(1)]).unwrap(), 1);
    assert!(s.scan("users").unwrap().is_empty());
    s.rollback_transaction().unwrap();
    s.flush_all_buffers().unwrap();

    assert_eq!(
        s.scan("users").unwrap(),
        vec![alice()],
        "the row was committed before the transaction, so ROLLBACK must bring it back"
    );
    drop(s);
    // ...and it must have reached disk, not just the cache.
    let mut reopened = FileStorage::new(d.clone()).unwrap();
    assert_eq!(reopened.scan("users").unwrap(), vec![alice()]);
}

/// The buffer is instance-level, not connection-level, so ROLLBACK must
/// remove **its own** rows and nothing else. This is the reason the
/// #5059 fix is value-based instead of the blanket sweep the old code
/// attempted.
#[test]
fn rollback_keeps_rows_it_did_not_insert() {
    let d = dir("rollback_foreign");
    let mut s = FileStorage::new(d.clone()).unwrap();
    s.create_table(&users_table()).unwrap();
    // Committed first, so this row is not in the buffer at all.
    s.insert("users", vec![alice()]).unwrap();
    s.flush_all_buffers().unwrap();

    s.begin_transaction().unwrap();
    s.insert(
        "users",
        vec![vec![Value::Integer(2), Value::Text("Bob".into())]],
    )
    .unwrap();
    s.rollback_transaction().unwrap();
    s.flush_all_buffers().unwrap();

    let rows = s.scan("users").unwrap();
    assert_eq!(
        rows,
        vec![alice()],
        "rollback removed a row it never inserted — the buffer is shared \
         across connections, so a blanket sweep loses other writers' data"
    );
}

#[test]
fn commit_keeps_the_buffered_insert() {
    let d = dir("commit_insert");
    let mut s = FileStorage::new(d.clone()).unwrap();
    s.create_table(&users_table()).unwrap();
    s.begin_transaction().unwrap();
    s.insert("users", vec![alice()]).unwrap();
    s.commit_transaction().unwrap();
    s.flush_all_buffers().unwrap();
    assert_eq!(on_disk(&d), vec![alice()]);
}

/// #5059's own shape, at the unit level: churn a transaction and count
/// what survives. The committed/rolled-back split must be exact.
#[test]
fn committed_and_rolled_back_inserts_are_counted_exactly() {
    let d = dir("churn");
    let mut s = FileStorage::new(d.clone()).unwrap();
    s.create_table(&users_table()).unwrap();
    let mut committed = 0;
    for cycle in 0..20i64 {
        s.begin_transaction().unwrap();
        s.insert(
            "users",
            vec![vec![Value::Integer(cycle), Value::Text("x".into())]],
        )
        .unwrap();
        if cycle % 2 == 0 {
            s.commit_transaction().unwrap();
            committed += 1;
        } else {
            s.rollback_transaction().unwrap();
        }
    }
    s.flush_all_buffers().unwrap();
    assert_eq!(committed, 10);
    assert_eq!(
        s.scan("users").unwrap().len(),
        committed,
        "exactly the committed transactions' rows may survive"
    );
}
