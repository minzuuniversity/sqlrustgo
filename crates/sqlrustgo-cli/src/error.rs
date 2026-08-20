//! Stable error prefixes + exit code policy for V312-57 sqlite-like CLI mode.
//!
//! All errors emitted by the CLI get a one-line `Error: <CODE>: <message>` format
//! on stderr. Codes never change between minor versions (per Anti-Fabrication-Policy
//! v1.0); messages after the code can change.

#![allow(dead_code)]

use std::fmt;

pub const EXIT_OK: i32 = 0;
pub const EXIT_QUERY_ERROR: i32 = 1;
pub const EXIT_STORAGE_INIT: i32 = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliError {
    /// SQL parser / lexer rejected input. Origin: sqlrustgo-parser error.
    Parse(String),
    /// Binder error: unknown table/column, type mismatch caught at bind time.
    Bind(String),
    /// Executor / storage error. Origin: sqlrustgo engine error.
    Runtime(String),
    /// File I/O failure (e.g. .read missing file, .output permission denied).
    Io(String),
    /// Dot-command parsing / dispatch error.
    DotCmd(String),
}

impl CliError {
    pub fn exit_code(&self) -> i32 {
        EXIT_QUERY_ERROR
    }
}

/// Stable error prefixes emitted on stderr. Codes never change between
/// minor versions (per Anti-Fabrication-Policy v1.0); messages after the
/// code can change.
impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (code, msg) = match self {
            CliError::Parse(m) => ("sqlrustgo:error:parse:", m.as_str()),
            CliError::Bind(m) => ("sqlrustgo:error:bind:", m.as_str()),
            CliError::Runtime(m) => ("sqlrustgo:error:runtime:", m.as_str()),
            CliError::Io(m) => ("sqlrustgo:error:io:", m.as_str()),
            CliError::DotCmd(m) => ("sqlrustgo:error:meta:", m.as_str()),
        };
        write!(f, "Error: {} {}", code, msg)
    }
}

impl std::error::Error for CliError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_error_format_includes_code_prefix() {
        let e = CliError::Parse("unexpected token".into());
        assert_eq!(
            e.to_string(),
            "Error: sqlrustgo:error:parse: unexpected token"
        );
        assert_eq!(e.exit_code(), 1);
    }

    #[test]
    fn bind_error_format_includes_code_prefix() {
        let e = CliError::Bind("column 'x' not found".into());
        assert_eq!(
            e.to_string(),
            "Error: sqlrustgo:error:bind: column 'x' not found"
        );
        assert_eq!(e.exit_code(), 1);
    }

    #[test]
    fn runtime_error_format_includes_code_prefix() {
        let e = CliError::Runtime("execution failed".into());
        assert_eq!(
            e.to_string(),
            "Error: sqlrustgo:error:runtime: execution failed"
        );
        assert_eq!(e.exit_code(), 1);
    }

    #[test]
    fn io_error_format_includes_code_prefix() {
        let e = CliError::Io("file not found: missing.sql".into());
        assert_eq!(
            e.to_string(),
            "Error: sqlrustgo:error:io: file not found: missing.sql"
        );
        assert_eq!(e.exit_code(), 1);
    }

    #[test]
    fn dotcmd_error_format_includes_code_prefix() {
        let e = CliError::DotCmd("unknown dot-command: .foo".into());
        assert_eq!(
            e.to_string(),
            "Error: sqlrustgo:error:meta: unknown dot-command: .foo"
        );
        assert_eq!(e.exit_code(), 1);
    }

    #[test]
    fn storage_init_exit_code_constant() {
        assert_eq!(EXIT_STORAGE_INIT, 3);
    }
}
