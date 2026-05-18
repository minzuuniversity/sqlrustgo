//! Trust Visualization API
//!
//! Provides data structures and functions for GMP Trust Visualization dashboard.
//! Aggregates audit chain, compliance, evidence, and workflow data for display.

use serde::{Deserialize, Serialize};

/// Trust score components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustScoreComponents {
    /// Audit chain integrity score (0-100)
    pub audit_chain_integrity: u8,
    /// Evidence completeness score (0-100)
    pub evidence_completeness: u8,
    /// Signature validity score (0-100)
    pub signature_validity: u8,
    /// Workflow compliance score (0-100)
    pub workflow_compliance: u8,
    /// Overall trust score (0-100)
    pub overall: u8,
}

impl TrustScoreComponents {
    /// Calculate overall trust score as weighted average
    pub fn calculate_overall(&self) -> u8 {
        let sum = (self.audit_chain_integrity as u32) * 30
            + (self.evidence_completeness as u32) * 25
            + (self.signature_validity as u32) * 25
            + (self.workflow_compliance as u32) * 20;
        (sum / 100) as u8
    }

    /// Create from individual scores
    pub fn new(
        audit_chain_integrity: u8,
        evidence_completeness: u8,
        signature_validity: u8,
        workflow_compliance: u8,
    ) -> Self {
        let overall = Self {
            audit_chain_integrity,
            evidence_completeness,
            signature_validity,
            workflow_compliance,
            overall: 0,
        };
        let calculated_overall = overall.calculate_overall();
        Self {
            overall: calculated_overall,
            ..overall
        }
    }

    /// Trust level based on overall score
    pub fn trust_level(&self) -> &'static str {
        match self.overall {
            90..=100 => "Critical",
            75..=89 => "High",
            50..=74 => "Medium",
            25..=49 => "Low",
            _ => "Critical",
        }
    }
}

/// Audit chain visualization node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditChainNode {
    /// Sequence number
    pub seq: u64,
    /// Node ID
    pub id: String,
    /// Timestamp
    pub timestamp: i64,
    /// Operation type
    pub operation: String,
    /// Checksum
    pub checksum: String,
    /// Status: valid, invalid, pending
    pub status: String,
}

/// Audit chain visualization link
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditChainLink {
    /// Source node ID
    pub source: String,
    /// Target node ID
    pub target: String,
    /// Link type
    pub link_type: String,
}

/// Audit chain visualization data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditChainVisualization {
    pub nodes: Vec<AuditChainNode>,
    pub links: Vec<AuditChainLink>,
    pub total_entries: usize,
    pub valid_entries: usize,
    pub invalid_entries: usize,
}

/// Risk item for heatmap
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskItem {
    /// Risk category
    pub category: String,
    /// Risk level: critical, high, medium, low
    pub level: String,
    /// Risk description
    pub description: String,
    /// Affected item count
    pub count: usize,
    /// Risk score (0-100)
    pub score: u8,
}

/// Risk heatmap data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskHeatmap {
    pub items: Vec<RiskItem>,
    pub total_risks: usize,
    pub critical_count: usize,
    pub high_count: usize,
    pub medium_count: usize,
    pub low_count: usize,
}

impl RiskHeatmap {
    /// Create empty heatmap
    pub fn empty() -> Self {
        RiskHeatmap {
            items: Vec::new(),
            total_risks: 0,
            critical_count: 0,
            high_count: 0,
            medium_count: 0,
            low_count: 0,
        }
    }

    /// Add a risk item and update counts
    pub fn add_risk(&mut self, item: RiskItem) {
        match item.level.as_str() {
            "critical" => self.critical_count += 1,
            "high" => self.high_count += 1,
            "medium" => self.medium_count += 1,
            "low" => self.low_count += 1,
            _ => {}
        }
        self.total_risks += 1;
        self.items.push(item);
    }
}

/// Evidence completeness status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceCompleteness {
    /// Total evidence records
    pub total: usize,
    /// Complete evidence (all signatures present)
    pub complete: usize,
    /// Partial evidence (some signatures missing)
    pub partial: usize,
    /// Missing evidence
    pub missing: usize,
    /// Completeness percentage (0-100)
    pub completeness_pct: u8,
}

impl EvidenceCompleteness {
    pub fn new(total: usize, complete: usize, partial: usize, missing: usize) -> Self {
        let completeness_pct = if total > 0 {
            ((complete as f64 / total as f64) * 100.0) as u8
        } else {
            0
        };
        EvidenceCompleteness {
            total,
            complete,
            partial,
            missing,
            completeness_pct,
        }
    }
}

/// Compliance rule status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleStatus {
    pub rule_name: String,
    pub rule_type: String,
    pub severity: String,
    pub enabled: bool,
    pub passing: bool,
    pub description: String,
}

/// Compliance summary for dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceDashboard {
    pub trust_score: TrustScoreComponents,
    pub evidence_completeness: EvidenceCompleteness,
    pub rules: Vec<RuleStatus>,
    pub passing_rules: usize,
    pub failing_rules: usize,
    pub risk_heatmap: RiskHeatmap,
}

/// Trust visualization response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustVisualization {
    pub compliance_dashboard: ComplianceDashboard,
    pub audit_chain: AuditChainVisualization,
    pub timestamp: i64,
    pub version: String,
}

impl TrustVisualization {
    /// Create new visualization with current timestamp
    pub fn new() -> Self {
        TrustVisualization {
            compliance_dashboard: ComplianceDashboard {
                trust_score: TrustScoreComponents::new(0, 0, 0, 0),
                evidence_completeness: EvidenceCompleteness::new(0, 0, 0, 0),
                rules: Vec::new(),
                passing_rules: 0,
                failing_rules: 0,
                risk_heatmap: RiskHeatmap::empty(),
            },
            audit_chain: AuditChainVisualization {
                nodes: Vec::new(),
                links: Vec::new(),
                total_entries: 0,
                valid_entries: 0,
                invalid_entries: 0,
            },
            timestamp: chrono::Utc::now().timestamp(),
            version: "1.0.0".to_string(),
        }
    }
}

impl Default for TrustVisualization {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trust_score_calculation() {
        let score = TrustScoreComponents::new(100, 80, 90, 70);
        assert_eq!(score.overall, 86); // 30*100 + 25*80 + 25*90 + 20*70 = 8650/100
        assert_eq!(score.trust_level(), "High");
    }

    #[test]
    fn test_trust_score_critical() {
        let score = TrustScoreComponents::new(30, 20, 40, 10);
        assert_eq!(score.overall, 26);
        assert_eq!(score.trust_level(), "Low");
    }

    #[test]
    fn test_evidence_completeness() {
        let ec = EvidenceCompleteness::new(100, 80, 15, 5);
        assert_eq!(ec.completeness_pct, 80);
    }

    #[test]
    fn test_risk_heatmap_counts() {
        let mut heatmap = RiskHeatmap::empty();
        heatmap.add_risk(RiskItem {
            category: "audit".to_string(),
            level: "critical".to_string(),
            description: "Test".to_string(),
            count: 1,
            score: 95,
        });
        heatmap.add_risk(RiskItem {
            category: "signature".to_string(),
            level: "high".to_string(),
            description: "Test".to_string(),
            count: 2,
            score: 75,
        });
        assert_eq!(heatmap.total_risks, 2);
        assert_eq!(heatmap.critical_count, 1);
        assert_eq!(heatmap.high_count, 1);
    }

    #[test]
    fn test_trust_visualization_default() {
        let tv = TrustVisualization::default();
        assert_eq!(tv.version, "1.0.0");
    }
}
