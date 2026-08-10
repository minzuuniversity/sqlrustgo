//! V312-24 Phase 1: sqlancer CLI smoke integration test.
//!
//! Verifies that the `sqlancer` binary (built by the same cargo
//! invocation as the test) can be invoked as a subprocess, runs to
//! completion, and produces a non-empty JSON report at the requested
//! output path. The report is also validated for shape.
//!
//! `env!("CARGO_BIN_EXE_sqlancer")` resolves to the absolute path of
//! the binary as built by the test runner — see
//! <https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates>.

use std::path::PathBuf;
use std::process::Command;

use serde::Deserialize;
use tempfile::tempdir;
/// Subset of `FuzzerResult` we assert on. `deny_unknown_fields` makes
/// the schema explicit; if a future change drops/renames a field, the
/// deserializer will surface it loudly. `errors` is held for shape
/// validation (it must exist in the report) but is not asserted on.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReportShape {
    successful_queries: u64,
    failed_queries: u64,
    timeout: bool,
    #[allow(dead_code)]
    errors: Vec<String>,
    duration_secs: f64,
    iterations_requested: u64,
}

fn sqlancer_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sqlancer"))
}

#[test]
fn cli_runs_writes_report_and_exits_zero() {
    let dir = tempdir().expect("tempdir");
    let out = dir.path().join("sqlancer-report.json");

    let status = Command::new(sqlancer_bin())
        .arg("--duration")
        .arg("1")
        .arg("--iterations")
        .arg("5")
        .arg("--out")
        .arg(&out)
        .status()
        .expect("spawn sqlancer");

    assert!(status.success(), "sqlancer exited non-zero: {:?}", status);
    assert!(
        out.exists(),
        "report file was not written at {}",
        out.display()
    );

    let raw = std::fs::read_to_string(&out).expect("read report");
    assert!(!raw.trim().is_empty(), "report is empty");
    let report: ReportShape =
        serde_json::from_str(&raw).expect("report must deserialize to FuzzerResult shape");

    // Parse-only oracle on 5 random queries: at least one should pass
    // (most random SQL is valid; a few may be syntactically invalid and
    // counted as failed). The structural invariants are stricter.
    assert_eq!(
        report.iterations_requested, 5,
        "iterations_requested mismatch"
    );
    assert_eq!(
        report.successful_queries + report.failed_queries,
        5,
        "successful + failed should equal total iterations"
    );
    assert!(
        !report.timeout,
        "1s budget should be plenty for 5 parse-only iterations"
    );
    assert!(report.duration_secs >= 0.0);
    assert!(
        report.duration_secs < 2.0,
        "5 iterations should not exceed 2s, took {}",
        report.duration_secs
    );
}

#[test]
fn cli_help_exits_zero() {
    // `--help` is a documentation path; verify it doesn't crash.
    let output = Command::new(sqlancer_bin())
        .arg("--help")
        .output()
        .expect("spawn sqlancer --help");
    assert!(output.status.success(), "--help should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);
    assert!(
        combined.contains("sqlancer"),
        "help text should mention 'sqlancer'"
    );
    assert!(combined.contains("--duration"));
}
