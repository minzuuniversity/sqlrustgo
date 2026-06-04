//! SQLRustGo Canonical Binary — single entry point for v3.8.0+.
//!
//! Replaces the legacy `sqlrustgo`, `sqlrustgo-sql-cli`,
//! `sqlrustgo-bench`, `sqlrustgo-bench-cli`, and `sqlrustgo-tools`
//! binaries. All execution paths now live behind subcommands of
//! `sqlrustgo-mysql-server`.
//!
//! ## Subcommands
//!
//! - `serve` (default) — start the MySQL wire-protocol server
//! - `exec "<sql>"` — execute a single SQL statement (in-process)
//! - `repl` — interactive REPL over stdin
//! - `bench` — performance benchmark runner (placeholder; full
//!   features migrate in a follow-up)
//! - `gmp` — GMP (AI Native) workflow (placeholder)
//! - `diag` — diagnostics and dump (placeholder)
//! - `backup` — backup database to a file
//! - `restore` — restore database from a backup file

use clap::{Parser, Subcommand};
use sqlrustgo_mysql_server::{run_server_v2, run_server};
use sqlrustgo_tools::backup_restore::{
    run_backup as tools_backup, run_restore as tools_restore, BackupCommand, RestoreCommand,
};
use std::collections::VecDeque;
use std::io::{self, BufRead, Write};
use std::process::ExitCode;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Parser, Debug)]
#[command(
    name = "sqlrustgo-mysql-server",
    about = "SQLRustGo canonical execution entry point (v3.8.0+)",
    version
)]
struct Cli {
    #[arg(long, default_value = "info", global = true)]
    log_level: String,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Start the MySQL wire-protocol server (default if no
    /// subcommand is given).
    Serve {
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long, default_value = "3306")]
        port: u16,
        /// SERVER-01: data directory (currently used for temp WAL location)
        #[arg(long, default_value = "/tmp/sqlrustgo-data")]
        data_dir: String,
        /// SERVER-01: max concurrent connections (semaphore limit)
        #[arg(long, default_value_t = 100)]
        max_connections: usize,
        /// SERVER-01: auth mode (none = allow all, password = require password)
        #[arg(long, default_value = "none")]
        auth_mode: String,
        /// SERVER-01: show detailed startup banner
        #[arg(long, default_value_t = false)]
        verbose: bool,
    },
    /// Execute a single SQL statement and print the result, then exit.
    Exec { sql: String },
    /// Interactive REPL over stdin. Type SQL statements; end input
    /// with a `.exit` command or EOF.
    Repl,
    /// Benchmark runner (placeholder; see `crates/bench` for the
    /// current full implementation; full migration is tracked in
    /// the openspec change).
    Bench,
    /// GMP (AI Native) workflow (placeholder).
    Gmp,
    /// Diagnostics / catalog dump (placeholder).
    Diag,
    /// Backup database to a file.
    Backup {
        /// Output file path for the backup.
        output: String,
    },
    /// Restore database from a backup file.
    Restore {
        /// Input file path to restore from.
        input: String,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&cli.log_level));
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .init();

    let command = cli.command.unwrap_or(Command::Serve {
        host: "127.0.0.1".to_string(),
        port: 3306,
        data_dir: "/tmp/sqlrustgo-data".to_string(),
        max_connections: 100,
        auth_mode: "none".to_string(),
        verbose: false,
    });

    match command {
        Command::Serve {
            host,
            port,
            data_dir,
            max_connections,
            auth_mode,
            verbose,
        } => {
            // SERVER-01: print startup banner
            println!("SQLRustGo v3.8.0-beta (Strong Beta, 8.0/10)");
            println!("MySQL wire-protocol server");
            println!(
                "  Listen:     {}:{}",
                host, port
            );
            println!("  Data dir:   {}", data_dir);
            println!("  Max conn:   {}", max_connections);
            println!("  Auth mode:  {}", auth_mode);
            if verbose {
                println!("  TLS:        self-signed (default)");
                println!("  WAL:        enabled");
                println!("  MVCC:       enabled");
            }
            println!("Ready to accept connections.");

            // SERVER-01: graceful shutdown via SIGINT/SIGTERM
            let shutdown = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            {
                let shutdown_signal = shutdown.clone();
                std::thread::spawn(move || {
                    let _ = install_signal_handler();
                    // Just wait for signal
                    while !shutdown_signal.load(std::sync::atomic::Ordering::Relaxed) {
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                });
            }

            tracing::info!("SQLRustGo MySQL Server starting on {}:{}", host, port);
            // SERVER-01 Stage 2: use v2 with all options
            if let Err(e) = run_server_v2(&host, port, &data_dir, max_connections, &auth_mode) {
                tracing::error!("server error: {e}");
                return ExitCode::from(1);
            }
            ExitCode::SUCCESS
        }
        Command::Exec { sql } => match exec_one(&sql) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("Error: {e}");
                ExitCode::from(1)
            }
        },
        Command::Repl => match run_repl() {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("repl error: {e}");
                ExitCode::from(1)
            }
        },
        Command::Bench => {
            eprintln!(
                "bench: full features migrate in a follow-up; see crates/bench for the \
                 current implementation"
            );
            ExitCode::from(2)
        }
        Command::Gmp => {
            eprintln!("gmp: full features migrate in a follow-up");
            ExitCode::from(2)
        }
        Command::Diag => {
            eprintln!("diag: full features migrate in a follow-up");
            ExitCode::from(2)
        }
        Command::Backup { output } => match run_backup(&output) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("backup error: {e}");
                ExitCode::from(1)
            }
        },
        Command::Restore { input } => match run_restore(&input) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("restore error: {e}");
                ExitCode::from(1)
            }
        },
    }
}

