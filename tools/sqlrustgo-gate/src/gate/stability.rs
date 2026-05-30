//! Stability Window Tracker
//!
//! Tracks historical CI results to compute stability score S.
//! S = 1 - (failure_rate + regression_variance)

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// A single gate execution snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateSnapshot {
    pub timestamp: String,
    pub stage: String,
    pub mode: String,
    pub passed: bool,
    /// regression score 0.0-1.0 (0 = no regression, 1 = severe)
    pub regression_score: f64,
    /// risk score computed at execution time
    pub risk_score: f64,
    /// hotfix/emergency flag
    pub hotfix: bool,
}

/// Stability window — tracks last N gate results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StabilityWindow {
    pub history: VecDeque<GateSnapshot>,
    max_size: usize,
}

impl Default for StabilityWindow {
    fn default() -> Self {
        Self {
            history: VecDeque::with_capacity(20),
            max_size: 20,
        }
    }
}

impl StabilityWindow {
    pub fn new(max_size: usize) -> Self {
        Self {
            history: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    /// Load from disk or return a fresh default window
    pub fn load_or_default() -> Self {
        let path = std::env::var("GATE_STATE_DIR")
            .map(|p| format!("{}/stability_window.json", p))
            .unwrap_or_else(|_| ".gate/state/stability_window.json".to_string());

        if let Ok(contents) = std::fs::read_to_string(&path) {
            if let Ok(w) = serde_json::from_str::<StabilityWindow>(&contents) {
                return w;
            }
        }
        Self::default()
    }

    /// Add a new gate result
    pub fn push(&mut self, snapshot: GateSnapshot) {
        if self.history.len() >= self.max_size {
            self.history.pop_front();
        }
        self.history.push_back(snapshot);
    }

    /// Compute stability score S ∈ [0, 1]
    /// S = 1 - (failure_rate + regression_avg)
    pub fn score(&self) -> f64 {
        if self.history.is_empty() {
            return 0.5; // neutral when no history
        }

        let n = self.history.len() as f64;
        let failures = self.history.iter().filter(|r| !r.passed).count() as f64;
        let regression_sum: f64 = self.history.iter().map(|r| r.regression_score).sum();

        let failure_rate = failures / n;
        let regression_avg = regression_sum / n;

        // Hotfix penalty: if recent hotfix, slightly reduce stability
        let hotfix_penalty = if self.history.iter().any(|r| r.hotfix) {
            0.05
        } else {
            0.0
        };

        (1.0 - failure_rate - regression_avg - hotfix_penalty).clamp(0.0, 1.0)
    }

    /// Compute consecutive pass streak
    pub fn streak(&self) -> usize {
        let mut streak = 0;
        for snapshot in self.history.iter().rev() {
            if snapshot.passed {
                streak += 1;
            } else {
                break;
            }
        }
        streak
    }

    /// Regression trend: positive = improving, negative = degrading
    pub fn regression_trend(&self) -> f64 {
        let recent = self.history.iter().rev().take(5).collect::<Vec<_>>();
        if recent.len() < 2 {
            return 0.0;
        }
        let mid = recent.len() / 2;
        let older: f64 = recent[..mid]
            .iter()
            .map(|r| r.regression_score)
            .sum::<f64>()
            / mid as f64;
        let newer: f64 = recent[mid..]
            .iter()
            .map(|r| r.regression_score)
            .sum::<f64>()
            / (recent.len() - mid) as f64;
        older - newer // positive = improving (newer has lower regression)
    }

    /// Anomaly score: how unusual is the latest run compared to history
    pub fn anomaly_score(&self) -> f64 {
        if self.history.len() < 3 {
            return 0.0;
        }
        let recent = self
            .history
            .back()
            .map(|r| r.regression_score)
            .unwrap_or(0.0);
        let avg: f64 = self.history.iter().map(|r| r.regression_score).sum::<f64>()
            / self.history.len() as f64;
        let variance: f64 = self
            .history
            .iter()
            .map(|r| (r.regression_score - avg).powi(2))
            .sum::<f64>()
            / self.history.len() as f64;
        let std_dev = variance.sqrt();
        if std_dev == 0.0 {
            return 0.0;
        }
        ((recent - avg) / std_dev).abs().min(1.0)
    }

    /// Check if system is in a degraded state
    pub fn is_degraded(&self) -> bool {
        self.score() < 0.5 || self.streak() == 0 && self.history.len() >= 3
    }

    /// Export for JSON evidence
    pub fn summary(&self) -> StabilitySummary {
        StabilitySummary {
            score: self.score(),
            streak: self.streak(),
            total_runs: self.history.len(),
            regression_trend: self.regression_trend(),
            anomaly_score: self.anomaly_score(),
            is_degraded: self.is_degraded(),
            recent_results: self
                .history
                .iter()
                .rev()
                .take(5)
                .map(|s| s.clone())
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct StabilitySummary {
    pub score: f64,
    pub streak: usize,
    pub total_runs: usize,
    pub regression_trend: f64,
    pub anomaly_score: f64,
    pub is_degraded: bool,
    pub recent_results: Vec<GateSnapshot>,
}

impl Default for GateSnapshot {
    fn default() -> Self {
        Self {
            timestamp: chrono::Utc::now().to_rfc3339(),
            stage: "unknown".to_string(),
            mode: "unknown".to_string(),
            passed: false,
            regression_score: 0.0,
            risk_score: 0.0,
            hotfix: false,
        }
    }
}
