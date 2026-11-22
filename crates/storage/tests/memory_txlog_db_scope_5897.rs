//! #5897: `MemoryStorage`'s transaction log must carry the **database**
//! a change happened in — not just the bare table name.
//!
//! Before the fix, `TxLog` held bare table names only, and every replay
//! site resolved them through `MemoryStorage::tbl()` — always `default`.
//! The database identity was therefore LOST between the write and the
//! replay, with two visible consequences:
//!
//! * **ROLLBACK in a named database undid nothing.** The row the undo had
//!   to remove lived in `d1\x01t`; the undo looked in `default\x01t`.
//! * **The commit broadcast landed in the wrong database.** A commit made
//!   in `d1` was replayed into `default` on the receiving connection —
//!   cross-database pollution — and a ROLLBACK there could not undo what
//!   it had just applied.
//!
//! #5072 is orthogonal and must not regress: the table name in the log
//! stays **bare** (unscoped). The database travels as its own element, so
//! the replay resolves `scoped_key(db, table)` exactly once, from the two
//! halves the log was written with. Folding the database into the table
//! name would change the shape of the keys `scoped_key` already produces —
//! the same "silently mis-keyed" class of bug #5072 was about.

use sqlrustgo_storage::engine::{Record, StorageEngine, TableInfo};
use sqlrustgo_storage::{ColumnDefinition, MemoryStorage};
use sqlrustgo_types::Value;

fn table(name: &str) -> TableInfo {
    TableInfo {
        name: name.to_string(),
        columns: vec![ColumnDefinition::new("id", "INTEGER")],
        ..Default::default()
    }
}

fn row(id: i64) -> Record {
    vec![Value::Integer(id)]
}

/// A storage with `d1` / `d2` / `default`, each holding a table `t` whose
/// single row identifies the database it came from. `default` holds `30`,
/// `d1` holds `10`, `d2` holds `20`.
fn seeded() -> MemoryStorage {
    let mut s = MemoryStorage::new();
    s.create_database("d1").unwrap();
    s.create_database("d2").unwrap();
    for (db, id) in [("default", 30i64), ("d1", 10), ("d2", 20)] {
        s.create_table_in_db(db, &table("t")).unwrap();
        s.insert_in_db(db, "t", vec![row(id)]).unwrap();
    }
    s
}

// ---------------------------------------------------------------------------
// ① the log carries the database, as its own element
// ---------------------------------------------------------------------------

/// The `TxLog` entry names the database the change happened in — as a
/// separate field, never folded into the table name.
#[test]
fn txlog_entries_carry_their_database_5897() {
    let mut s = MemoryStorage::new();
    s.create_database("d1").unwrap();
    s.create_table_in_db("d1", &table("t")).unwrap();

    s.begin_transaction().unwrap();
    s.insert_in_db("d1", "t", vec![row(7)]).unwrap();
    let log = s.commit_transaction_with_log().expect("a committed log");

    assert_eq!(log.inserted.len(), 1, "one INSERT is logged");
    let (db, table_name, record) = &log.inserted[0];
    assert_eq!(db, "d1", "the log must name the database of the change");
    assert_eq!(
        table_name, "t",
        "the table name stays bare — the database must not be folded into it"
    );
    assert_eq!(record, &row(7));
}

/// DELETE and UPDATE entries carry the database too.
#[test]
fn txlog_delete_and_update_entries_carry_their_database_5897() {
    let mut s = seeded();

    s.begin_transaction().unwrap();
    s.delete_in_db("d1", "t", &[Value::Integer(10)]).unwrap();
    s.update_in_db("d2", "t", &[Value::Integer(20)], &[(0, Value::Integer(99))])
        .unwrap();
    let log = s.commit_transaction_with_log().expect("a committed log");

    assert_eq!(log.deleted.len(), 1);
    let (db, table_name, _) = &log.deleted[0];
    assert_eq!(db, "d1", "the DELETE was made in d1");
    assert_eq!(table_name, "t");

    assert_eq!(log.updated.len(), 1);
    let (db, table_name, prior, new) = &log.updated[0];
    assert_eq!(db, "d2", "the UPDATE was made in d2");
    assert_eq!(table_name, "t");
    assert_eq!(prior, &row(20));
    assert_eq!(new, &row(99));
}

// ---------------------------------------------------------------------------
// ② the replay addresses the database named by the log
// ---------------------------------------------------------------------------

