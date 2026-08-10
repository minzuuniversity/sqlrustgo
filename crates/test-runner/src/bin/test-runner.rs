//! `test-runner` binary — exposes the library's TestRunner as an executable.
//!
//! V312-24 activation: this binary produces a JSON artifact so it can be
//! consumed as a gate input. Two modes:
//!
//! 1. **Manifest mode** (preferred) — pass `--manifest <PATH>` to a
//!    `test-registry.toml` file. The runner loads the on-disk `[[test]]`
//!    entries and dispatches each `binary` + `args` combination in parallel
//!    (bounded by `max_parallel`), honoring each entry's `timeout_ms`.
//! 2. **Probe mode** (fallback) — without `--manifest`, the runner executes
//!    a `cargo --version` probe and reports it as a single synthetic entry.
//!    This keeps the artifact non-empty and exercises the runner end-to-end
//!    before a manifest is wired in.
//!
//! Usage:
//!   test-runner --manifest target/test-registry.toml
//!                  --max-parallel 4 --out target/test-runner-report.json
//!   test-runner --out target/test-runner-report.json
//!                  [--max-parallel 4] [--timeout-ms 120000]

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::Serialize;
use test_registry::{ManagedTest, TestRegistry};
use test_runner::{TestRunConfig, TestRunSummary, TestRunner, TestStatus};

#[derive(Debug, Serialize)]
struct RunReport {
    started_at: DateTime<Utc>,
    finished_at: DateTime<Utc>,
    config: TestRunConfigReport,
    mode: &'static str,
    summary: TestRunSummary,
    results: Vec<test_runner::TestResult>,
}

#[derive(Debug, Serialize)]
struct TestRunConfigReport {
    cargo_binary: String,
    working_dir: PathBuf,
    max_parallel: usize,
    retry_count: u32,
    timeout_per_test_ms: u64,
    test_flags: Vec<String>,
}

impl From<TestRunConfig> for TestRunConfigReport {
    fn from(c: TestRunConfig) -> Self {
        Self {
            cargo_binary: c.cargo_binary,
            working_dir: c.working_dir,
            max_parallel: c.max_parallel,
            retry_count: c.retry_count,
            timeout_per_test_ms: c.timeout_per_test_ms,
            test_flags: c.test_flags,
        }
    }
}

#[derive(Debug, Default)]
struct Args {
    manifest: Option<PathBuf>,
    max_parallel: Option<usize>,
    retry_count: Option<u32>,
    timeout_ms: Option<u64>,
    test_flags: Vec<String>,
    out: PathBuf,
}

fn print_help() {
    eprintln!(
        "test-runner — execute managed tests and write JSON report\n\n\
         USAGE:\n  \
         test-runner [OPTIONS]\n\n\
         OPTIONS:\n  \
             --manifest <PATH>      Load a test-registry.toml manifest\n  \
                                   (dispatches each [[test]] entry)\n  \
         -j, --max-parallel <N>      Override max_parallel (default num_cpus)\n  \
             --retry-count <N>      Override retry_count (default 0)\n  \
         -t, --timeout-ms <MS>      Default timeout for the probe fallback\n  \
                                   (managed entries use their own timeout_ms)\n  \
             --test <NAME>          Pass a --test filter (repeatable)\n  \
         -o, --out <PATH>           Output JSON path (default target/test-runner-report.json)\n  \
         -h, --help                 Show this help\n"
    );
}

impl Args {
    fn parse() -> Result<Self, String> {
        let mut args = Self {
            out: PathBuf::from("target/test-runner-report.json"),
            ..Default::default()
        };
        let mut iter = std::env::args().skip(1);
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--manifest" | "-m" => {
                    args.manifest = Some(PathBuf::from(
                        iter.next().ok_or("--manifest requires <path>")?,
                    ));
                }
                "--max-parallel" | "-j" => {
                    args.max_parallel = Some(
                        iter.next()
                            .ok_or("--max-parallel requires <N>")?
                            .parse()
                            .map_err(|e: std::num::ParseIntError| {
                                format!("--max-parallel: {}", e)
                            })?,
                    );
                }
                "--retry-count" => {
                    args.retry_count = Some(
                        iter.next()
                            .ok_or("--retry-count requires <N>")?
                            .parse()
                            .map_err(|e: std::num::ParseIntError| {
                                format!("--retry-count: {}", e)
                            })?,
                    );
                }
                "--timeout-ms" | "-t" => {
                    args.timeout_ms = Some(
                        iter.next()
                            .ok_or("--timeout-ms requires <ms>")?
                            .parse()
                            .map_err(|e: std::num::ParseIntError| format!("--timeout-ms: {}", e))?,
                    );
                }
                "--test" => {
                    args.test_flags
                        .push(iter.next().ok_or("--test requires <name>")?);
                }
                "--out" | "-o" => {
                    args.out = PathBuf::from(iter.next().ok_or("--out requires <path>")?);
                }
                "--help" | "-h" => {
                    print_help();
                    return Err("help".into());
                }
                other => return Err(format!("unknown argument: {}", other)),
            }
        }
        Ok(args)
    }
}

