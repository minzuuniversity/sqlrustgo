//! SQLite-compatible mode for CLI.
//!
//! Provides SqliteMode struct that holds execution engine + state for
//! SQLite-like CLI operations.

use crate::error::CliError;
use crate::output::{OutputMode, OutputTarget};
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::FileStorage;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SqliteState {
    pub mode: OutputMode,
    pub headers: bool,
    pub timer: bool,
    pub explain: bool,
    pub output: OutputTarget,
}

impl Default for SqliteState {
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

#[allow(dead_code)]
pub struct SqliteMode {
    pub engine: ExecutionEngine<FileStorage>,
    pub state: SqliteState,
    pub error_seen: bool,
    pub continue_on_error: bool,
    pub db_path: PathBuf,
}

#[allow(dead_code)]
impl SqliteMode {
    pub fn open(db: &Path, state: SqliteState, continue_on_error: bool) -> Result<Self, CliError> {
        // Resolve storage path: if db is a file path, use its parent + basename as subdir.
        // If db is a directory, use as-is.
        let dir = if db.is_dir() {
            db.to_path_buf()
        } else {
            let parent = db.parent().unwrap_or(Path::new("."));
            let name = db.file_name().unwrap_or_else(|| std::ffi::OsStr::new("db"));
            parent.join(name)
        };

        std::fs::create_dir_all(&dir).map_err(|e| {
            CliError::Io(format!(
                "cannot create DB directory {}: {}",
                dir.display(),
                e
            ))
        })?;

        let storage = Arc::new(parking_lot::RwLock::new(
            FileStorage::new(dir.clone()).map_err(|e| {
                CliError::Io(format!("cannot open storage {}: {}", dir.display(), e))
            })?,
        ));
        let engine = ExecutionEngine::with_catalog(
            storage,
            Arc::new(parking_lot::RwLock::new(sqlrustgo_catalog::Catalog::new(
                "main",
            ))),
        );

        Ok(Self {
            engine,
            state,
            error_seen: false,
            continue_on_error,
            db_path: dir,
        })
    }

    /// Execute a single SQL statement, format result with current state,
    /// write to output target. Sets `error_seen` on failure.
    ///
    /// Column names are extracted from the parsed AST (per ledger ruling:
    /// ExecutorResult does not carry columns).
    pub fn execute_sql(&mut self, sql: &str) -> Result<(), CliError> {
        use sqlrustgo_parser::{parse, Statement};

        // Parse first to extract column names (only for SELECT)
        let columns: Vec<String> = match parse(sql) {
            Ok(Statement::Select(sel)) => sel
                .columns
                .iter()
                .map(|c| c.alias.clone().unwrap_or_else(|| c.name.clone()))
                .collect(),
            Ok(_) => Vec::new(),
            Err(_) => Vec::new(), // will surface as CliError below
        };

        let result = self.engine.execute(sql);
        match result {
            Ok(exec_result) => {
                let rows: Vec<Vec<sqlrustgo_types::Value>> = exec_result.rows;
                let csv_header = self.state.headers;
                let formatted = crate::output::format(self.state.mode, &columns, &rows, csv_header);
                self.write_output(&formatted)?;
                Ok(())
            }
            Err(e) => {
                self.error_seen = true;
                let err_str = e.to_string();
                let cli_err = if err_str.to_lowercase().contains("parse") {
                    CliError::Parse(err_str)
                } else {
                    CliError::Runtime(err_str)
                };
                Err(cli_err)
            }
        }
    }

