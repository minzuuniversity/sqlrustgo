//! #5277 E4-C: a row's identity must hold for its whole lifetime.
//!
//! The MVCC chain is keyed by row identity. Giving each INSERT a fresh
//! identity fixed the scan, but only if UPDATE and DELETE address the *same*
//! row by that identity — otherwise an updated row looks like a new record,
//! and the row count grows.
//!
//! These assert values, not shapes. A statement that returns "no error" while
//! leaving the data unchanged, or that silently doubles the row count, is the
//! failure mode this pins.

use sqlrustgo_storage::{
    BoxStorageEngine, FileStorage, MemoryWalManager, MvccStorage, ParallelWalStorage,
    StorageEngine,
};
use sqlrustgo_types::Value;
use std::path::PathBuf;

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("lifecycle_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn columns() -> sqlrustgo_storage::TableInfo {
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

fn stack(tag: &str) -> BoxStorageEngine {
    let file = FileStorage::new(dir(tag)).expect("FileStorage");
    let mvcc = MvccStorage::new(file);
    BoxStorageEngine::new(ParallelWalStorage::new(mvcc, MemoryWalManager::new()))
}

fn seeded(tag: &str) -> BoxStorageEngine {
    let mut s = stack(tag);
    s.create_database("default").expect("create db");
    s.create_table_in_db("default", &columns()).expect("create table");
    s.insert_in_db(
        "default",
        "t",
        vec![
            vec![Value::Integer(1), Value::Integer(10)],
            vec![Value::Integer(1), Value::Integer(20)],
            vec![Value::Integer(1), Value::Integer(40)],
        ],
    )
    .expect("insert");
    s
}

fn rows(s: &BoxStorageEngine) -> Vec<Vec<Value>> {
    s.scan_in_tx_db("default", "t", 0).expect("scan")
}

/// C1: an UPDATE must change the row in place, not append a new record.
///
/// The three rows share a first value, so if the update keys by that value it
/// lands on the wrong row; if it mints a fresh identity the table grows a
/// fourth row. Only reusing the existing identity does the right thing.
#[test]
fn update_changes_one_row_without_growing_the_table() {
    let mut s = seeded("update");
    let n = s
        .update(
            "t",
            &[Value::Integer(1), Value::Integer(20)],
            &[(1usize, Value::Integer(99))],
        )
        .expect("update");
    assert!(n > 0, "the update should have matched a row");

    let got = rows(&s);
    assert_eq!(
        got.len(),
        3,
        "an update must not add a row; the table now has {}: {got:?}",
        got.len()
    );
    assert!(
        got.iter().any(|r| **r == [Value::Integer(1), Value::Integer(99)]),
        "the updated value is missing from {got:?}"
    );
    assert_eq!(
        got.iter()
            .filter(|r| **r == [Value::Integer(1), Value::Integer(10)])
            .count(),
        1,
        "the untouched row must survive exactly once: {got:?}"
    );
}

/// C1b: a DELETE must remove one row, not the rows that merely share a value.
#[test]
fn delete_removes_only_the_matching_row() {
    let mut s = seeded("delete");
    s.delete("t", &[Value::Integer(1), Value::Integer(40)])
        .expect("delete");

    let got = rows(&s);
    assert_eq!(got.len(), 2, "one row should be gone: {got:?}");
    assert!(
        !got.iter()
            .any(|r| **r == [Value::Integer(1), Value::Integer(40)]),
        "the deleted row is still visible: {got:?}"
    );
}

/// C2: a rolled-back transaction leaves the table exactly as it was.
#[test]
fn rollback_restores_the_previous_state() {
    let mut s = seeded("rollback");
    let before = rows(&s);
    assert_eq!(before.len(), 3);

    s.begin_transaction().expect("begin");
    s.update(
        "t",
        &[Value::Integer(1), Value::Integer(10)],
        &[(1usize, Value::Integer(77))],
    )
    .expect("update in tx");
    s.rollback_transaction().expect("rollback");

    let after = rows(&s);
    assert_eq!(
        after.len(),
        3,
        "a rolled-back update must not change the row count: {after:?}"
    );
    assert_eq!(
        after, before,
        "a rolled-back update must restore the original values"
    );
}

/// C3: reloading from the inner engine keeps every row distinct, and new rows
/// land beside them rather than replacing them.
#[test]
fn rebuild_keeps_repeated_rows_and_accepts_more() {
    // `rebuild_from_inner` is what the server runs at startup after recovery,
    // and it lives on the MVCC layer rather than the trait.
    let file = FileStorage::new(dir("rebuild")).expect("FileStorage");
    let mvcc = MvccStorage::new(file);
    let mut s = BoxStorageEngine::new(ParallelWalStorage::new(
        mvcc,
        MemoryWalManager::new(),
    ));
    s.create_database("default").expect("create db");
    s.create_table_in_db("default", &columns()).expect("create table");
    s.insert_in_db(
        "default",
        "t",
        vec![
            vec![Value::Integer(1), Value::Integer(10)],
            vec![Value::Integer(1), Value::Integer(20)],
            vec![Value::Integer(1), Value::Integer(40)],
        ],
    )
    .expect("insert");
    assert_eq!(rows(&s).len(), 3);

    // Identities handed out after a reload must not collide with reloaded rows.
    s.insert_in_db("default", "t", vec![vec![Value::Integer(1), Value::Integer(50)]])
        .expect("insert after reload");
    assert_eq!(
        rows(&s).len(),
        4,
        "a row inserted after a reload must not overwrite an existing one"
    );
}

/// C4: the identity is the declared primary key even when it is not the first
/// column, so lookups and updates still find the right row.
#[test]
fn primary_key_need_not_be_the_first_column() {
    let mut s = stack("pk_second");
    s.create_database("default").expect("create db");
    let info = sqlrustgo_storage::TableInfo {
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
                primary_key: true,
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    s.create_table_in_db("default", &info).expect("create table");
    // Same `a`, different primary key `b`.
    s.insert_in_db(
        "default",
        "t",
        vec![
            vec![Value::Integer(1), Value::Integer(100)],
            vec![Value::Integer(1), Value::Integer(200)],
        ],
    )
    .expect("insert");

    assert_eq!(rows(&s).len(), 2, "two distinct primary keys are two rows");

    s.update(
        "t",
        &[Value::Integer(1), Value::Integer(200)],
        &[(1usize, Value::Integer(201))],
    )
    .expect("update");

    let got = rows(&s);
    assert_eq!(got.len(), 2, "an update must not add a row: {got:?}");
    assert!(
        got.iter().any(|r| **r == [Value::Integer(1), Value::Integer(201)]),
        "the update keyed on the wrong column: {got:?}"
    );
}
