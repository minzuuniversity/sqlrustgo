//! CLI client module tests
//!
//! Tests: QueryResult struct

use std::time::Duration;
use sqlrustgo_soak::client::QueryResult;

#[test]
fn test_query_result_empty() {
    let result = QueryResult {
        columns: vec![],
        rows: vec![],
        row_count: 0,
        duration: Duration::from_secs(0),
    };
    assert_eq!(result.columns.len(), 0);
    assert_eq!(result.rows.len(), 0);
    assert_eq!(result.row_count, 0);
}

#[test]
fn test_query_result_with_data() {
    let result = QueryResult {
        columns: vec!["id".to_string(), "name".to_string()],
        rows: vec![
            vec!["1".to_string(), "Alice".to_string()],
            vec!["2".to_string(), "Bob".to_string()],
        ],
        row_count: 2,
        duration: Duration::from_millis(5),
    };
    assert_eq!(result.columns.len(), 2);
    assert_eq!(result.columns[0], "id");
    assert_eq!(result.columns[1], "name");
    assert_eq!(result.rows.len(), 2);
    assert_eq!(result.rows[0][0], "1");
    assert_eq!(result.rows[1][1], "Bob");
    assert_eq!(result.row_count, 2);
}

#[test]
fn test_query_result_debug() {
    let result = QueryResult {
        columns: vec!["col1".to_string()],
        rows: vec![vec!["val".to_string()]],
        row_count: 1,
        duration: Duration::from_micros(500),
    };
    let debug = format!("{:?}", result);
    assert!(debug.contains("QueryResult"));
    assert!(debug.contains("col1"));
}

#[test]
fn test_query_result_clone() {
    let result = QueryResult {
        columns: vec!["a".to_string(), "b".to_string()],
        rows: vec![vec!["x".to_string(), "y".to_string()]],
        row_count: 1,
        duration: Duration::from_secs(1),
    };
    let r2 = result.clone();
    assert_eq!(r2.columns, result.columns);
    assert_eq!(r2.rows, result.rows);
    assert_eq!(r2.row_count, result.row_count);
}

#[test]
fn test_query_result_null_cells() {
    let result = QueryResult {
        columns: vec!["a".to_string(), "b".to_string()],
        rows: vec![
            vec!["".to_string(), "value".to_string()],
            vec!["hello".to_string(), "".to_string()],
        ],
        row_count: 2,
        duration: Duration::from_millis(10),
    };
    assert_eq!(result.rows[0][0], "");
    assert_eq!(result.rows[1][1], "");
    assert_eq!(result.rows[0][1], "value");
}

#[test]
fn test_query_result_duration() {
    let result = QueryResult {
        columns: vec![],
        rows: vec![],
        row_count: 0,
        duration: Duration::from_secs(30),
    };
    assert_eq!(result.duration, Duration::from_secs(30));
}
