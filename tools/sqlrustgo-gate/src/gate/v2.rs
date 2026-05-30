//! Adaptive Gate Engine v2 — Three-dimensional decision model
//!
//! Combines Risk (R) + Stability (S) + Drift (D) to decide gate mode.
//! This supersedes the simple preflight/full split in beta.rs
//!
//! Decision space:
//!   SAFE:       R<0.3, S>0.8, D<0.2  → full beta allowed
//!   STABLE:     R<0.5, S>0.6, D<0.4  → partial beta
//!   RISKY:      R>=0.6               → preflight only
//!   DRIFT_WARN: D>=0.6               → preflight + drift investigation
//!   BLOCK:      otherwise             → block pipeline

use crate::gate::decision::{make_decision, GateDecision};
use crate::gate::{drift::DriftResult, stability::StabilityWindow};

/// Compute the three-dimensional gate decision
pub fn decide(risk_score: f64, stability: &StabilityWindow, drift: &DriftResult) -> GateDecision {
    let stability_score = stability.score();
    let drift_score = drift.score;

    make_decision(risk_score, stability_score, drift_score)
}

/// Format decision for human-readable output
pub fn format_decision(d: &GateDecision) -> String {
    let mut out = Vec::new();
    out.push(format!("RISK:     {:.2}", d.risk_score));
    out.push(format!("STABLE:   {:.2}", d.stability_score));
    out.push(format!("DRIFT:    {:.2}", d.drift_score));
    out.push(format!(""));
    out.push(format!("decision: {}", d.mode));
    if !d.reasons.is_empty() {
        out.push(format!("reasons:"));
        for r in &d.reasons {
            out.push(format!("  - {}", r));
        }
    }
    out.push(format!(""));
    if let Some(warning) = &d.warning {
        out.push(format!("WARNING: {}", warning));
    }
    out.join("\n")
}
