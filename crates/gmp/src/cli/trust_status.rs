//! Trust Visualization CLI Command
//!
//! Provides `gmp trust status` command to display trust visualization data.

use crate::trust_visualization::{
    AuditChainLink, AuditChainNode, AuditChainVisualization, ComplianceDashboard,
    EvidenceCompleteness, RiskHeatmap, RiskItem, RuleStatus, TrustScoreComponents,
    TrustVisualization,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// CLI output format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustStatusOutput {
    pub version: String,
    pub timestamp: i64,
    pub trust_score: TrustScoreComponents,
    pub evidence_completeness: EvidenceCompleteness,
    pub risk_summary: RiskSummary,
    pub compliance_rules_pass: usize,
    pub compliance_rules_fail: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskSummary {
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
}

impl TrustStatusOutput {
    /// Create from trust visualization data
    pub fn from_visualization(tv: &TrustVisualization) -> Self {
        let cd = &tv.compliance_dashboard;
        TrustStatusOutput {
            version: tv.version.clone(),
            timestamp: tv.timestamp,
            trust_score: cd.trust_score.clone(),
            evidence_completeness: cd.evidence_completeness.clone(),
            risk_summary: RiskSummary {
                critical: cd.risk_heatmap.critical_count,
                high: cd.risk_heatmap.high_count,
                medium: cd.risk_heatmap.medium_count,
                low: cd.risk_heatmap.low_count,
            },
            compliance_rules_pass: cd.passing_rules,
            compliance_rules_fail: cd.failing_rules,
        }
    }

    /// Print human-readable output
    pub fn print(&self) {
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║               GMP Trust Status Report                       ║");
        println!("╚══════════════════════════════════════════════════════════════╝");
        println!();
        println!("Version: {}  Timestamp: {}", self.version, self.timestamp);
        println!();
        println!("┌─────────────────────────────────────────────────────────────┐");
        println!("│                    TRUST SCORE                               │");
        println!("├─────────────────────────────────────────────────────────────┤");
        println!(
            "│ Overall: {:>3}%  Level: {:>8}                              │",
            self.trust_score.overall,
            self.trust_score.trust_level()
        );
        println!("├─────────────────────────────────────────────────────────────┤");
        println!(
            "│ Audit Chain Integrity: {:>3}%                               │",
            self.trust_score.audit_chain_integrity
        );
        println!(
            "│ Evidence Completeness: {:>3}%                               │",
            self.trust_score.evidence_completeness
        );
        println!(
            "│ Signature Validity:    {:>3}%                               │",
            self.trust_score.signature_validity
        );
        println!(
            "│ Workflow Compliance:    {:>3}%                               │",
            self.trust_score.workflow_compliance
        );
        println!("└─────────────────────────────────────────────────────────────┘");
        println!();
        println!("┌─────────────────────────────────────────────────────────────┐");
        println!("│                   EVIDENCE STATUS                           │");
        println!("├─────────────────────────────────────────────────────────────┤");
        println!(
            "│ Total: {:>4}  Complete: {:>4}  Partial: {:>4}  Missing: {:>4}  │",
            self.evidence_completeness.total,
            self.evidence_completeness.complete,
            self.evidence_completeness.partial,
            self.evidence_completeness.missing
        );
        println!(
            "│ Completeness: {:>3}%                                            │",
            self.evidence_completeness.completeness_pct
        );
        println!("└─────────────────────────────────────────────────────────────┘");
        println!();
        println!("┌─────────────────────────────────────────────────────────────┐");
        println!("│                    RISK HEATMAP                             │");
        println!("├─────────────────────────────────────────────────────────────┤");
        println!(
            "│ Critical: {:>4}  High: {:>4}  Medium: {:>4}  Low: {:>4}           │",
            self.risk_summary.critical,
            self.risk_summary.high,
            self.risk_summary.medium,
            self.risk_summary.low
        );
        println!("└─────────────────────────────────────────────────────────────┘");
        println!();
        println!("┌─────────────────────────────────────────────────────────────┐");
        println!("│                 COMPLIANCE RULES                            │");
        println!("├─────────────────────────────────────────────────────────────┤");
        println!(
            "│ Passing: {:>4}  Failing: {:>4}                                   │",
            self.compliance_rules_pass, self.compliance_rules_fail
        );
        println!("└─────────────────────────────────────────────────────────────┘");
    }

    /// Print JSON output
    pub fn print_json(&self) {
        let json = serde_json::to_string_pretty(self).unwrap();
        println!("{}", json);
    }
}

/// Generate a demo trust visualization for testing
pub fn generate_demo_visualization() -> TrustVisualization {
    let mut tv = TrustVisualization::new();

    // Set trust scores
    tv.compliance_dashboard.trust_score = TrustScoreComponents::new(92, 85, 95, 88);

    // Set evidence completeness
    tv.compliance_dashboard.evidence_completeness = EvidenceCompleteness::new(100, 82, 12, 6);

    // Add some rules
    tv.compliance_dashboard.rules.push(RuleStatus {
        rule_name: "dual_signature_required".to_string(),
        rule_type: "signature".to_string(),
        severity: "critical".to_string(),
        enabled: true,
        passing: true,
        description: "Critical records require dual signatures".to_string(),
    });
    tv.compliance_dashboard.rules.push(RuleStatus {
        rule_name: "audit_chain_intact".to_string(),
        rule_type: "audit".to_string(),
        severity: "high".to_string(),
        enabled: true,
        passing: true,
        description: "Audit chain hash continuity verified".to_string(),
    });
    tv.compliance_dashboard.passing_rules = 2;
    tv.compliance_dashboard.failing_rules = 0;

    // Add risk items
    let mut heatmap = RiskHeatmap::empty();
    heatmap.add_risk(RiskItem {
        category: "signature".to_string(),
        level: "critical".to_string(),
        description: "Missing signature on record XYZ".to_string(),
        count: 1,
        score: 95,
    });
    heatmap.add_risk(RiskItem {
        category: "audit".to_string(),
        level: "low".to_string(),
        description: "Minor timestamp drift detected".to_string(),
        count: 1,
        score: 15,
    });
    tv.compliance_dashboard.risk_heatmap = heatmap;

    // Add audit chain nodes
    tv.audit_chain.nodes.push(AuditChainNode {
        seq: 1,
        id: "node_1".to_string(),
        timestamp: 1716000000,
        operation: "create".to_string(),
        checksum: "abc123".to_string(),
        status: "valid".to_string(),
    });
    tv.audit_chain.nodes.push(AuditChainNode {
        seq: 2,
        id: "node_2".to_string(),
        timestamp: 1716000100,
        operation: "sign".to_string(),
        checksum: "def456".to_string(),
        status: "valid".to_string(),
    });
    tv.audit_chain.links.push(AuditChainLink {
        source: "node_1".to_string(),
        target: "node_2".to_string(),
        link_type: "next".to_string(),
    });
    tv.audit_chain.total_entries = 2;
    tv.audit_chain.valid_entries = 2;
    tv.audit_chain.invalid_entries = 0;

    tv
}

/// Build trust visualization from actual GMP data (placeholder for real implementation)
pub struct TrustVisualizationBuilder {
    trust_score: TrustScoreComponents,
    evidence_completeness: EvidenceCompleteness,
    rules: Vec<RuleStatus>,
    risk_heatmap: RiskHeatmap,
    audit_chain: AuditChainVisualization,
}

impl TrustVisualizationBuilder {
    pub fn new() -> Self {
        TrustVisualizationBuilder {
            trust_score: TrustScoreComponents::new(0, 0, 0, 0),
            evidence_completeness: EvidenceCompleteness::new(0, 0, 0, 0),
            rules: Vec::new(),
            risk_heatmap: RiskHeatmap::empty(),
            audit_chain: AuditChainVisualization {
                nodes: Vec::new(),
                links: Vec::new(),
                total_entries: 0,
                valid_entries: 0,
                invalid_entries: 0,
            },
        }
    }

    pub fn with_trust_score(mut self, score: TrustScoreComponents) -> Self {
        self.trust_score = score;
        self
    }

    pub fn with_evidence(mut self, evidence: EvidenceCompleteness) -> Self {
        self.evidence_completeness = evidence;
        self
    }

    pub fn with_rules(mut self, rules: Vec<RuleStatus>) -> Self {
        self.rules = rules;
        self
    }

    pub fn with_risk(mut self, heatmap: RiskHeatmap) -> Self {
        self.risk_heatmap = heatmap;
        self
    }

    pub fn with_audit_chain(mut self, chain: AuditChainVisualization) -> Self {
        self.audit_chain = chain;
        self
    }

    pub fn build(self) -> TrustVisualization {
        let passing = self.rules.iter().filter(|r| r.passing).count();
        let failing = self.rules.iter().filter(|r| !r.passing).count();

        TrustVisualization {
            compliance_dashboard: ComplianceDashboard {
                trust_score: self.trust_score,
                evidence_completeness: self.evidence_completeness,
                rules: self.rules,
                passing_rules: passing,
                failing_rules: failing,
                risk_heatmap: self.risk_heatmap,
            },
            audit_chain: self.audit_chain,
            timestamp: chrono::Utc::now().timestamp(),
            version: "1.0.0".to_string(),
        }
    }
}

impl Default for TrustVisualizationBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trust_status_output_creation() {
        let tv = generate_demo_visualization();
        let output = TrustStatusOutput::from_visualization(&tv);

        assert_eq!(output.trust_score.overall, 91);
        assert_eq!(output.evidence_completeness.total, 100);
        assert_eq!(output.risk_summary.critical, 1);
        assert_eq!(output.compliance_rules_pass, 2);
    }

    #[test]
    fn test_trust_visualization_builder() {
        let builder = TrustVisualizationBuilder::new()
            .with_trust_score(TrustScoreComponents::new(90, 80, 85, 75))
            .with_evidence(EvidenceCompleteness::new(50, 40, 5, 5))
            .with_rules(vec![RuleStatus {
                rule_name: "test".to_string(),
                rule_type: "test".to_string(),
                severity: "low".to_string(),
                enabled: true,
                passing: true,
                description: "Test rule".to_string(),
            }]);

        let tv = builder.build();
        assert_eq!(tv.compliance_dashboard.trust_score.overall, 84);
        assert_eq!(tv.compliance_dashboard.passing_rules, 1);
    }

    #[test]
    fn test_trust_status_output_json() {
        let tv = generate_demo_visualization();
        let output = TrustStatusOutput::from_visualization(&tv);

        // Should not panic
        let json = serde_json::to_string(&output);
        assert!(json.is_ok());
    }
}
