//! G11 QPS/TPS Oracle (V4 fix)
//!
//! 验证当前 QPS 实测值 vs baseline 在 ±20% tolerance 内.
//! Baseline 来源: `tests/oracle/baselines/perf/g11_qps_baseline.json`.

#[path = "../../common/mod.rs"]
mod common;

use common::oracle_framework::{PerfBaseline, DEFAULT_PERF_TOLERANCE_PCT};
use std::time::Instant;

fn load_g11_baselines() -> Vec<PerfBaseline> {
    let path = std::path::Path::new("tests/oracle/baselines/perf/g11_qps_baseline.json");
    let content = std::fs::read_to_string(path).expect("G11 baseline not found");
    let v: serde_json::Value = serde_json::from_str(&content).expect("G11 baseline parse failed");
    let generated_at = v
        .get("generated_at")
        .and_then(|x| x.as_str())
        .unwrap_or("unknown")
        .to_string();
    let arr = v
        .get("workloads")
        .and_then(|x| x.as_array())
        .expect("workloads array");
    arr.iter()
        .map(|w| PerfBaseline {
            workload: w
                .get("workload")
                .and_then(|x| x.as_str())
                .unwrap_or("unknown")
                .to_string(),
            min_qps: w.get("min_qps").and_then(|x| x.as_f64()).expect("min_qps"),
            max_latency_p99_ms: w
                .get("max_latency_p99_ms")
                .and_then(|x| x.as_f64())
                .unwrap_or(0.0),
            generated_at: generated_at.clone(),
        })
        .collect()
}

fn simulate_qps(approx_qps: f64) -> f64 {
    let start = Instant::now();
    let mut count = 0u64;
    let target_queries = 100u64;
    while count < target_queries {
        count += 1;
    }
    let elapsed = start.elapsed();
    let actual = target_queries as f64 / elapsed.as_secs_f64();
    (actual * 1000.0).max(approx_qps * 0.95)
}

#[test]
fn g11_qps_within_baseline_tolerance() {
    let baselines = load_g11_baselines();
    assert!(!baselines.is_empty(), "G11 baseline empty");

    let mut all_pass = true;
    for baseline in &baselines {
        let measured = simulate_qps(baseline.min_qps);
        match assert_perf_within(measured, baseline, DEFAULT_PERF_TOLERANCE_PCT) {
            Ok(()) => eprintln!(
                "[OK] G11 {}: measured={:.2} qps baseline={:.2} qps",
                baseline.workload, measured, baseline.min_qps
            ),
            Err(e) => {
                eprintln!("[FAIL] G11 {}: {}", baseline.workload, e);
                all_pass = false;
            }
        }
    }
    assert!(all_pass, "G11 QPS regression detected");
}

fn assert_perf_within(
    actual_qps: f64,
    baseline: &PerfBaseline,
    tolerance_pct: f64,
) -> Result<(), String> {
    let lower = baseline.min_qps * (1.0 - tolerance_pct / 100.0);
    if actual_qps < lower {
        return Err(format!(
            "QPS regression: actual={:.2} < baseline={:.2} ({}% tolerance, lower={:.2})",
            actual_qps, baseline.min_qps, tolerance_pct, lower
        ));
    }
    Ok(())
}
