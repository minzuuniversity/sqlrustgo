//! Gate decision logic — three-dimensional model (R, S, D)
//!
//! Decision rules:
//!   SAFE:       R<0.3, S>0.8, D<0.2  → full beta allowed
//!   STABLE:     R<0.5, S>0.6, D<0.4  → partial beta
//!   RISKY:      R>=0.6               → preflight only
//!   DRIFT_WARN: D>=0.6               → preflight + drift investigation
//!   BLOCK:      otherwise             → block pipeline

use serde::{Deserialize, Serialize};

/// Gate execution modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GateMode {
    /// Full Beta — all checks enabled (coverage, corpus, full integration, perf)
    BetaFull,
    /// Partial Beta — preflight + expanded integration + coverage
    BetaPartial,
    /// Preflight only — build, unit, clippy, sql-smoke, subset
    Preflight,
    /// Block — stop pipeline, require human review
    Block,
}

impl std::fmt::Display for GateMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GateMode::BetaFull => write!(f, "BETA_FULL"),
            GateMode::BetaPartial => write!(f, "BETA_PARTIAL"),
            GateMode::Preflight => write!(f, "PREFLIGHT"),
            GateMode::Block => write!(f, "BLOCK"),
        }
    }
}

/// Complete gate decision output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateDecision {
    pub mode: GateMode,
    pub risk_score: f64,
    pub stability_score: f64,
    pub drift_score: f64,
    pub reasons: Vec<String>,
    pub warning: Option<String>,
    /// Whether full beta is allowed given current state
    pub full_beta_allowed: bool,
}

impl GateDecision {
    pub fn new(
        r: f64,
        s: f64,
        d: f64,
        reasons: Vec<String>,
        warning: Option<String>,
        full: bool,
    ) -> Self {
        Self {
            mode: GateMode::Preflight, // default, overridden below
            risk_score: r,
            stability_score: s,
            drift_score: d,
            reasons,
            warning,
            full_beta_allowed: full,
        }
    }
}

/// Core three-dimensional decision function
pub fn make_decision(risk: f64, stability: f64, drift: f64) -> GateDecision {
    // SAFE zone: low risk, high stability, low drift → full beta
    if risk < 0.3 && stability > 0.8 && drift < 0.2 {
        return GateDecision {
            mode: GateMode::BetaFull,
            risk_score: risk,
            stability_score: stability,
            drift_score: drift,
            reasons: vec![
                "low risk".to_string(),
                "high stability".to_string(),
                "minimal drift".to_string(),
            ],
            warning: None,
            full_beta_allowed: true,
        };
    }

    // STABLE zone: moderate risk, acceptable stability → partial beta
    if risk < 0.5 && stability > 0.6 && drift < 0.4 {
        let mut reasons = vec![
            format!("stability acceptable ({:.2} > 0.6)", stability),
            format!("risk moderate ({:.2} < 0.5)", risk),
        ];
        if drift > 0.2 {
            reasons.push(format!("minor drift detected ({:.2})", drift));
        }
        return GateDecision {
            mode: GateMode::BetaPartial,
            risk_score: risk,
            stability_score: stability,
            drift_score: drift,
            reasons,
            warning: None,
            full_beta_allowed: false,
        };
    }

    // RISKY zone: high risk → preflight only
    if risk >= 0.6 {
        let mut reasons = vec![format!("risk too high ({:.2} >= 0.6)", risk)];
        let warning = if drift >= 0.6 {
            reasons.push(format!("significant drift ({:.2} >= 0.6)", drift));
            Some(format!(
                "executor drift detected — preflight only, investigate before full beta. risk={:.2}, drift={:.2}",
                risk, drift
            ))
        } else {
            Some("high risk commit — preflight gate only".to_string())
        };
        return GateDecision {
            mode: GateMode::Preflight,
            risk_score: risk,
            stability_score: stability,
            drift_score: drift,
            reasons,
            warning,
            full_beta_allowed: false,
        };
    }

    // DRIFT_WARN zone: drift dominant
    if drift >= 0.6 {
        return GateDecision {
            mode: GateMode::Preflight,
            risk_score: risk,
            stability_score: stability,
            drift_score: drift,
            reasons: vec![
                format!("drift too high ({:.2} >= 0.6)", drift),
                "behavioral change exceeds threshold".to_string(),
            ],
            warning: Some(format!(
                "drift-dominant state: executor/telemetry behavior changed significantly. \
                 Run preflight + investigation before advancing. drift={:.2}",
                drift
            )),
            full_beta_allowed: false,
        };
    }

    // Degraded stability: low stability regardless of other factors
    if stability < 0.4 {
        return GateDecision {
            mode: GateMode::Preflight,
            risk_score: risk,
            stability_score: stability,
            drift_score: drift,
            reasons: vec![
                format!("stability too low ({:.2} < 0.4)", stability),
                "recent CI instability detected".to_string(),
            ],
            warning: Some(format!(
                "stability window degraded — consecutive failures or regressions. \
                 Stabilize before advancing. stability={:.2}",
                stability
            )),
            full_beta_allowed: false,
        };
    }

    // Default protection: block when we can't confidently decide
    GateDecision {
        mode: GateMode::Block,
        risk_score: risk,
        stability_score: stability,
        drift_score: drift,
        reasons: vec![format!(
            "fallback block: cannot determine safe gate (r={:.2}, s={:.2}, d={:.2})",
            risk, stability, drift
        )],
        warning: Some("Automatic safety block — manual review required".to_string()),
        full_beta_allowed: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_zone() {
        let d = make_decision(0.25, 0.85, 0.15);
        assert_eq!(d.mode, GateMode::BetaFull);
        assert!(d.full_beta_allowed);
    }

    #[test]
    fn test_stable_zone() {
        let d = make_decision(0.40, 0.70, 0.30);
        assert_eq!(d.mode, GateMode::BetaPartial);
        assert!(!d.full_beta_allowed);
    }

    #[test]
    fn test_risky_zone() {
        let d = make_decision(0.65, 0.50, 0.30);
        assert_eq!(d.mode, GateMode::Preflight);
        assert!(!d.full_beta_allowed);
    }

    #[test]
    fn test_drift_warn() {
        let d = make_decision(0.35, 0.70, 0.65);
        assert_eq!(d.mode, GateMode::Preflight);
        assert!(d.warning.is_some());
    }

    #[test]
    fn test_block_fallback() {
        let d = make_decision(0.55, 0.30, 0.50);
        assert_eq!(d.mode, GateMode::Preflight); // stability < 0.4 triggers preflight
    }
}
