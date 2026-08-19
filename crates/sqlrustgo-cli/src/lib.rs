//! SQLRustGo Canonical CLI Library
//!
//! Provides the `run()` entry point used by `sqlrustgo` binary.
//! Thin wrapper around `sqlrustgo-mysql-server` for most subcommands.

mod dotcmd;
mod error;
mod implicit_alias;
mod output;
mod sqlite_mode;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::Command as Proc;

use crate::error::EXIT_STORAGE_INIT;
use crate::output::{OutputMode, OutputTarget};
use crate::sqlite_mode::{SqliteMode, SqliteState};

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
enum OutputModeArg {
    Table,
    List,
    Csv,
    Json,
}

impl From<OutputModeArg> for OutputMode {
    fn from(v: OutputModeArg) -> Self {
        match v {
            OutputModeArg::Table => OutputMode::Table,
            OutputModeArg::List => OutputMode::List,
            OutputModeArg::Csv => OutputMode::Csv,
            OutputModeArg::Json => OutputMode::Json,
        }
    }
}

#[derive(Parser, Debug)]
#[command(name = "sqlrustgo", about = "SQLRustGo canonical CLI", version)]
struct Cli {
    #[command(subcommand)]
    command: Option<SubCmd>,
}

#[derive(Subcommand, Debug)]
enum SubCmd {
    /// Start the MySQL wire-protocol server.
    Serve {
        #[arg(long, default_value = "3307")]
        port: u16,
        #[arg(long)]
        data_dir: Option<String>,
    },
    /// Execute a single SQL statement and print the result.
    Exec {
        sql: String,
    },
    /// Interactive REPL.
    Repl {
        #[arg(long, default_value = "3307")]
        port: u16,
    },
    Bench,
    Gmp,
    Diag,
    Backup {
        output_dir: String,
    },
    Restore {
        backup_id: String,
        database: String,
    },
    /// Connect to a running server and execute a query (NEW).
    Cli {
        #[arg(short, long, default_value = "3307")]
        port: u16,
        #[arg(short, long, default_value = "127.0.0.1")]
        host: String,
        #[arg(short, long)]
        user: Option<String>,
        #[arg(short, long)]
        password: Option<String>,
        query: String,
    },
    /// Soak REPL mode: hold a persistent MySQL connection and serve
    /// queries read from stdin, writing tab-separated results to stdout.
    /// Designed for SOAK testing (PR #3347 alternative to mysql CLI).
    ///
    /// Wire protocol:
    ///   Input (one per line):  SQL statement
    ///   Output:
    ///     OK\t<affected_rows>
    ///     ROWS\t<column_count>
    ///     COL\t<name>\t<type>
    ///     DATA\t<row_count>
    ///     ROW\t<col1>\t<col2>\t...
    Soak {
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(short = 'p', long, default_value = "3307")]
        port: u16,
        #[arg(short = 'u', long)]
        user: Option<String>,
        #[arg(long = "pass", short = 'w')]
        password: Option<String>,
    },
    /// sqlite3-like local DB mode (BustubX-EDU teaching CLI, V312-57 #4359).
    Sqlite {
        /// Path to local DB (file or directory).
        db: PathBuf,
        #[arg(long)]
        batch: bool,
        #[arg(long)]
        cmd: Option<String>,
        #[arg(long, value_enum, default_value = "table")]
        mode: OutputModeArg,
        #[arg(long)]
        headers: Option<bool>,
        #[arg(long)]
        timer: Option<bool>,
        #[arg(long)]
        explain: Option<bool>,
        #[arg(long)]
        continue_on_error: bool,
    },
}

