//! sqlrustgo-perf-baseline
//!
//! Performance baseline and governance system for SQLRustGo.
//! Tracks benchmark results over time and detects performance regressions.

pub mod commands;
pub mod db;
pub mod models;

use std::path::PathBuf;

pub use commands::{CheckCmd, QueryCmd, SaveCmd};
pub use db::BaselineDb;
pub use models::{BenchmarkResult, MetricType, RegressionResult};

/// Get the default baseline database path
pub fn default_db_path() -> PathBuf {
    let mut path = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("sqlrustgo");
    path.push("perf_baseline");
    path
}

/// CLI entry point
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    match args[1].as_str() {
        "save" => {
            let commit = args.get(2).cloned().unwrap_or_else(|| {
                std::process::Command::new("git")
                    .args(["rev-parse", "HEAD"])
                    .output()
                    .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                    .unwrap_or_default()
            });
            let db_path = args
                .get(3)
                .map(PathBuf::from)
                .unwrap_or_else(default_db_path);
            let cmd = SaveCmd { commit, db_path };
            cmd.execute()?;
        }
        "query" => {
            let metric = args.get(2).cloned().unwrap_or_default();
            let range = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(7);
            let db_path = args
                .get(4)
                .map(PathBuf::from)
                .unwrap_or_else(default_db_path);
            let cmd = QueryCmd {
                metric,
                range_days: range,
                db_path,
            };
            cmd.execute()?;
        }
        "check" => {
            let baseline = args.get(2).cloned().unwrap_or_else(|| "HEAD~1".to_string());
            let head = args.get(3).cloned().unwrap_or_else(|| "HEAD".to_string());
            let threshold: f64 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.10);
            let db_path = args
                .get(5)
                .map(PathBuf::from)
                .unwrap_or_else(default_db_path);
            let cmd = CheckCmd {
                baseline,
                head,
                threshold,
                db_path,
            };
            cmd.execute()?;
        }
        _ => {
            print_usage();
        }
    }

    Ok(())
}

fn print_usage() {
    eprintln!("sqlrustgo-perf-baseline - Performance baseline and governance");
    eprintln!();
    eprintln!("Usage:");
    eprintln!("  perf-baseline save [commit] [db_path]     Save current benchmark results");
    eprintln!("  perf-baseline query <metric> [days] [db]  Query historical metrics");
    eprintln!("  perf-baseline check <base> <head> <thresh> [db]  Check for regressions");
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  perf-baseline save $(git rev-parse HEAD)");
    eprintln!("  perf-baseline query update_qps 30");
    eprintln!("  perf-baseline check HEAD~1 HEAD 0.10");
}
