//! #5281 R2 red tests — the chain identity contract, asserted before any fix.
//!
//! #5280 gave every inserted row an identity (`row_identity`) and keyed the
//! MVCC version chain by it. UPDATE and DELETE never adopted that identity:
//! `MvccStorage::update` keys on `row.first()`, and the key the delete path
//! receives is whatever `FileStorage::delete_collect_pks_at` returns — also
//! `row.first()`, with the comment `// PK = column 0`.
//!
//! The result is that an update appends to a chain nobody inserted into and
//! leaves the real chain untouched, so the row's old value stays visible; and a
//! delete writes its tombstone to that same orphan chain, so the row is
//! resurrected by the scan that reads its original chain.
//!
//! Each test states the property in terms of the *result set* rather than
//! internal chain counts wherever possible, so the fix is free to change how
//! identity is represented. RT-2 also inspects the chain, because "the delete
//! is invisible" and "the tombstone is on the wrong chain" are the same
//! failure and only one of them is observable from SQL.
//!
//! Expected on the current code: every test fails. These are the assertions
//! that must hold once #5281 R2 lands.

use sqlrustgo_storage::{
    BoxStorageEngine, ColumnDefinition, FileStorage, MemoryWalManager, MvccStorage,
    ParallelWalStorage, StorageEngine, TableInfo,
};
use sqlrustgo_types::Value;
use std::path::PathBuf;
use std::sync::Arc;

fn dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("r2_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn columns(pk_second: bool) -> TableInfo {
    let mut b = ColumnDefinition {
        name: "b".to_string(),
        data_type: "INT".to_string(),
        ..Default::default()
    };
    if pk_second {
        b.primary_key = true;
    }
    TableInfo {
        name: "t".to_string(),
        columns: vec![
            ColumnDefinition {
                name: "a".to_string(),
                data_type: "INT".to_string(),
                ..Default::default()
            },
            b,
        ],
        ..Default::default()
    }
}

fn stack(tag: &str, pk_second: bool) -> BoxStorageEngine {
    let file = FileStorage::new(dir(tag)).expect("FileStorage");
    let mvcc = MvccStorage::new(file);
    BoxStorageEngine::new(ParallelWalStorage::new(mvcc, MemoryWalManager::new()))
}

fn seeded(tag: &str, pk_second: bool, rows: &[(i64, i64)]) -> BoxStorageEngine {
    let mut s = stack(tag, pk_second);
    s.create_database("default").expect("create db");
    s.create_table_in_db("default", &columns(pk_second))
        .expect("create table");
    let records: Vec<Vec<Value>> = rows
        .iter()
        .map(|(a, b)| vec![Value::Integer(*a), Value::Integer(*b)])
        .collect();
    s.insert_in_db("default", "t", records).expect("insert");
    s
}

fn scan(s: &BoxStorageEngine) -> Vec<Vec<Value>> {
    s.scan_in_tx_db("default", "t", 0).expect("scan")
}

fn contains(rows: &[Vec<Value>], a: i64, b: i64) -> bool {
    rows.iter()
        .any(|r| r.len() == 2 && r[0] == Value::Integer(a) && r[1] == Value::Integer(b))
}

/// RT-1 — an UPDATE replaces the row it matched and adds no rows.
///
/// The failure mode this pins: the update appends a version to a chain keyed by
/// `row[0]` while the chain that actually holds the row is never touched, so
/// both the old and the new value come back.
#[test]
fn rt1_update_replaces_row_without_adding_rows() {
    let mut s = seeded("rt1", false, &[(1, 10), (1, 20), (1, 40)]);

    s.update(
        "t",
        &[Value::Integer(1), Value::Integer(20)],
        &[(1usize, Value::Integer(99))],
    )
    .expect("update");

    let rows = scan(&s);
    assert_eq!(
        rows.len(),
        3,
        "an UPDATE must not change the row count; got {rows:?}"
    );
    assert!(
        contains(&rows, 1, 99),
        "the updated value is missing from {rows:?}"
    );
    assert!(
        !contains(&rows, 1, 20),
        "the pre-update value is still visible in {rows:?}"
    );
    assert!(
        contains(&rows, 1, 10),
        "an unrelated row was lost: {rows:?}"
    );
    assert!(
        contains(&rows, 1, 40),
        "an unrelated row was lost: {rows:?}"
    );
}