/// ROLLBACK of an INSERT made in `d1` removes the row from `d1.t` — and
/// leaves `default.t` alone.
///
/// Before the fix the undo looked in `default\x01t` (the bare name scoped
/// to the replay-time database), so the row in `d1\x01t` survived: the
/// ROLLBACK silently did nothing.
#[test]
fn rollback_undoes_an_insert_in_a_named_database_5897() {
    let mut s = seeded();

    s.begin_transaction().unwrap();
    s.insert_in_db("d1", "t", vec![row(11)]).unwrap();
    assert_eq!(
        s.scan_in_db("d1", "t").unwrap().len(),
        2,
        "fixture: the in-transaction row is visible in d1"
    );

    s.rollback_transaction().unwrap();

    assert_eq!(
        s.scan_in_db("d1", "t").unwrap(),
        vec![row(10)],
        "ROLLBACK must remove the row from the database the INSERT was made in"
    );
    assert_eq!(
        s.scan("t").unwrap(),
        vec![row(30)],
        "and must not touch the same table name in `default`"
    );
}

/// ROLLBACK of a DELETE made in `d1` restores the row into `d1.t`.
#[test]
fn rollback_restores_a_deleted_row_in_a_named_database_5897() {
    let mut s = seeded();

    s.begin_transaction().unwrap();
    s.delete_in_db("d1", "t", &[Value::Integer(10)]).unwrap();
    assert!(
        s.scan_in_db("d1", "t").unwrap().is_empty(),
        "fixture: the row is gone inside the transaction"
    );

    s.rollback_transaction().unwrap();

    assert_eq!(
        s.scan_in_db("d1", "t").unwrap(),
        vec![row(10)],
        "ROLLBACK must restore the row in d1, not in default"
    );
    assert_eq!(
        s.scan("t").unwrap(),
        vec![row(30)],
        "and must not touch the same table name in `default`"
    );
}

/// ROLLBACK of an UPDATE made in `d1` restores the prior row in `d1.t`.
#[test]
fn rollback_restores_an_updated_row_in_a_named_database_5897() {
    let mut s = seeded();

    s.begin_transaction().unwrap();
    s.update_in_db("d1", "t", &[Value::Integer(10)], &[(0, Value::Integer(99))])
        .unwrap();
    assert_eq!(
        s.scan_in_db("d1", "t").unwrap(),
        vec![row(99)],
        "fixture: the update is visible inside the transaction"
    );

    s.rollback_transaction().unwrap();

    assert_eq!(
        s.scan_in_db("d1", "t").unwrap(),
        vec![row(10)],
        "ROLLBACK must restore the prior row in d1, not in default"
    );
    assert_eq!(
        s.scan("t").unwrap(),
        vec![row(30)],
        "and must not touch the same table name in `default`"
    );
}

// ---------------------------------------------------------------------------
// ③ the broadcast lands in the database the log names
// ---------------------------------------------------------------------------

/// A commit made in `d1`, broadcast to another connection's storage, must
/// land in `d1.t` — and in neither `d2.t` nor `default.t`.
///
/// Before the fix the broadcast scoped the bare table name to the
/// replay-time database (`default`), so the row showed up in `default.t`:
/// cross-database pollution.
#[test]
fn broadcast_of_a_named_database_commit_lands_in_that_database_5897() {
    let mut source = MemoryStorage::new();
    source.create_database("d1").unwrap();
    source.create_table_in_db("d1", &table("t")).unwrap();
    source.begin_transaction().unwrap();
    source.insert_in_db("d1", "t", vec![row(7)]).unwrap();
    let log = source.commit_transaction_with_log().expect("a committed log");

    let mut receiver = seeded();
    receiver.apply_committed_log(&log).unwrap();

    assert_eq!(
        receiver.scan_in_db("d1", "t").unwrap(),
        vec![row(10), row(7)],
        "the broadcast must land in the database the commit was made in"
    );
    assert_eq!(
        receiver.scan_in_db("d2", "t").unwrap(),
        vec![row(20)],
        "the broadcast must not touch d2"
    );
    assert_eq!(
        receiver.scan("t").unwrap(),
        vec![row(30)],
        "the broadcast must not touch `default`"
    );
}

/// The same for a DELETE broadcast: the row disappears from `d1.t` and
/// from nowhere else.
#[test]
fn broadcast_delete_stays_in_the_logged_database_5897() {
    let mut source = seeded();
    source.begin_transaction().unwrap();
    source.delete_in_db("d1", "t", &[Value::Integer(10)]).unwrap();
    let log = source.commit_transaction_with_log().expect("a committed log");

    let mut receiver = seeded();
    receiver.apply_committed_log(&log).unwrap();

    assert!(
        receiver.scan_in_db("d1", "t").unwrap().is_empty(),
        "the deleted row must be gone from d1"
    );
    assert_eq!(
        receiver.scan_in_db("d2", "t").unwrap(),
        vec![row(20)],
        "the broadcast must not touch d2"
    );
    assert_eq!(
        receiver.scan("t").unwrap(),
        vec![row(30)],
        "the broadcast must not touch `default`"
    );
}

