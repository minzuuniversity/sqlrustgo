//! SQLRustGo Canonical CLI Entry Point
//!
//! Thin wrapper around `sqlrustgo-mysql-server` for most subcommands.
//! The `cli` subcommand is the new interactive client.

use clap::{Parser, Subcommand};
use std::process::{Command as Proc, ExitCode};

#[derive(Parser, Debug)]
#[command(
    name = "sqlrustgo",
    about = "SQLRustGo canonical CLI",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Option<SubCmd>,
}

#[derive(Subcommand, Debug)]
enum SubCmd {
    /// Start the MySQL wire-protocol server.
    Serve {
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long, default_value = "3306")]
        port: u16,
        #[arg(long, default_value = "/tmp/sqlrustgo-data")]
        data_dir: String,
        #[arg(long, default_value_t = 100)]
        max_connections: usize,
        #[arg(long, default_value_t = 16)]
        server_threads: usize,
        #[arg(long, default_value = "none")]
        auth_mode: String,
    },
    /// Execute a single SQL statement and print the result.
    Exec { sql: String },
    /// Interactive REPL.
    Repl {
        #[arg(long)]
        init_sql: Option<String>,
        #[arg(long)]
        save_on_exit: Option<String>,
    },
    Bench,
    Gmp,
    Diag,
    Backup { output_dir: String },
    Restore { backup_id: String, database: String },
    /// Connect to a running server and execute a query (NEW).
    Cli {
        query: String,
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(short = 'P', long, default_value_t = 3306)]
        port: u16,
        #[arg(short = 'u', long, default_value = "tester")]
        user: String,
        #[arg(short = 'p', long, default_value = "tester")]
        password: String,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let cmd = cli.command.unwrap_or(SubCmd::Serve {
        host: "127.0.0.1".to_string(),
        port: 3306,
        data_dir: "/tmp/sqlrustgo-data".to_string(),
        max_connections: 100,
        server_threads: 16,
        auth_mode: "none".to_string(),
    });

    match cmd {
        SubCmd::Serve { host, port, data_dir, max_connections, server_threads, auth_mode } => {
            run_bin("serve", &[
                ("--host", host),
                ("--port", port.to_string()),
                ("--data-dir", data_dir),
                ("--max-connections", max_connections.to_string()),
                ("--server-threads", server_threads.to_string()),
                ("--auth-mode", auth_mode),
            ])
        }
        SubCmd::Exec { sql } => {
            // mysql-server exec takes positional <SQL>
            run_bin_arg_positional("exec", &sql)
        }
        SubCmd::Repl { init_sql, save_on_exit } => {
            let mut args = vec![];
            if let Some(f) = init_sql {
                args.push("--init-sql".to_string());
                args.push(f);
            }
            if let Some(f) = save_on_exit {
                args.push("--save-on-exit".to_string());
                args.push(f);
            }
            run_bin_args("repl", &args)
        }
        SubCmd::Bench => run_bin("bench", &[]),
        SubCmd::Gmp => run_bin("gmp", &[]),
        SubCmd::Diag => run_bin("diag", &[]),
        SubCmd::Backup { output_dir } => {
            run_bin("backup", &[("--output-dir", output_dir)])
        }
        SubCmd::Restore { backup_id, database } => {
            run_bin("restore", &[
                ("--backup-id", backup_id),
                ("--database", database),
            ])
        }
        SubCmd::Cli { query, host, port, user, password } => {
            run_cli(&query, &host, port, &user, &password)
        }
    }
}

fn run_bin(subcmd: &str, args: &[(&str, String)]) -> ExitCode {
    let bin = mysql_server_bin();
    let mut cmd = Proc::new(&bin);
    cmd.arg(subcmd);
    for (k, v) in args {
        cmd.arg(k);
        cmd.arg(v);
    }
    exec_status(&bin, cmd)
}

fn run_bin_arg_positional(subcmd: &str, positional: &str) -> ExitCode {
    let bin = mysql_server_bin();
    let mut cmd = Proc::new(&bin);
    cmd.arg(subcmd);
    cmd.arg(positional);
    exec_status(&bin, cmd)
}

fn run_bin_args(subcmd: &str, args: &[String]) -> ExitCode {
    let bin = mysql_server_bin();
    let mut cmd = Proc::new(&bin);
    cmd.arg(subcmd);
    for a in args {
        cmd.arg(a);
    }
    exec_status(&bin, cmd)
}

fn mysql_server_bin() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|par| par.join("sqlrustgo-mysql-server")))
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| "sqlrustgo-mysql-server".to_string())
}

fn exec_status(bin: &str, mut cmd: Proc) -> ExitCode {
    match cmd.status() {
        Ok(s) => ExitCode::from(s.code().unwrap_or(1) as u8),
        Err(e) => {
            eprintln!("Failed to run {}: {}", bin, e);
            ExitCode::from(1)
        }
    }
}

// ============================================================================
// CLI subcommand: connect to server, execute query (Phase 3 skeleton)
// ============================================================================

fn run_cli(query: &str, host: &str, port: u16, _user: &str, _password: &str) -> ExitCode {
    use std::io::Read;
    use std::net::TcpStream;
    use std::time::Duration;

    tracing::info!("Connecting to {}:{}", host, port);

    let addr = format!("{}:{}", host, port);
    let mut stream = match TcpStream::connect_timeout(&addr.parse().unwrap(), Duration::from_secs(5)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Connection failed: {}", e);
            return ExitCode::from(1);
        }
    };

    stream.set_read_timeout(Some(Duration::from_secs(30))).ok();
    stream.set_write_timeout(Some(Duration::from_secs(30))).ok();

    // Read server MySQL Protocol handshake (HandshakeV10)
    let mut handshake = [0u8; 8192];
    match stream.read(&mut handshake) {
        Ok(n) => {
            tracing::debug!("Received {} byte handshake from server", n);
        }
        Err(e) => {
            eprintln!("Failed to read handshake: {}", e);
            return ExitCode::from(1);
        }
    }

    println!("Connected to {}:{}", host, port);
    println!("Executing: {}", query);
    println!("\n[Phase 3: Full client implementation pending — see V310_CLI_BINARY_PLAN.md]");

    ExitCode::from(0)
}
