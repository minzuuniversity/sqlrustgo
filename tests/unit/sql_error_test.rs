//! SQL Error Types Tests
//!
//! Tests for sqlrustgo_types::SqlError and SqlState types

use sqlrustgo_types::error::{SqlError, SqlState};

#[test]
fn test_sql_state_codes() {
    assert_eq!(SqlState::SyntaxError.code(), "42000");
    assert_eq!(SqlState::NoSuchTable.code(), "42S02");
    assert_eq!(SqlState::NoSuchColumn.code(), "42S22");
    assert_eq!(SqlState::IntegrityConstraintViolation.code(), "23000");
    assert_eq!(SqlState::DivisionByZero.code(), "22012");
    assert_eq!(SqlState::NumericValueOutOfRange.code(), "22003");
    assert_eq!(SqlState::DataException.code(), "22005");
    assert_eq!(SqlState::Warning.code(), "01000");
    assert_eq!(SqlState::Success.code(), "00000");
    assert_eq!(SqlState::Unknown.code(), "HY000");
}

#[test]
fn test_sql_state_default() {
    let state = SqlState::default();
    assert_eq!(state, SqlState::Unknown);
}

#[test]
fn test_sql_error_parse() {
    let error = SqlError::ParseError("syntax error".to_string());
    assert!(error.to_string().contains("syntax error"));
}

#[test]
fn test_sql_error_execution() {
    let error = SqlError::ExecutionError("execution failed".to_string());
    assert!(error.to_string().contains("execution failed"));
}

#[test]
fn test_sql_error_type_mismatch() {
    let error = SqlError::TypeMismatch("type mismatch".to_string());
    assert!(error.to_string().contains("type mismatch"));
}

#[test]
fn test_sql_error_division_by_zero() {
    let error = SqlError::DivisionByZero;
    assert!(error.to_string().contains("Division by zero"));
}

#[test]
fn test_sql_error_null_value() {
    let error = SqlError::NullValueError("null value".to_string());
    assert!(error.to_string().contains("null value"));
}

#[test]
fn test_sql_error_constraint_violation() {
    let error = SqlError::ConstraintViolation("constraint violation".to_string());
    assert!(error.to_string().contains("constraint violation"));
}

#[test]
fn test_sql_error_table_not_found() {
    let error = SqlError::TableNotFound {
        table: "users".to_string(),
    };
    assert!(error.to_string().contains("users"));
}

#[test]
fn test_sql_error_column_not_found() {
    let error = SqlError::ColumnNotFound {
        column: "id".to_string(),
        location: "SELECT".to_string(),
    };
    assert!(error.to_string().contains("id"));
    assert!(error.to_string().contains("SELECT"));
}

#[test]
fn test_sql_error_duplicate_key() {
    let error = SqlError::DuplicateKey {
        value: "123".to_string(),
        key: "PRIMARY".to_string(),
    };
    assert!(error.to_string().contains("123"));
    assert!(error.to_string().contains("PRIMARY"));
}

#[test]
fn test_sql_error_io() {
    let error = SqlError::IoError("io error".to_string());
    assert!(error.to_string().contains("io error"));
}

#[test]
fn test_sql_error_protocol() {
    let error = SqlError::ProtocolError("protocol error".to_string());
    assert!(error.to_string().contains("protocol error"));
}

#[test]
fn test_sql_error_sqlstate() {
    assert_eq!(
        SqlError::ParseError("x".into()).sqlstate(),
        SqlState::SyntaxError
    );
    assert_eq!(
        SqlError::ExecutionError("x".into()).sqlstate(),
        SqlState::Unknown
    );
    assert_eq!(
        SqlError::TypeMismatch("x".into()).sqlstate(),
        SqlState::DataException
    );
    assert_eq!(
        SqlError::DivisionByZero.sqlstate(),
        SqlState::DivisionByZero
    );
    assert_eq!(
        SqlError::NullValueError("x".into()).sqlstate(),
        SqlState::DataException
    );
    assert_eq!(
        SqlError::ConstraintViolation("x".into()).sqlstate(),
        SqlState::IntegrityConstraintViolation
    );
    assert_eq!(
        SqlError::TableNotFound { table: "x".into() }.sqlstate(),
        SqlState::NoSuchTable
    );
    assert_eq!(
        SqlError::ColumnNotFound {
            column: "x".into(),
            location: "y".into()
        }
        .sqlstate(),
        SqlState::NoSuchColumn
    );
    assert_eq!(
        SqlError::DuplicateKey {
            value: "x".into(),
            key: "y".into()
        }
        .sqlstate(),
        SqlState::IntegrityConstraintViolation
    );
}

#[test]
fn test_sql_error_error_number() {
    assert_eq!(SqlError::ParseError("x".into()).error_number(), 1064);
    assert_eq!(SqlError::ExecutionError("x".into()).error_number(), 1105);
    assert_eq!(SqlError::TypeMismatch("x".into()).error_number(), 1110);
    assert_eq!(SqlError::DivisionByZero.error_number(), 1365);
    assert_eq!(SqlError::NullValueError("x".into()).error_number(), 1048);
    assert_eq!(
        SqlError::ConstraintViolation("x".into()).error_number(),
        1216
    );
    assert_eq!(
        SqlError::TableNotFound { table: "x".into() }.error_number(),
        1146
    );
    assert_eq!(
        SqlError::ColumnNotFound {
            column: "x".into(),
            location: "y".into()
        }
        .error_number(),
        1054
    );
    assert_eq!(
        SqlError::DuplicateKey {
            value: "x".into(),
            key: "y".into()
        }
        .error_number(),
        1062
    );
}

#[test]
fn test_sql_error_mysql_format() {
    let error = SqlError::ParseError("syntax error".to_string());
    let mysql_msg = error.to_mysql_format();
    assert!(mysql_msg.contains("ERROR"));
    assert!(mysql_msg.contains("42000"));
    assert!(mysql_msg.contains("syntax error"));
}

#[test]
fn test_sql_error_from_string() {
    let error: SqlError = "test error".into();
    assert!(error.to_string().contains("test error"));
}

#[test]
fn test_sql_error_from_io_error() {
    use std::io;
    let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
    let error: SqlError = io_err.into();
    assert!(error.to_string().contains("file not found"));
}
