//! #5057: every `MemoryStorage` `*_in_db` method must resolve against the
//! database it is given, not the shared `storage.current_db()`.
//!
//! Seven of them were trait defaults of the form
//! `{ let _ = db; self.op(table, ..) }` — the explicit-database API with
//! the explicit database thrown away. They now delegate to an `_in_key`
//! inherent method that takes the resolved key.

use sqlrustgo_storage::engine::{
    IndexInfo, Record, RowFilter, RowMutation, StorageEngine, TableInfo,
};
use sqlrustgo_storage::MemoryStorage;
use sqlrustgo_types::Value;

fn table(name: &str) -> TableInfo {
    let mut info = TableInfo::default();
    info.name = name.to_string();
    info.columns = vec![
        sqlrustgo_storage::ColumnDefinition::new("id", "INTEGER"),
        sqlrustgo_storage::ColumnDefinition::new("v", "INTEGER"),
    ];
    info
}

fn seeded() -> MemoryStorage {
    let mut s = MemoryStorage::new();
    s.create_database("d1").unwrap();
    s.create_database("d2").unwrap();
    for (db, id) in [("d1", 1i64), ("d2", 2)] {
        s.set_current_db(db).unwrap();
        s.create_table(&table("t")).unwrap();
        s.insert("t", vec![vec![Value::Integer(id), Value::Integer(id * 10)]])
            .unwrap();
    }
    s
}

/// Every method below is called with the database that is *not* stored, so
/// a method that fell back to the trait default would touch the wrong rows.
#[test]
fn insert_in_db_lands_in_the_named_database() {
    let mut s = seeded();
    s.set_current_db("d2").unwrap();
    s.insert_in_db("d1", "t", vec![vec![Value::Integer(99), Value::Integer(0)]])
        .unwrap();
    assert_eq!(s.scan_in_db("d1", "t").unwrap().len(), 2);
    assert_eq!(
        s.scan_in_db("d2", "t").unwrap().len(),
        1,
        "d2 must not have received the row written for d1"
    );
}

#[test]
fn force_insert_in_db_lands_in_the_named_database() {
    let mut s = seeded();
    s.set_current_db("d2").unwrap();
    s.force_insert_in_db("d1", "t", vec![Value::Integer(77), Value::Integer(0)])
        .unwrap();
    assert_eq!(s.scan_in_db("d1", "t").unwrap().len(), 2);
    assert_eq!(s.scan_in_db("d2", "t").unwrap().len(), 1);
}

#[test]
fn delete_in_db_spares_the_other_database() {
    let mut s = seeded();
    let n = s.delete_in_db("d1", "t", &[]).unwrap();
    assert_eq!(n, 1);
    assert!(s.scan_in_db("d1", "t").unwrap().is_empty());
    assert_eq!(s.scan_in_db("d2", "t").unwrap().len(), 1);
}

#[test]
fn delete_collect_pks_in_db_reports_the_right_keys() {
    let mut s = seeded();
    // d2 is stored; ask for d1. An empty filter means "wipe the table",
    // which by design yields no PK list (the MVCC wrapper tombstones the
    // whole table instead), so target one row by its key.
    s.set_current_db("d2").unwrap();
    let pks = s
        .delete_collect_pks_in_db("d1", "t", &[Value::Integer(1)])
        .unwrap();
    assert_eq!(pks, vec![Value::Integer(1)]);
}

#[test]
fn update_in_db_changes_only_the_named_database() {
    let mut s = seeded();
    let n = s
        .update_in_db("d1", "t", &[Value::Integer(1)], &[(1, Value::Integer(111))])
        .unwrap();
    assert_eq!(n, 1);
    let d1 = s.scan_in_db("d1", "t").unwrap();
    let d2 = s.scan_in_db("d2", "t").unwrap();
    assert_eq!(d1[0][1], Value::Integer(111), "d1's row should be updated");
    assert_eq!(d2[0][1], Value::Integer(20), "d2's row must be untouched");
}

#[test]
fn update_if_in_db_changes_only_the_named_database() {
    let mut s = seeded();
    let filter: RowFilter = Box::new(|r: &Record| r[0] == Value::Integer(1));
    let mutation = RowMutation::new(vec![(1, Value::Integer(222))], 0);
    let n = s.update_if_in_db("d1", "t", &filter, &mutation).unwrap();
    assert_eq!(n, 1);
    assert_eq!(s.scan_in_db("d1", "t").unwrap()[0][1], Value::Integer(222));
    assert_eq!(s.scan_in_db("d2", "t").unwrap()[0][1], Value::Integer(20));
}

#[test]
fn scan_with_index_in_db_reads_the_named_database() {
    let mut s = seeded();
    // The index belongs to whichever database was active when it was
    // created — register it inside d1, where the test then looks.
    s.set_current_db("d1").unwrap();
    s.create_index(IndexInfo {
        name: "id".into(),
        table: "t".into(),
        columns: vec![sqlrustgo_parser::IndexColumnSpec::column("id")],
        is_unique: true,
        original_sql: String::new(),
    })
    .unwrap();

    let rows = s
        .scan_with_index_in_db("d1", "t", "id", &Value::Integer(1))
        .unwrap();
    assert_eq!(rows, vec![vec![Value::Integer(1), Value::Integer(10)]]);
    assert!(
        s.scan_with_index_in_db("d2", "t", "id", &Value::Integer(1))
            .is_err(),
        "d2 has no row with id 1"
    );
}
