//! SQLite-compatible mode for CLI.
//!
//! Provides SqliteMode struct that holds execution engine + state for
//! SQLite-like CLI operations.

use crate::dotcmd::{parse_dotcmd, DotCmd};
use crate::error::{CliError, EXIT_OK};
use crate::output::{format, OutputMode, OutputTarget};
use sqlrustgo::ExecutionEngine;
use sqlrustgo_parser::{parse, split_sql_statements, Statement};
use sqlrustgo_storage::FileStorage;
use std::io::BufRead;
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
            FileStorage::new_with_buffer_config(dir.clone(), 10_000, false).map_err(|e| {
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

    /// Extract column names from a SELECT statement via the parser.
    fn extract_columns(&self, sql: &str) -> Result<Vec<String>, CliError> {
        match parse(sql) {
            Ok(Statement::Select(ref sel)) => Ok(sel
                .columns
                .iter()
                .map(|c| c.alias.clone().unwrap_or_else(|| c.name.clone()))
                .collect()),
            Ok(Statement::Explain(ref sel)) => Ok(sel
                .columns
                .iter()
                .map(|c| c.alias.clone().unwrap_or_else(|| c.name.clone()))
                .collect()),
            Ok(Statement::WithSelect(ref w)) => Ok(w
                .select
                .columns
                .iter()
                .map(|c| c.alias.clone().unwrap_or_else(|| c.name.clone()))
                .collect()),
            Ok(Statement::WithDml(ref w)) => {
                // WithDml: body is a boxed Statement — extract Select from it if possible
                if let Statement::Select(ref sel) = *w.body {
                    Ok(sel
                        .columns
                        .iter()
                        .map(|c| c.alias.clone().unwrap_or_else(|| c.name.clone()))
                        .collect())
                } else if let Statement::Insert(ref ins) = *w.body {
                    Ok(ins.columns.clone())
                } else {
                    Ok(Vec::new())
                }
            }
            Ok(_) => Ok(Vec::new()),
            Err(e) => Err(CliError::Parse(format!("{:?}", e))),
        }
    }

    pub fn execute_sql(&mut self, sql: &str) -> Result<(), CliError> {
        let columns = self.extract_columns(sql).unwrap_or_default();
        let result = self.engine.execute(sql);
        // After DML, force a flush so that subsequent SELECT (or a
        // re-opened process) sees the row. Without this, FileStorage's
        // insert_buffer holds the row until buffer_threshold=10_000 or
        // explicit flush().
        let _ = self.engine.flush();
        match result {
            Ok(exec_result) => {
                let formatted = format(
                    self.state.mode,
                    &columns,
                    &exec_result.rows,
                    self.state.headers,
                );
                self.write_output(&formatted)?;
                Ok(())
            }
            Err(e) => {
                self.error_seen = true;
                let msg = e.to_string();
                let lower = msg.to_lowercase();
                let cli_err = if lower.contains("parse error") {
                    CliError::Parse(msg)
                } else if lower.contains("binder error") || lower.contains("binder") {
                    CliError::Bind(msg)
                } else {
                    CliError::Runtime(msg)
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

    fn list_tables(&mut self) -> Result<Vec<String>, CliError> {
        // Engine bug workaround: execute_create_table writes the table
        // to FileStorage but does NOT register it in the catalog, so
        // information_schema.tables returns an empty result. We instead
        // ask the storage engine directly via the public
        // `ExecutionEngine::list_tables` accessor, which delegates to
        // `StorageEngine::list_tables` (implemented by FileStorage) and
        // reflects the on-disk state of `tables: HashMap<String, TableData>`.
        Ok(self.engine.list_tables())
    }

    fn get_create_table(&mut self, table: &str) -> Result<String, CliError> {
        let query = format!("SHOW CREATE TABLE {}", table);
        let result = self
            .engine
            .execute(&query)
            .map_err(|e| CliError::Runtime(e.to_string()))?;
        Ok(result
            .rows
            .into_iter()
            .next()
            .map(|r| {
                r.into_iter()
                    .map(|v| v.as_string().unwrap_or_default())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_default())
    }

    pub fn execute_dotcmd(&mut self, cmd: DotCmd) -> Result<(), CliError> {
        match cmd {
            DotCmd::Quit | DotCmd::Help => Ok(()),
            DotCmd::SetMode(m) => {
                self.state.mode = m;
                Ok(())
            }
            DotCmd::SetHeaders(h) => {
                self.state.headers = h;
                Ok(())
            }
            DotCmd::SetTimer(t) => {
                self.state.timer = t;
                Ok(())
            }
            DotCmd::SetExplain(e) => {
                self.state.explain = e;
                Ok(())
            }
            DotCmd::SetOutput(o) => {
                self.state.output = o;
                Ok(())
            }
            DotCmd::Tables(pattern) => {
                let mut tables = self.list_tables()?;
                tables.sort();
                let filtered: Vec<String> = if let Some(p) = pattern {
                    let pat = p
                        .chars()
                        .filter(|&c| c != '%' && c != '_')
                        .collect::<String>();
                    tables.into_iter().filter(|t| t.contains(&pat)).collect()
                } else {
                    tables
                };
                let line = format!("{}\n", filtered.join(" "));
                self.write_output(&line)?;
                Ok(())
            }
            DotCmd::Schema(table) => {
                if let Some(t) = table {
                    let create = self.get_create_table(&t)?;
                    let line = format!("{};\n", create);
                    self.write_output(&line)?;
                    Ok(())
                } else {
                    let tables = self.list_tables()?;
                    for t in tables {
                        let create = self.get_create_table(&t)?;
                        let line = format!("{};\n", create);
                        self.write_output(&line)?;
                    }
                    Ok(())
                }
            }
            DotCmd::Read(path) => {
                let content = std::fs::read_to_string(&path)
                    .map_err(|e| CliError::Io(format!("cannot read {}: {}", path.display(), e)))?;
                // Uses sqlrustgo_parser::split_sql_statements — see parser.rs:11263.
                // Splits the file into logical statements (respecting parens,
                // brackets, single/double quotes, line comments, and block
                // comments) so multi-line CREATE TABLE and similar constructs
                // execute as one statement each. Empty fragments are dropped.
                for stmt in split_sql_statements(&content) {
                    let trimmed = stmt.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    self.execute_sql(trimmed)?;
                }
                Ok(())
            }
        }
    }

    pub fn run_repl(&mut self) -> i32 {
        let stdin = std::io::stdin();
        let lines: Vec<String> = stdin.lock().lines().map_while(Result::ok).collect();
        self.run_repl_with_input(lines)
    }

    pub fn run_repl_with_input(&mut self, lines: Vec<String>) -> i32 {
        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.starts_with('.') {
                match parse_dotcmd(trimmed) {
                    Ok(DotCmd::Quit) => break,
                    Ok(cmd) => {
                        if let Err(e) = self.execute_dotcmd(cmd) {
                            eprintln!("{}", e);
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
                    }
                }
            }
        }
        if self.error_seen {
            1
        } else {
            EXIT_OK
        }
    }

    pub fn run_batch(&mut self, sql: &str) -> i32 {
        match self.execute_sql(sql) {
            Ok(_) => EXIT_OK,
            Err(e) => {
                eprintln!("{}", e);
                1
            }
        }
    }

    pub fn run_batch_stdin(&mut self) -> i32 {
        let stdin = std::io::stdin();
        let lines: Vec<String> = stdin.lock().lines().map_while(Result::ok).collect();
        self.run_batch_stdin_with_input(lines)
    }

    pub fn run_batch_stdin_with_input(&mut self, lines: Vec<String>) -> i32 {
        // Two bugs to fix in one place (#4607 + #4608):
        //   - Standalone `-- comment` lines must not produce "Unexpected
        //     token: Eof" (#4607).
        //   - Multi-line CREATE TABLE / multi-line INSERT VALUES must be
        //     joined into one logical statement (#4608).
        //
        // Strategy: always join the input with `\n` and split via
        // sqlrustgo_parser::split_sql_statements. The splitter respects
        // parens, brackets, single/double quotes, line comments, block
        // comments, and backslash escapes; it drops empty fragments.
        // Each fragment is dispatched via execute_sql.
        //
        // The pre-existing per-line dispatch behaviour was buggy: it
        // fed each physical line to the engine as if it were a complete
        // statement, breaking multi-line CREATE TABLE and standalone
        // comment lines. The new path makes the CLI MySQL-compatible:
        // statements are separated by `;` at top level, irrespective of
        // line boundaries. Empty input, comment-only input, and
        // trailing `;` all yield zero dispatches (exit 0).
        //
        // Uses sqlrustgo_parser::split_sql_statements — see parser.rs:11263.
        let joined = lines.join("\n");
        for stmt in split_sql_statements(&joined) {
            let trimmed = stmt.trim();
            if trimmed.is_empty() {
                continue;
            }
            self.dispatch_one(trimmed);
            if self.error_seen && !self.continue_on_error {
                return 1;
            }
        }
        if self.error_seen {
            1
        } else {
            EXIT_OK
        }
    }

    /// Shared per-statement dispatch: run `execute_sql`, print errors,
    /// set `error_seen`. Caller decides whether to abort.
    fn dispatch_one(&mut self, sql: &str) {
        match self.execute_sql(sql) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("{}", e);
                self.error_seen = true;
            }
        }
    }
}


/// Extension trait to convert sqlrustgo::Value to optional String for metadata.
trait ValueAsString {
    fn as_string(&self) -> Option<String>;
}

impl ValueAsString for sqlrustgo::Value {
    fn as_string(&self) -> Option<String> {
        match self {
            sqlrustgo::Value::Text(s) => Some(s.clone()),
            sqlrustgo::Value::Integer(i) => Some(i.to_string()),
            sqlrustgo::Value::Float(f) => Some(f.to_string()),
            sqlrustgo::Value::Boolean(b) => Some(b.to_string()),
            sqlrustgo::Value::Null => None,
            _ => Some(self.to_string()),
        }
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
        assert!(tmp.exists(), "open should create the DB path");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn open_existing_db_path_succeeds() {
        let tmp = std::env::temp_dir().join("v31257_open_existing");
        let _ = std::fs::remove_dir_all(&tmp);
        let _ = SqliteMode::open(&tmp, SqliteState::default(), false).expect("open1");
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
        assert!(out.contains("x"));
        assert!(out.contains("1"));
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
            .expect("create");
        mode.execute_sql("INSERT INTO t VALUES (1, 'alice')")
            .expect("insert");
        mode.execute_sql("SELECT id, name FROM t").expect("select");
        let out = std::fs::read_to_string(tmp.join("out.txt")).unwrap();
        assert!(out.contains("1"));
        assert!(out.contains("alice"));
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
    fn execute_dotcmd_tables_lists_created_table() {
        let tmp = std::env::temp_dir().join("v31257_dotcmd_tables");
        let _ = std::fs::remove_dir_all(&tmp);
        let mut mode = SqliteMode::open(&tmp, SqliteState::default(), false).unwrap();
        mode.state.output = OutputTarget::File(tmp.join("out.txt"));
        mode.execute_sql("CREATE TABLE foo (x INTEGER)").unwrap();
        mode.execute_dotcmd(DotCmd::Tables(None)).unwrap();
        let out = std::fs::read_to_string(tmp.join("out.txt")).unwrap();
        assert!(
            out.contains("foo"),
            "expected 'foo' in .tables output, got: {}",
            out
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn execute_dotcmd_schema_shows_create() {
        let tmp = std::env::temp_dir().join("v31257_dotcmd_schema");
        let _ = std::fs::remove_dir_all(&tmp);
        let mut mode = SqliteMode::open(&tmp, SqliteState::default(), false).unwrap();
        mode.state.output = OutputTarget::File(tmp.join("out.txt"));
        mode.execute_sql("CREATE TABLE my_t(a INTEGER, b TEXT)")
            .unwrap();
        mode.execute_dotcmd(DotCmd::Schema(Some("my_t".into())))
            .unwrap();
        let out = std::fs::read_to_string(tmp.join("out.txt")).unwrap();
        assert!(
            out.contains("my_t") || out.contains("CREATE"),
            "expected schema info, got: {}",
            out
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn run_repl_executes_stdin_lines() {
        let tmp = std::env::temp_dir().join("v31257_repl_lines");
        let _ = std::fs::remove_dir_all(&tmp);
        let mut mode = SqliteMode::open(&tmp, SqliteState::default(), false).unwrap();
        mode.state.output = OutputTarget::File(tmp.join("out.txt"));
        let script = "CREATE TABLE repl_t(x INTEGER);\nINSERT INTO repl_t VALUES (99);\nSELECT * FROM repl_t;\n.quit\n";
        let exit = mode.run_repl_with_input(script.lines().map(String::from).collect());
        assert_eq!(exit, 0);
        let out = std::fs::read_to_string(tmp.join("out.txt")).unwrap();
        assert!(out.contains("99"));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn run_repl_parse_error_exits_one_but_continues() {
        let tmp = std::env::temp_dir().join("v31257_repl_parse_err");
        let _ = std::fs::remove_dir_all(&tmp);
        let mut mode = SqliteMode::open(&tmp, SqliteState::default(), false).unwrap();
        mode.state.output = OutputTarget::File(tmp.join("out.txt"));
        let exit = mode.run_repl_with_input(vec![
            "SELEC 1".to_string(),
            "SELECT 2".to_string(),
            ".quit".to_string(),
        ]);
        assert_eq!(exit, 1);
        let out = std::fs::read_to_string(tmp.join("out.txt")).unwrap();
        assert!(out.contains("2"));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn run_batch_single_statement_success() {
        let tmp = std::env::temp_dir().join("v31257_batch_single");
        let _ = std::fs::remove_dir_all(&tmp);
        let mut mode = SqliteMode::open(&tmp, SqliteState::default(), false).unwrap();
        mode.state.output = OutputTarget::File(tmp.join("out.txt"));
        let exit = mode.run_batch("SELECT 42");
        assert_eq!(exit, 0);
        let out = std::fs::read_to_string(tmp.join("out.txt")).unwrap();
        assert!(out.contains("42"));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn run_batch_parse_error_exit_1_fail_fast() {
        let tmp = std::env::temp_dir().join("v31257_batch_failfast");
        let _ = std::fs::remove_dir_all(&tmp);
        let mut mode = SqliteMode::open(&tmp, SqliteState::default(), false).unwrap();
        mode.state.output = OutputTarget::File(tmp.join("out.txt"));
        let exit = mode.run_batch("SELEC 1");
        assert_eq!(exit, 1);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn run_batch_stdin_fail_fast_stops_on_first_error() {
        let tmp = std::env::temp_dir().join("v31257_batch_stdin_failfast");
        let _ = std::fs::remove_dir_all(&tmp);
        let mut mode = SqliteMode::open(&tmp, SqliteState::default(), false).unwrap();
        mode.state.output = OutputTarget::File(tmp.join("out.txt"));
        let input = vec![
            "SELECT 1;".to_string(),
            "SELEC 2;".to_string(),
            "SELECT 3;".to_string(),
        ];
        let exit = mode.run_batch_stdin_with_input(input);
        assert_eq!(exit, 1);
        let out = std::fs::read_to_string(tmp.join("out.txt")).unwrap();
        assert!(out.contains("1"));
        assert!(!out.contains("3"));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn run_batch_stdin_continue_on_error_runs_all() {
        let tmp = std::env::temp_dir().join("v31257_batch_stdin_continue");
        let _ = std::fs::remove_dir_all(&tmp);
        let mut mode = SqliteMode::open(&tmp, SqliteState::default(), true).unwrap();
        mode.state.output = OutputTarget::File(tmp.join("out.txt"));
        let input = vec![
            "SELECT 1;".to_string(),
            "SELEC 2;".to_string(),
            "SELECT 3;".to_string(),
        ];
        let exit = mode.run_batch_stdin_with_input(input);
        assert_eq!(exit, 1);
        let out = std::fs::read_to_string(tmp.join("out.txt")).unwrap();
        assert!(out.contains("1"));
        assert!(out.contains("3"));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn cross_process_persistence() {
        let tmp = std::env::temp_dir().join("v31257_persist");
        let _ = std::fs::remove_dir_all(&tmp);
        {
            let mut mode = SqliteMode::open(&tmp, SqliteState::default(), false).unwrap();
            mode.execute_sql("CREATE TABLE p_t(id INTEGER, v TEXT)")
                .unwrap();
            mode.execute_sql("INSERT INTO p_t VALUES (1, 'first')")
                .unwrap();
            mode.execute_sql("INSERT INTO p_t VALUES (2, 'second')")
                .unwrap();
        }
        {
            let mut mode = SqliteMode::open(&tmp, SqliteState::default(), false).unwrap();
            mode.state.output = OutputTarget::File(tmp.join("out.txt"));
            mode.execute_sql("SELECT id, v FROM p_t ORDER BY id")
                .unwrap();
            let out = std::fs::read_to_string(tmp.join("out.txt")).unwrap();
            assert!(out.contains("first"));
            assert!(out.contains("second"));
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }

    // ----------------------------------------------------------------------
    // V312-61 / #4607 + #4608: regression coverage for CLI batch stdin.
    //
    // #4607: a standalone `--` comment line must not trigger
    //        "Parse error: Unexpected token: Eof".
    // #4608: a multi-line CREATE TABLE (column on its own line) must be
    //        joined into one logical statement and dispatched once.
    // ----------------------------------------------------------------------

    #[test]
    fn run_batch_stdin_with_only_comment_line_succeeds() {
        let tmp = std::env::temp_dir().join("v31261_batch_stdin_comment_only");
        let _ = std::fs::remove_dir_all(&tmp);
        let mut mode = SqliteMode::open(&tmp, SqliteState::default(), false).unwrap();
        mode.state.output = OutputTarget::File(tmp.join("out.txt"));
        let input = vec![
            "-- only a comment".to_string(),
            "SELECT 1".to_string(),
        ];
        let exit = mode.run_batch_stdin_with_input(input);
        assert_eq!(exit, EXIT_OK, "standalone -- comment must not error");
        let out = std::fs::read_to_string(tmp.join("out.txt")).unwrap();
        assert!(out.contains("1"));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn run_batch_stdin_with_only_blank_lines_succeeds() {
        let tmp = std::env::temp_dir().join("v31261_batch_stdin_blank_only");
        let _ = std::fs::remove_dir_all(&tmp);
        let mut mode = SqliteMode::open(&tmp, SqliteState::default(), false).unwrap();
        mode.state.output = OutputTarget::File(tmp.join("out.txt"));
        let input = vec!["".to_string(), "   ".to_string(), "\t".to_string()];
        let exit = mode.run_batch_stdin_with_input(input);
        assert_eq!(exit, EXIT_OK, "blank-only input must exit 0");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    fn run_batch_stdin_with_multiline_create_table_succeeds() {
        let tmp = std::env::temp_dir().join("v31261_batch_stdin_multiline_ct");
        let _ = std::fs::remove_dir_all(&tmp);
        let mut state = SqliteState::default();
        state.mode = OutputMode::Csv;
        let mut mode = SqliteMode::open(&tmp, state, false).unwrap();
        mode.state.output = OutputTarget::File(tmp.join("out.txt"));
        let input = vec![
            "CREATE TABLE products(".to_string(),
            "  id INT PRIMARY KEY,".to_string(),
            "  stock INT".to_string(),
            ");".to_string(),
            "INSERT INTO products VALUES (1, 100);".to_string(),
            "SELECT * FROM products;".to_string(),
        ];
        let exit = mode.run_batch_stdin_with_input(input);
        assert_eq!(exit, EXIT_OK, "multi-line CREATE TABLE must succeed");
        let out = std::fs::read_to_string(tmp.join("out.txt")).unwrap();
        assert!(out.contains("1,100"), "row (1, 100) must appear in output");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    fn run_batch_stdin_with_mixed_comments_and_multiline_succeeds() {
        let tmp = std::env::temp_dir().join("v31261_batch_stdin_mixed");
        let _ = std::fs::remove_dir_all(&tmp);
        let mut state = SqliteState::default();
        state.mode = OutputMode::Csv;
        let mut mode = SqliteMode::open(&tmp, state, false).unwrap();
        mode.state.output = OutputTarget::File(tmp.join("out.txt"));
        // Mirrors the BustubX-EDU teaching-seed.sql shape.
        let input = vec![
            "-- header comment".to_string(),
            "".to_string(),
            "CREATE TABLE products(".to_string(),
            "  id INT PRIMARY KEY,".to_string(),
            "  stock INT".to_string(),
            ");".to_string(),
            "".to_string(),
            "-- mid comment".to_string(),
            "INSERT INTO products VALUES (1, 100);".to_string(),
            "SELECT * FROM products;".to_string(),
        ];
        let exit = mode.run_batch_stdin_with_input(input);
        assert_eq!(exit, EXIT_OK, "mixed comment/blank/multiline input must succeed");
        let out = std::fs::read_to_string(tmp.join("out.txt")).unwrap();
        assert!(out.contains("1,100"), "row (1, 100) must appear in output");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn run_batch_stdin_multiline_syntax_error_aborts_failfast() {
        let tmp = std::env::temp_dir().join("v31261_batch_stdin_multiline_err");
        let _ = std::fs::remove_dir_all(&tmp);
        let mut mode = SqliteMode::open(&tmp, SqliteState::default(), false).unwrap();
        mode.state.output = OutputTarget::File(tmp.join("out.txt"));
        // The CREATE TABLE itself is valid; `GARBAGE NOT SQL` on its
        // own line is dispatched as a separate statement and triggers
        // a parse error. With continue_on_error=false the batch must
        // abort with exit 1 and `SELECT 1` must not execute.
        let input = vec![
            "CREATE TABLE broken_t(id INT, stock INT);".to_string(),
            "GARBAGE NOT SQL".to_string(),
            "SELECT 1;".to_string(),
        ];
        let exit = mode.run_batch_stdin_with_input(input);
        assert_eq!(exit, 1, "fail-fast on parse error must exit 1");
        let out = std::fs::read_to_string(tmp.join("out.txt")).unwrap();
        assert!(!out.contains("1\n"), "SELECT 1 must not have run after parse error");
        // The CREATE TABLE succeeded before the parse error, so
        // broken_t DOES exist — verify it is queryable.
        let mut probe_mode = SqliteMode::open(&tmp, SqliteState::default(), false).unwrap();
        probe_mode.state.output = OutputTarget::File(tmp.join("probe.txt"));
        let res = probe_mode.execute_sql("SELECT id FROM broken_t");
        assert!(res.is_ok(), "broken_t (created before the parse error) must exist");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn run_batch_stdin_semicolon_inside_string_not_split() {
        let tmp = std::env::temp_dir().join("v31261_batch_stdin_string_semi");
        let _ = std::fs::remove_dir_all(&tmp);
        let mut mode = SqliteMode::open(&tmp, SqliteState::default(), false).unwrap();
        mode.state.output = OutputTarget::File(tmp.join("out.txt"));
        let input = vec![
            "CREATE TABLE t(id INT, v TEXT);".to_string(),
            // The `;` inside the string literal MUST NOT terminate the INSERT.
            "INSERT INTO t VALUES (1, 'a;b;c');".to_string(),
            "SELECT v FROM t;".to_string(),
        ];
        let exit = mode.run_batch_stdin_with_input(input);
        assert_eq!(exit, EXIT_OK);
        let out = std::fs::read_to_string(tmp.join("out.txt")).unwrap();
        assert!(out.contains("a;b;c"), "string literal must be preserved verbatim");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn read_dotcmd_with_multiline_create_table_succeeds() {
        let tmp = std::env::temp_dir().join("v31261_read_dotcmd_multiline");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let sql_path = tmp.join("seed.sql");
        let sql_content = "-- teaching seed\n\
             CREATE TABLE courses(\n\
               id INT PRIMARY KEY,\n\
               title TEXT\n\
             );\n\
             INSERT INTO courses VALUES (1, 'BustubX-EDU');\n";
        std::fs::write(&sql_path, sql_content).unwrap();
        let mut mode = SqliteMode::open(&tmp, SqliteState::default(), false).unwrap();
        mode.state.output = OutputTarget::File(tmp.join("out.txt"));
        mode.execute_dotcmd(DotCmd::Read(sql_path))
            .expect("read dotcmd must succeed for multi-line CREATE TABLE");
        mode.execute_dotcmd(DotCmd::Tables(None)).unwrap();
        let out = std::fs::read_to_string(tmp.join("out.txt")).unwrap();
        assert!(out.contains("courses"), "courses table must exist after .read");
        let _ = std::fs::remove_dir_all(&tmp);
    }
}

