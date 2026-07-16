// mysql-server unit tests for public functions

use sqlrustgo_mysql_server::{
    parse_stmt_execute_params, replace_placeholders, MySqlError, StmtParam,
};

// ============ replace_placeholders tests ============

#[test]
fn test_replace_placeholders_no_params() {
    let sql = "SELECT * FROM t";
    let result = replace_placeholders(sql, &[]);
    assert_eq!(result, "SELECT * FROM t");
}

#[test]
fn test_replace_placeholders_single_string() {
    let sql = "SELECT * FROM t WHERE name = ?";
    let params: &[StmtParam] = &[(b"Alice".to_vec(), false)];
    let result = replace_placeholders(sql, params);
    assert_eq!(result, "SELECT * FROM t WHERE name = 'Alice'");
}

#[test]
fn test_replace_placeholders_single_numeric() {
    let sql = "SELECT * FROM t WHERE age = ?";
    let params: &[StmtParam] = &[("42".as_bytes().to_vec(), true)];
    let result = replace_placeholders(sql, params);
    assert_eq!(result, "SELECT * FROM t WHERE age = 42");
}

#[test]
fn test_replace_placeholders_multiple() {
    let sql = "INSERT INTO t (a, b, c) VALUES (?, ?, ?)";
    let params: &[StmtParam] = &[
        ("1".as_bytes().to_vec(), true),
        (b"hello".to_vec(), false),
        ("100".as_bytes().to_vec(), true),
    ];
    let result = replace_placeholders(sql, params);
    assert_eq!(result, "INSERT INTO t (a, b, c) VALUES (1, 'hello', 100)");
}

#[test]
fn test_replace_placeholders_null() {
    let sql = "SELECT * FROM t WHERE col = ?";
    let params: &[StmtParam] = &[(vec![], false)];
    let result = replace_placeholders(sql, params);
    assert_eq!(result, "SELECT * FROM t WHERE col = NULL");
}

#[test]
fn test_replace_placeholders_string_with_single_quote() {
    let sql = "SELECT * FROM t WHERE name = ?";
    let params: &[StmtParam] = &[("O'Reilly".as_bytes().to_vec(), false)];
    let result = replace_placeholders(sql, params);
    assert_eq!(result, "SELECT * FROM t WHERE name = 'O''Reilly'");
}

#[test]
fn test_replace_placeholders_string_with_backslash() {
    let sql = "SELECT * FROM t WHERE path = ?";
    let params: &[StmtParam] = &[("a\\b".as_bytes().to_vec(), false)];
    let result = replace_placeholders(sql, params);
    // Backslash should be escaped
    assert!(result.contains("a"));
}

#[test]
fn test_replace_placeholders_empty_string() {
    // Empty bytes becomes NULL per replace_placeholders contract
    let sql = "SELECT * FROM t WHERE name = ?";
    let params: &[StmtParam] = &[("".as_bytes().to_vec(), false)];
    let result = replace_placeholders(sql, params);
    assert_eq!(result, "SELECT * FROM t WHERE name = NULL");
}

#[test]
fn test_replace_placeholders_extra_params_ignored() {
    let sql = "SELECT * FROM t WHERE id = 1";
    let params: &[StmtParam] = &[(b"x".to_vec(), false)];
    let result = replace_placeholders(sql, params);
    assert_eq!(result, "SELECT * FROM t WHERE id = 1");
}

#[test]
fn test_replace_placeholders_fewer_params() {
    let sql = "INSERT INTO t (a, b) VALUES (?, ?)";
    let params: &[StmtParam] = &[(b"x".to_vec(), false)];
    let result = replace_placeholders(sql, params);
    // Only first placeholder replaced
    assert!(result.contains("'x'"));
    assert!(result.contains("?"));
}

// ============ parse_stmt_execute_params tests ============

#[test]
fn test_parse_stmt_execute_params_empty_payload() {
    let params = parse_stmt_execute_params(&[], 0, &[]);
    assert!(params.is_empty());
}

#[test]
fn test_parse_stmt_execute_params_null_param() {
    // Build: stmt_id(4) + flags(1) + iter_count(4) + null_bitmap(1, 0x01=null) + new_params_bound(1, 0x01) + type(VAR_STRING=15)
    let payload: Vec<u8> = vec![
        0, 0, 0, 0, // stmt_id = 0
        0, // flags
        1, 0, 0, 0,    // iteration_count = 1
        0x01, // null_bitmap: param 0 is null
        0x01, // new_params_bound_flag = 1
        15, 0, // VAR_STRING type
    ];
    let params = parse_stmt_execute_params(&payload, 1, &[]);
    assert_eq!(params.len(), 1);
    assert_eq!(params[0].0, Vec::<u8>::new()); // NULL = empty Vec
}