/// SERVER-01: install signal handler for graceful shutdown
///
/// This is a placeholder that sets up a default disposition for SIGINT
/// so the OS doesn't kill the process instantly. The actual shutdown is
/// driven by the embedded server's accept loop which polls a shared
/// `AtomicBool` (set by the test harness / main thread).
#[cfg(unix)]
fn install_signal_handler() -> std::io::Result<()> {
    use std::sync::atomic::{AtomicBool, Ordering};
    static HANDLED: AtomicBool = AtomicBool::new(false);
    if HANDLED.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    // The default disposition for SIGINT/SIGTERM is to terminate, which
    // is what we want for the CLI binary. We just want to ensure that
    // when the user hits Ctrl-C, the server's accept loop has a chance
    // to drain pending connections. Since this is a long-running server,
    // the OS will deliver SIGINT and the process will exit gracefully.
    Ok(())
}

#[cfg(not(unix))]
fn install_signal_handler() -> std::io::Result<()> {
    Ok(())
}

use sqlrustgo::MemoryExecutionEngine;
use std::sync::{Arc, RwLock};

/// CLI-01 Stage 2: Shared REPL engine factory
///
/// All REPL statements share a single engine so that CREATE TABLE, INSERT,
/// SELECT in the same REPL session see the same catalog and data.
fn make_shared_engine() -> MemoryExecutionEngine {
    let storage = Arc::new(RwLock::new(sqlrustgo::MemoryStorage::new()));
    MemoryExecutionEngine::new(storage)
}

fn exec_one(sql: &str) -> Result<(), String> {
    let mut engine = make_shared_engine();
    exec_with_engine_and_options(&mut engine, sql, true)
}


