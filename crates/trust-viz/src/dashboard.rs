//! Compliance dashboard

#[derive(Debug, Clone)]
pub struct ComplianceDashboard {
    pub summary: ComplianceSummary,
    pub recent_violations: Vec<ViolationItem>,
    pub evidence_chains: Vec<EvidenceChainSummary>,
}

#[derive(Debug, Clone)]
pub struct ComplianceSummary {
    pub total_batches: u32,
    pub compliant_batches: u32,
    pub violations: u32,
    pub compliance_rate: f64,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl ComplianceSummary {
    pub fn compliance_percentage(&self) -> f64 {
        if self.total_batches == 0 {
            100.0
        } else {
            (self.compliant_batches as f64 / self.total_batches as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_timestamp() -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc)
    }

    #[test]
    fn test_compliance_summary_percentage_with_batches() {
        let summary = ComplianceSummary {
            total_batches: 100,
            compliant_batches: 95,
            violations: 5,
            compliance_rate: 0.95,
            last_updated: test_timestamp(),
        };
        assert_eq!(summary.compliance_percentage(), 95.0);
    }

    #[test]
    fn test_compliance_summary_percentage_zero_batches() {
        let summary = ComplianceSummary {
            total_batches: 0,
            compliant_batches: 0,
            violations: 0,
            compliance_rate: 0.0,
            last_updated: test_timestamp(),
        };
        assert_eq!(summary.compliance_percentage(), 100.0);
    }

    #[test]
    fn test_compliance_summary_percentage_perfect() {
        let summary = ComplianceSummary {
            total_batches: 50,
            compliant_batches: 50,
            violations: 0,
            compliance_rate: 1.0,
            last_updated: test_timestamp(),
        };
        assert_eq!(summary.compliance_percentage(), 100.0);
    }

    #[test]
    fn test_compliance_dashboard_empty() {
        let dashboard = ComplianceDashboard {
            summary: ComplianceSummary {
                total_batches: 0,
                compliant_batches: 0,
                violations: 0,
                compliance_rate: 0.0,
                last_updated: test_timestamp(),
            },
            recent_violations: vec![],
            evidence_chains: vec![],
        };
        assert!(dashboard.recent_violations.is_empty());
        assert!(dashboard.evidence_chains.is_empty());
    }

    #[test]
    fn test_compliance_dashboard_with_violations() {
        let dashboard = ComplianceDashboard {
            summary: ComplianceSummary {
                total_batches: 10,
                compliant_batches: 7,
                violations: 3,
                compliance_rate: 0.7,
                last_updated: test_timestamp(),
            },
            recent_violations: vec![ViolationItem {
                batch_id: "batch-1".to_string(),
                rule_name: "Rule-001".to_string(),
                severity: "high".to_string(),
                description: "Test violation".to_string(),
                timestamp: test_timestamp(),
            }],
            evidence_chains: vec![],
        };
        assert_eq!(dashboard.recent_violations.len(), 1);
        assert_eq!(dashboard.summary.violations, 3);
    }

    #[test]
    fn test_violation_item_creation() {
        let violation = ViolationItem {
            batch_id: "batch-abc".to_string(),
            rule_name: "GDPR-001".to_string(),
            severity: "critical".to_string(),
            description: "Data retention policy violation".to_string(),
            timestamp: test_timestamp(),
        };
        assert_eq!(violation.batch_id, "batch-abc");
        assert_eq!(violation.severity, "critical");
    }

    #[test]
    fn test_evidence_chain_summary() {
        let summary = EvidenceChainSummary {
            batch_id: "batch-xyz".to_string(),
            chain_length: 5,
            is_complete: true,
            last_signature: Some("sig-abc123".to_string()),
        };
        assert!(summary.is_complete);
        assert_eq!(summary.chain_length, 5);
    }

    #[test]
    fn test_evidence_chain_summary_incomplete() {
        let summary = EvidenceChainSummary {
            batch_id: "batch-incomplete".to_string(),
            chain_length: 3,
            is_complete: false,
            last_signature: None,
        };
        assert!(!summary.is_complete);
        assert!(summary.last_signature.is_none());
    }
}

#[derive(Debug, Clone)]
pub struct ViolationItem {
    pub batch_id: String,
    pub rule_name: String,
    pub severity: String,
    pub description: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct EvidenceChainSummary {
    pub batch_id: String,
    pub chain_length: u32,
    pub is_complete: bool,
    pub last_signature: Option<String>,
}
