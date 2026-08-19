//! Dot-command parser for SQLite-like CLI mode.
//!
//! Parses commands like .help, .quit, .tables, .schema, .mode, .headers,
//! .read, .output, .timer, .explain.

#![allow(dead_code)]

use crate::error::CliError;
use crate::output::{OutputMode, OutputTarget};
use std::path::PathBuf;

/// DotCmdState holds the runtime state modified by dot-commands.
/// This struct will be moved to sqlite_mode.rs in Task 9.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DotCmdState {
    pub mode: OutputMode,
    pub headers: bool,
    pub timer: bool,
    pub explain: bool,
    pub output: OutputTarget,
}

impl Default for DotCmdState {
    fn default() -> Self {
        Self {
            mode: OutputMode::Table,
            headers: true,
            timer: false,
            explain: false,
            output: OutputTarget::Stdout,
        }
    }
}

/// Dot-command variants for SQLite-like CLI mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DotCmd {
    Help,
    Quit,
    Tables(Option<String>),
    Schema(Option<String>),
    SetMode(OutputMode),
    SetHeaders(bool),
    Read(PathBuf),
    SetOutput(OutputTarget),
    SetTimer(bool),
    SetExplain(bool),
}

pub fn parse_dotcmd(input: &str) -> Result<DotCmd, CliError> {
    let trimmed = input.trim();
    if !trimmed.starts_with('.') {
        return Err(CliError::DotCmd(format!("not a dot-command: {}", trimmed)));
    }
    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let cmd = parts
        .next()
        .unwrap()
        .trim_start_matches('.')
        .to_ascii_lowercase();
    let rest = parts.next().unwrap_or("").trim().to_string();

    match cmd.as_str() {
        "help" => Ok(DotCmd::Help),
        "quit" | "exit" => Ok(DotCmd::Quit),
        "tables" => Ok(DotCmd::Tables(if rest.is_empty() {
            None
        } else {
            Some(strip_quotes(&rest))
        })),
        "schema" => Ok(DotCmd::Schema(if rest.is_empty() {
            None
        } else {
            Some(rest)
        })),
        "mode" => match rest.to_ascii_lowercase().as_str() {
            "table" => Ok(DotCmd::SetMode(OutputMode::Table)),
            "list" => Ok(DotCmd::SetMode(OutputMode::List)),
            "csv" => Ok(DotCmd::SetMode(OutputMode::Csv)),
            "json" => Ok(DotCmd::SetMode(OutputMode::Json)),
            _ => Err(CliError::DotCmd(format!(
                "invalid mode '{}' (must be table|list|csv|json)",
                rest
            ))),
        },
        "headers" => match rest.to_ascii_lowercase().as_str() {
            "on" => Ok(DotCmd::SetHeaders(true)),
            "off" => Ok(DotCmd::SetHeaders(false)),
            _ => Err(CliError::DotCmd(format!(
                "invalid headers arg '{}' (must be on|off)",
                rest
            ))),
        },
        "read" => {
            if rest.is_empty() {
                Err(CliError::DotCmd(".read requires a file argument".into()))
            } else {
                Ok(DotCmd::Read(PathBuf::from(strip_quotes(&rest))))
            }
        }
        "output" => {
            if rest.is_empty() || rest == "stdout" {
                Ok(DotCmd::SetOutput(OutputTarget::Stdout))
            } else {
                Ok(DotCmd::SetOutput(OutputTarget::File(PathBuf::from(
                    strip_quotes(&rest),
                ))))
            }
        }
        "timer" => match rest.to_ascii_lowercase().as_str() {
            "on" => Ok(DotCmd::SetTimer(true)),
            "off" => Ok(DotCmd::SetTimer(false)),
            _ => Err(CliError::DotCmd(format!(
                "invalid timer arg '{}' (must be on|off)",
                rest
            ))),
        },
        "explain" => match rest.to_ascii_lowercase().as_str() {
            "on" => Ok(DotCmd::SetExplain(true)),
            "off" => Ok(DotCmd::SetExplain(false)),
            _ => Err(CliError::DotCmd(format!(
                "invalid explain arg '{}' (must be on|off)",
                rest
            ))),
        },
        _ => Err(CliError::DotCmd(format!(
            "unknown dot-command: .{} (try .help)",
            cmd
        ))),
    }
}