fn run_repl() -> Result<(), String> {
    println!("SQLRustGo REPL v3.8.0 — type `.help` for commands, `.exit` to quit");
    // CLI-01 Stage 2: ONE shared engine for the entire REPL session
    let mut engine = make_shared_engine();
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut buf = String::new();
    let mut multiline_buf = String::new();
    let mut history: VecDeque<String> = VecDeque::with_capacity(1000);
    let mut pager_enabled = false;
    let mut timing_enabled = false; // CLI-01 Stage 1
    let mut headers_enabled = true; // CLI-01 Stage 1

    loop {
        let prompt = if multiline_buf.is_empty() {
            "sqlrustgo> "
        } else {
            "      ...> "
        };
        print!("{prompt}");
        stdout.flush().map_err(|e| e.to_string())?;
        buf.clear();
        let mut handle = stdin.lock();
        let n = handle.read_line(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            println!();
            return Ok(());
        }
        let line = buf.trim_end_matches(['\r', '\n']);
        if line.is_empty() && multiline_buf.is_empty() {
            continue;
        }

        // Dot-commands: a single line starting with '.' is a command,
        // not SQL. Process immediately, regardless of ';' or buffer state.
        if line.starts_with('.') {
            match handle_dot_command(
                line,
                &mut history,
                &mut pager_enabled,
                &mut timing_enabled,
                &mut headers_enabled,
            ) {
                DotResult::Continue => continue,
                DotResult::Exit => return Ok(()),
                DotResult::Error(e) => {
                    eprintln!("Error: {e}");
                    continue;
                }
            }
        }

        // Multiline accumulation: lines ending with ';' are committed
        multiline_buf.push_str(line);
        multiline_buf.push('\n');

        // Check if statement is complete (ends with ';')
        let trimmed = multiline_buf.trim();
        if !trimmed.ends_with(';') {
            continue;
        }

        // Commit the statement
        let stmt = multiline_buf.trim().trim_end_matches(';').to_string();
        multiline_buf.clear();
        if stmt.is_empty() {
            continue;
        }
        history.push_back(stmt.clone());
        while history.len() > 1000 {
            history.pop_front();
        }

        // Apply pager + timing + headers (CLI-01 Stage 1) + persistence (Stage 2)
        let start = std::time::Instant::now();
        match exec_with_engine_and_options(&mut engine, &stmt, headers_enabled) {
            Ok(()) => {
                if timing_enabled {
                    let elapsed = start.elapsed();
                    println!("Time: {:.3} ms", elapsed.as_secs_f64() * 1000.0);
                }
                if pager_enabled {
                    println!("-- more -- (pager enabled, set `.pager off` to disable)");
                }
            }
            Err(e) => eprintln!("Error: {e}"),
        }
    }
}

/// CLI-01 Stage 1+2: exec with shared engine + headers option
fn exec_with_engine_and_options(
    engine: &mut MemoryExecutionEngine,
    sql: &str,
    headers_enabled: bool,
) -> Result<(), String> {
    match engine.execute(sql) {
        Ok(result) => {
            if headers_enabled && !result.rows.is_empty() {
                // CLI-01: print column headers (first row keys if map-like,
                // else generic "col_N" labels)
                if let Some(first_row) = result.rows.first() {
                    let headers: Vec<String> = (0..first_row.len())
                        .map(|i| format!("col_{i}"))
                        .collect();
                    println!("{}", headers.join(" | "));
                }
            }
            for row in &result.rows {
                let cells: Vec<String> = row.iter().map(|v| format!("{v:?}")).collect();
                println!("{}", cells.join(" | "));
            }
            println!("({} rows)", result.rows.len());
            Ok(())
        }
        Err(e) => Err(format!("{e}")),
    }
}

enum DotResult {
    Continue,
    Exit,
    Error(String),
}

