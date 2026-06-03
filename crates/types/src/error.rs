//! SQL Error types
//! Error handling for SQLRustGo database system

/// SQL Error enum representing all possible error types
#[derive(thiserror::Error, Debug)]
pub enum SqlError {
    /// Syntax error during parsing
    #[error("Parse error: {0}")]
    ParseError(String),

    /// Execution error during query processing
    #[error("Execution error: {0}")]
    ExecutionError(String),

    /// Type mismatch error
    #[error("Type mismatch: {0}")]
    TypeMismatch(String),

    /// Division by zero
    #[error("Division by zero")]
    DivisionByZero,

    /// Null value error (operation on NULL)
    #[error("Null value error: {0}")]
    NullValueError(String),

    /// Constraint violation
    #[error("Constraint violation: {0}")]
    ConstraintViolation(String),

    /// Table not found
    #[error("Table not found: {0}")]
    TableNotFound(String),

    /// Column not found
    #[error("Column not found: {0}")]
    ColumnNotFound(String),

    /// Duplicate key error
    #[error("Duplicate key: {0}")]
    DuplicateKey(String),

    /// I/O error
    #[error("I/O error: {0}")]
    IoError(String),

    /// Network protocol error
    #[error("Protocol error: {0}")]
    ProtocolError(String),

    /// Timeout error
    #[error("Timeout: {0}")]
    TimeoutError(String),

    /// Overflow error (numeric overflow)
    #[error("Overflow: {0}")]
    OverflowError(String),

    /// Authentication error
    #[error("Authentication failed: {0}")]
    AuthError(String),
}

impl SqlError {
    /// MySQL 5.7 error code for this error.
    /// Returns 0 (no specific code) for variants without a defined mapping.
    pub fn mysql_error_code(&self) -> u16 {
        match self {
            SqlError::ParseError(_) => 1064,        // ER_PARSE_ERROR
            SqlError::ExecutionError(_) => 1105,     // ER_UNKNOWN_ERROR (generic)
            SqlError::TypeMismatch(_) => 1264,        // ER_WARN_DATA_OUT_OF_RANGE (closest)
            SqlError::DivisionByZero => 1365,        // ER_DIVISION_BY_ZERO
            SqlError::NullValueError(_) => 1048,      // ER_BAD_NULL_ERROR
            SqlError::ConstraintViolation(_) => 3819, // ER_CHECK_CONSTRAINT_VIOLATED
            SqlError::TableNotFound(_) => 1146,       // ER_NO_SUCH_TABLE
            SqlError::ColumnNotFound(_) => 1054,      // ER_BAD_FIELD_ERROR
            SqlError::DuplicateKey(_) => 1062,        // ER_DUP_ENTRY
            SqlError::IoError(_) => 1105,             // ER_UNKNOWN_ERROR
            SqlError::ProtocolError(_) => 1105,       // ER_UNKNOWN_ERROR
            SqlError::TimeoutError(_) => 1205,        // ER_LOCK_WAIT_TIMEOUT
            SqlError::OverflowError(_) => 1366,       // ER_DATA_TOO_LONG
            SqlError::AuthError(_) => 1045,           // ER_ACCESS_DENIED_ERROR
        }
    }

    /// SQLSTATE (5-char) for this error.
    /// See https://dev.mysql.com/doc/mysql-errors/8.0/en/server-error-reference.html
    pub fn sqlstate(&self) -> &'static str {
        match self {
            SqlError::ParseError(_) => "42000",       // syntax error or access rule violation
            SqlError::ExecutionError(_) => "HY000",   // general error
            SqlError::TypeMismatch(_) => "HY000",
            SqlError::DivisionByZero => "22012",      // division by zero
            SqlError::NullValueError(_) => "23000",   // integrity constraint violation
            SqlError::ConstraintViolation(_) => "23000",
            SqlError::TableNotFound(_) => "42S02",    // base table or view not found
            SqlError::ColumnNotFound(_) => "42S22",   // column not found
            SqlError::DuplicateKey(_) => "23000",
            SqlError::IoError(_) => "HY000",
            SqlError::ProtocolError(_) => "08S01",    // communication link failure
            SqlError::TimeoutError(_) => "HY000",
            SqlError::OverflowError(_) => "22001",    // string data, right truncation
            SqlError::AuthError(_) => "28000",        // invalid authorization specification
        }
    }
}

