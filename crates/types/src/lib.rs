//! Core Type System Module
//!
//! This module defines the fundamental types and errors for SQLRustGo.
//! All SQL data types and error handling are centralized here.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::hash::{Hash, Hasher};

// Re-export
pub use error::{SqlError, SqlResult};
pub use value::Value;

// Error module
mod error {
    use thiserror::Error;

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
}

// Value module
mod value {
    use serde::{Deserialize, Serialize};
    use std::fmt;
    use std::hash::{Hash, Hasher};

    /// SQL Value enum representing all supported SQL data types
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum Value {
        /// NULL value
        Null,
        /// Boolean (TRUE/FALSE)
        Boolean(bool),
        /// 64-bit signed integer
        Integer(i64),
        /// 64-bit floating point
        Float(f64),
        /// Text string
        Text(String),
        /// Binary large object
        Blob(Vec<u8>),
    }

    impl Hash for Value {
        fn hash<H: Hasher>(&self, state: &mut H) {
            match self {
                Value::Null => 0.hash(state),
                Value::Boolean(b) => b.hash(state),
                Value::Integer(i) => i.hash(state),
                Value::Float(f) => {
                    if f.is_nan() {
                        0.hash(state);
                    } else {
                        f.to_bits().hash(state);
                    }
                }
                Value::Text(s) => s.hash(state),
                Value::Blob(b) => b.hash(state),
            }
        }
    }

    impl PartialEq for Value {
        fn eq(&self, other: &Self) -> bool {
            match (self, other) {
                (Value::Null, Value::Null) => true,
                (Value::Boolean(a), Value::Boolean(b)) => a == b,
                (Value::Integer(a), Value::Integer(b)) => a == b,
                (Value::Float(a), Value::Float(b)) => a == b,
                (Value::Text(a), Value::Text(b)) => a == b,
                (Value::Blob(a), Value::Blob(b)) => a == b,
                _ => false,
            }
        }
    }

    impl Eq for Value {}

    impl Value {
        /// Get integer value if this is an Integer
        pub fn as_integer(&self) -> Option<i64> {
            match self {
                Value::Integer(i) => Some(*i),
                _ => None,
            }
        }

        /// Convert Value to SQL string representation
        pub fn to_sql_string(&self) -> String {
            match self {
                Value::Null => "NULL".to_string(),
                Value::Boolean(b) => b.to_string(),
                Value::Integer(i) => i.to_string(),
                Value::Float(f) => f.to_string(),
                Value::Text(s) => s.clone(),
                Value::Blob(b) => format!("X'{}'", hex::encode(b)),
            }
        }

        /// Get the SQL type name
        pub fn type_name(&self) -> &'static str {
            match self {
                Value::Null => "NULL",
                Value::Boolean(_) => "BOOLEAN",
                Value::Integer(_) => "INTEGER",
                Value::Float(_) => "FLOAT",
                Value::Text(_) => "TEXT",
                Value::Blob(_) => "BLOB",
            }
        }

        /// Convert value to index key (i64)
        /// Used for B+Tree index key extraction
        pub fn to_index_key(&self) -> Option<i64> {
            match self {
                Value::Integer(i) => Some(*i),
                Value::Text(s) => {
                    use std::hash::{Hash, Hasher};
                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    s.hash(&mut hasher);
                    Some(hasher.finish() as i64)
                }
                _ => None,
            }
        }
    }

    impl fmt::Display for Value {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.to_sql_string())
        }
    }
}

/// Convert a SQL literal string to Value
/// Supports: NULL, TRUE, FALSE, numbers, strings
pub fn parse_sql_literal(s: &str) -> Value {
    let s = s.trim();

    match s.to_uppercase().as_str() {
        "NULL" => Value::Null,
        "TRUE" => Value::Boolean(true),
        "FALSE" => Value::Boolean(false),
        _ if s.starts_with('\'') && s.ends_with('\'') => Value::Text(s[1..s.len() - 1].to_string()),
        _ if s.parse::<i64>().is_ok() => Value::Integer(s.parse().unwrap()),
        _ if s.parse::<f64>().is_ok() => Value::Float(s.parse().unwrap()),
        _ => Value::Text(s.to_string()),
    }
}
