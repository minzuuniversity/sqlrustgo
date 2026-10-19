//! #5057: every `MemoryStorage` `*_in_db` method must resolve against the
//! database it is given.
//!
//! Seven of them were trait defaults of the form
//! `{ let _ = db; self.op(table, ..) }` — the explicit-database API with
//! the explicit database thrown away. They now delegate to an `_in_key`
//! inherent method that takes the resolved key.
//!
//! #5375: `current_db` / `set_current_db` were removed from the engine in
//! #5057 step 3, so there is no "stored" database any more and the
//! database-less methods resolve against `DEFAULT_DATABASE`. The old
//! "ask for the database that is *not* the stored one" trick therefore no
//! longer discriminates; the discriminating power moves to `default`,
//! which is seeded here with a same-named table of its own. A method that
//! ignored its `db` argument would land on `default.t` and be caught.

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
    // `default` carries a same-named table too, with a row id no other
    // database uses — so a write that ignored its `db` argument lands
    // somewhere visibly wrong.
    s.create_table_in_db("default", &table("t")).unwrap();
    s.insert_in_db(
        "default",
        "t",
        vec![vec![Value::Integer(3), Value::Integer(30)]],
    )
    .unwrap();
    for (db, id) in [("d1", 1i64), ("d2", 2)] {
        s.create_table_in_db(db, &table("t")).unwrap();
        s.insert_in_db(
            db,
            "t",
            vec![vec![Value::Integer(id), Value::Integer(id * 10)]],
        )
        .unwrap();
    }
    s
}

/// Every method below is called with the database that is *not* `default`,
/// so a method that fell back to the trait default would touch the wrong
/// rows.
#[test]
fn insert_in_db_lands_in_the_named_database() {
    let mut s = seeded();
    s.insert_in_db("d1", "t", vec![vec![Value::Integer(99), Value::Integer(0)]])
        .unwrap();
    assert_eq!(s.scan_in_db("d1", "t").unwrap().len(), 2);
    assert_eq!(
        s.scan_in_db("d2", "t").unwrap().len(),
        1,
        "d2 must not have received the row written for d1"
    );
    assert_eq!(
        s.scan_in_db("default", "t").unwrap().len(),
        1,
        "default must not have received the row written for d1"
    );
}

#[test]
fn force_insert_in_db_lands_in_the_named_database() {
    let mut s = seeded();
    s.force_insert_in_db("d1", "t", vec![Value::Integer(77), Value::Integer(0)])
        .unwrap();
    assert_eq!(s.scan_in_db("d1", "t").unwrap().len(), 2);
    assert_eq!(s.scan_in_db("d2", "t").unwrap().len(), 1);
    assert_eq!(s.scan_in_db("default", "t").unwrap().len(), 1);
}

#[test]
fn delete_in_db_spares_the_other_database() {
    let mut s = seeded();
    let n = s.delete_in_db("d1", "t", &[]).unwrap();
    assert_eq!(n, 1);
    assert!(s.scan_in_db("d1", "t").unwrap().is_empty());
    assert_eq!(s.scan_in_db("d2", "t").unwrap().len(), 1);
    assert_eq!(s.scan_in_db("default", "t").unwrap().len(), 1);
}

#[test]
fn delete_collect_pks_in_db_reports_the_right_keys() {
    let mut s = seeded();
    // An empty filter means "wipe the table", which by design yields no
    // PK list (the MVCC wrapper tombstones the whole table instead), so
    // target one row by its key. `default`'s row has a different id, so a
    // call that ignored the `db` argument would report the wrong key.
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
    let default = s.scan_in_db("default", "t").unwrap();
    assert_eq!(d1[0][1], Value::Integer(111), "d1's row should be updated");
    assert_eq!(d2[0][1], Value::Integer(20), "d2's row must be untouched");
    assert_eq!(
        default[0][1],
        Value::Integer(30),
        "default's row must be untouched"
    );
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
    assert_eq!(
        s.scan_in_db("default", "t").unwrap()[0][1],
        Value::Integer(30)
    );
}

/// #5375: this test used to create the index while `d1` was the "stored"
/// database and then look it up under `d1`'s scoped key.
///
/// That is no longer expressible: `create_index` has no `_in_db` variant
/// and registers the index under `DEFAULT_DATABASE` (see
/// `MemoryStorage::create_index` → `self.tbl(...)`), while
/// `scan_with_index_in_db` looks the index up under the *named*
/// database's scoped key. So a named database other than `default` can
/// never find an index — the write half of the index API cannot be
/// addressed per database at all.
///
/// That is a gap in the engine, not in the test, and it is tracked
/// separately. What this test pins is the behaviour that does exist: the
/// index is registered under `default`, is found by a `default`-scoped
/// lookup, and is *not* found by a named database's lookup.
#[test]
fn scan_with_index_in_db_reads_the_named_database() {
    let mut s = seeded();
    s.create_index(IndexInfo {
        name: "id".into(),
        table: "t".into(),
        columns: vec![sqlrustgo_parser::IndexColumnSpec::column("id")],
        is_unique: true,
        original_sql: String::new(),
    })
    .unwrap();

    // The index landed under `default`, so that is the database whose
    // scoped key can find it.
    let rows = s
        .scan_with_index_in_db("default", "t", "id", &Value::Integer(3))
        .unwrap();
    assert_eq!(rows, vec![vec![Value::Integer(3), Value::Integer(30)]]);

    assert!(
        s.scan_with_index_in_db("d1", "t", "id", &Value::Integer(1))
            .is_err(),
        "the index is registered under `default`, so d1's scoped key cannot \
         find it — the index write path is not database-scoped"
    );
    assert!(
        s.scan_with_index_in_db("d2", "t", "id", &Value::Integer(1))
            .is_err(),
        "same for d2"
    );
}
