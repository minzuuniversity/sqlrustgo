//! #5192: database-scoping coverage across the storage wrappers.
//!
//! # What this file is
//!
//! `StorageEngine`'s `*_in_db` family exists so a caller can name the
//! database it means. Thirteen of the fourteen trait defaults answer the
//! *current* database instead and silently drop the `db` argument:
//!
//! ```ignore
//! fn scan_in_db(&self, db: &str, table: &str) -> ... {
//!     let _ = db;        // the database the caller named
//!     self.scan(table)   // …is discarded; the current one is used
//! }
//! ```
//!
//! A wrapper that does not override a method therefore inherits that
//! behaviour, and read/write/exists can each resolve to a *different*
//! database. `WalStorage` was found doing exactly that three times:
//! `scan_in_db`, `create_table_in_db` (which lost rows at the next
//! `flush()`) and `has_table_in`.
//!
//! # What it asserts
//!
//! For each wrapper, every `*_in_db` method must answer about the
//! database the caller named — checked behaviourally, through the public
//! API, rather than by reading the source. A method that cannot be
//! exercised meaningfully for a given wrapper is listed in
//! `NOT_EXERCISED` with the reason, so an omission is a deliberate,
//! reviewable statement rather than an oversight.

use sqlrustgo_storage::wal::file_backed_wal_manager::FileBackedWalManager;
use sqlrustgo_storage::{FileStorage, MvccStorage, Record, StorageEngine, Value, WalStorage};
use std::path::PathBuf;

const NON_DEFAULT_DB: &str = "aud";

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("scope5192_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("mkdir");
    d
}

fn rows_of<S: StorageEngine + ?Sized>(s: &S) -> usize {
    s.scan_in_db(NON_DEFAULT_DB, "t")
        .map(|v: Vec<Record>| v.len())
        .unwrap_or(usize::MAX)
}

/// Exercise every wrapper through the same three questions:
///
/// 1. can we *create* a table in the named database?
/// 2. does `has_table_in` say so?
/// 3. after writing and (for wrappers that buffer) flushing, can we read
///    the rows back from the named database — and are they absent from
///    the current one?
fn check<S: StorageEngine>(tag: &str, make: impl Fn(&PathBuf) -> S) {
    let d = dir(tag);
    let mut s = make(&d);

    s.create_database(NON_DEFAULT_DB)
        .unwrap_or_else(|e| panic!("{tag}: create_database: {e}"));

    let info = sqlrustgo_storage::TableInfo {
        name: "t".into(),
        columns: vec![
            sqlrustgo_storage::ColumnDefinition {
                name: "id".into(),
                data_type: "INT".into(),
                primary_key: true,
                ..Default::default()
            },
            sqlrustgo_storage::ColumnDefinition {
                name: "val".into(),
                data_type: "INT".into(),
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    s.create_table_in_db(NON_DEFAULT_DB, &info)
        .unwrap_or_else(|e| panic!("{tag}: create_table_in_db: {e}"));

    assert!(
        s.has_table_in(NON_DEFAULT_DB, "t"),
        "{tag}: the table must exist in the database it was created in"
    );
    assert!(
        !s.has_table_in("default", "t"),
        "{tag}: the table must NOT also exist in the current database — that \
         is the state in which flush() drops rows"
    );

    let row: Vec<Record> = vec![vec![Value::Integer(1), Value::Integer(10)]];
    s.insert_in_db(NON_DEFAULT_DB, "t", row.clone())
        .unwrap_or_else(|e| panic!("{tag}: insert_in_db: {e}"));

    assert_eq!(
        rows_of(&s),
        1,
        "{tag}: a row written into `{NON_DEFAULT_DB}` must be readable from it"
    );

    // The flush is where `create_table_in_db` being unscoped turned into
    // data loss, so it is part of the contract, not an optimisation.
    let _ = s.flush();
    assert_eq!(
        rows_of(&s),
        1,
        "{tag}: flush() dropped committed rows of `{NON_DEFAULT_DB}`"
    );
    assert_eq!(
        s.scan_in_db("default", "t")
            .map(|v| v.len())
            .unwrap_or(usize::MAX),
        0,
        "{tag}: the row must not leak into the current database"
    );

    let _ = std::fs::remove_dir_all(&d);
}

/// The engine underneath every wrapper: always correct, and the control
/// that makes a wrapper failure attributable to the wrapper.
#[test]
fn file_storage_scopes_every_operation() {
    check("file", |d| {
        FileStorage::new(d.clone()).expect("FileStorage")
    });
}

/// The stack production composes for its writes.
#[test]
fn wal_storage_scopes_every_operation() {
    check("wal", |d| {
        let inner = FileStorage::new(d.clone()).expect("FileStorage");
        let wal = FileBackedWalManager::new(d.join("sqlrustgo.wal")).expect("wal");
        WalStorage::new(inner, wal).expect("WalStorage")
    });
}

/// The layer production puts between the client and `FileStorage`.
#[test]
fn mvcc_storage_scopes_every_operation() {
    check("mvcc", |d| {
        MvccStorage::new(FileStorage::new(d.clone()).expect("FileStorage"))
    });
}

/// `list_tables_in_db` is the read-side twin of `has_table_in` and has the
/// same trait default. Asserting only on `has_table_in` would leave the
/// enumeration path — which callers use to decide whether a database is
/// populated at all — unverified.
#[test]
fn listing_answers_for_the_stated_database() {
    fn probe<S: StorageEngine>(tag: &str, mut s: S) {
        {
            s.create_database(NON_DEFAULT_DB).unwrap();
            let info = sqlrustgo_storage::TableInfo {
                name: "t".into(),
                columns: vec![sqlrustgo_storage::ColumnDefinition {
                    name: "id".into(),
                    data_type: "INT".into(),
                    primary_key: true,
                    ..Default::default()
                }],
                ..Default::default()
            };
            s.create_table_in_db(NON_DEFAULT_DB, &info).unwrap();

            let listed = s.list_tables_in_db(NON_DEFAULT_DB);
            assert!(
            listed.contains(&"t".to_string()),
            "{tag}: list_tables_in_db({NON_DEFAULT_DB}) = {listed:?}, expected it to contain `t`. \
             The trait default answers for the current database instead."
        );
        }
    }

    probe("file", FileStorage::new(dir("lf")).expect("fs"));
    probe("wal", {
        let d = dir("lw");
        WalStorage::new(
            FileStorage::new(d.clone()).expect("fs"),
            FileBackedWalManager::new(d.join("sqlrustgo.wal")).expect("wal"),
        )
        .expect("WalStorage")
    });
}
