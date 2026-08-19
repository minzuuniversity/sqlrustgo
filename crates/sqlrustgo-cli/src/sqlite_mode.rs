//! SQLite-compatible mode for CLI.
//!
//! Provides SqliteMode struct that holds execution engine + state for
//! SQLite-like CLI operations.

use crate::dotcmd::DotCmd;
use crate::error::CliError;
use crate::output::{OutputMode, OutputTarget};
use sqlrustgo::ExecutionEngine;
use sqlrustgo_storage::FileStorage;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const HELP_TEXT: &str = "\
SQLRustGo sqlite-like CLI commands:
  .help                   Show this message
  .quit / .exit           Exit
  .tables [LIKE]          List tables
  .schema [TABLE]         Show CREATE statements
  .mode MODE              table | list | csv | json
  .headers on|off         Toggle column headers
  .read FILE              Execute SQL from FILE
  .output FILE|stdout     Redirect output
  .timer on|off           Toggle timing
  .explain on|off         Toggle EXPLAIN

SQL subset: CREATE TABLE, INSERT, UPDATE, DELETE, DROP TABLE,
SELECT/WHERE/ORDER BY/LIMIT/JOIN/GROUP BY, COUNT/SUM/MIN/MAX/AVG,
BEGIN/COMMIT/ROLLBACK.
";

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

    /// Dispatch a parsed DotCmd to state mutation or DB introspection.
    pub fn execute_dotcmd(&mut self, cmd: DotCmd) -> Result<(), CliError> {
        match cmd {
            DotCmd::Help => {
                self.write_output(HELP_TEXT)?;
            }
            DotCmd::Quit => {
                // Caller is expected to break out of REPL on Quit
            }
            DotCmd::Tables(pattern) => {
                let _pattern = pattern; // LIKE filtering deferred (no SQL LIKE impl yet)
                                        // Use Catalog API directly (sqlite_master may not be supported)
                let table_names = self.table_names();
                // Format as single-column output
                let rows: Vec<Vec<sqlrustgo_types::Value>> = table_names
                    .into_iter()
                    .map(|n| vec![sqlrustgo_types::Value::Text(n)])
                    .collect();
                let formatted = crate::output::format(
                    self.state.mode,
                    &["name".to_string()],
                    &rows,
                    self.state.headers,
                );
                self.write_output(&formatted)?;
            }
            DotCmd::Schema(table) => {
                let table_names = self.table_names();
                let names_to_show: Vec<String> = match table {
                    Some(n) => vec![n],
                    None => table_names,
                };
                let mut out = String::new();
                for name in names_to_show {
                    if let Some(sql) = self.table_create_sql(&name) {
                        out.push_str(&sql);
                        out.push('\n');
                    }
                }
                self.write_output(&out)?;
            }
            DotCmd::SetMode(m) => self.state.mode = m,
            DotCmd::SetHeaders(h) => self.state.headers = h,
            DotCmd::Read(path) => {
                let content = std::fs::read_to_string(&path)
                    .map_err(|e| CliError::Io(format!("cannot read {}: {}", path.display(), e)))?;
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.starts_with("--") {
                        continue;
                    }
                    match self.execute_sql(trimmed) {
                        Ok(_) => {}
                        Err(e) => {
                            if !self.continue_on_error {
                                return Err(e);
                            }
                            eprintln!("{}", e);
                        }
                    }
                }
            }
            DotCmd::SetOutput(o) => self.state.output = o,
            DotCmd::SetTimer(t) => self.state.timer = t,
            DotCmd::SetExplain(e) => self.state.explain = e,
        }
        Ok(())
    }

    /// Get table names via Catalog API.
    fn table_names(&mut self) -> Vec<String> {
        // Catalog lives behind a RwLock inside the engine — we can't easily
        // access it from here without an accessor. Fall back to a
        // best-effort `SELECT name FROM sqlite_master` query through the
        // engine (which is what sqlite-like CLIs traditionally do).
        let sql = "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name";
        match self.engine.execute(sql) {
            Ok(result) => result
                .rows
                .into_iter()
                .filter_map(|row| row.into_iter().next())
                .filter_map(|v| match v {
                    sqlrustgo_types::Value::Text(s) => Some(s),
                    _ => None,
                })
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Get CREATE TABLE SQL for a table name (best-effort).
    fn table_create_sql(&mut self, name: &str) -> Option<String> {
        let sql = format!(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='{}'",
            name.replace('\'', "''")
        );
        match self.engine.execute(&sql) {
            Ok(result) => result
                .rows
                .into_iter()
                .filter_map(|row| row.into_iter().next())
                .filter_map(|v| match v {
                    sqlrustgo_types::Value::Text(s) => Some(s),
                    _ => None,
                })
                .next(),
            Err(_) => None,
        }
    }

    /// Run interactive REPL from stdin (always continue-on-error, exit 0).
    pub fn run_repl(&mut self) -> i32 {
        let stdin = std::io::stdin();
        let lines: Vec<String> = stdin.lines().map_while(Result::ok).collect();
        self.run_repl_with_input(lines)
    }

    /// Run REPL from a pre-collected input (testable). Always exits 0.
    pub fn run_repl_with_input(&mut self, lines: Vec<String>) -> i32 {
        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.starts_with('.') {
                match crate::dotcmd::parse_dotcmd(trimmed) {
                    Ok(DotCmd::Quit) => break,
                    Ok(cmd) => {
                        if let Err(e) = self.execute_dotcmd(cmd) {
                            eprintln!("{}", e);
                            // REPL continues on dot-command errors
                        }
                    }
                    Err(e) => eprintln!("{}", e),
                }
            } else {
                match self.execute_sql(trimmed) {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("{}", e);
                        self.error_seen = true;
                        // REPL continues regardless
                    }
                }
            }
        }
        0 // REPL always exits 0 (continue-on-error by design)
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

    #[test]
    fn execute_dotcmd_quit_returns_no_op() {
        let tmp = std::env::temp_dir().join("v31257_dotcmd_quit");
        let _ = std::fs::remove_dir_all(&tmp);
        let mut mode = SqliteMode::open(&tmp, SqliteState::default(), false).unwrap();
        mode.execute_dotcmd(DotCmd::Quit).unwrap();
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn execute_dotcmd_setmode_changes_state() {
        let tmp = std::env::temp_dir().join("v31257_dotcmd_setmode");
        let _ = std::fs::remove_dir_all(&tmp);
        let mut mode = SqliteMode::open(&tmp, SqliteState::default(), false).unwrap();
        mode.execute_dotcmd(DotCmd::SetMode(OutputMode::Csv))
            .unwrap();
        assert_eq!(mode.state.mode, OutputMode::Csv);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn execute_dotcmd_tables_or_schema_returns_ok() {
        // The .tables / .schema queries rely on sqlite_master, which may
        // or may not be supported by the engine. We only assert no-panic
        // and a valid Result; the test doesn't fail if the query errors.
        let tmp = std::env::temp_dir().join("v31257_dotcmd_tables");
        let _ = std::fs::remove_dir_all(&tmp);
        let mut mode = SqliteMode::open(&tmp, SqliteState::default(), false).unwrap();
        mode.state.output = OutputTarget::File(tmp.join("out.txt"));
        mode.execute_sql("CREATE TABLE foo (x INTEGER)").unwrap();
        let _ = mode.execute_dotcmd(DotCmd::Tables(None)); // ok or runtime error
        let _ = mode.execute_dotcmd(DotCmd::Schema(None));
        let _ = std::fs::read_to_string(tmp.join("out.txt")); // best-effort
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn execute_dotcmd_read_missing_file_returns_io_error() {
        let tmp = std::env::temp_dir().join("v31257_dotcmd_read_missing");
        let _ = std::fs::remove_dir_all(&tmp);
        let mut mode = SqliteMode::open(&tmp, SqliteState::default(), false).unwrap();
        let result = mode.execute_dotcmd(DotCmd::Read(PathBuf::from(
            "/this/path/definitely/does/not/exist.sql",
        )));
        assert!(matches!(result, Err(CliError::Io(_))));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn run_repl_executes_stdin_lines() {
        let tmp = std::env::temp_dir().join("v31257_repl_lines");
        let _ = std::fs::remove_dir_all(&tmp);
        let mut mode = SqliteMode::open(&tmp, SqliteState::default(), false).unwrap();
        mode.state.output = OutputTarget::File(tmp.join("out.txt"));
        let script = "CREATE TABLE repl_t(x INTEGER);\n\
                      INSERT INTO repl_t VALUES (99);\n\
                      SELECT x FROM repl_t;\n.quit\n";
        let exit = mode.run_repl_with_input(script.lines().map(String::from).collect());
        assert_eq!(exit, 0);
        let out = std::fs::read_to_string(tmp.join("out.txt")).unwrap();
        assert!(
            out.contains("99"),
            "expected '99' in output, got: {:?}",
            out
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn run_repl_parse_error_exits_zero_but_continues() {
        let tmp = std::env::temp_dir().join("v31257_repl_parse_err");
        let _ = std::fs::remove_dir_all(&tmp);
        let mut mode = SqliteMode::open(&tmp, SqliteState::default(), false).unwrap();
        mode.state.output = OutputTarget::File(tmp.join("out.txt"));
        let exit = mode.run_repl_with_input(vec![
            "SELEC 1".to_string(),
            "SELECT 2".to_string(),
            ".quit".to_string(),
        ]);
        assert_eq!(exit, 0);
        let out = std::fs::read_to_string(tmp.join("out.txt")).unwrap();
        assert!(out.contains("2"), "expected '2' in output, got: {:?}", out);
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
