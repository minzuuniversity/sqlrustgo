//! V312-12 coverage improvement tests for `sqlrustgo_storage::BinaryTableStorageV2`
//! (Issue #4431 followup — B8 COVERAGE_MIN_PER_CRATE).
//!
//! Uses BIGINT column type (8 bytes) to match binary_storage_v2's fixed-width
//! encoding requirements.

use sqlrustgo_storage::binary_storage_v2::BinaryTableStorageV2;
use sqlrustgo_storage::engine::{ColumnDefinition, TableInfo};
use sqlrustgo_storage::Record;
use tempfile::TempDir;

fn make_dir() -> (TempDir, BinaryTableStorageV2) {
    let temp_dir = TempDir::new().unwrap();
    let storage = BinaryTableStorageV2::new(temp_dir.path().to_path_buf()).unwrap();
    (temp_dir, storage)
}

fn bigint_col(name: &str) -> ColumnDefinition {
    ColumnDefinition {
        name: name.to_string(),
        data_type: "BIGINT".to_string(),
        nullable: false,
        primary_key: false,
        char_max_length: None,
        collation: None,
        default_value: None,
        auto_increment: false,
    }
}

fn make_bigint_info(name: &str) -> TableInfo {
    TableInfo {
        name: name.to_string(),
        columns: vec![bigint_col("a")],
        foreign_keys: vec![],
        unique_constraints: vec![],
        check_constraints: vec![],
        partition_info: None,
        collations: Default::default(),
        compression: None,
    }
}

#[test]
fn cov_v2_new() {
    let (_t, _s) = make_dir();
}

#[test]
fn cov_v2_create_table() {
    let (_t, mut s) = make_dir();
    s.create_table("t", vec![bigint_col("a")]).unwrap();
}

#[test]
fn cov_v2_create_table_multi() {
    let (_t, mut s) = make_dir();
    s.create_table("t", vec![bigint_col("a"), bigint_col("b")]).unwrap();
}

#[test]
fn cov_v2_create_table_duplicate() {
    let (_t, mut s) = make_dir();
    s.create_table("t", vec![bigint_col("a")]).unwrap();
    let r = s.create_table("t", vec![bigint_col("a")]);
    assert!(r.is_err());
}

#[test]
fn cov_v2_create_table_many() {
    let (_t, mut s) = make_dir();
    for i in 0..5 {
        s.create_table(&format!("t{}", i), vec![bigint_col("a")]).unwrap();
    }
}

#[test]
fn cov_v2_insert_streaming_single_int() {
    let (_t, mut s) = make_dir();
    s.create_table("t", vec![bigint_col("a")]).unwrap();
    let record: Record = vec![sqlrustgo_storage::Value::Integer(42)];
    s.insert_streaming("t", vec![record]).unwrap();
}

#[test]
fn cov_v2_insert_streaming_multi_int() {
    let (_t, mut s) = make_dir();
    s.create_table("t", vec![bigint_col("a")]).unwrap();
    let records: Vec<Record> = (0..10i64).map(|i| vec![sqlrustgo_storage::Value::Integer(i)]).collect();
    s.insert_streaming("t", records).unwrap();
}

#[test]
fn cov_v2_insert_streaming_iter() {
    let (_t, mut s) = make_dir();
    s.create_table("t", vec![bigint_col("a")]).unwrap();
    let records: Vec<Record> = (0..20i64).map(|i| vec![sqlrustgo_storage::Value::Integer(i)]).collect();
    s.insert_streaming_iter("t", records.into_iter()).unwrap();
}

#[test]
fn cov_v2_insert_streaming_iter_empty() {
    let (_t, mut s) = make_dir();
    s.create_table("t", vec![bigint_col("a")]).unwrap();
    let empty: Vec<Record> = vec![];
    s.insert_streaming_iter("t", empty.into_iter()).unwrap();
}

#[test]
fn cov_v2_insert_streaming_missing_table() {
    let (_t, mut s) = make_dir();
    let r = s.insert_streaming("nonexistent", vec![vec![sqlrustgo_storage::Value::Integer(1)]]);
    assert!(r.is_err());
}

#[test]
fn cov_v2_insert_streaming_iter_missing_table() {
    let (_t, mut s) = make_dir();
    let records: Vec<Record> = vec![vec![sqlrustgo_storage::Value::Integer(1)]];
    let r = s.insert_streaming_iter("nonexistent", records.into_iter());
    assert!(r.is_err());
}

#[test]
fn cov_v2_insert_then_create_duplicate_fails() {
    let (_t, mut s) = make_dir();
    s.create_table("t", vec![bigint_col("a")]).unwrap();
    s.insert_streaming("t", vec![vec![sqlrustgo_storage::Value::Integer(1)]]).unwrap();
    let r = s.create_table("t", vec![bigint_col("a")]);
    assert!(r.is_err());
}

#[test]
fn cov_v2_flush_empty() {
    let (_t, mut s) = make_dir();
    s.flush().unwrap();
}

#[test]
fn cov_v2_flush_with_data() {
    let (_t, mut s) = make_dir();
    s.create_table("t", vec![bigint_col("a")]).unwrap();
    s.insert_streaming("t", vec![vec![sqlrustgo_storage::Value::Integer(1)]]).unwrap();
    s.flush().unwrap();
}

#[test]
fn cov_v2_flush_then_more() {
    let (_t, mut s) = make_dir();
    s.create_table("t", vec![bigint_col("a")]).unwrap();
    s.insert_streaming("t", vec![vec![sqlrustgo_storage::Value::Integer(1)]]).unwrap();
    s.flush().unwrap();
    s.insert_streaming("t", vec![vec![sqlrustgo_storage::Value::Integer(2)]]).unwrap();
    s.flush().unwrap();
}

#[test]
fn cov_v2_multiple_inserts_no_flush() {
    let (_t, mut s) = make_dir();
    s.create_table("t", vec![bigint_col("a")]).unwrap();
    for i in 0..20i64 {
        s.insert_streaming("t", vec![vec![sqlrustgo_storage::Value::Integer(i)]]).unwrap();
    }
}

#[test]
fn cov_v2_multiple_tables_no_flush() {
    let (_t, mut s) = make_dir();
    s.create_table("a", vec![bigint_col("x")]).unwrap();
    s.create_table("b", vec![bigint_col("y")]).unwrap();
    s.insert_streaming("a", vec![vec![sqlrustgo_storage::Value::Integer(1)]]).unwrap();
    s.insert_streaming("b", vec![vec![sqlrustgo_storage::Value::Integer(2)]]).unwrap();
}

#[test]
fn cov_v2_insert_bigint_range() {
    let (_t, mut s) = make_dir();
    s.create_table("t", vec![bigint_col("a")]).unwrap();
    // Test with max i64
    let record: Record = vec![sqlrustgo_storage::Value::Integer(i64::MAX)];
    s.insert_streaming("t", vec![record]).unwrap();
    let record2: Record = vec![sqlrustgo_storage::Value::Integer(i64::MIN)];
    s.insert_streaming("t", vec![record2]).unwrap();
}

#[test]
fn cov_v2_many_tables_insert() {
    let (_t, mut s) = make_dir();
    for i in 0..10 {
        s.create_table(&format!("t{}", i), vec![bigint_col("a")]).unwrap();
    }
    for i in 0..10 {
        s.insert_streaming(
            &format!("t{}", i),
            vec![vec![sqlrustgo_storage::Value::Integer(i)]],
        )
        .unwrap();
    }
}