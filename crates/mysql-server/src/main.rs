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

use clap::{Parser, Subcommand};
use sqlrustgo_mysql_server::run_server;
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
    });

    match command {
        Command::Serve { host, port } => {
            tracing::info!("SQLRustGo MySQL Server starting on {}:{}", host, port);
            if let Err(e) = run_server(&host, port) {
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
    }
}

fn exec_one(sql: &str) -> Result<(), String> {
    use sqlrustgo::{ExecutionEngine, MemoryStorage};
    use std::sync::{Arc, RwLock};
    let storage = Arc::new(RwLock::new(MemoryStorage::new()));
    let mut engine = ExecutionEngine::new(storage);
    match engine.execute(sql) {
        Ok(result) => {
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

fn run_repl() -> Result<(), String> {
    println!("SQLRustGo REPL — type `.exit` to quit");
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut buf = String::new();
    loop {
        print!("sqlrustgo> ");
        stdout.flush().map_err(|e| e.to_string())?;
        buf.clear();
        let mut handle = stdin.lock();
        let n = handle.read_line(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            println!();
            return Ok(());
        }
        let line = buf.trim();
        if line.is_empty() {
            continue;
        }
        if matches!(line, ".exit" | ".quit") {
            return Ok(());
        }
        if let Err(e) = exec_one(line) {
            eprintln!("Error: {e}");
        }
    }
}
