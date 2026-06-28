//! `sqlrustgo-cli soak` — Real SQL SOAK test runner.
//!
//! Connects to a running server and executes a continuous workload
//! of SQL queries for a specified duration, collecting latency,
//! throughput, and error metrics.
//!
//! Unlike the simulated soak harness (`tests/soak_test_harness.rs`),
//! this runs **real SQL queries** over the wire protocol against a
//! live server — suitable for 24h/72h/168h real wall-clock SOAK tests.
//!
//! # Query workload
//!
//! By default it uses a built-in set of generic queries that work on
//! any database.  With `--query-file <path>` you can supply a custom
//! workload (one SQL statement per line).  TPC-H query sets are
//! recommended for realistic SOAK workloads.

use crate::client::Client;
use std::time::{Duration, Instant};

/// SOAK test configuration.
#[derive(Debug, Clone)]
pub struct SoakConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    /// Wall-clock duration in seconds.
    pub duration_secs: u64,
    /// Target queries per second (0 = fire as fast as possible).
    pub target_qps: f64,
    /// Path to custom query file (one SQL per line).
    pub query_file: Option<String>,
    /// Emit progress every N seconds.
    pub report_interval: u64,
}

/// SOAK test result report.
#[derive(Debug, Clone)]
pub struct SoakReport {
    pub duration_secs: u64,
    pub queries_executed: u64,
    pub errors: u64,
    pub p50_latency_ms: f64,
    pub p90_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub avg_latency_ms: f64,
    pub max_latency_ms: f64,
    pub actual_qps: f64,
}

// ─── Built-in query pool (all verified to work with sqlrustgo-mysql-server) ─

const BUILTIN_QUERIES: &[&str] = &[
    "SELECT 1",
    "SELECT 1 + 1 AS two",
    "SELECT 2 * 3 AS six",
    "SELECT 64 / 8 AS eight",
    "SELECT 42 AS answer",
    "SELECT 3.14 AS pi",
    "SELECT 'hello' AS greeting",
    "SELECT 'sqlrustgo' AS name",
    "SELECT LENGTH('sqlrustgo') AS len",
    "SELECT 100 + 200 AS sum",
    "SELECT 1000 - 1 AS minus",
    "SELECT 7 * 8 AS product",
];

