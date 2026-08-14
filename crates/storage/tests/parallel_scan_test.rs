//! FileStorage::parallel_scan integration tests
//!
//! v3.10.0 Issue #3703: Verify storage layer parallel scan works
//! correctly for FileStorage (in-memory cached) and respects the
//! insert_buffer for transaction visibility.

use sqlrustgo_storage::engine::StorageEngine;
use sqlrustgo_storage::file_storage::FileStorage;
use sqlrustgo_storage::ColumnDefinition;
use sqlrustgo_storage::TableInfo;
use sqlrustgo_types::Value;
use tempfile::TempDir;

fn make_test_storage() -> (FileStorage, TempDir) {
    let dir = TempDir::new().expect("create temp dir");
    let mut storage = FileStorage::new(dir.path().to_path_buf()).expect("create storage");
    let table_info = TableInfo {
        name: "t".to_string(),
        columns: vec![ColumnDefinition {
            name: "id".to_string(),
            data_type: "INTEGER".to_string(),
            nullable: false,
            primary_key: true,
            char_max_length: None,
            collation: None,

            default_value: None,
        }],
        foreign_keys: vec![],
        unique_constraints: vec![],
        check_constraints: vec![],
        compression: None,
        collations: std::collections::HashMap::new(),
        partition_info: None,
    };
    storage.create_table(&table_info).expect("create table");
    (storage, dir)
}

/// Materialize a parallel_scan result into a single Vec for testing
fn materialize(partitions: Vec<Box<dyn Iterator<Item = Vec<Value>> + Send>>) -> Vec<Vec<Value>> {
    let mut all = Vec::new();
    for partition in partitions {
        all.extend(partition);
    }
    all
}

#[test]
fn test_parallel_scan_empty_table() {
    let (storage, _dir) = make_test_storage();
    let result = storage.parallel_scan("t", 4).expect("parallel_scan");
    assert!(result.is_empty());
}

#[test]
fn test_parallel_scan_small_dataset_preserves_all_rows() {
    let (mut storage, _dir) = make_test_storage();
    let rows: Vec<Vec<Value>> = (0..10).map(|i| vec![Value::Integer(i)]).collect();
    storage.insert("t", rows).expect("insert");
    let result = storage.parallel_scan("t", 4).expect("parallel_scan");
    let all = materialize(result);
    assert_eq!(all.len(), 10);
}

#[test]
fn test_parallel_scan_large_dataset() {
    let (mut storage, _dir) = make_test_storage();
    let rows: Vec<Vec<Value>> = (0..1000).map(|i| vec![Value::Integer(i)]).collect();
    storage.insert("t", rows).expect("insert");
    let result = storage.parallel_scan("t", 4).expect("parallel_scan");
    let all = materialize(result);
    assert_eq!(all.len(), 1000);
}

#[test]
fn test_parallel_scan_zero_partitions() {
    let (mut storage, _dir) = make_test_storage();
    let rows: Vec<Vec<Value>> = (0..10).map(|i| vec![Value::Integer(i)]).collect();
    storage.insert("t", rows).expect("insert");
    let result = storage.parallel_scan("t", 0).expect("parallel_scan");
    assert!(result.is_empty());
}

#[test]
fn test_parallel_scan_more_partitions_than_rows() {
    let (mut storage, _dir) = make_test_storage();
    let rows: Vec<Vec<Value>> = (0..5).map(|i| vec![Value::Integer(i)]).collect();
    storage.insert("t", rows).expect("insert");
    let result = storage.parallel_scan("t", 100).expect("parallel_scan");
    let all = materialize(result);
    assert_eq!(all.len(), 5);
}

#[test]
fn test_parallel_scan_uneven_distribution() {
    let (mut storage, _dir) = make_test_storage();
    let rows: Vec<Vec<Value>> = (0..1001).map(|i| vec![Value::Integer(i)]).collect();
    storage.insert("t", rows).expect("insert");
    let result = storage.parallel_scan("t", 4).expect("parallel_scan");
    let all = materialize(result);
    assert_eq!(
        all.len(),
        1001,
        "All rows must be preserved across partitions"
    );
}

#[test]
fn test_parallel_scan_nonexistent_table() {
    let (storage, _dir) = make_test_storage();
    let result = storage
        .parallel_scan("nonexistent", 4)
        .expect("parallel_scan");
    assert!(result.is_empty());
}

#[test]
fn test_parallel_scan_all_values_preserved() {
    // Verify that values are not lost or duplicated across partitions
    let (mut storage, _dir) = make_test_storage();
    let rows: Vec<Vec<Value>> = (0..100).map(|i| vec![Value::Integer(i)]).collect();
    storage.insert("t", rows).expect("insert");
    let result = storage.parallel_scan("t", 4).expect("parallel_scan");
    let mut all = materialize(result);
    all.sort_by_key(|r| if let Value::Integer(i) = &r[0] { *i } else { 0 });
    let expected: Vec<i64> = (0..100).collect();
    let actual: Vec<i64> = all
        .iter()
        .filter_map(|r| {
            if let Value::Integer(i) = &r[0] {
                Some(*i)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(actual, expected);
}
