//! Test Runner Binary
//!
//! Entry point for the regression test runner.

use std::sync::Arc;
use test_runner::{TestRunConfig, TestRunner};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Test Runner");
    println!("==========");
    println!();

    let config = TestRunConfig::default();
    let runner = Arc::new(TestRunner::new(config));

    println!("Test Runner initialized");
    println!("Usage:");
    println!("  test-runner run <test-id>    - Run a single test");
    println!("  test-runner list             - List available tests");
    println!("  test-runner run-all          - Run all managed tests");

    Ok(())
}