#[test]
fn test_parse_stmt_execute_params_string_value() {
    let mut payload: Vec<u8> = vec![
        0, 0, 0, 0, // stmt_id
        0, // flags
        1, 0, 0, 0,    // iteration_count
        0x00, // null_bitmap: not null
        0x01, // new_params_bound_flag = 1
        15, 0, // VAR_STRING type
    ];
    // String value: length-encoded
    payload.push(5); // length
    payload.extend_from_slice(b"hello");
    let params = parse_stmt_execute_params(&payload, 1, &[]);
    assert_eq!(params.len(), 1);
    assert_eq!(params[0].0, b"hello".to_vec());
    assert!(!params[0].1); // not numeric
}

// ============ MySqlError tests ============

#[test]
fn test_mysql_error_io_display() {
    let err = MySqlError::Io(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "file not found",
    ));
    let display = format!("{}", err);
    assert!(display.contains("IO") && display.contains("file not found"));
}

#[test]
fn test_mysql_error_protocol_display() {
    let err = MySqlError::Protocol("bad packet".to_string());
    let display = format!("{}", err);
    assert!(display.contains("Protocol") && display.contains("bad packet"));
}

#[test]
fn test_mysql_error_sql_display() {
    let err = MySqlError::Sql("syntax error".to_string());
    let display = format!("{}", err);
    assert!(display.contains("SQL") && display.contains("syntax error"));
}

#[test]
fn test_mysql_error_other_display() {
    let err = MySqlError::Other("unknown error".to_string());
    let display = format!("{}", err);
    // Other variant shows just the message
    assert!(display.contains("unknown error"));
}

#[test]
fn test_mysql_error_from_io() {
    let io_err = std::io::Error::new(std::io::ErrorKind::Other, "test io");
    let err: MySqlError = io_err.into();
    let display = format!("{}", err);
    assert!(display.contains("test io"));
}

#[test]
fn test_mysql_error_from_string() {
    let err: MySqlError = "direct string".to_string().into();
    let display = format!("{}", err);
    assert!(display.contains("direct string"));
}

#[test]
fn test_mysql_error_from_str() {
    let err: MySqlError = "direct &str".into();
    let display = format!("{}", err);
    assert!(display.contains("direct"));
}
#[test]
fn test_mysql_error_debug() {
    let err = MySqlError::Protocol("test".to_string());
    let debug = format!("{:?}", err);
    assert!(debug.contains("Protocol"));
}

#[test]
fn test_mysql_error_all_variants_display() {
    // Io variant
    let io_err = MySqlError::Io(std::io::Error::new(std::io::ErrorKind::Other, "disk full"));
    assert!(format!("{}", io_err).contains("disk full"));
    // Protocol variant
    let proto_err = MySqlError::Protocol("bad packet".to_string());
    assert!(format!("{}", proto_err).contains("Protocol"));
    assert!(format!("{}", proto_err).contains("bad packet"));
    // Sql variant
    let sql_err = MySqlError::Sql("syntax error".to_string());
    assert!(format!("{}", sql_err).contains("SQL"));
    assert!(format!("{}", sql_err).contains("syntax error"));
    // Other variant
    let other_err = MySqlError::Other("custom error".to_string());
    assert!(format!("{}", other_err).contains("custom error"));
}

#[test]
fn test_mysql_error_source() {
    // Io variant: std::io::Error doesn't expose a source chain in its Error impl
    let io_err = MySqlError::Io(std::io::Error::new(std::io::ErrorKind::Other, "inner error"));
    // Io variant's source() returns None (io::Error is flat, no inner cause)
    assert!(std::error::Error::source(&io_err).is_none());
    // Protocol variant has no source
    let proto_err = MySqlError::Protocol("msg".to_string());
    assert!(std::error::Error::source(&proto_err).is_none());
    // Sql variant has no source
    let sql_err = MySqlError::Sql("msg".to_string());
    assert!(std::error::Error::source(&sql_err).is_none());
    // Other variant has no source
    let other_err = MySqlError::Other("msg".to_string());
    assert!(std::error::Error::source(&other_err).is_none());
}


// Atomic counters tests remain below

// ============ Atomic counters ============

#[test]
fn test_active_connections_atomic_init() {
    use sqlrustgo_mysql_server::ACTIVE_CONNECTIONS;
    use std::sync::atomic::Ordering;
    assert_eq!(ACTIVE_CONNECTIONS.load(Ordering::SeqCst), 0);
}

#[test]
fn test_total_connections_atomic_init() {
    use sqlrustgo_mysql_server::TOTAL_CONNECTIONS_ACCEPTED;
    use std::sync::atomic::Ordering;
    assert_eq!(TOTAL_CONNECTIONS_ACCEPTED.load(Ordering::SeqCst), 0);
}

#[test]
fn test_total_queries_atomic_init() {
    use sqlrustgo_mysql_server::TOTAL_QUERIES_SERVED;
    use std::sync::atomic::Ordering;
    assert_eq!(TOTAL_QUERIES_SERVED.load(Ordering::SeqCst), 0);
}

#[test]
fn test_total_errors_atomic_init() {
    use sqlrustgo_mysql_server::TOTAL_QUERY_ERRORS;
    use std::sync::atomic::Ordering;
    assert_eq!(TOTAL_QUERY_ERRORS.load(Ordering::SeqCst), 0);
}
