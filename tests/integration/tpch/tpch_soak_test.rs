//! TPC-H soak test via real `mysql` CLI — sustained stability & performance.
//!
//! Runs TPC-H Q1 + Q6 repeatedly against a live `sqlrustgo-mysql-server`
//! subprocess via the real `mysql` CLI binary. Tracks latency distribution,
//! error rate, and throughput to detect regressions in server stability
//! under sustained wire-level load.
//!
//! This is NOT a correctness test — it is a stability/performance soak.
//! Use `tpch_full_22_test` or `tpch_gate_test` for correctness assertions.
//!
//! Run: cargo test --test tpch_soak_test -- --nocapture
//!
//! # Soak parameters
//!
//! - `SOAK_ITERATIONS`: number of full query-set iterations (default: 10)
//! - `SOAK_QUERIES`: comma-separated query numbers to run (default: "Q1,Q6")
//! - `SOAK_SCALE`: "sf001" or "sf01" (default: "sf001")

#[path = "../../common/mod.rs"]
mod common;

use common::tpch_cli_harness::{mysql_query, start_sf001_cli};
use std::collections::HashMap;
use std::env;
use std::time::Instant;

/// TPC-H queries available for soak testing (simplified, fast-executing).
fn soak_queries() -> Vec<(&'static str, &'static str)> {
    vec![
        ("Q1", "SELECT l_returnflag, l_linestatus, COUNT(*) FROM lineitem WHERE l_shipdate <= '1998-09-02' GROUP BY l_returnflag, l_linestatus ORDER BY l_returnflag, l_linestatus"),
        ("Q6", "SELECT COUNT(*) FROM lineitem WHERE l_shipdate >= '1994-01-01' AND l_shipdate < '1995-01-01' AND l_quantity < 25"),
        ("Q4", "SELECT o_orderpriority, COUNT(*) FROM orders WHERE o_orderdate >= '1993-07-01' AND o_orderdate < '1993-10-01' GROUP BY o_orderpriority ORDER BY o_orderpriority"),
        ("Q22", "SELECT SUBSTR(c_phone, 1, 2) AS cntrycode, COUNT(*) FROM customer WHERE c_acctbal > 0 GROUP BY SUBSTR(c_phone, 1, 2) ORDER BY cntrycode"),
        ("Q15", "SELECT s_suppkey, s_name, s_address FROM supplier ORDER BY s_suppkey LIMIT 100"),
        ("Q20", "SELECT s_name, s_address FROM supplier, nation WHERE s_nationkey = n_nationkey AND n_name = 'GERMANY' ORDER BY s_name"),
    ]
}

/// Statistics for a set of latency samples.
#[derive(Default)]
struct LatencyStats {
    count: usize,
    sum_ms: f64,
    min_ms: f64,
    max_ms: f64,
    samples: Vec<f64>,
}

impl LatencyStats {
    fn add(&mut self, ms: f64) {
        self.count += 1;
        self.sum_ms += ms;
        self.min_ms = if self.count == 1 {
            ms
        } else {
            self.min_ms.min(ms)
        };
        self.max_ms = self.max_ms.max(ms);
        self.samples.push(ms);
    }

    fn percentile(&self, p: f64) -> f64 {
        if self.samples.is_empty() {
            return 0.0;
        }
        let mut sorted = self.samples.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let idx = ((p / 100.0) * (sorted.len() - 1) as f64).round() as usize;
        sorted[idx.min(sorted.len() - 1)]
    }

    fn avg_ms(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.sum_ms / self.count as f64
        }
    }

    fn qps(&self, total_time_ms: f64) -> f64 {
        if total_time_ms == 0.0 {
            0.0
        } else {
            self.count as f64 / (total_time_ms / 1000.0)
        }
    }
}