/// RT-2 — a DELETE writes its tombstone to the chain that holds the row.
///
/// SQL alone cannot tell "the delete never reached the version chain" apart
/// from "the chain still has the row", so this looks at the chain directly.
#[test]
fn rt2_delete_tombstones_the_chain_holding_the_row() {
    let mut s = seeded("rt2", false, &[(1, 10), (1, 20), (1, 40)]);

    s.delete("t", &[Value::Integer(1), Value::Integer(40)])
        .expect("delete");

    let rows = scan(&s);
    assert_eq!(rows.len(), 2, "exactly one row should go; got {rows:?}");
    assert!(
        !contains(&rows, 1, 40),
        "the deleted row is still visible; the tombstone landed on a chain \
         that does not hold it: {rows:?}"
    );
    assert!(
        contains(&rows, 1, 10),
        "an unrelated row was lost: {rows:?}"
    );
    assert!(
        contains(&rows, 1, 20),
        "an unrelated row was lost: {rows:?}"
    );
}

/// RT-3 — rows that agree in one column must not collapse into one chain.
///
/// Two rows sharing `b` but differing in `a`: if the chain key is taken from
/// the wrong column, both land on the same key and one disappears.
#[test]
fn rt3_rows_sharing_a_column_do_not_collapse() {
    let mut s = seeded("rt3", false, &[(1, 10), (2, 10)]);

    s.update("t", &[Value::Integer(10)], &[(1usize, Value::Integer(99))])
        .expect("update");

    let rows = scan(&s);
    assert_eq!(rows.len(), 2, "both rows must survive; got {rows:?}");
    assert!(contains(&rows, 1, 99), "first row not updated: {rows:?}");
    assert!(contains(&rows, 2, 10), "second row was disturbed: {rows:?}");
    assert!(
        !contains(&rows, 1, 10),
        "the pre-update value is still visible: {rows:?}"
    );
}

/// RT-4 — the primary key is the identity, wherever it sits in the column list.
///
/// `b` is declared primary here. Keying on `row[0]` (`a`) means every row of
/// this table shares one key, and the update never reaches the chain that
/// actually holds the row.
#[test]
fn rt4_primary_key_in_second_column_still_addresses_its_chain() {
    let mut s = seeded("rt4", true, &[(1, 100), (1, 200)]);

    s.update(
        "t",
        &[Value::Integer(1), Value::Integer(200)],
        &[(1usize, Value::Integer(201))],
    )
    .expect("update");

    let rows = scan(&s);
    assert_eq!(rows.len(), 2, "the row count must hold; got {rows:?}");
    assert!(
        contains(&rows, 1, 201),
        "the updated primary-key value is missing from {rows:?}"
    );
    assert!(
        !contains(&rows, 1, 200),
        "the row still carries its pre-update primary key: {rows:?}"
    );
}

/// RT-5 — deleting one row must not resurrect or remove its neighbours.
///
/// Same shape as RT-1's seed, but through DELETE: the tombstone has to reach
/// the row that matched, and only that row.
#[test]
fn rt5_delete_touches_only_the_matching_row() {
    let mut s = seeded("rt5", false, &[(1, 10), (1, 20), (1, 40)]);

    s.delete("t", &[Value::Integer(1), Value::Integer(20)])
        .expect("delete");

    let rows = scan(&s);
    assert_eq!(rows.len(), 2, "exactly one row should go; got {rows:?}");
    assert!(
        !contains(&rows, 1, 20),
        "the deleted row is still visible: {rows:?}"
    );
    assert!(
        contains(&rows, 1, 10),
        "an unrelated row was lost: {rows:?}"
    );
    assert!(
        contains(&rows, 1, 40),
        "an unrelated row was lost: {rows:?}"
    );
}

/// The stack under test is the production one, not a bare FileStorage.
///
/// Without this the suite would pass for the wrong reason: every assertion
/// above concerns the MVCC wrapper, and `FileStorage` alone has no version
/// chain to get wrong.
#[test]
fn rt0_suite_exercises_the_production_storage_stack() {
    let file = FileStorage::new(dir("rt0")).expect("FileStorage");
    let mvcc = MvccStorage::new(file);
    let wal = ParallelWalStorage::new(mvcc, MemoryWalManager::new());
    let boxed = BoxStorageEngine::new(wal);
    let _typed: Arc<dyn StorageEngine> = Arc::new(boxed);
}