/// Result type alias for SQL operations
pub type SqlResult<T> = Result<T, SqlError>;

impl From<String> for SqlError {
    fn from(s: String) -> Self {
        SqlError::ExecutionError(s)
    }
}

impl From<&str> for SqlError {
    fn from(s: &str) -> Self {
        SqlError::ExecutionError(s.to_string())
    }
}

impl From<std::io::Error> for SqlError {
    fn from(e: std::io::Error) -> Self {
        SqlError::IoError(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_messages() {
        let err = SqlError::ParseError("invalid syntax".to_string());
        assert!(err.to_string().contains("Parse error"));

        let err = SqlError::TableNotFound("users".to_string());
        assert!(err.to_string().contains("Table not found"));
    }

    #[test]
    fn test_all_error_types() {
        // Test all error variants
        let errors = vec![
            SqlError::ParseError("test".to_string()),
            SqlError::ExecutionError("test".to_string()),
            SqlError::TypeMismatch("test".to_string()),
            SqlError::DivisionByZero,
            SqlError::NullValueError("test".to_string()),
            SqlError::ConstraintViolation("test".to_string()),
            SqlError::TableNotFound("test".to_string()),
            SqlError::ColumnNotFound("test".to_string()),
            SqlError::DuplicateKey("test".to_string()),
            SqlError::IoError("test".to_string()),
            SqlError::ProtocolError("test".to_string()),
            SqlError::TimeoutError("test".to_string()),
            SqlError::OverflowError("test".to_string()),
            SqlError::AuthError("test".to_string()),
        ];

        for err in errors {
            let msg = err.to_string();
            assert!(!msg.is_empty());
        }
    }

    #[test]
    fn test_error_from_string() {
        // Test From<String>
        let err: SqlError = String::from("test error").into();
        assert!(matches!(err, SqlError::ExecutionError(_)));

        // Test From<&str>
        let err: SqlError = "test error".into();
        assert!(matches!(err, SqlError::ExecutionError(_)));
    }

    #[test]
    fn test_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: SqlError = io_err.into();
        assert!(matches!(err, SqlError::IoError(_)));
        assert!(err.to_string().contains("file not found"));
    }

    #[test]
    fn test_sql_result_alias() {
        let ok_result: SqlResult<i32> = Ok(42);
        assert_eq!(ok_result.ok(), Some(42));

        let err_result: SqlResult<i32> = Err(SqlError::TableNotFound("test".to_string()));
        assert!(err_result.is_err());
    }

    #[test]
    fn test_error_debug() {
        let err = SqlError::ParseError("debug test".to_string());
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("ParseError"));
    }

    #[test]
    fn test_error_display_parse() {
        let err = SqlError::ParseError("syntax error".to_string());
        let display = format!("{}", err);
        assert!(display.contains("syntax error"));
    }

    #[test]
    fn test_error_display_execution() {
        let err = SqlError::ExecutionError("query failed".to_string());
        let display = format!("{}", err);
        assert!(display.contains("query failed"));
    }

    #[test]
    fn test_error_display_type_mismatch() {
        let err = SqlError::TypeMismatch("expected int".to_string());
        let display = format!("{}", err);
        assert!(display.contains("expected int"));
    }

    #[test]
    fn test_error_display_division_by_zero() {
        let err = SqlError::DivisionByZero;
        let display = format!("{}", err);
        assert!(display.contains("Division by zero"));
    }

    #[test]
    fn test_error_display_null_value() {
        let err = SqlError::NullValueError("cannot be null".to_string());
        let display = format!("{}", err);
        assert!(display.contains("cannot be null"));
    }

    #[test]
    fn test_error_display_constraint() {
        let err = SqlError::ConstraintViolation("unique key".to_string());
        let display = format!("{}", err);
        assert!(display.contains("unique key"));
    }

