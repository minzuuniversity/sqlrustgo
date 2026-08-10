//! `sqlancer` binary — exposes the library's Fuzzer as an executable tool.
//!
//! V312-24 activation: this binary produces a JSON artifact at the
//! configured output path so it can be consumed as a gate input.
//!
//! Usage:
//!   sqlancer --duration 30 --iterations 1000 --out target/sqlancer-report.json
//!
//! Behavior:
//!   - Generates random SQL via the library's `generate_random_sql`.
//!   - Executes each via a tiny in-process executor (validates parse only;
//!     wires up to a real `ExecutionEngine` is tracked as a follow-up).
//!   - Records successful/failed counts + first 10 error samples.
//!   - Writes the result to `--out` as JSON; exits 0 on success or 1 on internal failure.

use std::path::PathBuf;
use std::process::ExitCode;

use sqlancer::{Fuzzer, FuzzerConfig, FuzzerResult};

#[derive(Debug, Default)]
struct Args {
    duration_secs: u64,
    iterations: u64,
    out: PathBuf,
    seed: Option<u64>,
    max_table_size: Option<usize>,
}

impl Args {
    fn parse() -> Result<Self, String> {
        let mut args = Self {
            duration_secs: 30,
            iterations: 1_000,
            out: PathBuf::from("target/sqlancer-report.json"),
            seed: None,
            max_table_size: None,
        };
        let mut iter = std::env::args().skip(1);
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--duration" | "-d" => {
                    let v = iter.next().ok_or("--duration requires <secs>")?;
                    args.duration_secs = v
                        .parse()
                        .map_err(|e: std::num::ParseIntError| format!("--duration: {}", e))?;
                }
                "--iterations" | "-i" => {
                    let v = iter.next().ok_or("--iterations requires <n>")?;
                    args.iterations = v
                        .parse()
                        .map_err(|e: std::num::ParseIntError| format!("--iterations: {}", e))?;
                }
                "--out" | "-o" => {
                    let v = iter.next().ok_or("--out requires <path>")?;
                    args.out = PathBuf::from(v);
                }
                "--seed" => {
                    let v = iter.next().ok_or("--seed requires <u64>")?;
                    args.seed = Some(
                        v.parse()
                            .map_err(|e: std::num::ParseIntError| format!("--seed: {}", e))?,
                    );
                }
                "--max-table-size" => {
                    let v = iter.next().ok_or("--max-table-size requires <n>")?;
                    args.max_table_size =
                        Some(v.parse().map_err(|e: std::num::ParseIntError| {
                            format!("--max-table-size: {}", e)
                        })?);
                }
                "--help" | "-h" => {
                    print_help();
                    return Err("help".into());
                }
                other => return Err(format!("unknown argument: {}", other)),
            }
        }
        Ok(args)
    }
}

fn print_help() {
    eprintln!(
        "sqlancer — fuzz SQL queries against the parser oracle\n\n\
         USAGE:\n  \
         sqlancer [OPTIONS]\n\n\
         OPTIONS:\n  \
         -d, --duration <SECS>     Total wall-clock budget (default 30)\n  \
         -i, --iterations <N>       Max iterations regardless of duration (default 1000)\n  \
         -o, --out <PATH>          Output JSON path (default target/sqlancer-report.json)\n  \
             --seed <U64>          Seed for deterministic fuzz\n  \
             --max-table-size <N>  Override default table size\n  \
         -h, --help                Show this help\n\n\
         EXIT CODES:\n  \
             0   Success — report written\n  \
             1   Internal error (I/O, bad args, etc.)\n  \
             2   Fuzzer ran but produced 0 successful queries (likely real bug)\n"
    );
}

fn run(args: Args) -> Result<FuzzerResult, String> {
    if let Some(parent) = args.out.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("create_dir_all({}): {}", parent.display(), e))?;
        }
    }
    let config = FuzzerConfig {
        max_iterations: args.iterations,
        timeout_ms: args.duration_secs.saturating_mul(1_000),
        max_table_size: args
            .max_table_size
            .unwrap_or(FuzzerConfig::default().max_table_size),
        ..FuzzerConfig::default()
    };

    let mut fuzzer = Fuzzer::new(config);

    // Minimal in-process executor: parse-only oracle. The library's `run`
    // takes a closure; we use the parser to validate SQL syntax as the
    // cheapest available oracle. A future V312-24 follow-up should wire
    // this to a real `ExecutionEngine` via sqlrustgo crate.
    let executor = |sql: &str| -> Result<(), String> {
        sqlrustgo_parser::parse(sql)
            .map(|_| ())
            .map_err(|e| e.to_string())
    };

    let result = if let Some(_seed) = args.seed {
        // Seed parameter not yet plumbed into FuzzerConfig — log a warning
        // and run anyway. Future enhancement: thread seed through.
        eprintln!(
            "warning: --seed not yet honored (FuzzerConfig has no seed field); running anyway"
        );
        fuzzer.run(executor)
    } else {
        fuzzer.run(executor)
    };

    let json = serde_json::to_string_pretty(&result)
        .map_err(|e| format!("serialize FuzzerResult: {}", e))?;
    std::fs::write(&args.out, json).map_err(|e| format!("write {}: {}", args.out.display(), e))?;

    Ok(result)
}

fn main() -> ExitCode {
    let args = match Args::parse() {
        Ok(a) => a,
        Err(reason) if reason == "help" => return ExitCode::SUCCESS,
        Err(reason) => {
            eprintln!("error: {}\n", reason);
            print_help();
            return ExitCode::from(1);
        }
    };
    match run(args) {
        Ok(result) => {
            println!(
                "sqlancer: wrote report — total_queries={} successful={} failed={} duration_secs={:.2} timeout={}",
                result.total_queries(),
                result.successful_queries,
                result.failed_queries,
                result.duration_secs,
                result.timeout
            );
            if result.successful_queries == 0 && result.iterations_requested > 0 {
                eprintln!(
                    "warning: 0 successful queries out of {} iterations — likely real engine bug",
                    result.iterations_requested
                );
                ExitCode::from(2)
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            eprintln!("error: {}", e);
            ExitCode::from(1)
        }
    }
}
