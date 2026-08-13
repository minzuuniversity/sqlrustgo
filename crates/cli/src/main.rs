//! SQLRustGo CLI — exec, repl, soak.
//!
//! Usage:
//!   sqlrustgo-cli exec [options] <sql>
//!   sqlrustgo-cli repl [options]
//!   sqlrustgo-cli soak [options]
//!
//! Connects to a running `sqlrustgo-mysql-server` via MySQL wire protocol.

use clap::parser::ValueSource;
use clap::{CommandFactory, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "sqlrustgo-cli", about = "SQLRustGo CLI client")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Execute a single SQL query and print results
    Exec {
        /// Server host
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        /// Server port
        #[arg(long, default_value_t = 3306)]
        port: u16,
        /// Username
        #[arg(long, default_value = "root")]
        user: String,
        /// Password
        #[arg(long, default_value = "")]
        password: String,
        /// SQL statement to execute
        #[arg(required = true)]
        sql: String,
        /// JSON output format
        #[arg(long)]
        json: bool,
    },
    /// Interactive REPL shell
    Repl {
        /// Server host
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        /// Server port
        #[arg(long, default_value_t = 3306)]
        port: u16,
        /// Username
        #[arg(long, default_value = "root")]
        user: String,
        /// Password
        #[arg(long, default_value = "")]
        password: String,
    },
    /// Real SQL SOAK test runner
    Soak {
        /// Server host
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        /// Server port
        #[arg(long, default_value_t = 3306)]
        port: u16,
        /// Username
        #[arg(long, default_value = "root")]
        user: String,
        /// Password
        #[arg(long, default_value = "")]
        password: String,
        /// Wall-clock duration in seconds
        #[arg(long, default_value_t = 60)]
        duration: u64,
        /// Target queries per second (0 = max speed)
        #[arg(long, default_value_t = 5.0)]
        rate: f64,
        /// Path to query file (one SQL per line)
        #[arg(long)]
        query_file: Option<String>,
        /// Progress report interval in seconds
        #[arg(long, default_value_t = 10)]
        report_interval: u64,
        /// Output report as JSON
        #[arg(long)]
        json: bool,
    },
}

/// True iff every one of `host`, `port`, `user`, `password` came from
/// its `default_value` (i.e. the user did NOT pass the flag explicitly).
///
/// V312-38 / Issue #4176: this drives the "you may have hit a system
/// MySQL on 127.0.0.1:3306" hint. Showing it when the user has
/// explicitly pointed `--host X` at some other server would be a false
/// positive and noise.
///
/// `matches` MUST be the matches returned by `Cli::command().get_matches()`
/// (or the subcommand matches inside it) — not the parsed struct, which
/// loses the value-origin information.
fn all_connection_args_default(matches: &clap::ArgMatches) -> bool {
    ["host", "port", "user", "password"]
        .iter()
        .all(|id| matches.value_source(id) == Some(ValueSource::DefaultValue))
}

/// Print a connection-failure diagnostic block. V312-38 / Issue #4176.
///
/// Always shows:
///   * the underlying reason (from `run_repl` / `run_exec` / `run_soak`)
///   * the effective connection target and credentials mode
///     (NEVER echoes the password)
///
/// Shows context-dependent hints:
///   * if all connection args are defaults AND the reason mentions
///     "auth" / "Access denied" → suggest the user may have hit a
///     system MySQL/MariaDB instance listening on 127.0.0.1:3306 and
///     should start `sqlrustgo-mysql-server` first
///   * if the reason mentions "Connection refused" / "No route" →
///     suggest the user start `sqlrustgo-mysql-server`
fn print_connection_diag(
    subcommand: &str,
    host: &str,
    port: u16,
    user: &str,
    password_bytes: usize,
    reason: &str,
    matches: &clap::ArgMatches,
) {
    eprintln!("{subcommand} connection failed: {reason}");
    eprintln!("  Target:      {host}:{port}");
    eprintln!("  User:        {user}");
    let creds = if password_bytes == 0 {
        "empty password".to_string()
    } else {
        // Show byte count only — never echo the password itself.
        format!("password supplied ({} bytes)", password_bytes)
    };
    eprintln!("  Credentials: {creds}");

    let lower = reason.to_ascii_lowercase();
    let auth_failure = lower.contains("auth") || lower.contains("access denied");
    let no_listener = lower.contains("connection refused")
        || lower.contains("no route to host")
        || lower.contains("connection reset");

    if all_connection_args_default(matches) && auth_failure {
        eprintln!();
        eprintln!(
            "Hint: 127.0.0.1:3306 may belong to a system MySQL/MariaDB instance, \
             not sqlrustgo-mysql-server."
        );
        eprintln!(
            "      Start sqlrustgo-mysql-server first, or pass --host/--port \
             explicitly."
        );
    } else if no_listener {
        eprintln!();
        eprintln!(
            "Hint: no MySQL listener on {host}:{port}. Start \
             sqlrustgo-mysql-server, or pass --host/--port to point at a \
             running server."
        );
    }
}