    #[test]
    fn test_error_display_table_not_found() {
        let err = SqlError::TableNotFound("users".to_string());
        let display = format!("{}", err);
        assert!(display.contains("users"));
    }

    #[test]
    fn test_error_display_column_not_found() {
        let err = SqlError::ColumnNotFound("id".to_string());
        let display = format!("{}", err);
        assert!(display.contains("id"));
    }

    #[test]
    fn test_error_display_duplicate_key() {
        let err = SqlError::DuplicateKey("email".to_string());
        let display = format!("{}", err);
        assert!(display.contains("email"));
    }

    #[test]
    fn test_error_display_io() {
        let err = SqlError::IoError("file not found".to_string());
        let display = format!("{}", err);
        assert!(display.contains("file not found"));
    }

    #[test]
    fn test_error_display_protocol() {
        let err = SqlError::ProtocolError("invalid packet".to_string());
        let display = format!("{}", err);
        assert!(display.contains("invalid packet"));
    }

    #[test]
    fn test_error_display_timeout() {
        let err = SqlError::TimeoutError("connection timeout".to_string());
        let display = format!("{}", err);
        assert!(display.contains("Timeout"));
    }

    #[test]
    fn test_error_display_overflow() {
        let err = SqlError::OverflowError("i32 overflow".to_string());
        let display = format!("{}", err);
        assert!(display.contains("Overflow"));
    }

    #[test]
    fn test_error_display_auth() {
        let err = SqlError::AuthError("invalid credentials".to_string());
        let display = format!("{}", err);
        assert!(display.contains("Authentication"));
    }

    #[test]
    fn test_mysql_error_codes() {
        assert_eq!(SqlError::ParseError("x".into()).mysql_error_code(), 1064);
        assert_eq!(SqlError::TableNotFound("t".into()).mysql_error_code(), 1146);
        assert_eq!(SqlError::ColumnNotFound("c".into()).mysql_error_code(), 1054);
        assert_eq!(SqlError::DuplicateKey("k".into()).mysql_error_code(), 1062);
        assert_eq!(SqlError::DivisionByZero.mysql_error_code(), 1365);
        assert_eq!(SqlError::AuthError("u".into()).mysql_error_code(), 1045);
        assert_eq!(SqlError::NullValueError("c".into()).mysql_error_code(), 1048);
    }

    #[test]
    fn test_sqlstate_codes() {
        assert_eq!(SqlError::ParseError("x".into()).sqlstate(), "42000");
        assert_eq!(SqlError::TableNotFound("t".into()).sqlstate(), "42S02");
        assert_eq!(SqlError::ColumnNotFound("c".into()).sqlstate(), "42S22");
        assert_eq!(SqlError::DivisionByZero.sqlstate(), "22012");
        assert_eq!(SqlError::DuplicateKey("k".into()).sqlstate(), "23000");
        assert_eq!(SqlError::AuthError("u".into()).sqlstate(), "28000");
        assert_eq!(SqlError::ProtocolError("p".into()).sqlstate(), "08S01");
        assert_eq!(SqlError::OverflowError("o".into()).sqlstate(), "22001");
    }

    #[test]
    fn test_all_errors_have_codes() {
        // Every variant must return a non-zero code and a 5-char SQLSTATE
        let errors = vec![
            SqlError::ParseError("t".into()),
            SqlError::ExecutionError("t".into()),
            SqlError::TypeMismatch("t".into()),
            SqlError::DivisionByZero,
            SqlError::NullValueError("t".into()),
            SqlError::ConstraintViolation("t".into()),
            SqlError::TableNotFound("t".into()),
            SqlError::ColumnNotFound("t".into()),
            SqlError::DuplicateKey("t".into()),
            SqlError::IoError("t".into()),
            SqlError::ProtocolError("t".into()),
            SqlError::TimeoutError("t".into()),
            SqlError::OverflowError("t".into()),
            SqlError::AuthError("t".into()),
        ];
        for err in &errors {
            assert!(err.mysql_error_code() > 0, "{err:?} has zero code");
            assert_eq!(err.sqlstate().len(), 5, "{err:?} bad SQLSTATE");
        }
    }
}