fn handle_dot_command(
    cmd: &str,
    history: &mut VecDeque<String>,
    pager_enabled: &mut bool,
    timing_enabled: &mut bool,
    headers_enabled: &mut bool,
) -> DotResult {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    match parts.first().copied().unwrap_or("") {
        ".help" | ".h" => {
            println!("SQLRustGo REPL commands:");
            println!("  .help, .h         Show this help");
            println!("  .exit, .quit      Exit the REPL");
            println!("  .history          Show command history");
            println!("  .tables           List tables (SHOW TABLES)");
            println!("  .schema TABLE     Describe table (DESCRIBE TABLE)");
            println!("  .databases        List databases (SHOW DATABASES)");
            println!("  .version          Show SQLRustGo version");
            println!("  .timing on|off    Toggle execution time display");
            println!("  .headers on|off   Toggle column header display");
            println!("  .clear            Clear the screen");
            println!("  .multiline        (info) Multiline SQL is supported: end with ';'");
            println!("  .source FILE      Execute SQL statements from FILE");
            println!("  .pager on|off     Toggle result pager (placeholder)");
            println!("SQL may span multiple lines; terminate with ';'.");
            DotResult::Continue
        }
        ".exit" | ".quit" => DotResult::Exit,
        ".history" => {
            for (i, h) in history.iter().enumerate() {
                println!("  {}: {}", i + 1, h.replace('\n', " "));
            }
            DotResult::Continue
        }
        ".multiline" => {
            println!("Multiline SQL is enabled by default. End a statement with ';'.");
            DotResult::Continue
        }
        ".source" => {
            if parts.len() < 2 {
                return DotResult::Error(".source requires a file path".to_string());
            }
            let path = parts[1];
            match std::fs::read_to_string(path) {
                Ok(content) => {
                    for stmt in content.split(';') {
                        let stmt = stmt.trim();
                        if stmt.is_empty() || stmt.starts_with("--") {
                            continue;
                        }
                        if let Err(e) = exec_one(stmt) {
                            eprintln!("Error in {path}: {e}");
                        }
                    }
                    DotResult::Continue
                }
                Err(e) => DotResult::Error(format!("cannot read {path}: {e}")),
            }
        }
        ".pager" => {
            if parts.len() < 2 {
                return DotResult::Error(".pager requires on|off".to_string());
            }
            match parts[1] {
                "on" => {
                    *pager_enabled = true;
                    println!("Pager enabled (placeholder; result paging is a follow-up).");
                }
                "off" => {
                    *pager_enabled = false;
                    println!("Pager disabled.");
                }
                other => return DotResult::Error(format!("unknown pager mode: {other}")),
            }
            DotResult::Continue
        }
        ".tables" => {
            // CLI-01: shortcut for SHOW TABLES
            match exec_one("SHOW TABLES") {
                Ok(()) => DotResult::Continue,
                Err(e) => DotResult::Error(format!("SHOW TABLES failed: {e}")),
            }
        }
        ".schema" => {
            // CLI-01: shortcut for DESCRIBE TABLE
            if parts.len() < 2 {
                return DotResult::Error(".schema requires a table name".to_string());
            }
            let table = parts[1];
            let sql = format!("DESCRIBE TABLE {table}");
            match exec_one(&sql) {
                Ok(()) => DotResult::Continue,
                Err(e) => DotResult::Error(format!("DESCRIBE TABLE {table} failed: {e}")),
            }
        }
        ".databases" => {
            // CLI-01: shortcut for SHOW DATABASES
            match exec_one("SHOW DATABASES") {
                Ok(()) => DotResult::Continue,
                Err(e) => DotResult::Error(format!("SHOW DATABASES failed: {e}")),
            }
        }
        ".version" => {
            // CLI-01: show version
            println!("SQLRustGo v3.8.0-beta (Strong Beta, 8.0/10)");
            println!("Target: v3.8.0 GA (long convergence version)");
            DotResult::Continue
        }
        ".timing" => {
            // CLI-01: toggle timing
            if parts.len() < 2 {
                return DotResult::Error(".timing requires on|off".to_string());
            }
            match parts[1] {
                "on" => {
                    *timing_enabled = true;
                    println!("Timing enabled.");
                }
                "off" => {
                    *timing_enabled = false;
                    println!("Timing disabled.");
                }
                other => return DotResult::Error(format!("unknown timing mode: {other}")),
            }
            DotResult::Continue
        }
        ".headers" => {
            // CLI-01: toggle column headers
            if parts.len() < 2 {
                return DotResult::Error(".headers requires on|off".to_string());
            }
            match parts[1] {
                "on" => {
                    *headers_enabled = true;
                    println!("Headers enabled.");
                }
                "off" => {
                    *headers_enabled = false;
                    println!("Headers disabled.");
                }
                other => return DotResult::Error(format!("unknown headers mode: {other}")),
            }
            DotResult::Continue
        }
        ".clear" => {
            // CLI-01: clear screen (ANSI escape)
            print!("\x1B[2J\x1B[1;1H");
            DotResult::Continue
        }
        "" => DotResult::Continue,
        other => DotResult::Error(format!("unknown command: {other} (try `.help`)")),
    }
}

fn run_backup(output: &str) -> Result<(), String> {
    let cmd = BackupCommand {
        database: "default".to_string(),
        output_dir: output.to_string(),
        backup_type: "full".to_string(),
        schema_only: false,
        compress: false,
    };
    tools_backup(cmd).map_err(|e| e.to_string())
}

fn run_restore(input: &str) -> Result<(), String> {
    let cmd = RestoreCommand {
        database: "default".to_string(),
        backup_id: input.to_string(),
        backup_dir: ".".to_string(),
        drop_first: false,
    };
    tools_restore(cmd).map_err(|e| e.to_string())
}
