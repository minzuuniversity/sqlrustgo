// Window Function Integration Tests
// Tests for complete window function execution through the SQL engine
//
// Run with:
//   cargo test --test window_function_test -- --nocapture
//
// Note: These tests verify:
// 1. Window function SQL parsing works correctly
// 2. Basic database operations work

use parking_lot::RwLock;
use sqlrustgo::{parse, ExecutionEngine, MemoryStorage};
use std::sync::Arc;

/// Helper to test window function parsing
fn test_window_parse(sql: &str) {
    let result = parse(sql);
    assert!(
        result.is_ok(),
        "Failed to parse window function: {:?} - SQL: {}",
        result,
        sql
    );
}

// ============================================================================
// Parsing Tests - Verify window function SQL can be parsed
// These tests verify the parser can handle window function syntax
// ============================================================================

#[test]
fn test_parse_row_number_basic() {
    test_window_parse("SELECT ROW_NUMBER() OVER (ORDER BY id) FROM users");
}

#[test]
fn test_parse_row_number_with_partition() {
    test_window_parse(
        "SELECT ROW_NUMBER() OVER (PARTITION BY dept ORDER BY salary) FROM employees",
    );
}

#[test]
fn test_parse_rank_window() {
    test_window_parse("SELECT RANK() OVER (ORDER BY score DESC) FROM students");
}

#[test]
fn test_parse_dense_rank_window() {
    test_window_parse("SELECT DENSE_RANK() OVER (ORDER BY value) FROM data");
}

#[test]
fn test_parse_lead_window() {
    test_window_parse("SELECT LEAD(salary) OVER (ORDER BY hire_date) FROM employees");
}

#[test]
fn test_parse_lead_with_offset() {
    test_window_parse("SELECT LEAD(value, 2) OVER (ORDER BY id) FROM timeline");
}

#[test]
fn test_parse_lag_window() {
    test_window_parse("SELECT LAG(prev_value) OVER (ORDER BY id) FROM history");
}

#[test]
fn test_parse_lag_with_default() {
    test_window_parse("SELECT LAG(value, 1, 0) OVER (ORDER BY id) FROM prices");
}

#[test]
fn test_parse_first_value_window() {
    test_window_parse(
        "SELECT FIRST_VALUE(salary) OVER (PARTITION BY dept ORDER BY hire_date) FROM employees",
    );
}

#[test]
fn test_parse_last_value_window() {
    test_window_parse("SELECT LAST_VALUE(value) OVER (ORDER BY ts) FROM metrics");
}

#[test]
fn test_parse_window_with_rows_frame() {
    test_window_parse("SELECT ROW_NUMBER() OVER (ORDER BY id ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) FROM data");
}

#[test]
fn test_parse_window_with_range_frame() {
    test_window_parse("SELECT RANK() OVER (ORDER BY score RANGE BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) FROM scores");
}

#[test]
fn test_parse_window_with_exclude() {
    test_window_parse("SELECT ROW_NUMBER() OVER (ORDER BY id ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW EXCLUDE CURRENT ROW) FROM data");
}

#[test]
fn test_parse_window_without_order_by() {
    test_window_parse("SELECT ROW_NUMBER() OVER () FROM users");
}

#[test]
fn test_parse_window_partition_only() {
    test_window_parse("SELECT ROW_NUMBER() OVER (PARTITION BY region) FROM sales");
}

#[test]
fn test_parse_window_nth_value() {
    test_window_parse("SELECT NTH_VALUE(salary, 2) OVER (ORDER BY hire_date) FROM employees");
}

#[test]
fn test_parse_multiple_partitions() {
    // Multiple PARTITION BY columns
    test_window_parse(
        "SELECT ROW_NUMBER() OVER (PARTITION BY dept, location ORDER BY salary) FROM employees",
    );
}

#[test]
fn test_parse_descending_order() {
    // ORDER BY with DESC
    test_window_parse("SELECT ROW_NUMBER() OVER (ORDER BY salary DESC) FROM employees");
}

#[test]
fn test_parse_nulls_first() {
    // ORDER BY with NULLS FIRST/LAST
    test_window_parse("SELECT ROW_NUMBER() OVER (ORDER BY salary NULLS FIRST) FROM employees");
}

#[test]
fn test_parse_nulls_last() {
    // NULLS LAST postfix
    test_window_parse("SELECT ROW_NUMBER() OVER (ORDER BY salary NULLS LAST) FROM employees");
}

#[test]
fn test_parse_nulls_first_asc_default() {
    // ASC NULLS FIRST
    test_window_parse("SELECT ROW_NUMBER() OVER (ORDER BY salary ASC NULLS FIRST) FROM employees");
}