    fn write_output(&mut self, text: &str) -> Result<(), CliError> {
        use std::io::Write;
        match &self.state.output {
            OutputTarget::Stdout => {
                print!("{}", text);
                std::io::stdout()
                    .flush()
                    .map_err(|e| CliError::Io(e.to_string()))?;
            }
            OutputTarget::File(path) => {
                let mut f = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                    .map_err(|e| {
                        CliError::Io(format!("cannot write to {}: {}", path.display(), e))
                    })?;
                f.write_all(text.as_bytes())
                    .map_err(|e| CliError::Io(e.to_string()))?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_creates_new_db_path() {
        let tmp = std::env::temp_dir().join("v31257_open_creates");
        let _ = std::fs::remove_dir_all(&tmp);
        let mode = SqliteMode::open(&tmp, SqliteState::default(), false).expect("open");
        assert!(!mode.error_seen);
        assert!(!mode.continue_on_error);
        // Path should exist after open
        assert!(tmp.exists(), "open should create the DB path");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn open_existing_db_path_succeeds() {
        let tmp = std::env::temp_dir().join("v31257_open_existing");
        let _ = std::fs::remove_dir_all(&tmp);
        // First open creates
        let _ = SqliteMode::open(&tmp, SqliteState::default(), false).expect("open1");
        // Second open succeeds on existing
        let _ = SqliteMode::open(&tmp, SqliteState::default(), false).expect("open2");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn default_state_has_table_mode_headers_on() {
        let s = SqliteState::default();
        assert_eq!(s.mode, OutputMode::Table);
        assert!(s.headers);
        assert!(!s.timer);
        assert!(!s.explain);
        assert_eq!(s.output, OutputTarget::Stdout);
    }

    #[test]
    fn execute_select_one_returns_table_output() {
        let tmp = std::env::temp_dir().join("v31257_exec_select");
        let _ = std::fs::remove_dir_all(&tmp);
        let mut mode = SqliteMode::open(&tmp, SqliteState::default(), false).unwrap();
        mode.state.output = OutputTarget::File(tmp.join("out.txt"));
        mode.execute_sql("SELECT 1 AS x").expect("execute");
        let out = std::fs::read_to_string(tmp.join("out.txt")).unwrap();
        assert!(out.contains("x"), "expected 'x' in output, got: {:?}", out);
        assert!(out.contains("1"), "expected '1' in output, got: {:?}", out);
        assert!(!mode.error_seen);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn execute_create_table_then_select() {
        let tmp = std::env::temp_dir().join("v31257_exec_crud");
        let _ = std::fs::remove_dir_all(&tmp);
        let mut mode = SqliteMode::open(&tmp, SqliteState::default(), false).unwrap();
        mode.state.output = OutputTarget::File(tmp.join("out.txt"));
        mode.execute_sql("CREATE TABLE t(id INTEGER, name TEXT)")
            .unwrap();
        mode.execute_sql("INSERT INTO t VALUES (1, 'alice')")
            .unwrap();
        // Use explicit column names (SELECT * yields column-name "*" in our AST).
        mode.execute_sql("SELECT id, name FROM t").unwrap();
        let out = std::fs::read_to_string(tmp.join("out.txt")).unwrap();
        assert!(out.contains("1"), "expected '1' in output, got: {:?}", out);
        assert!(
            out.contains("alice"),
            "expected 'alice' in output, got: {:?}",
            out
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn execute_parse_error_sets_error_seen() {
        let tmp = std::env::temp_dir().join("v31257_exec_parse_err");
        let _ = std::fs::remove_dir_all(&tmp);
        let mut mode = SqliteMode::open(&tmp, SqliteState::default(), false).unwrap();
        mode.state.output = OutputTarget::File(tmp.join("err.txt"));
        let result = mode.execute_sql("SELEC 1");
        assert!(result.is_err());
        assert!(mode.error_seen);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn execute_runtime_error_sets_error_seen() {
        let tmp = std::env::temp_dir().join("v31257_exec_runtime_err");
        let _ = std::fs::remove_dir_all(&tmp);
        let mut mode = SqliteMode::open(&tmp, SqliteState::default(), false).unwrap();
        mode.state.output = OutputTarget::File(tmp.join("err.txt"));
        let result = mode.execute_sql("SELECT * FROM nonexistent");
        assert!(result.is_err());
        assert!(mode.error_seen);
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