#[test]
fn tpch_soak_sf001_q1_q6() {
    let iterations: usize = env::var("SOAK_ITERATIONS")
        .unwrap_or_else(|_| "10".to_string())
        .parse()
        .unwrap_or(10);

    eprintln!("=== TPC-H Soak Test (mysql CLI, SF=0.001) ===");
    eprintln!("Iterations: {iterations}");
    eprintln!();

    let server = start_sf001_cli().expect("start sf001 server");
    let host = "127.0.0.1";
    let port = server.port();
    let user = "tester";

    let allowed = env::var("SOAK_QUERIES").unwrap_or_else(|_| "Q1,Q6".to_string());
    let allowed_set: Vec<&str> = allowed.split(',').collect();

    let all_queries = soak_queries();
    let queries: Vec<(&str, &str)> = all_queries
        .into_iter()
        .filter(|(name, _)| allowed_set.contains(name))
        .collect();

    eprintln!(
        "Queries: {:?}",
        queries.iter().map(|(n, _)| n).collect::<Vec<_>>()
    );
    eprintln!();

    let overall_start = Instant::now();
    let mut total_errors = 0;
    let mut query_stats: HashMap<&str, LatencyStats> = HashMap::new();

    for iter in 0..iterations {
        let iter_start = Instant::now();
        eprintln!("--- Iteration {}/{} ---", iter + 1, iterations);

        for (q_name, q_sql) in &queries {
            let stats = query_stats.entry(q_name).or_default();
            let q_start = Instant::now();
            let result = mysql_query(host, port, user, None, q_sql);
            let q_elapsed = q_start.elapsed();

            match result {
                Ok(rows) => {
                    let ms = q_elapsed.as_secs_f64() * 1000.0;
                    stats.add(ms);
                    eprintln!("  ✅ {q_name}: {} rows in {:.2?}ms", rows.len(), ms);
                }
                Err(e) => {
                    total_errors += 1;
                    eprintln!(
                        "  ❌ {q_name}: {e} ({:.2?}ms)",
                        q_elapsed.as_secs_f64() * 1000.0
                    );
                }
            }
        }

        eprintln!("  iter time: {:.2?}", iter_start.elapsed());
    }

    let total_elapsed = overall_start.elapsed();

    eprintln!();
    eprintln!("====================== SOAK SUMMARY ======================");
    let total_queries: usize = query_stats.values().map(|s| s.count).sum();
    let total_time_ms = total_elapsed.as_secs_f64() * 1000.0;
    let overall_qps = total_queries as f64 / total_elapsed.as_secs_f64();
    eprintln!(
        "Total: {} iterations × {} queries = {} executions in {:.2?}",
        iterations,
        queries.len(),
        total_queries,
        total_elapsed
    );
    eprintln!("Errors: {}", total_errors);
    eprintln!();

    eprintln!(
        "{:<6} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>10}",
        "Query", "count", "avg_ms", "min_ms", "max_ms", "p50ms", "p95ms", "qps"
    );
    eprintln!("{}", "-".repeat(70));

    for (q_name, stats) in &query_stats {
        eprintln!(
            "{:<6} {:>8} {:>8.2} {:>8.2} {:>8.2} {:>8.2} {:>8.2} {:>10.2}",
            q_name,
            stats.count,
            stats.avg_ms(),
            stats.min_ms,
            stats.max_ms,
            stats.percentile(50.0),
            stats.percentile(95.0),
            stats.qps(total_time_ms),
        );
    }

    eprintln!("{}", "-".repeat(70));
    eprintln!("===========================================================");

    let error_rate = if total_queries > 0 {
        total_errors as f64 / total_queries as f64
    } else {
        1.0
    };

    if total_errors > 0 {
        eprintln!(
            "\n⚠️  {} errors detected (error rate: {:.2}%)",
            total_errors,
            error_rate * 100.0
        );
    }

    if error_rate > 0.01 {
        panic!(
            "Soak FAILED: error rate {:.2}% exceeds 1% threshold ({} errors / {} queries)",
            error_rate * 100.0,
            total_errors,
            total_queries
        );
    }

    for (q_name, stats) in &query_stats {
        let p99 = stats.percentile(99.0);
        if p99 > 5000.0 {
            eprintln!(
                "\n⚠️  WARNING: {q_name} p99 latency {:.0}ms exceeds 5000ms threshold",
                p99
            );
        }
    }

    eprintln!(
        "\n✅ Soak test PASSED — {} errors, {:.2} qps overall",
        total_errors, overall_qps
    );
}