/// Load queries from a file, one per line (skips empty/comment lines).
fn load_queries(path: &str) -> anyhow::Result<Vec<String>> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("cannot read query file '{path}': {e}"))?;
    let queries: Vec<String> = content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with("--") && !l.starts_with('#'))
        .map(|l| l.to_string())
        .collect();
    if queries.is_empty() {
        return Err(anyhow::anyhow!(
            "query file '{path}' is empty or all-comment"
        ));
    }
    Ok(queries)
}
/// Run the SOAK test and return a report.
pub fn run_soak(config: &SoakConfig) -> anyhow::Result<SoakReport> {
    let queries: Vec<String> = if let Some(path) = &config.query_file {
        load_queries(path)?
    } else {
        BUILTIN_QUERIES.iter().map(|s| s.to_string()).collect()
    };

    eprintln!(
        "SOAK: connecting to {}:{} as '{}' ...",
        config.host, config.port, config.user
    );
    let mut client = Client::connect(&config.host, config.port, &config.user, &config.password)?;
    eprintln!("SOAK: connected. Running for {}s ...", config.duration_secs);

    let mut latencies: Vec<f64> = Vec::with_capacity(100_000);
    let mut errors: u64 = 0;
    let start = Instant::now();
    let end = start + Duration::from_secs(config.duration_secs);

    // Warm-up: run each builtin query once to establish baseline
    for sql in &queries {
        if start.elapsed() >= Duration::from_secs(config.duration_secs) {
            break;
        }
        let _ = client.query(sql);
    }

    let main_start = Instant::now();
    let mut query_idx = 0;
    let mut last_report = Instant::now();

    while Instant::now() < end {
        let sql = &queries[query_idx % queries.len()];
        query_idx += 1;

        let q_start = Instant::now();
        match client.query(sql) {
            Ok(_) => {
                let elapsed_ms = q_start.elapsed().as_secs_f64() * 1000.0;
                latencies.push(elapsed_ms);
            }
            Err(e) => {
                errors += 1;
                if errors <= 10 {
                    eprintln!("SOAK error: {e}");
                }
            }
        }

        // Rate limiting: if we're ahead of schedule, sleep.
        if config.target_qps > 0.0 {
            let expected_elapsed = query_idx as f64 / config.target_qps;
            let actual_elapsed = main_start.elapsed().as_secs_f64();
            if actual_elapsed < expected_elapsed {
                std::thread::sleep(Duration::from_secs_f64(expected_elapsed - actual_elapsed));
            }
        }

        // Periodic progress report
        if last_report.elapsed().as_secs() >= config.report_interval {
            let elapsed = main_start.elapsed().as_secs_f64();
            let done = query_idx as f64;
            let qps = done / elapsed;
            eprintln!(
                "  SOAK: {done} queries, {errors} errors, {qps:.1} qps, elapsed {:.0}s",
                elapsed
            );
            last_report = Instant::now();
        }
    }

    let elapsed_total = main_start.elapsed().as_secs_f64();
    let total_queries = query_idx as u64;
    let actual_qps = if elapsed_total > 0.0 {
        total_queries as f64 / elapsed_total
    } else {
        0.0
    };

    // Compute latency percentiles
    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let p50 = percentile(&latencies, 50.0);
    let p90 = percentile(&latencies, 90.0);
    let p99 = percentile(&latencies, 99.0);
    let avg = if latencies.is_empty() {
        0.0
    } else {
        latencies.iter().sum::<f64>() / latencies.len() as f64
    };
    let max = latencies.last().copied().unwrap_or(0.0);

    let report = SoakReport {
        duration_secs: config.duration_secs,
        queries_executed: total_queries,
        errors,
        p50_latency_ms: p50,
        p90_latency_ms: p90,
        p99_latency_ms: p99,
        avg_latency_ms: avg,
        max_latency_ms: max,
        actual_qps,
    };

    let _ = client.quit();
    Ok(report)
}

/// Print the SOAK report in a human-readable format.
pub fn print_report(report: &SoakReport) {
    println!();
    println!("═ SOAK Test Report ═══════════════════════════════");
    println!("  Duration:              {}s", report.duration_secs);
    println!("  Queries executed:      {}", report.queries_executed);
    println!("  Errors:                {}", report.errors);
    println!("  Actual QPS:            {:.1}", report.actual_qps);
    println!("  Latency:");
    println!("    P50:                 {:.2}ms", report.p50_latency_ms);
    println!("    P90:                 {:.2}ms", report.p90_latency_ms);
    println!("    P99:                 {:.2}ms", report.p99_latency_ms);
    println!("    Avg:                 {:.2}ms", report.avg_latency_ms);
    println!("    Max:                 {:.2}ms", report.max_latency_ms);
    println!("═══════════════════════════════════════════════════");
    println!();

    // Determine PASS/FAIL based on thresholds
    let mut passed = true;
    if report.errors > 0 {
        println!("  ⚠  WARNING: {} query errors detected", report.errors);
    }
    if report.p99_latency_ms > 5000.0 {
        println!(
            "  ⚠  WARNING: P99 latency > 5000ms ({:.0}ms)",
            report.p99_latency_ms
        );
        passed = false;
    }
    if report.queries_executed == 0 {
        println!("  ❌ FAIL: zero queries executed");
        passed = false;
    }
    if passed {
        println!("  ✅ SOAK PASSED");
    } else {
        println!("  ❌ SOAK FAILED — review thresholds");
    }
}

fn percentile(sorted: &[f64], pct: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((pct / 100.0) * (sorted.len() as f64 - 1.0)).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}
