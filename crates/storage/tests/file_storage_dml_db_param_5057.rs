//! #5057 P1-c-b: the DML methods must delete and update the table in the
//! database the caller named.
//!
//! # The defect class
//!
//! `delete` / `delete_collect_pks` / `delete_if` / `update` / `update_if`
//! each resolved their table through `self.current_db`, so on the shared
//! storage a statement deleted or rewrote whichever database the last
//! connection had selected. For `delete` that meant rows vanishing from a
//! database the statement never named.
//!
//! # What these pin
//!
//! For every method: naming a database acts on that database only, and the
//! same-named table in another database is untouched. Each is paired with a
//! control that omits the database and must still follow `current_db` —
//! those controls are what would break first if the two families were ever
//! collapsed into one.

use sqlrustgo_storage::engine::RowFilter;
use sqlrustgo_storage::{
    ColumnDefinition, FileStorage, RowMutation, StorageEngine, TableInfo, Value,
};

fn tbl(name: &str) -> TableInfo {
    TableInfo {
        name: name.to_string(),
        columns: vec![ColumnDefinition {
            name: "id".to_string(),
            data_type: "INTEGER".to_string(),
            nullable: false,
            primary_key: true,
            char_max_length: None,
            collation: None,
            default_value: None,
            auto_increment: false,
        }],
        foreign_keys: vec![],
        unique_constraints: vec![],
        check_constraints: vec![],
        partition_info: None,
        collations: Default::default(),
        compression: None,
        original_sql: String::new(),
    }
}

/// Two databases holding a same-named table with one row each.
fn two_dbs() -> (tempfile::TempDir, FileStorage) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut s = FileStorage::new(tmp.path().to_path_buf()).expect("open");
    s.create_database("d1").expect("d1");
    s.create_database("d2").expect("d2");
    s.set_current_db("d1").expect("switch to d1");
    s.create_table(&tbl("shared")).expect("create in d1");
    s.set_current_db("d2").expect("switch to d2");
    s.create_table(&tbl("shared")).expect("create in d2");
    s.set_current_db("d1").expect("back to d1");
    s.insert("shared", vec![vec![Value::Integer(1)]])
        .expect("d1 row");
    s.insert_in_db("d2", "shared", vec![vec![Value::Integer(2)]])
        .expect("d2 row");
    (tmp, s)
}

fn count(s: &FileStorage, db: &str) -> usize {
    s.scan_in_db(db, "shared").expect("scan").len()
}

/// `delete_in_db` must remove rows only from the named database.
#[test]
fn issue_5057_delete_in_db_only_touches_the_named_database() {
    let (_t, mut s) = two_dbs();
    let removed = s
        .delete_in_db("d2", "shared", &[Value::Integer(2)])
        .expect("delete from d2");

    assert_eq!(removed, 1, "one row removed from d2");
    assert_eq!(count(&s, "d2"), 0, "d2's row must be gone");
    assert_eq!(
        count(&s, "d1"),
        1,
        "d1's identically-valued row must survive — the delete named d2"
    );
}

/// Control: the database-less `delete` follows `current_db`.
#[test]
fn issue_5057_delete_without_a_database_follows_current_db() {
    let (_t, mut s) = two_dbs();
    let removed = s
        .delete("shared", &[Value::Integer(1)])
        .expect("delete from current db");

    assert_eq!(removed, 1);
    assert_eq!(count(&s, "d1"), 0, "current_db was d1, so d1 lost its row");
    assert_eq!(count(&s, "d2"), 1, "d2 must be untouched");
}

/// The MVCC wrapper's variant returns the deleted primary keys; it resolves
/// the table through a different code path and needs its own check.
#[test]
fn issue_5057_delete_collect_pks_in_db_only_touches_the_named_database() {
    let (_t, mut s) = two_dbs();
    let pks = s
        .delete_collect_pks_in_db("d2", "shared", &[Value::Integer(2)])
        .expect("collect pks from d2");

    assert_eq!(pks.len(), 1, "expected exactly one pk from d2");
    assert_eq!(count(&s, "d2"), 0, "d2's row must be gone");
    assert_eq!(count(&s, "d1"), 1, "d1's row must survive");
}

/// `delete_if` filters inside the storage lock, a separate body from
/// `delete`'s positional-filter path.
#[test]
fn issue_5057_delete_if_in_db_only_touches_the_named_database() {
    let (_t, mut s) = two_dbs();
    let filter: RowFilter = Box::new(|row| row.first() == Some(&Value::Integer(2)));
    let removed = s
        .delete_if_in_db("d2", "shared", &filter)
        .expect("delete_if from d2");

    assert_eq!(removed, 1);
    assert_eq!(count(&s, "d2"), 0);
    assert_eq!(count(&s, "d1"), 1, "d1's row must survive");
}

/// `update` rewrites rows; naming the wrong database would corrupt the
/// other one rather than merely lose rows, so it gets its own case.
#[test]
fn issue_5057_update_in_db_only_touches_the_named_database() {
    let (_t, mut s) = two_dbs();

    let n = s
        .update_in_db(
            "d2",
            "shared",
            &[Value::Integer(2)],
            &[(0, Value::Integer(22))],
        )
        .expect("update d2");

    assert_eq!(n, 1, "one row in d2 updated");

    let d2 = s.scan_in_db("d2", "shared").expect("scan d2");
    let d1 = s.scan_in_db("d1", "shared").expect("scan d1");
    assert_eq!(d2[0][0], Value::Integer(22), "d2's row was rewritten");
    assert_eq!(
        d1[0][0],
        Value::Integer(1),
        "d1's row must be untouched — the update named d2"
    );
}

/// `update_if` is a separate body again.
#[test]
fn issue_5057_update_if_in_db_only_touches_the_named_database() {
    let (_t, mut s) = two_dbs();

    let filter: RowFilter = Box::new(|row| row.first() == Some(&Value::Integer(2)));
    let mutation = RowMutation::new(vec![(0, Value::Integer(99))], 7);
    let n = s
        .update_if_in_db("d2", "shared", &filter, &mutation)
        .expect("update_if d2");

    assert_eq!(n, 1);
    let d2 = s.scan_in_db("d2", "shared").expect("scan d2");
    let d1 = s.scan_in_db("d1", "shared").expect("scan d1");
    assert_eq!(d2[0][0], Value::Integer(99), "d2's row was rewritten");
    assert_eq!(d1[0][0], Value::Integer(1), "d1's row must be untouched");
}

/// Control for the update family: no database means `current_db`.
#[test]
fn issue_5057_update_without_a_database_follows_current_db() {
    let (_t, mut s) = two_dbs();
    s.update("shared", &[Value::Integer(1)], &[(0, Value::Integer(11))])
        .expect("update current db");

    let d1 = s.scan_in_db("d1", "shared").expect("scan d1");
    let d2 = s.scan_in_db("d2", "shared").expect("scan d2");
    assert_eq!(d1[0][0], Value::Integer(11), "current_db was d1");
    assert_eq!(d2[0][0], Value::Integer(2), "d2 must be untouched");
}
