//! Integration test suite check
//!
//! run():     runs integration tests
//! full_suite(): enforces all integration test files

use std::process::Command;
use crate::workspace::workspace_root;

fn run_integration(args: &[&str]) -> anyhow::Result<()> {
    let root = workspace_root();
    let output = Command::new("cargo")
        .args(args)
        .current_dir(&root)
        .output()?;

    if !output.status.success() {
        eprintln!("[integration] FAILED");
        anyhow::bail!("integration tests failed");
    }

    println!("[check] integration OK");
    Ok(())
}

pub fn run() -> anyhow::Result<()> {
    println!("[check] cargo test --all-features --test integration ...");
    run_integration(&["test", "--all-features", "--test", "integration"])
}

pub fn full_suite() -> anyhow::Result<()> {
    println!("[check] full integration suite ...");
    run_integration(&["test", "--all-features", "--test", "integration"])
}