fn strip_quotes(s: &str) -> String {
    let s = s.trim();
    if (s.starts_with('\'') && s.ends_with('\'')) || (s.starts_with('"') && s.ends_with('"')) {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_quit() {
        assert_eq!(parse_dotcmd(".quit").unwrap(), DotCmd::Quit);
        assert_eq!(parse_dotcmd(".exit").unwrap(), DotCmd::Quit);
        assert_eq!(parse_dotcmd(".QUIT").unwrap(), DotCmd::Quit);
    }

    #[test]
    fn parse_help() {
        assert_eq!(parse_dotcmd(".help").unwrap(), DotCmd::Help);
    }

    #[test]
    fn parse_tables_no_arg() {
        assert_eq!(parse_dotcmd(".tables").unwrap(), DotCmd::Tables(None));
    }

    #[test]
    fn parse_tables_with_pattern() {
        assert_eq!(
            parse_dotcmd(".tables 't%'").unwrap(),
            DotCmd::Tables(Some("t%".into()))
        );
    }

    #[test]
    fn parse_schema_no_arg() {
        assert_eq!(parse_dotcmd(".schema").unwrap(), DotCmd::Schema(None));
    }

    #[test]
    fn parse_schema_with_table() {
        assert_eq!(
            parse_dotcmd(".schema users").unwrap(),
            DotCmd::Schema(Some("users".into()))
        );
    }

    #[test]
    fn parse_mode_table() {
        assert_eq!(
            parse_dotcmd(".mode table").unwrap(),
            DotCmd::SetMode(OutputMode::Table)
        );
    }

    #[test]
    fn parse_mode_csv_case_insensitive() {
        assert_eq!(
            parse_dotcmd(".mode CSV").unwrap(),
            DotCmd::SetMode(OutputMode::Csv)
        );
    }

    #[test]
    fn parse_mode_invalid() {
        assert!(parse_dotcmd(".mode xml").is_err());
    }

    #[test]
    fn parse_headers_on_off() {
        assert_eq!(
            parse_dotcmd(".headers on").unwrap(),
            DotCmd::SetHeaders(true)
        );
        assert_eq!(
            parse_dotcmd(".headers off").unwrap(),
            DotCmd::SetHeaders(false)
        );
    }

    #[test]
    fn parse_read_with_file() {
        assert_eq!(
            parse_dotcmd(".read script.sql").unwrap(),
            DotCmd::Read(PathBuf::from("script.sql"))
        );
    }

    #[test]
    fn parse_output_to_file() {
        assert_eq!(
            parse_dotcmd(".output out.txt").unwrap(),
            DotCmd::SetOutput(OutputTarget::File(PathBuf::from("out.txt")))
        );
    }

    #[test]
    fn parse_output_to_stdout() {
        assert_eq!(
            parse_dotcmd(".output stdout").unwrap(),
            DotCmd::SetOutput(OutputTarget::Stdout)
        );
    }

    #[test]
    fn parse_timer_on_off() {
        assert_eq!(parse_dotcmd(".timer on").unwrap(), DotCmd::SetTimer(true));
        assert_eq!(parse_dotcmd(".timer off").unwrap(), DotCmd::SetTimer(false));
    }

    #[test]
    fn parse_explain_on_off() {
        assert_eq!(
            parse_dotcmd(".explain on").unwrap(),
            DotCmd::SetExplain(true)
        );
        assert_eq!(
            parse_dotcmd(".explain off").unwrap(),
            DotCmd::SetExplain(false)
        );
    }

    #[test]
    fn parse_unknown_dotcmd_returns_error() {
        assert!(matches!(parse_dotcmd(".foo"), Err(CliError::DotCmd(_))));
    }
}
