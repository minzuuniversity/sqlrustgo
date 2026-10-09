//! #5209: `CREATE INDEX` must index rows that are still in the insert buffer.
//!
//! # The defect
//!
//! `FileStorage::create_index` built its B+ tree by iterating `tables.rows`
//! only. Rows written since the last flush live in `insert_buffer`, which
//! is a *second copy* of table state (see `file_storage.rs`'s own comment
//! on the field). Creating an index before a flush therefore produced an
//! index over an empty or partial table.
//!
//! The loss is silent and permanent. `scan_with_index_in` does scan
//! `insert_buffer` as a fallback, but that fallback stops matching once
//! those rows are flushed into `tables.rows` — at which point they are
//! reachable only through the index, which is empty.
//!
//! # Why this test goes to the storage layer
//!
//! A SQL-level test cannot see this. #5209's first attempt asserted
//! `SELECT ... WHERE k = 20` and passed both before and after the fix,
//! because the SELECT path does not consult an index for that predicate —
//! it falls back to a full scan, which merges both row stores and returns
//! the right answer either way. Asserting on SQL results here would have
//! been a test that proves nothing.
//!
//! So these call `scan_with_index_in` directly and assert on what the
//! index actually returns.

use sqlrustgo_parser::IndexColumnSpec;
use sqlrustgo_storage::{
    ColumnDefinition, FileStorage, IndexInfo, Record, StorageEngine, TableInfo, Value,
};
use std::path::PathBuf;

const DB: &str = "default";

fn tmpdir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("idx5209_{tag}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("mkdir");
    d
}

fn col(name: &str, pk: bool) -> ColumnDefinition {
    ColumnDefinition {
        name: name.to_string(),
        data_type: "INT".to_string(),
        primary_key: pk,
        ..Default::default()
    }
}

fn empty_table() -> TableInfo {
    TableInfo {
        name: "t".to_string(),
        columns: vec![col("id", true), col("k", false)],
        ..Default::default()
    }
}

fn idx_info() -> IndexInfo {
    IndexInfo {
        name: "idx_k".to_string(),
        table: "t".to_string(),
        columns: vec![IndexColumnSpec {
            name: Some("k".to_string()),
            expression: None,
        }],
        is_unique: false,
        original_sql: "CREATE INDEX idx_k ON t(k)".to_string(),
    }
}

/// The ordering that produced an empty index: insert, `CREATE INDEX` with
/// no flush in between, then read *through the index*.
///
/// Keys here are distinct (10/20/30) so the separate `BPlusTree`
/// single-value-per-key limitation does not confound the result.
#[test]
fn index_built_before_flush_still_finds_buffered_rows() {
    let dir = tmpdir("preflush");
    let mut s = FileStorage::new_with_buffer_config(dir.clone(), 1_000_000, true).expect("fs");
    s.create_table(&empty_table()).expect("create table");

    let records: Vec<Record> = vec![
        vec![Value::Integer(1), Value::Integer(10)],
        vec![Value::Integer(2), Value::Integer(20)],
        vec![Value::Integer(3), Value::Integer(30)],
    ];
    s.insert("t", records).expect("insert");

    // Threshold is 1_000_000, so these rows stay in `insert_buffer`.
    // CREATE INDEX here is the ordering that used to build an empty tree.
    StorageEngine::create_index(&mut s, idx_info()).expect("create index");
    s.flush().ok();

    let found = s
        .scan_with_index_in_db(DB, "t", "k", &Value::Integer(20))
        .expect("index scan");
    assert!(
        !found.is_empty(),
        "#5209: the index was built while every row was still in \
         insert_buffer, so it held nothing. After the flush the \
         insert_buffer fallback no longer matches and the row is \
         unreachable through the index."
    );
    assert_eq!(found[0][0], Value::Integer(2), "#5209: wrong row returned");

    assert!(
        s.scan_with_index_in_db(DB, "t", "k", &Value::Integer(999))
            .expect("index scan")
            .is_empty(),
        "#5209: a key with no row must still return nothing"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// Every buffered row must be reachable, not just the last one per key.
#[test]
fn index_covers_every_buffered_row() {
    let dir = tmpdir("cover");
    let mut s = FileStorage::new_with_buffer_config(dir.clone(), 1_000_000, true).expect("fs");
    s.create_table(&empty_table()).expect("create table");

    let records: Vec<Record> = (1..=50)
        .map(|i| vec![Value::Integer(i), Value::Integer(i * 10)])
        .collect();
    s.insert("t", records).expect("insert");

    StorageEngine::create_index(&mut s, idx_info()).expect("create index");
    s.flush().ok();

    let mut missing = Vec::new();
    for i in 1..=50i64 {
        let key = Value::Integer(i * 10);
        if s.scan_with_index_in_db(DB, "t", "k", &key)
            .expect("index scan")
            .is_empty()
        {
            missing.push(i);
        }
    }
    assert!(
        missing.is_empty(),
        "#5209: {} of 50 buffered rows unreachable through the index: {missing:?}",
        missing.len()
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// Control case: with a flush *before* `CREATE INDEX` the index is found
/// even without the fix. Without this, "index empty" and "index works"
/// would be indistinguishable and the two tests above could pass for the
/// wrong reason.
#[test]
fn index_built_after_flush_works() {
    let dir = tmpdir("postflush");
    let mut s = FileStorage::new_with_buffer_config(dir.clone(), 1_000_000, true).expect("fs");
    s.create_table(&empty_table()).expect("create table");
    s.insert("t", vec![vec![Value::Integer(7), Value::Integer(70)]])
        .expect("insert");
    s.flush().ok();

    StorageEngine::create_index(&mut s, idx_info()).expect("create index");

    let found = s
        .scan_with_index_in_db(DB, "t", "k", &Value::Integer(70))
        .expect("index scan");
    assert_eq!(found.len(), 1, "#5209: post-flush index must find the row");
    assert_eq!(found[0][0], Value::Integer(7));

    let _ = std::fs::remove_dir_all(&dir);
}