#[test]
fn test_parse_nulls_last_desc() {
    // DESC NULLS LAST (overrides DESC default of NULLS FIRST)
    test_window_parse("SELECT ROW_NUMBER() OVER (ORDER BY salary DESC NULLS LAST) FROM employees");
}

#[test]
fn test_parse_exclude_current_row() {
    // EXCLUDE CURRENT ROW drops the current row from the frame
    test_window_parse(
        "SELECT ROW_NUMBER() OVER (ORDER BY id ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW EXCLUDE CURRENT ROW) FROM data",
    );
}

#[test]
fn test_parse_exclude_ties() {
    // EXCLUDE TIES drops all rows tied with the current row
    test_window_parse(
        "SELECT ROW_NUMBER() OVER (ORDER BY id ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW EXCLUDE TIES) FROM data",
    );
}

#[test]
fn test_parse_exclude_group() {
    // EXCLUDE GROUP drops the entire peer group
    test_window_parse(
        "SELECT ROW_NUMBER() OVER (ORDER BY id ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW EXCLUDE GROUP) FROM data",
    );
}

#[test]
fn test_parse_exclude_no_others() {
    // EXCLUDE NO OTHERS keeps only the current peer group
    test_window_parse(
        "SELECT ROW_NUMBER() OVER (ORDER BY id ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW EXCLUDE NO OTHERS) FROM data",
    );
}

#[test]
fn test_parse_exclude_without_frame_omitted() {
    // EXCLUDE without an explicit frame should still parse (EXCLUDE is
    // an optional postfix; if frame is None the exclusion has no effect).
    test_window_parse("SELECT ROW_NUMBER() OVER (ORDER BY id EXCLUDE CURRENT ROW) FROM data");
}

#[test]
fn test_parse_exclude_lowercase() {
    // EXCLUDE keyword is case-insensitive
    test_window_parse(
        "SELECT ROW_NUMBER() OVER (ORDER BY id ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW exclude current row) FROM data",
    );
}

#[test]
fn test_parse_exclude_unknown_mode_rejected() {
    // EXCLUDE with an unknown mode must fail at parse time, not silently
    // bind to a no-op.
    let result = parse(
        "SELECT ROW_NUMBER() OVER (ORDER BY id ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW EXCLUDE FOO) FROM data",
    );
    assert!(
        result.is_err(),
        "EXCLUDE FOO must be rejected, got: {:?}",
        result
    );
}

#[test]
fn test_parse_exclude_current_requires_row() {
    // EXCLUDE CURRENT without ROW is a malformed SQL:2003 frame postfix.
    let result = parse(
        "SELECT ROW_NUMBER() OVER (ORDER BY id ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW EXCLUDE CURRENT) FROM data",
    );
    assert!(
        result.is_err(),
        "EXCLUDE CURRENT (without ROW) must be rejected, got: {:?}",
        result
    );
}

// ============================================================================
// Summary Test - Verify key window functions work
// ============================================================================

#[test]
fn test_summary_window_parse_works() {
    // This test verifies that the key window functions can be parsed
    // The exact functions tested here are known to work in the parser
    let test_cases = vec![
        "SELECT ROW_NUMBER() OVER (ORDER BY id) FROM users",
        "SELECT RANK() OVER (ORDER BY id) FROM users",
        "SELECT DENSE_RANK() OVER (ORDER BY id) FROM users",
        "SELECT LEAD(col) OVER (ORDER BY id) FROM users",
        "SELECT LAG(col) OVER (ORDER BY id) FROM users",
    ];

    for sql in test_cases {
        let result = parse(sql);
        assert!(
            result.is_ok(),
            "Failed to parse: {} - Error: {:?}",
            sql,
            result
        );
    }
}

// ============================================================================
// NULLS FIRST / NULLS LAST Executor Behaviour
// ============================================================================

#[test]
fn test_execute_nulls_first_default_asc() {
    // ASC NULLS FIRST: NULLs sort before non-NULLs.
    // SQL:1999 §6.10 default for ASC is NULLS LAST, so NULLS FIRST
    // explicitly overrides the default.
    let mut engine = ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())));
    engine
        .execute("CREATE TABLE t (id INTEGER, v INTEGER)")
        .unwrap();
    engine
        .execute("INSERT INTO t VALUES (1, 10), (2, NULL), (3, 30), (4, NULL), (5, 20)")
        .unwrap();

    let r = engine
        .execute("SELECT ROW_NUMBER() OVER (ORDER BY v NULLS FIRST) AS r, v FROM t ORDER BY r")
        .unwrap();
    // ROW_NUMBER assigns 1, 2 to NULLs (in input order), then 3, 4, 5 to 10, 20, 30.
    assert_eq!(r.rows.len(), 5);
    // First two rows are NULLs (r=1, r=2 in some order)
    assert!(matches!(r.rows[0][1], sqlrustgo::Value::Null));
    assert!(matches!(r.rows[1][1], sqlrustgo::Value::Null));
    // Remaining three are 10, 20, 30 in ascending order
    assert_eq!(r.rows[2][1], sqlrustgo::Value::Integer(10));
    assert_eq!(r.rows[3][1], sqlrustgo::Value::Integer(20));
    assert_eq!(r.rows[4][1], sqlrustgo::Value::Integer(30));
}