fn main() {
    // V312-38 / Issue #4176: parse twice — once for the typed struct
    // (so the rest of the dispatch stays unchanged), once for the raw
    // `ArgMatches` so we can ask clap whether each connection arg came
    // from `default_value` or was supplied on the command line.
    let cli = Cli::parse();
    let raw = Cli::command().get_matches();

    match cli.command {
        Command::Exec {
            host,
            port,
            user,
            password,
            sql,
            json,
        } => {
            let pw_len = password.len();
            if let Err(e) =
                sqlrustgo_soak::exec::run_exec(&host, port, &user, &password, &sql, json)
            {
                print_connection_diag("exec", &host, port, &user, pw_len, &e.to_string(),
                    raw.subcommand_matches("exec").expect("exec subcommand"));
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
        Command::Repl {
            host,
            port,
            user,
            password,
        } => {
            let pw_len = password.len();
            if let Err(e) = sqlrustgo_soak::repl::run_repl(&host, port, &user, &password) {
                print_connection_diag("REPL", &host, port, &user, pw_len, &e.to_string(),
                    raw.subcommand_matches("repl").expect("repl subcommand"));
                eprintln!("Error: {e}");
                std::process::exit(1);
            }
        }
        Command::Soak {
            host,
            port,
            user,
            password,
            duration,
            rate,
            query_file,
            report_interval,
            json,
        } => {
            // V312-38 / Issue #4176: capture host/user/password slices
            // BEFORE moving the Strings into SoakConfig, so the
            // diagnostic block can still print them on failure.
            let host_diag = host.clone();
            let user_diag = user.clone();
            let pw_len = password.len();
            let config = sqlrustgo_soak::soak::SoakConfig {
                host,
                port,
                user,
                password,
                duration_secs: duration,
                target_qps: rate,
                query_file,
                report_interval,
            };
            match sqlrustgo_soak::soak::run_soak(&config) {
                Ok(report) => {
                    if json {
                        let output = serde_json::to_string_pretty(&serde_json::json!({
                            "duration_secs": report.duration_secs,
                            "queries_executed": report.queries_executed,
                            "errors": report.errors,
                            "p50_latency_ms": report.p50_latency_ms,
                            "p90_latency_ms": report.p90_latency_ms,
                            "p99_latency_ms": report.p99_latency_ms,
                            "avg_latency_ms": report.avg_latency_ms,
                            "max_latency_ms": report.max_latency_ms,
                            "actual_qps": report.actual_qps,
                        }));
                        match output {
                            Ok(json) => println!("{json}"),
                            Err(e) => eprintln!("JSON error: {e}"),
                        }
                    } else {
                        sqlrustgo_soak::soak::print_report(&report);
                    }
                    if report.errors > 0 || report.queries_executed == 0 {
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    print_connection_diag("SOAK", &host_diag, port, &user_diag, pw_len,
                        &e.to_string(),
                        raw.subcommand_matches("soak").expect("soak subcommand"));
                    eprintln!("SOAK error: {e}");
                    std::process::exit(1);
                }
            }
        }
    }
}
