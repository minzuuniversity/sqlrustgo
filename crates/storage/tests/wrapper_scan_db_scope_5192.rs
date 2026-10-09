//! #5192: a wrapper must scope its reads to the same database it scoped
//! its writes to.
//!
//! # The defect
//!
//! `WalStorage` and `MvccStorage` implement the *write* half of database
//! scoping — `insert_in_db`, `delete_in_db`, `update_in_db` — but not the
//! read half. Every read fell through to the trait default at
//! `engine.rs:1333`:
//!
//! ```rust
//! fn scan_in_db(&self, db: &str, table: &str) -> SqlResult<Vec<Record>> {
//!     let _ = db;          // ← the stated database is discarded
//!     self.scan(table)
//! }
//! ```
//!
//! which resolves against the *current* database. A row written into a
//! named database was therefore invisible to a read that named it.
//!
//! # How it was isolated
//!
//! The same four operations against `FileStorage` alone behave correctly,
//! which is what pointed at the wrapper rather than the engine beneath it:
//!
//! ```text
//! FileStorage        insert_in_db("aud","t",r); scan_in_db("aud","t") -> 1 row
//! WalStorage         insert_in_db("aud","t",r); scan_in_db("aud","t") -> 0 rows
//! ```
//!
//! # Why it matters beyond the test
//!
//! This is the read side of the same omission as `insert_in_db`, which
//! #5192 already fixed on the write side. Left unfixed, a database other
//! than the current one can be written and then appears empty — the exact
//! "silent empty result" shape #5191 was filed for, reached through a
//! different door.

use sqlrustgo_storage::wal::file_backed_wal_manager::FileBackedWalManager;
use sqlrustgo_storage::{
    ColumnDefinition, FileStorage, MvccStorage, Record, StorageEngine, TableInfo, Value, WalStorage,
};
use std::path::PathBuf;

fn col(n: &str, pk: bool) -> ColumnDefinition {
    ColumnDefinition {
        name: n.into(),
        data_type: "INT".into(),
        primary_key: pk,
        ..Default::default()
    }
}

fn table(n: &str) -> TableInfo {
    TableInfo {
        name: n.into(),
        columns: vec![col("id", true), col("val", false)],
        ..Default::default()
    }
}

fn row(id: i64) -> Record {
    vec![Value::Integer(id), Value::Integer(id * 10)]
}

