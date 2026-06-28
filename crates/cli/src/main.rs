//! SQLRustGo CLI — exec, repl, soak.
//!
//! Usage:
//!   sqlrustgo-cli exec [options] <sql>
//!   sqlrustgo-cli repl [options]
//!   sqlrustgo-cli soak [options]
//!
//! Connects to a running `sqlrustgo-mysql-server` via MySQL wire protocol.

use clap::{Parser, Subcommand};

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

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Exec {
            host,
            port,
            user,
            password,
            sql,
            json,
        } => {
            if let Err(e) = sqlrustgo_cli::exec::run_exec(&host, port, &user, &password, &sql, json)
            {
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
            if let Err(e) = sqlrustgo_cli::repl::run_repl(&host, port, &user, &password) {
                eprintln!("REPL error: {e}");
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
            let config = sqlrustgo_cli::soak::SoakConfig {
                host,
                port,
                user,
                password,
                duration_secs: duration,
                target_qps: rate,
                query_file,
                report_interval,
            };
            match sqlrustgo_cli::soak::run_soak(&config) {
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
                        sqlrustgo_cli::soak::print_report(&report);
                    }
                    if report.errors > 0 || report.queries_executed == 0 {
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("SOAK error: {e}");
                    std::process::exit(1);
                }
            }
        }
    }
}