pub fn run() -> i32 {
    // Implicit-alias fast-path: `sqlrustgo <db-path>` with exactly one
    // positional arg that looks like a DB path enters sqlite-mode immediately.
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 2 && implicit_alias::looks_like_db_path(&args[1]) {
        return run_sqlite_subcommand(
            PathBuf::from(&args[1]),
            false, // batch
            None,  // cmd
            OutputMode::Table,
            None,
            None,
            None,
            false, // headers, timer, explain, continue_on_error
        );
    }

    let cli = Cli::parse();

    match cli.command {
        None => {
            let bin = mysql_server_bin();
            let status = Proc::new(&bin).arg("--help").status();
            match status {
                Ok(s) => s.code().unwrap_or(1),
                Err(_) => {
                    eprintln!("sqlrustgo-mysql-server not found. Run with --help to see available subcommands.");
                    1
                }
            }
        }
        Some(SubCmd::Serve { port, data_dir }) => {
            let mut args = vec![("--port", port.to_string())];
            if let Some(dir) = data_dir {
                args.push(("--data-dir", dir));
            }
            run_bin("serve", &args)
        }
        Some(SubCmd::Exec { sql }) => run_bin_arg_positional("exec", &sql),
        Some(SubCmd::Repl { port }) => run_bin("repl", &[("--port", port.to_string())]),
        Some(SubCmd::Bench) => run_bin("bench", &[]),
        Some(SubCmd::Gmp) => run_bin("gmp", &[]),
        Some(SubCmd::Diag) => run_bin("diag", &[]),
        Some(SubCmd::Backup { output_dir }) => run_bin("backup", &[("--output", output_dir)]),
        Some(SubCmd::Restore {
            backup_id,
            database,
        }) => run_bin(
            "restore",
            &[("--input", backup_id), ("--database", database)],
        ),
        Some(SubCmd::Cli {
            port,
            host,
            user,
            password,
            query,
        }) => run_cli(
            &query,
            &host,
            port,
            user.as_deref().unwrap_or("root"),
            password.as_deref().unwrap_or(""),
        ),
        Some(SubCmd::Soak {
            host,
            port,
            user,
            password,
        }) => run_soak_repl(
            &host,
            port,
            user.as_deref().unwrap_or("root"),
            password.as_deref().unwrap_or(""),
        ),
        Some(SubCmd::Sqlite {
            db,
            batch,
            cmd,
            mode,
            headers,
            timer,
            explain,
            continue_on_error,
        }) => run_sqlite_subcommand(
            db,
            batch,
            cmd,
            mode.into(),
            headers,
            timer,
            explain,
            continue_on_error,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn run_sqlite_subcommand(
    db: PathBuf,
    batch: bool,
    cmd: Option<String>,
    mode: OutputMode,
    headers: Option<bool>,
    timer: Option<bool>,
    explain: Option<bool>,
    continue_on_error: bool,
) -> i32 {
    let _ = timer; // accepted but not yet plumbed to executor
    let _ = explain; // accepted but not yet plumbed to executor

    let state = SqliteState {
        mode,
        headers: headers.unwrap_or(!batch),
        timer: timer.unwrap_or(false),
        explain: explain.unwrap_or(false),
        output: OutputTarget::Stdout,
    };
    let mut mode_runner = match SqliteMode::open(&db, state, continue_on_error) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{}", e);
            return EXIT_STORAGE_INIT;
        }
    };

    if let Some(sql) = cmd {
        return mode_runner.run_batch(&sql);
    }
    if batch {
        return mode_runner.run_batch_stdin();
    }
    mode_runner.run_repl()
}

fn run_bin(subcmd: &str, args: &[(&str, String)]) -> i32 {
    let bin = mysql_server_bin();
    let mut cmd = Proc::new(&bin);
    cmd.arg(subcmd);
    for (k, v) in args {
        cmd.arg(k).arg(v);
    }
    exec_status(&bin, cmd)
}

fn run_bin_arg_positional(subcmd: &str, positional: &str) -> i32 {
    let bin = mysql_server_bin();
    let mut cmd = Proc::new(&bin);
    cmd.arg(subcmd).arg(positional);
    exec_status(&bin, cmd)
}

fn mysql_server_bin() -> String {
    let candidates = [
        "sqlrustgo-mysql-server",
        "./target/debug/sqlrustgo-mysql-server",
        "./target/release/sqlrustgo-mysql-server",
    ];
    for c in candidates {
        if std::path::Path::new(c).exists() || which(c).is_some() {
            return c.to_string();
        }
    }
    "sqlrustgo-mysql-server".to_string()
}

fn which(name: &str) -> Option<String> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let full = dir.join(name);
        if full.exists() {
            return Some(full.to_string_lossy().to_string());
        }
    }
    None
}

fn exec_status(bin: &str, mut cmd: Proc) -> i32 {
    match cmd.status() {
        Ok(s) => s.code().unwrap_or(1),
        Err(e) => {
            eprintln!("Failed to execute {bin}: {e}");
            1
        }
    }
}

