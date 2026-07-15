//! Integration test suite check
//!
//! run():     runs all integration tests
//! subset():  lightweight smoke (cbo_integration_test + ci_test)
//! full_suite(): deprecated, use run()

use crate::workspace::workspace_root;
use std::process::Command;

fn run_integration(args: &[&str]) -> anyhow::Result<()> {
    let root = workspace_root();
    let output = Command::new("cargo")
        .args(args)
        .current_dir(&root)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("[integration] FAILED\n{}", stderr);
        anyhow::bail!("integration tests failed");
    }

    println!("[check] integration OK");
    Ok(())
}

pub fn run() -> anyhow::Result<()> {
    // Run all available integration tests (exclude long-running benchmarks)
    println!("[check] running all integration tests ...");
    run_integration(&["test", "--all-features", "--", "--test-threads=8"])
}

pub fn subset() -> anyhow::Result<()> {
    // Preflight: smoke tests — fast and representative
    println!("[check] running integration subset (cbo_integration_test + ci_test) ...");
    let root = workspace_root();
    let tests = ["cbo_integration_test", "ci_test"];

    for test in &tests {
        println!("  running {test} ...");
        let output = std::process::Command::new("cargo")
            .args(["test", "--all-features", "--test", test])
            .current_dir(&root)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("[integration] {test} FAILED\n{}", stderr);
            anyhow::bail!("integration test {test} failed");
        }
        println!("  {test} OK");
    }

    Ok(())
}

pub fn full_suite() -> anyhow::Result<()> {
    // Alias for run() — full integration suite
    run()
}
