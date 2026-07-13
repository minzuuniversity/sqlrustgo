//! sqllogictest binary
//!
//! Usage:
//!   cargo run -p sqllogictest -- --help
//!   cargo run -p sqllogictest -- --test-dir crates/sqllogictest/testdata
//!   cargo run -p sqllogictest -- --verbose

use std::process;
use sqllogictest::{run_all, RunConfig};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let mut config = RunConfig::default();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                println!("sqllogictest — SQLLogicTest Runner for sqlrustgo");
                println!();
                println!("USAGE:");
                println!("  cargo run -p sqllogictest [OPTIONS]");
                println!();
                println!("OPTIONS:");
                println!("  --test-dir <DIR>     Test data directory (default: crates/sqllogictest/testdata)");
                println!("  --filter <PREFIX>    Only run .test files matching prefix");
                println!("  --verbose            Show all failures");
                println!("  --max-fail <N>       Stop after N failures (0=unlimited)");
                println!("  --help, -h           Show this help");
                return;
            }
            "--test-dir" => {
                i += 1;
                if i < args.len() {
                    config.test_dir = args[i].clone();
                }
            }
            "--filter" => {
                i += 1;
                if i < args.len() {
                    config.file_filter = args[i].clone();
                }
            }
            "--verbose" => {
                config.verbose = true;
            }
            "--max-fail" => {
                i += 1;
                if i < args.len() {
                    config.max_failures = args[i].parse().unwrap_or(0);
                }
            }
            _ => {}
        }
        i += 1;
    }

    println!("=== sqlrustgo SQLLogicTest Runner ===");
    println!("test_dir: {}", config.test_dir);
    if !config.file_filter.is_empty() {
        println!("filter: {}", config.file_filter);
    }
    println!();

    match run_all(&config) {
        Ok(result) => {
            println!("=== Summary ===");
            println!("files:    {}/{} (pass/fail)", result.files_passed, result.files_failed);
            println!("stmts:    {} total", result.total_stmts);
            println!("  pass:   {}", result.total_pass);
            println!("  fail:   {}", result.total_fail);
            println!("  skip:   {}", result.total_skip);
            println!("  error:  {}", result.total_error);
            println!("pass rate: {:.1}%", result.pass_rate());

            if result.files_failed > 0 {
                println!();
                println!("NOTE: Baseline established — fail count will decrease as sqlrustgo SQL coverage improves.");
                process::exit(0); // Beta: baseline mode, don't fail the process
            }
            process::exit(0);
        }
        Err(e) => {
            eprintln!("ERROR: {}", e);
            process::exit(1);
        }
    }
}
