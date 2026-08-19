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
    /// Binder / executor / storage error. Origin: sqlrustgo engine error.
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

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (code, msg) = match self {
            CliError::Parse(m) => ("ParseError", m.as_str()),
            CliError::Runtime(m) => ("RuntimeError", m.as_str()),
            CliError::Io(m) => ("IoError", m.as_str()),
            CliError::DotCmd(m) => ("DotCmdError", m.as_str()),
        };
        write!(f, "Error: {}: {}", code, msg)
    }
}

impl std::error::Error for CliError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_error_format_includes_code_prefix() {
        let e = CliError::Parse("unexpected token".into());
        assert_eq!(e.to_string(), "Error: ParseError: unexpected token");
        assert_eq!(e.exit_code(), 1);
    }

    #[test]
    fn runtime_error_format_includes_code_prefix() {
        let e = CliError::Runtime("column 'x' not found".into());
        assert_eq!(e.to_string(), "Error: RuntimeError: column 'x' not found");
        assert_eq!(e.exit_code(), 1);
    }

    #[test]
    fn io_error_format_includes_code_prefix() {
        let e = CliError::Io("file not found: missing.sql".into());
        assert_eq!(e.to_string(), "Error: IoError: file not found: missing.sql");
        assert_eq!(e.exit_code(), 1);
    }

    #[test]
    fn dotcmd_error_format_includes_code_prefix() {
        let e = CliError::DotCmd("unknown dot-command: .foo".into());
        assert_eq!(
            e.to_string(),
            "Error: DotCmdError: unknown dot-command: .foo"
        );
        assert_eq!(e.exit_code(), 1);
    }

    #[test]
    fn storage_init_exit_code_constant() {
        assert_eq!(EXIT_STORAGE_INIT, 3);
    }
}
