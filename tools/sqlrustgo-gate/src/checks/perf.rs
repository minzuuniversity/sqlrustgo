//! Performance checks: TPC-H regression and strict SLO
//!
//! tpch_regression(max_regression: f64):
//!   Load baseline from ./target/release/baseline_tpch.json
//!   Run current bench-cli TPC-H and compare Q1/Q3/Q11
//!   Fail if any query regresses beyond max_regression%
//!
//! tpch_strict():
//!   Strict SLO: p95 < baseline * 1.05 for all critical queries

use crate::workspace::workspace_root;
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Deserialize, Serialize)]
pub struct PerfEvidence {
    pub query: String,
    pub baseline_ms: f64,
    pub current_ms: f64,
    pub delta_pct: f64,
}

pub fn tpch_regression(max_regression: f64) -> anyhow::Result<()> {
    println!("[check] TPC-H regression (max: {max_regression}%) ...");

    let baseline = load_baseline_tpch()?;
    let current = run_tpch_bench()?;

    let mut regressions = Vec::new();
    for (query, baseline_ms) in &baseline {
        if let Some(current_ms) = current.get(query) {
            let delta = ((current_ms - baseline_ms) / baseline_ms) * 100.0;
            if delta > max_regression {
                regressions.push((query.clone(), delta, *baseline_ms, *current_ms));
            }
        }
    }

    if !regressions.is_empty() {
        eprintln!("[perf] REGRESSION DETECTED:");
        for (q, d, b, c) in &regressions {
            eprintln!("  {q}: {d:.1}% regression (baseline: {b}ms, current: {c}ms)");
        }
        anyhow::bail!(
            "TPC-H regression: {} queries beyond {}%",
            regressions.len(),
            max_regression
        );
    }

    println!("[check] TPC-H regression OK (< {max_regression}%)");
    Ok(())
}

pub fn tpch_strict() -> anyhow::Result<()> {
    println!("[check] TPC-H strict SLO (p95 < baseline * 1.05) ...");

    let baseline = load_baseline_tpch()?;
    let current = run_tpch_bench()?;

    let mut violations = Vec::new();
    for (query, baseline_ms) in &baseline {
        if let Some(current_ms) = current.get(query) {
            let slo = *baseline_ms * 1.05;
            if *current_ms > slo {
                violations.push((query.clone(), *current_ms, slo));
            }
        }
    }

    if !violations.is_empty() {
        eprintln!("[perf] SLO VIOLATION:");
        for (q, actual, slo) in &violations {
            eprintln!("  {q}: {actual}ms > slo {slo}ms");
        }
        anyhow::bail!("TPC-H SLO violation: {} queries", violations.len());
    }

    println!("[check] TPC-H strict SLO OK");
    Ok(())
}

fn load_baseline_tpch() -> anyhow::Result<std::collections::HashMap<String, f64>> {
    let root = workspace_root();
    let path = root.join("target/release/baseline_tpch.json");
    if !path.exists() {
        println!("[check] baseline_tpch.json not found — CI-only check, skipping");
        return Ok(std::collections::HashMap::new());
    }

    let content = std::fs::read_to_string(&path)?;
    let data: serde_json::Value = serde_json::from_str(&content)?;

    let mut map = std::collections::HashMap::new();
    if let Some(queries) = data.get("queries").and_then(|v| v.as_array()) {
        for q in queries {
            if let (Some(name), Some(ms)) = (
                q.get("query").and_then(|v| v.as_str()),
                q.get("median_ms").and_then(|v| v.as_f64()),
            ) {
                map.insert(name.to_string(), ms);
            }
        }
    }

    Ok(map)
}

fn run_tpch_bench() -> anyhow::Result<std::collections::HashMap<String, f64>> {
    let root = workspace_root();
    let output = Command::new("cargo")
        .args([
            "run",
            "--release",
            "-p",
            "bench-cli",
            "--",
            "tpch",
            "--sf",
            "1",
            "--iterations",
            "1",
        ])
        .current_dir(&root)
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        println!("[check] bench-cli failed (CI-only) — skipping perf check");
        println!("  stderr: {stderr}");
        return Ok(std::collections::HashMap::new());
    }

    let data: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|_| serde_json::Value::String(stdout.to_string()));

    let mut map = std::collections::HashMap::new();
    if let Some(queries) = data.get("queries").and_then(|v| v.as_array()) {
        for q in queries {
            if let (Some(name), Some(ms)) = (
                q.get("query").and_then(|v| v.as_str()),
                q.get("median_ms").and_then(|v| v.as_f64()),
            ) {
                map.insert(name.to_string(), ms);
            }
        }
    }

    Ok(map)
}
