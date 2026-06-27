//! Clippy checks

use crate::workspace::workspace_root;
use std::process::Command;

fn run_clippy(args: &[&str]) -> anyhow::Result<()> {
    let root = workspace_root();
    let output = Command::new("cargo")
        .args(args)
        .current_dir(&root)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprint!("[clippy] FAILED\n{}", stderr);
        anyhow::bail!("clippy found warnings/errors");
    }

    println!("[check] clippy OK");
    Ok(())
}

pub fn check() -> anyhow::Result<()> {
    println!("[check] cargo clippy --all-features -D warnings ...");
    run_clippy(&["clippy", "--all-features", "--", "-D", "warnings"])
}

pub fn check_strict() -> anyhow::Result<()> {
    println!("[check] cargo clippy --all-features -D warnings (strict mode) ...");
    run_clippy(&["clippy", "--all-features", "--", "-D", "warnings"])
}
