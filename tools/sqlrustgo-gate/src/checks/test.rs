//! Test checks: unit / integration / all

use crate::workspace::workspace_root;
use std::process::Command;

fn run_test(args: &[&str]) -> anyhow::Result<()> {
    let root = workspace_root();
    let output = Command::new("cargo")
        .args(args)
        .current_dir(&root)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("[test] FAILED\n{}", stderr);
        anyhow::bail!("test failed");
    }

    Ok(())
}

pub fn unit() -> anyhow::Result<()> {
    println!("[check] cargo test --all-features --quiet ...");
    run_test(&["test", "--all-features", "--quiet"])
}

pub fn integration() -> anyhow::Result<()> {
    println!("[check] cargo test --all-features --test integration ...");
    run_test(&["test", "--all-features", "--test", "integration"])
}

pub fn all() -> anyhow::Result<()> {
    println!("[check] cargo test --all ...");
    run_test(&["test", "--all"])
}