fn dir(t: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("wrap5192_{t}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// The wrapper production composes for its writes: a row written into a
/// named database must be readable from it.
#[test]
fn wal_storage_reads_back_from_the_database_it_wrote_to() {
    let d = dir("wal");
    let inner = FileStorage::new(d.clone()).unwrap();
    let wal = FileBackedWalManager::new(d.join("sqlrustgo.wal")).unwrap();
    let mut s = WalStorage::new(inner, wal).unwrap();

    s.create_database("aud").unwrap();
    s.create_table_in_db("aud", &table("t")).unwrap();
    s.insert_in_db("aud", "t", vec![row(1)]).unwrap();

    assert_eq!(
        s.scan_in_db("aud", "t").expect("scan aud.t"),
        vec![row(1)],
        "#5192: a row inserted into `aud` must be readable from `aud`. \
         An empty result here means the read fell through to the \
         database-blind trait default while the write was scoped."
    );
    assert_eq!(
        s.scan_in_db("default", "t").expect("scan default.t"),
        Vec::<Record>::new(),
        "#5192: the row must not also appear in `default` — the wrapper \
         would be resolving reads against the current database"
    );

    let _ = std::fs::remove_dir_all(&d);
}

/// Same omission in the other wrapper production puts between the client
/// and `FileStorage`.
#[test]
fn mvcc_storage_reads_back_from_the_database_it_wrote_to() {
    let d = dir("mvcc");
    let inner = FileStorage::new(d.clone()).unwrap();
    let mut s = MvccStorage::new(inner);

    s.create_database("aud").unwrap();
    s.create_table_in_db("aud", &table("t")).unwrap();
    s.insert_in_db("aud", "t", vec![row(1)]).unwrap();

    assert_eq!(
        s.scan_in_db("aud", "t").expect("scan aud.t"),
        vec![row(1)],
        "#5192: MvccStorage scoped the write but not the read"
    );
    assert!(s
        .scan_in_db("default", "t")
        .expect("scan default.t")
        .is_empty());

    let _ = std::fs::remove_dir_all(&d);
}

/// Control: the unwrapped engine always behaved correctly. Without this,
/// a regression in `FileStorage` and a regression in the wrapper would
/// look identical.
#[test]
fn file_storage_itself_scopes_reads_correctly() {
    let d = dir("plain");
    let mut s = FileStorage::new(d.clone()).unwrap();
    s.create_database("aud").unwrap();
    s.create_table_in_db("aud", &table("t")).unwrap();
    s.insert_in_db("aud", "t", vec![row(1)]).unwrap();

    assert_eq!(s.scan_in_db("aud", "t").expect("aud"), vec![row(1)]);
    assert!(s
        .scan_in_db("default", "t")
        .expect("scan default.t")
        .is_empty());

    let _ = std::fs::remove_dir_all(&d);
}

/// Two databases must not bleed into each other through a wrapper.
#[test]
fn wrapper_keeps_two_databases_separate() {
    let d = dir("twodb");
    let inner = FileStorage::new(d.clone()).unwrap();
    let wal = FileBackedWalManager::new(d.join("sqlrustgo.wal")).unwrap();
    let mut s = WalStorage::new(inner, wal).unwrap();

    for (db, id) in [("d1", 1_i64), ("d2", 2)] {
        s.create_database(db).unwrap();
        s.create_table_in_db(db, &table("t")).unwrap();
        s.insert_in_db(db, "t", vec![row(id)]).unwrap();
    }

    assert_eq!(s.scan_in_db("d1", "t").expect("d1"), vec![row(1)]);
    assert_eq!(s.scan_in_db("d2", "t").expect("d2"), vec![row(2)]);
    assert_eq!(
        s.scan_in_db("d1", "t").unwrap()[0][0],
        Value::Integer(1),
        "#5192: d1.t returned d2's row — a cross-database read"
    );

    let _ = std::fs::remove_dir_all(&d);
}

/// #5192: a table created in a named database must survive `flush()`.
///
/// This is the fourth instance of the same omission, and the only one that
/// loses data rather than merely misreporting it. `WalStorage` scoped
/// `insert_in_db` / `delete_in_db` / `update_in_db` / `scan_in_db` but not
/// `create_table_in_db`, so the trait default created the table under
/// `current_db` while the rows were buffered under the named one.
///
/// `flush_buffer` then removes those rows from `insert_buffer` and finds
/// no table to write them into — they are dropped. No crash and no
/// restart are involved; `flush()` is ordinary maintenance.
#[test]
fn wal_storage_flush_does_not_drop_rows_of_a_named_database() {
    let d = dir("flush");
    let inner = FileStorage::new(d.clone()).unwrap();
    let wal = FileBackedWalManager::new(d.join("sqlrustgo.wal")).unwrap();
    let mut s = WalStorage::new(inner, wal).unwrap();

    s.create_database("aud").unwrap();
    s.create_table_in_db("aud", &table("t")).unwrap();
    s.insert_in_db("aud", "t", vec![row(1), row(2)]).unwrap();

    assert_eq!(
        s.scan_in_db("aud", "t").expect("before flush"),
        vec![row(1), row(2)],
        "#5192: precondition — the rows are visible before the flush"
    );

    s.flush().expect("flush");

    assert_eq!(
        s.scan_in_db("aud", "t").expect("after flush"),
        vec![row(1), row(2)],
        "#5192: flush() dropped committed rows. The table was registered \
         under the current database while the rows were buffered under the \
         named one, so flush_buffer removed them and had nowhere to put them."
    );

    let _ = std::fs::remove_dir_all(&d);
}

/// The table itself must live in the database it was created in, not in
/// whichever one happened to be current. Without this the row-loss case
/// above would be a flush bug rather than a create bug.
#[test]
fn wal_storage_creates_the_table_in_the_stated_database() {
    let d = dir("create");
    let inner = FileStorage::new(d.clone()).unwrap();
    let wal = FileBackedWalManager::new(d.join("sqlrustgo.wal")).unwrap();
    let mut s = WalStorage::new(inner, wal).unwrap();

    s.create_database("aud").unwrap();
    s.create_table_in_db("aud", &table("t")).unwrap();

    assert!(
        s.has_table_in("aud", "t"),
        "#5192: the table must be registered under the database it was \
         created in"
    );
    assert!(
        !s.has_table_in("default", "t"),
        "#5192: the table must NOT also be registered under the current \
         database — that is the state in which flush() drops rows"
    );

    let _ = std::fs::remove_dir_all(&d);
}
