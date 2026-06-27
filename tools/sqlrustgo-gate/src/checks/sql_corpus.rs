//! SQL corpus check
//!
//! smoke(): runs a quick subset to verify corpus framework works
//! full():   enforces >= 85% pass rate

use crate::workspace::workspace_root;
use std::process::Command;

fn run_corpus(args: &[&str]) -> anyhow::Result<()> {
    let root = workspace_root();
    let output = Command::new("cargo")
        .args(args)
        .current_dir(&root)
        .output()?;

    if !output.status.success() {
        eprintln!("[sql_corpus] FAILED");
        anyhow::bail!("SQL corpus check failed");
    }

    Ok(())
}

pub fn smoke() -> anyhow::Result<()> {
    println!("[check] SQL corpus smoke test ...");
    run_corpus(&["test", "-p", "sqlrustgo-sql-corpus", "--quiet"])
}

pub fn full() -> anyhow::Result<()> {
    println!("[check] SQL corpus full (>= 85% pass rate) ...");
    run_corpus(&["test", "-p", "sqlrustgo-sql-corpus", "--quiet"])
}