/// The same for an UPDATE broadcast.
#[test]
fn broadcast_update_stays_in_the_logged_database_5897() {
    let mut source = seeded();
    source.begin_transaction().unwrap();
    source.update_in_db("d1", "t", &[Value::Integer(10)], &[(0, Value::Integer(99))])
        .unwrap();
    let log = source.commit_transaction_with_log().expect("a committed log");

    let mut receiver = seeded();
    receiver.apply_committed_log(&log).unwrap();

    assert_eq!(
        receiver.scan_in_db("d1", "t").unwrap(),
        vec![row(99)],
        "the update must land in d1"
    );
    assert_eq!(
        receiver.scan_in_db("d2", "t").unwrap(),
        vec![row(20)],
        "the broadcast must not touch d2"
    );
    assert_eq!(
        receiver.scan("t").unwrap(),
        vec![row(30)],
        "the broadcast must not touch `default`"
    );
}

/// A receiver that is itself in a transaction can still ROLLBACK the
/// broadcast it just received — and the undo must address the database the
/// broadcast came from.
#[test]
fn rollback_on_the_receiver_reverts_a_broadcast_from_another_database_5897() {
    let mut source = MemoryStorage::new();
    source.create_database("d1").unwrap();
    source.create_table_in_db("d1", &table("t")).unwrap();
    source.begin_transaction().unwrap();
    source.insert_in_db("d1", "t", vec![row(7)]).unwrap();
    let log = source.commit_transaction_with_log().expect("a committed log");

    let mut receiver = seeded();
    receiver.begin_transaction().unwrap();
    receiver.apply_committed_log(&log).unwrap();
    assert_eq!(
        receiver.scan_in_db("d1", "t").unwrap().len(),
        2,
        "fixture: the broadcast is visible on the receiver"
    );

    receiver.rollback_transaction().unwrap();

    assert_eq!(
        receiver.scan_in_db("d1", "t").unwrap(),
        vec![row(10)],
        "the receiver's ROLLBACK must undo the broadcast in d1"
    );
    assert_eq!(
        receiver.scan("t").unwrap(),
        vec![row(30)],
        "and must not touch `default`"
    );
}

// ---------------------------------------------------------------------------
// ④ the `default` database behaves exactly as before
// ---------------------------------------------------------------------------

/// The `default` database keeps working: the log names it `default`, and
/// the replay resolves the very same key it always did.
///
/// This is the shape of #5072's own regression test
/// (`test_rollback_removes_inserted_rows`): a bare-name INSERT, undone by
/// ROLLBACK.
#[test]
fn default_database_behaviour_is_unchanged_5897() {
    let mut s = MemoryStorage::new();
    s.create_table(&table("t")).unwrap();
    s.insert("t", vec![row(1)]).unwrap();

    s.begin_transaction().unwrap();
    s.insert("t", vec![row(99)]).unwrap();
    assert_eq!(s.scan("t").unwrap().len(), 2, "visible in-transaction");

    let log = s.commit_transaction_with_log().expect("a committed log");
    assert_eq!(log.inserted.len(), 1);
    let (db, table_name, _) = &log.inserted[0];
    assert_eq!(db, "default", "the bare-name call resolves against `default`");
    assert_eq!(table_name, "t");

    // The same shape as a rollback: begin again, insert, roll back.
    //
    // Note what is expected here: the row inserted in the first
    // transaction was **committed** (the log was taken by
    // `commit_transaction_with_log`), so it stays. Only the row of the
    // second transaction is undone by the ROLLBACK.
    s.begin_transaction().unwrap();
    s.insert("t", vec![row(98)]).unwrap();
    assert_eq!(
        s.scan("t").unwrap().len(),
        3,
        "visible in-transaction (1 committed + 99 committed + 98 in flight)"
    );
    s.rollback_transaction().unwrap();
    assert_eq!(
        s.scan("t").unwrap(),
        vec![row(1), row(99)],
        "ROLLBACK must remove the in-transaction INSERT in `default` too, \
         and leave the committed rows alone"
    );
}

/// The `default` database's DELETE and UPDATE undo keeps working.
#[test]
fn default_database_delete_and_update_undo_is_unchanged_5897() {
    let mut s = MemoryStorage::new();
    s.create_table(&table("t")).unwrap();
    s.insert("t", vec![row(1)]).unwrap();

    s.begin_transaction().unwrap();
    s.delete("t", &[Value::Integer(1)]).unwrap();
    s.rollback_transaction().unwrap();
    assert_eq!(
        s.scan("t").unwrap(),
        vec![row(1)],
        "ROLLBACK must restore the deleted row in `default`"
    );

    s.begin_transaction().unwrap();
    s.update("t", &[Value::Integer(1)], &[(0, Value::Integer(9))])
        .unwrap();
    s.rollback_transaction().unwrap();
    assert_eq!(
        s.scan("t").unwrap(),
        vec![row(1)],
        "ROLLBACK must restore the updated row in `default`"
    );
}