#[allow(unused_variables)]
fn run_cli(query: &str, host: &str, port: u16, user: &str, password: &str) -> i32 {
    use sqlrustgo_mysql_client::MySqlConnection;
    use std::net::SocketAddr;

    let addr: SocketAddr = match format!("{host}:{port}").parse() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Invalid address {host}:{port}: {e}");
            return 1;
        }
    };

    let mut conn = match MySqlConnection::connect(&addr, user, password, "") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Connection failed: {e}");
            return 1;
        }
    };

    println!(
        "Connected to {}:{} (server: {})",
        host, port, conn.server_version
    );

    match conn.execute(query) {
        Ok(result) => {
            use sqlrustgo_mysql_client::ResultSet;
            match result {
                ResultSet::Select { columns, rows, .. } => {
                    // Print column headers
                    let header: Vec<&str> = columns.iter().map(|c| c.name.as_str()).collect();
                    println!("{}", header.join(" | "));
                    println!("{}", vec!["-"; header.len()].join("---"));

                    // Print rows
                    for row in &rows {
                        println!("{}", row.join(" | "));
                    }
                    println!("\n{} row(s) in set", rows.len());
                }
                ResultSet::Ok {
                    affected_rows,
                    info,
                    ..
                } => {
                    println!("Query OK, {} row(s) affected", affected_rows);
                    if !info.is_empty() {
                        println!("{}", info);
                    }
                }
                ResultSet::Error {
                    error_code,
                    error_message,
                    ..
                } => {
                    eprintln!("Error {}: {}", error_code, error_message);
                    return 1;
                }
            }
            0
        }
        Err(e) => {
            eprintln!("Query execution failed: {e}");
            1
        }
    }
}

/// Soak REPL mode: 保持一个持久 MySQL 连接，从 stdin 循环读 SQL，
/// 写 tab-separated 结果到 stdout。设计为 SOAK 测试前端 (PR #3347 替代 mysql CLI)。
///
/// 协议 (line-delimited):
///   输入:  SQL 语句
///   输出:
///     OK\t<affected_rows>     -- DML/INSERT/UPDATE/DELETE 成功
///     ROWS\t<column_count>    -- SELECT 成功
///     COL\t<name>\t<type>     -- 列定义 (重复 N 次)
///     DATA\t<row_count>      -- 行数据
///     ROW\t<col1>\t<col2>\t... -- 单行 (重复 row_count 次)
///     ERR\t<code>\t<message>  -- 错误
///   特殊命令:  QUIT (退出), 行首 # (注释)
fn run_soak_repl(host: &str, port: u16, user: &str, password: &str) -> i32 {
    use sqlrustgo_mysql_client::MySqlConnection;
    use std::io::{self, BufRead, Write};

    let addr: std::net::SocketAddr = match format!("{host}:{port}").parse() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("# ERR\t-2\tInvalid address {host}:{port}: {e}");
            return 1;
        }
    };

    let mut conn = match MySqlConnection::connect(&addr, user, password, "") {
        Ok(c) => c,
        Err(e) => {
            eprintln!("# ERR\t-3\tConnection failed: {e}");
            return 1;
        }
    };

    // stderr 提示信息 (不影响 stdout 协议)
    eprintln!(
        "# Soak REPL ready: server={}, connection=1",
        conn.server_version
    );

    let stdin = io::stdin();
    let out = io::stdout();
    let mut out_lock = out.lock();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed == "QUIT" || trimmed == "EXIT" {
            eprintln!("# Soak REPL shutting down");
            break;
        }

        match conn.execute(trimmed) {
            Ok(result) => {
                use sqlrustgo_mysql_client::ResultSet;
                match result {
                    ResultSet::Ok { affected_rows, .. } => {
                        writeln!(out_lock, "OK\t{}", affected_rows).ok();
                    }
                    ResultSet::Select { columns, rows, .. } => {
                        writeln!(out_lock, "ROWS\t{}", columns.len()).ok();
                        for c in &columns {
                            // column_type 1 byte, e.g. 0xfd for VARCHAR
                            writeln!(out_lock, "COL\t{}\t{}", c.name, c.column_type).ok();
                        }
                        writeln!(out_lock, "DATA\t{}", rows.len()).ok();
                        for r in &rows {
                            writeln!(out_lock, "ROW\t{}", r.join("\t")).ok();
                        }
                    }
                    ResultSet::Error {
                        error_code,
                        error_message,
                        ..
                    } => {
                        writeln!(out_lock, "ERR\t{}\t{}", error_code, error_message).ok();
                    }
                }
            }
            Err(e) => {
                writeln!(out_lock, "ERR\t-1\t{}", e).ok();
            }
        }
        out_lock.flush().ok();
    }
    0
}
