//! V4.0.0 / SOAK-leak fix: `StorageEngine::scan_with_filter`
//!
//! Root cause of V4.0.0 secondary SOAK leak (~1.7 GB/h):
//!   `execute_delete` (engine_dml.rs:1001-1017) called `storage.scan()` for
//!   every DELETE, which cloned the entire table (Vec<Record>) before filtering.
//!   At sysbench oltp_read_write with table_size=10000 + 8 threads, each
//!   DELETE cloned ~2.6 MB; jemalloc (prof_active, lg_prof_sample=14) retained
//!   the dirty pages → RSS grew linearly at ~30 MB/min.
//!
//! This test exercises the new `scan_with_filter` trait method that filters
//! **inside the read lock**, so non-matching rows are never cloned.

use sqlrustgo_storage::engine::ColumnDefinition;
use sqlrustgo_storage::engine::TableInfo;
use sqlrustgo_storage::{FileStorage, MemoryStorage, StorageEngine, Value as SqlValue};
use tempfile::TempDir;

// --------------------------------------------------------------------------
// Helpers
// --------------------------------------------------------------------------

fn table_info() -> TableInfo {
    TableInfo {
        name: "t".to_string(),
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

fn make_file_storage() -> (TempDir, FileStorage) {
    let temp_dir = TempDir::new().unwrap();
    let storage = FileStorage::new(temp_dir.path().to_path_buf()).unwrap();
    (temp_dir, storage)
}

fn make_memory_storage() -> MemoryStorage {
    MemoryStorage::new()
}

fn populate<S: StorageEngine>(storage: &mut S, n: usize) {
    storage.create_table(&table_info()).unwrap();
    let records: Vec<Vec<SqlValue>> = (0..n as i64).map(|i| vec![SqlValue::Integer(i)]).collect();
    storage.insert("t", records).unwrap();
}

// --------------------------------------------------------------------------
// Behaviour tests — prove correctness of filter
// --------------------------------------------------------------------------

#[test]
fn file_storage_scan_with_filter_returns_only_matching_rows() {
    let (_t, mut s) = make_file_storage();
    populate(&mut s, 100);

    let matched: Vec<Vec<SqlValue>> = s
        .scan_with_filter("t", |row| match row.first() {
            Some(SqlValue::Integer(i)) => i % 7 == 0,
            _ => false,
        })
        .expect("scan_with_filter should succeed");

    // Rows where id % 7 == 0: 0, 7, 14, 21, 28, 35, 42, 49, 56, 63, 70, 77, 84, 91, 98
    assert_eq!(matched.len(), 15);
    for row in &matched {
        if let SqlValue::Integer(i) = row[0] {
            assert_eq!(i % 7, 0, "filter returned non-matching row id={}", i);
        } else {
            panic!("expected Integer");
        }
    }
}

#[test]
fn memory_storage_scan_with_filter_returns_only_matching_rows() {
    let mut s = make_memory_storage();
    populate(&mut s, 100);

    let matched: Vec<Vec<SqlValue>> = s
        .scan_with_filter("t", |row| match row.first() {
            Some(SqlValue::Integer(i)) => *i >= 90,
            _ => false,
        })
        .expect("scan_with_filter should succeed");

    // ids 90..=99 → 10 rows
    assert_eq!(matched.len(), 10);
    for row in &matched {
        if let SqlValue::Integer(i) = row[0] {
            assert!(i >= 90 && i < 100, "out of range id={}", i);
        } else {
            panic!("expected Integer");
        }
    }
}

#[test]
fn file_storage_scan_with_filter_empty_predicate_returns_all() {
    let (_t, mut s) = make_file_storage();
    populate(&mut s, 50);

    let matched = s
        .scan_with_filter("t", |_| true)
        .expect("scan_with_filter should succeed");
    assert_eq!(matched.len(), 50);
}

#[test]
fn file_storage_scan_with_filter_no_match_returns_empty() {
    let (_t, mut s) = make_file_storage();
    populate(&mut s, 10);

    let matched = s
        .scan_with_filter("t", |_| false)
        .expect("scan_with_filter should succeed");
    assert!(matched.is_empty());
}

#[test]
fn file_storage_scan_with_filter_missing_table_returns_empty() {
    let (_t, s) = make_file_storage();
    let matched = s
        .scan_with_filter("nope", |_| true)
        .expect("scan_with_filter on missing table should return empty");
    assert!(matched.is_empty());
}

// --------------------------------------------------------------------------
// Allocation-bounded test — guards against regressions to O(N) cloning
// --------------------------------------------------------------------------
//
// We can't directly count allocations behind `Box<dyn FnMut>`, but we CAN
// prove that for a 10000-row table where the predicate matches exactly ONE
// row, `scan_with_filter` returns a Vec of length 1. A naive `scan+filter`
// clone-everything-then-filter implementation would still return length 1,
// so this test alone doesn't prove allocation efficiency — that's verified
// by the SOAK after the fix. But it does prove the new method is wired up
// and semantically correct, which is the minimum we need to ship.

#[test]
fn file_storage_scan_with_filter_single_match_returns_one_row() {
    let (_t, mut s) = make_file_storage();
    populate(&mut s, 10_000);

    let matched = s
        .scan_with_filter("t", |row| match row.first() {
            Some(SqlValue::Integer(i)) => *i == 4_242,
            _ => false,
        })
        .expect("scan_with_filter should succeed");

    assert_eq!(matched.len(), 1);
    assert_eq!(matched[0][0], SqlValue::Integer(4_242));
}
