//! Drift Detection Engine
//!
//! Detects behavioral/语义 changes in executor, storage, and telemetry.
//! D = semantic_diff_score + execution_diff_score
//!
//! Drift sources:
//!   - query plan change (executor)
//!   - WAL behavior shift (storage)
//!   - telemetry span change (observability)
//!   - SQL output diff (corpus)

use serde::{Deserialize, Serialize};

/// A trace snapshot for drift comparison
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TraceSnapshot {
    pub timestamp: String,
    pub executor_plan_hash: String,
    pub output_row_count: usize,
    pub span_count: usize,
    pub wal_write_bytes: u64,
    pub query_duration_ms: u64,
}

/// Drift detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftResult {
    /// Overall drift score D ∈ [0, 1]
    pub score: f64,
    /// Component scores
    pub plan_drift: f64,
    pub output_drift: f64,
    pub span_drift: f64,
    pub wal_drift: f64,
    /// Whether drift exceeds threshold
    pub is_anomalous: bool,
    /// Human-readable explanation
    pub explanation: String,
}

/// Lightweight trace entry for drift computation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TraceEntry {
    pub query_signature: String,
    pub plan_hash: String,
    pub rows: usize,
    pub duration_ms: u64,
}

impl DriftResult {
    pub fn compute(baseline: &[TraceEntry], current: &[TraceEntry]) -> Self {
        let plan_drift = Self::plan_drift(baseline, current);
        let output_drift = Self::output_drift(baseline, current);
        let span_drift = Self::span_drift(baseline, current);
        let wal_drift = Self::wal_drift(baseline, current);

        let score =
            (0.35 * plan_drift + 0.30 * output_drift + 0.20 * span_drift + 0.15 * wal_drift)
                .clamp(0.0, 1.0);

        let explanation = Self::explain(score, plan_drift, output_drift, span_drift, wal_drift);

        DriftResult {
            score,
            plan_drift,
            output_drift,
            span_drift,
            wal_drift,
            is_anomalous: score > 0.4,
            explanation,
        }
    }

    /// Plan drift: hash difference in query execution plans
    fn plan_drift(baseline: &[TraceEntry], current: &[TraceEntry]) -> f64 {
        if baseline.is_empty() || current.is_empty() {
            return 0.0;
        }
        let baseline_plans: std::collections::HashSet<_> =
            baseline.iter().map(|e| e.plan_hash.clone()).collect();
        let current_plans: std::collections::HashSet<_> =
            current.iter().map(|e| e.plan_hash.clone()).collect();

        let changed = current_plans.symmetric_difference(&baseline_plans).count() as f64;
        let total = current_plans.union(&baseline_plans).count().max(1) as f64;
        changed / total
    }

    /// Output drift: row count differences per query signature
    fn output_drift(baseline: &[TraceEntry], current: &[TraceEntry]) -> f64 {
        use std::collections::HashMap;
        let base_map: HashMap<_, _> = baseline
            .iter()
            .map(|e| (e.query_signature.clone(), e.rows))
            .collect();
        let curr_map: HashMap<_, _> = current
            .iter()
            .map(|e| (e.query_signature.clone(), e.rows))
            .collect();

        let mut total_diff = 0.0;
        let mut count = 0;
        for (sig, curr_rows) in &curr_map {
            if let Some(&base_rows) = base_map.get(sig) {
                if base_rows == 0 {
                    continue;
                }
                let diff = (*curr_rows as f64 - base_rows as f64).abs() / base_rows as f64;
                total_diff += diff.min(1.0);
                count += 1;
            }
        }
        if count == 0 {
            return 0.0;
        }
        total_diff / count as f64
    }

    /// Span drift: telemetry span count changes
    fn span_drift(_baseline: &[TraceEntry], _current: &[TraceEntry]) -> f64 {
        // In v3.7.0 we don't have per-query span data yet
        // Placeholder — would need telemetry v2 integration
        0.0
    }

    /// WAL drift: write size variance
    fn wal_drift(_baseline: &[TraceEntry], _current: &[TraceEntry]) -> f64 {
        // Would need storage instrumentation
        0.0
    }

    fn explain(score: f64, plan: f64, output: f64, span: f64, wal: f64) -> String {
        if score < 0.1 {
            "negligible drift".to_string()
        } else if score < 0.3 {
            format!(
                "minor drift (plan={:.1}%, output={:.1}%)",
                plan * 100.0,
                output * 100.0
            )
        } else if score < 0.5 {
            format!(
                "moderate drift detected (plan={:.1}%, output={:.1}%, span={:.1}%)",
                plan * 100.0,
                output * 100.0,
                span * 100.0
            )
        } else {
            format!(
                "significant drift (plan={:.1}%, output={:.1}%, span={:.1}%, wal={:.1}%)",
                plan * 100.0,
                output * 100.0,
                span * 100.0,
                wal * 100.0
            )
        }
    }

    /// Create from preflight checks (lightweight, no traces needed)
    pub fn from_git_diff() -> Self {
        // Lightweight drift estimate from git activity
        let executor_changed = std::process::Command::new("git")
            .args(["diff", "--stat", "crates/executor"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();

        let plan_drift: f64 =
            if executor_changed.contains("100 file") || executor_changed.contains("50 file") {
                0.4
            } else if executor_changed.contains("20 file") || executor_changed.contains("10 file") {
                0.2
            } else {
                0.05
            };

        let score: f64 = (0.5 * plan_drift).clamp(0.0, 1.0);
        DriftResult {
            score,
            plan_drift,
            output_drift: 0.0,
            span_drift: 0.0,
            wal_drift: 0.0,
            is_anomalous: score > 0.4,
            explanation: if plan_drift > 0.3 {
                "moderate executor plan drift detected".to_string()
            } else {
                "minimal executor drift".to_string()
            },
        }
    }
}

impl Default for DriftResult {
    fn default() -> Self {
        Self {
            score: 0.0,
            plan_drift: 0.0,
            output_drift: 0.0,
            span_drift: 0.0,
            wal_drift: 0.0,
            is_anomalous: false,
            explanation: "no data".to_string(),
        }
    }
}