async fn run(args: Args) -> Result<RunReport, String> {
    if let Some(parent) = args.out.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("create_dir_all({}): {}", parent.display(), e))?;
        }
    }

    let mut config = TestRunConfig::default();
    if let Some(mp) = args.max_parallel {
        config.max_parallel = mp;
    }
    if let Some(rc) = args.retry_count {
        config.retry_count = rc;
    }
    if let Some(t) = args.timeout_ms {
        config.timeout_per_test_ms = t;
    }
    if !args.test_flags.is_empty() {
        config.test_flags = args.test_flags.clone();
    }

    let started_at = Utc::now();
    let runner = Arc::new(TestRunner::new(config.clone()));

    let (mode, results): (&'static str, Vec<test_runner::TestResult>) = match &args.manifest {
        Some(manifest_path) => {
            let registry = TestRegistry::from_toml(manifest_path)
                .map_err(|e| format!("load manifest {}: {}", manifest_path.display(), e))?;
            let entries: Vec<ManagedTest> = registry.managed().cloned().collect();
            if entries.is_empty() {
                (
                    "manifest-empty",
                    vec![synthetic(
                        "manifest-empty",
                        "manifest loaded but contained no [[test]] entries",
                    )],
                )
            } else {
                let results = runner.run_managed_all(entries).await;
                ("manifest", results)
            }
        }
        None => {
            // Probe mode: invoke `cargo --version` directly via a synthetic
            // `ManagedTest` so the report has non-trivial, passing content.
            // The library's `run_test` API expects a cargo test id, so we
            // route through `run_managed` instead.
            let probe_entry = ManagedTest::new("cargo-version-probe", "cargo")
                .with_args(vec!["--version".to_string()])
                .with_timeout_ms(args.timeout_ms.unwrap_or(30_000))
                .with_priority(test_registry::TestPriority::P3)
                .with_category(test_registry::TestCategory::CI);
            let probe = runner.run_managed(&probe_entry).await;
            ("probe", vec![probe])
        }
    };

    let finished_at = Utc::now();
    let summary = build_summary(&results);

    let report = RunReport {
        started_at,
        finished_at,
        config: config.into(),
        mode,
        summary,
        results,
    };
    let json =
        serde_json::to_string_pretty(&report).map_err(|e| format!("serialize report: {}", e))?;
    std::fs::write(&args.out, json).map_err(|e| format!("write {}: {}", args.out.display(), e))?;
    Ok(report)
}

fn synthetic(test_id: &str, msg: &str) -> test_runner::TestResult {
    let now = Utc::now();
    test_runner::TestResult {
        test_id: test_id.to_string(),
        name: msg.to_string(),
        status: TestStatus::Crashed,
        duration_ms: 0,
        started_at: now,
        finished_at: now,
        output: String::new(),
        error_message: Some(msg.to_string()),
        retries: 0,
    }
}

fn build_summary(results: &[test_runner::TestResult]) -> TestRunSummary {
    let mut s = TestRunSummary {
        total: results.len(),
        passed: 0,
        failed: 0,
        skipped: 0,
        timed_out: 0,
        crashed: 0,
        total_duration_ms: 0,
    };
    for r in results {
        match r.status {
            TestStatus::Passed => s.passed += 1,
            TestStatus::Failed => s.failed += 1,
            TestStatus::Skipped => s.skipped += 1,
            TestStatus::TimedOut => s.timed_out += 1,
            TestStatus::Crashed => s.crashed += 1,
            _ => {}
        }
        s.total_duration_ms = s.total_duration_ms.saturating_add(r.duration_ms);
    }
    s
}

fn main() -> ExitCode {
    let args = match Args::parse() {
        Ok(a) => a,
        Err(reason) if reason == "help" => return ExitCode::SUCCESS,
        Err(reason) => {
            eprintln!("error: {}\n", reason);
            print_help();
            return ExitCode::from(1);
        }
    };
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("tokio runtime: {}", e);
            return ExitCode::from(1);
        }
    };
    let result = runtime.block_on(run(args));
    match result {
        Ok(r) => {
            println!(
                "test-runner: mode={} wrote {} entries — total={} passed={} failed={} timed_out={} crashed={} duration_ms={}",
                r.mode,
                r.results.len(),
                r.summary.total,
                r.summary.passed,
                r.summary.failed,
                r.summary.timed_out,
                r.summary.crashed,
                r.summary.total_duration_ms
            );
            if r.summary.failed == 0 && r.summary.timed_out == 0 && r.summary.crashed == 0 {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(e) => {
            eprintln!("error: {}", e);
            ExitCode::from(1)
        }
    }
}
