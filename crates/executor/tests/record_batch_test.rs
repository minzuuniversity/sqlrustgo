//! RecordBatch (columnar format) integration tests
//!
//! v3.10.0 Issue #3703 Phase 5: Columnar data format

use sqlrustgo_executor::record_batch::RecordBatch;
use sqlrustgo_executor::simd_eval::BitMask;
use sqlrustgo_types::Value;

#[test]
fn test_empty_batch() {
    let batch = RecordBatch::empty();
    assert_eq!(batch.num_rows(), 0);
    assert_eq!(batch.num_columns(), 0);
}

#[test]
fn test_from_rows_basic() {
    let rows = vec![
        vec![Value::Integer(1), Value::Text("a".into())],
        vec![Value::Integer(2), Value::Text("b".into())],
        vec![Value::Integer(3), Value::Text("c".into())],
    ];
    let columns = vec!["id".to_string(), "name".to_string()];

    let batch = RecordBatch::from_rows(columns, rows).unwrap();
    assert_eq!(batch.num_rows(), 3);
    assert_eq!(batch.num_columns(), 2);

    // Verify columnar layout
    let id_col = batch.column(0).unwrap();
    assert_eq!(id_col.len(), 3);
    assert_eq!(id_col[0], Value::Integer(1));
    assert_eq!(id_col[2], Value::Integer(3));
}

#[test]
fn test_from_rows_empty() {
    let columns = vec!["id".to_string()];
    let batch = RecordBatch::from_rows(columns, vec![]).unwrap();
    assert_eq!(batch.num_rows(), 0);
    assert_eq!(batch.num_columns(), 1);
}

#[test]
fn test_to_rows_roundtrip() {
    let rows = vec![
        vec![Value::Integer(1), Value::Text("a".into())],
        vec![Value::Integer(2), Value::Text("b".into())],
    ];
    let columns = vec!["id".to_string(), "name".to_string()];

    let batch = RecordBatch::from_rows(columns, rows.clone()).unwrap();
    let roundtrip = batch.to_rows();
    assert_eq!(roundtrip, rows);
}

#[test]
fn test_column_by_name() {
    let rows = vec![
        vec![Value::Integer(1), Value::Text("a".into())],
        vec![Value::Integer(2), Value::Text("b".into())],
    ];
    let columns = vec!["id".to_string(), "name".to_string()];
    let batch = RecordBatch::from_rows(columns, rows).unwrap();

    let id_col = batch.column_by_name("id").unwrap();
    assert_eq!(id_col.len(), 2);
    assert_eq!(id_col[0], Value::Integer(1));

    let missing = batch.column_by_name("nonexistent");
    assert!(missing.is_none());
}

#[test]
fn test_inconsistent_rows_rejected() {
    let rows = vec![
        vec![Value::Integer(1), Value::Text("a".into())],
        vec![Value::Integer(2)], // Missing column
    ];
    let columns = vec!["id".to_string(), "name".to_string()];
    let batch = RecordBatch::from_rows(columns, rows);
    assert!(batch.is_none());
}

#[test]
fn test_filter_batch() {
    let rows = vec![
        vec![Value::Integer(1), Value::Text("a".into())],
        vec![Value::Integer(2), Value::Text("b".into())],
        vec![Value::Integer(3), Value::Text("c".into())],
        vec![Value::Integer(4), Value::Text("d".into())],
    ];
    let columns = vec!["id".to_string(), "name".to_string()];
    let batch = RecordBatch::from_rows(columns, rows).unwrap();

    // Pass indices 1 and 3
    let mask = BitMask::from_bits(0b1010);
    let filtered = batch.filter(&mask);

    assert_eq!(filtered.num_rows(), 2);
    let id_col = filtered.column(0).unwrap();
    assert_eq!(id_col[0], Value::Integer(2));
    assert_eq!(id_col[1], Value::Integer(4));
}

#[test]
fn test_filter_all_pass() {
    let rows = vec![
        vec![Value::Integer(1)],
        vec![Value::Integer(2)],
        vec![Value::Integer(3)],
    ];
    let columns = vec!["id".to_string()];
    let batch = RecordBatch::from_rows(columns, rows).unwrap();

    let mask = BitMask::all_true(3);
    let filtered = batch.filter(&mask);
    assert_eq!(filtered.num_rows(), 3);
}

#[test]
fn test_filter_all_fail() {
    let rows = vec![vec![Value::Integer(1)], vec![Value::Integer(2)]];
    let columns = vec!["id".to_string()];
    let batch = RecordBatch::from_rows(columns, rows).unwrap();

    let mask = BitMask::all_false();
    let filtered = batch.filter(&mask);
    assert_eq!(filtered.num_rows(), 0);
}

#[test]
fn test_columnar_vs_rowar() {
    // Verify that RecordBatch is actually columnar
    let rows = vec![
        vec![Value::Integer(1), Value::Text("a".into())],
        vec![Value::Integer(2), Value::Text("b".into())],
    ];
    let columns = vec!["id".to_string(), "name".to_string()];
    let batch = RecordBatch::from_rows(columns, rows).unwrap();

    // Each column is a separate Vec - can be accessed independently
    let id_col = batch.column(0).unwrap();
    let name_col = batch.column(1).unwrap();

    assert_eq!(id_col[0], Value::Integer(1));
    assert_eq!(name_col[0], Value::Text("a".into()));
    assert_eq!(id_col[1], Value::Integer(2));
    assert_eq!(name_col[1], Value::Text("b".into()));
}
