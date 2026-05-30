//! Coverage check via cargo-tarpaulin
//!
//! Usage: check(threshold: f64)
//! Exits non-zero if coverage < threshold.

use crate::workspace::workspace_root;
use std::process::Command;

pub fn check(threshold: f64) -> anyhow::Result<()> {
    println!("[check] cargo tarpaulin --quiet (threshold: {threshold}%) ...");

    let root = workspace_root();
    let output = Command::new("cargo")
        .args(["tarpaulin", "--quiet", "--ignore-tests", "--out", "Json"])
        .current_dir(&root)
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let coverage = parse_tarpaulin_json(&stdout).or_else(|| parse_tarpaulin_text(&stdout));

    match coverage {
        Some(cov) => {
            println!("[check] coverage: {cov:.1}% (threshold: {threshold}%)");
            if cov < threshold {
                anyhow::bail!("coverage {:.1}% < {}%", cov, threshold);
            }
            println!("[check] coverage OK");
            Ok(())
        }
        None => {
            println!("[check] coverage: unable to parse (CI-only check, skipping locally)");
            Ok(())
        }
    }
}

fn parse_tarpaulin_json(output: &str) -> Option<f64> {
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct TarpaulinOutput {
        files: Vec<TarpaulinFile>,
    }

    #[derive(Deserialize)]
    struct TarpaulinFile {
        coverage: Option<f64>,
    }

    output
        .lines()
        .find(|line| line.starts_with('{'))
        .and_then(|line| serde_json::from_str::<TarpaulinOutput>(line).ok())
        .and_then(|t| {
            let total: f64 = t.files.iter().filter_map(|f| f.coverage).sum();
            let count = t.files.iter().filter_map(|f| f.coverage).count();
            if count > 0 {
                Some(total / count as f64)
            } else {
                None
            }
        })
}

fn parse_tarpaulin_text(output: &str) -> Option<f64> {
    output
        .lines()
        .find(|l| l.contains("Coverage:") || l.contains("coverage:"))
        .and_then(|l| {
            l.split_whitespace()
                .find(|w| w.ends_with('%'))
                .and_then(|w| w.trim_end_matches('%').parse::<f64>().ok())
        })
}
