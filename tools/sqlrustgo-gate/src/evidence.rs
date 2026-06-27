//! Evidence collection and export
//!
//! Evidence is the SSOT (Single Source of Truth) for gate execution results.
//! This module collects all check outputs into a machine-readable JSON report.

use chrono::Utc;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
pub struct Evidence {
    pub version: String,
    pub stage: String,
    pub timestamp: String,
    pub checks: HashMap<String, CheckResult>,
    pub build: bool,
    pub tests: bool,
    pub clippy: bool,
    pub coverage_pct: Option<f64>,
    pub sql_corpus_pct: Option<f64>,
    pub integration_passed: bool,
    pub perf_regression_pct: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct CheckResult {
    pub passed: bool,
    pub details: String,
}

pub fn collect(stage: &str) -> Evidence {
    Evidence {
        version: "v3.7.0".to_string(),
        stage: stage.to_string(),
        timestamp: Utc::now().to_rfc3339(),
        checks: HashMap::new(),
        build: false,
        tests: false,
        clippy: false,
        coverage_pct: None,
        sql_corpus_pct: None,
        integration_passed: false,
        perf_regression_pct: None,
    }
}

pub fn export(path: &str) -> anyhow::Result<()> {
    let evidence = collect("manual");

    let json = serde_json::to_string_pretty(&evidence)?;
    std::fs::write(path, json)?;

    println!("[evidence] exported to {}", path);
    Ok(())
}

pub fn report() -> anyhow::Result<()> {
    println!("============================================");
    println!("  SQLRustGo Release Gate — Evidence Report");
    println!("  v3.7.0");
    println!("============================================");
    println!();
    println!("No evidence file found. Run 'sqlrustgo-gate run alpha' first.");
    println!("Evidence will be written to: target/release/evidence_v3.7.0.json");
    println!();
    Ok(())
}
