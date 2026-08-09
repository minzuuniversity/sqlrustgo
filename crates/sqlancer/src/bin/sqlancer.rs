//! SQLancer - SQL Fuzz Testing Binary
//!
//! Entry point for the SQL fuzz testing tool.

use sqlancer::{Fuzzer, FuzzerConfig};
use std::process;

fn main() {
    println!("SQLancer - SQL Fuzz Testing Tool");
    println!("=================================");

    let config = FuzzerConfig {
        max_iterations: 1000,
        max_table_size: 100,
        thread_count: 1,
        timeout_ms: 60000,
    };

    println!("Config: {:?}", config);

    let mut fuzzer = Fuzzer::new(config);

    // Simple executor that prints generated SQL
    let result = fuzzer.run(|sql| {
        println!("Generated SQL: {}", sql);
        Ok(())
    });

    println!("\nResults:");
    println!("  Successful queries: {}", result.successful_queries);
    println!("  Failed queries: {}", result.failed_queries);
    println!("  Timeout: {}", result.timeout);
    println!("  Duration: {:.2}s", result.duration_secs);

    if !result.errors.is_empty() {
        println!("\nErrors:");
        for error in &result.errors {
            println!("  {}", error);
        }
    }

    if result.failed_queries > 0 {
        println!("\nWarning: {} queries failed during fuzzing", result.failed_queries);
    }

    process::exit(0);
}
