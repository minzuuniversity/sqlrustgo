//! #5277: a wrapper must not lose the database or transaction of a scan.
//!
//! `StorageEngine::scan_in_tx_db` had a default body of
//!
//! ```ignore
//! fn scan_in_tx_db(&self, db: &str, table: &str, reader_tx: u64) -> … {
//!     let _ = db;
//!     self.scan_in(table, reader_tx)
//! }
//! ```
//!
//! and `BoxStorageEngine` did not override it. So a query reaching
//! `scan_for_reader_in_db` on the boxed stack fell through to the default,
//! which resolves the table against the storage's current database instead of
//! the stated one. The same data read through `scan_in_db` gave three rows; read
//! through `scan_in_tx_db` it gave one.
//!
//! These tests go through the production stack —
//! `FileStorage → MvccStorage → ParallelWalStorage → BoxStorageEngine` —
//! because the defect is in the delegation, not in any single layer. Testing
//! `MvccStorage` directly would pass both before and after the fix.

use sqlrustgo_storage::{
    BoxStorageEngine, FileStorage, MemoryWalManager, MvccStorage, ParallelWalStorage, StorageEngine,
};
use sqlrustgo_types::Value;
use std::path::PathBuf;

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("boxed_tx_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn two_columns() -> sqlrustgo_storage::TableInfo {
    sqlrustgo_storage::TableInfo {
        name: "t".to_string(),
        columns: vec![
            sqlrustgo_storage::ColumnDefinition {
                name: "a".to_string(),
                data_type: "INT".to_string(),
                ..Default::default()
            },
            sqlrustgo_storage::ColumnDefinition {
                name: "b".to_string(),
                data_type: "INT".to_string(),
                ..Default::default()
            },
        ],
        ..Default::default()
    }
}

fn three_rows() -> Vec<Vec<Value>> {
    vec![
        vec![Value::Integer(1), Value::Integer(10)],
        vec![Value::Integer(1), Value::Integer(20)],
        vec![Value::Integer(1), Value::Integer(40)],
    ]
}

/// The production stack the MySQL server actually builds.
fn boxed_stack(tag: &str) -> BoxStorageEngine {
    let file = FileStorage::new(dir(tag)).expect("FileStorage");
    let mvcc = MvccStorage::new(file);
    let wal = ParallelWalStorage::new(mvcc, MemoryWalManager::new());
    BoxStorageEngine::new(wal)
}

fn seed(tag: &str) -> BoxStorageEngine {
    let mut s = boxed_stack(tag);
    s.create_database("default").expect("create db");
    s.create_table_in_db("default", &two_columns())
        .expect("create table");
    s.insert_in_db("default", "t", three_rows())
        .expect("insert");
    s
}

/// The plain scan and the transactional scan must agree when no transaction is
/// open — a statement outside a transaction sees everything committed.
#[test]
fn boxed_transactional_scan_sees_every_row() {
    let s = seed("agree");
    let plain = s.scan_in_db("default", "t").expect("scan_in_db");
    let tx = s.scan_in_tx_db("default", "t", 0).expect("scan_in_tx_db");

    assert_eq!(
        plain.len(),
        3,
        "the plain scan should return all three rows"
    );
    assert_eq!(
        tx.len(),
        3,
        "the transactional scan returned {} of 3 rows — a wrapper dropped the \
         database or transaction instead of delegating",
        tx.len()
    );
}

/// Rows identical in every column are still three logical rows. SQL is a bag
/// semantics without DISTINCT, and nothing here may collapse them.
#[test]
fn boxed_transactional_scan_keeps_fully_duplicate_rows() {
    let mut s = boxed_stack("dup");
    s.create_database("default").expect("create db");
    s.create_table_in_db("default", &two_columns())
        .expect("create table");

    // Three rows identical in every column, including the identity column.
    let dup = vec![
        vec![Value::Integer(1), Value::Integer(10)],
        vec![Value::Integer(1), Value::Integer(10)],
        vec![Value::Integer(1), Value::Integer(10)],
    ];
    s.insert_in_db("default", "t", dup)
        .expect("insert duplicates");

    let tx = s.scan_in_tx_db("default", "t", 0).expect("scan_in_tx_db");
    assert_eq!(
        tx.len(),
        3,
        "three identical rows are still three rows — SQL is a bag without DISTINCT"
    );
}

/// The stated database has to be the one that is read, not whatever the
/// storage considers current.
#[test]
fn boxed_transactional_scan_respects_the_stated_database() {
    let mut s = seed("ns");
    for db in ["d1", "d2"] {
        s.create_database(db).expect("create db");
        let mut info = two_columns();
        info.name = "t".to_string();
        s.create_table_in_db(db, &info).expect("create table");
    }
    s.insert_in_db("d1", "t", three_rows()).expect("d1 rows");

    let d1 = s.scan_in_tx_db("d1", "t", 0).expect("d1");
    assert_eq!(d1.len(), 3, "d1.t has three rows");

    let d2 = s.scan_in_tx_db("d2", "t", 0).expect("d2");
    assert!(
        d2.is_empty(),
        "d2.t is empty; reading it must not fall back to another database"
    );
}