#[test]
fn test_execute_nulls_last_default_asc() {
    // ASC NULLS LAST: NULLs sort after non-NULLs (matches SQL:1999 default).
    let mut engine = ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())));
    engine
        .execute("CREATE TABLE t (id INTEGER, v INTEGER)")
        .unwrap();
    engine
        .execute("INSERT INTO t VALUES (1, 10), (2, NULL), (3, 30), (4, NULL), (5, 20)")
        .unwrap();

    let r = engine
        .execute("SELECT ROW_NUMBER() OVER (ORDER BY v NULLS LAST) AS r, v FROM t ORDER BY r")
        .unwrap();
    assert_eq!(r.rows.len(), 5);
    // First three rows are non-NULLs in ascending order: 10, 20, 30
    assert_eq!(r.rows[0][1], sqlrustgo::Value::Integer(10));
    assert_eq!(r.rows[1][1], sqlrustgo::Value::Integer(20));
    assert_eq!(r.rows[2][1], sqlrustgo::Value::Integer(30));
    // Last two rows are NULLs (r=4, r=5 in some order)
    assert!(matches!(r.rows[3][1], sqlrustgo::Value::Null));
    assert!(matches!(r.rows[4][1], sqlrustgo::Value::Null));
}

#[test]
fn test_execute_nulls_first_default_desc() {
    // DESC NULLS FIRST: NULLs sort before non-NULLs (matches SQL:1999 default
    // for DESC).
    let mut engine = ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())));
    engine
        .execute("CREATE TABLE t (id INTEGER, v INTEGER)")
        .unwrap();
    engine
        .execute("INSERT INTO t VALUES (1, 10), (2, NULL), (3, 30), (4, NULL), (5, 20)")
        .unwrap();

    let r = engine
        .execute("SELECT ROW_NUMBER() OVER (ORDER BY v DESC NULLS FIRST) AS r, v FROM t ORDER BY r")
        .unwrap();
    assert_eq!(r.rows.len(), 5);
    // First two rows are NULLs
    assert!(matches!(r.rows[0][1], sqlrustgo::Value::Null));
    assert!(matches!(r.rows[1][1], sqlrustgo::Value::Null));
    // Remaining three are 30, 20, 10 in descending order
    assert_eq!(r.rows[2][1], sqlrustgo::Value::Integer(30));
    assert_eq!(r.rows[3][1], sqlrustgo::Value::Integer(20));
    assert_eq!(r.rows[4][1], sqlrustgo::Value::Integer(10));
}

#[test]
fn test_execute_nulls_last_desc() {
    // DESC NULLS LAST: NULLs sort after non-NULLs, overriding the DESC default.
    let mut engine = ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())));
    engine
        .execute("CREATE TABLE t (id INTEGER, v INTEGER)")
        .unwrap();
    engine
        .execute("INSERT INTO t VALUES (1, 10), (2, NULL), (3, 30), (4, NULL), (5, 20)")
        .unwrap();

    let r = engine
        .execute("SELECT ROW_NUMBER() OVER (ORDER BY v DESC NULLS LAST) AS r, v FROM t ORDER BY r")
        .unwrap();
    assert_eq!(r.rows.len(), 5);
    // First three rows are non-NULLs in descending order: 30, 20, 10
    assert_eq!(r.rows[0][1], sqlrustgo::Value::Integer(30));
    assert_eq!(r.rows[1][1], sqlrustgo::Value::Integer(20));
    assert_eq!(r.rows[2][1], sqlrustgo::Value::Integer(10));
    // Last two rows are NULLs
    assert!(matches!(r.rows[3][1], sqlrustgo::Value::Null));
    assert!(matches!(r.rows[4][1], sqlrustgo::Value::Null));
}

// ============================================================================
// Basic Database Operation Tests
// ============================================================================

#[test]
fn test_basic_table_operations() {
    // Test basic CREATE and INSERT works
    let mut engine = ExecutionEngine::new(Arc::new(RwLock::new(MemoryStorage::new())));
    engine.execute("CREATE TABLE test (id INTEGER)").unwrap();
    engine
        .execute("INSERT INTO test VALUES (1), (2), (3)")
        .unwrap();

    let result = engine.execute("SELECT * FROM test").unwrap();
    assert_eq!(result.rows.len(), 3);
}
