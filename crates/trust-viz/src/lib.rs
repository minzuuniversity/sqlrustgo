//! Trust Visualization for SQLRustGo GMP
//!
//! Provides compliance dashboards and evidence chain visualization for GMP workflows.
pub mod dashboard;
pub mod evidence_chain;
pub mod metrics;

pub use dashboard::{ComplianceDashboard, ComplianceSummary};
pub use evidence_chain::EvidenceChainViz;
pub use metrics::{MetricType, MetricValue, TrustMetrics};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dashboard::{EvidenceChainSummary, ViolationItem};

    #[test]
    fn test_compliance_summary_percentage() {
        let summary = ComplianceSummary {
            total_batches: 100,
            compliant_batches: 95,
            violations: 5,
            compliance_rate: 95.0,
            last_updated: chrono::Utc::now(),
        };
        assert_eq!(summary.compliance_percentage(), 95.0);
    }

    #[test]
    fn test_compliance_summary_zero_batches() {
        let summary = ComplianceSummary {
            total_batches: 0,
            compliant_batches: 0,
            violations: 0,
            compliance_rate: 0.0,
            last_updated: chrono::Utc::now(),
        };
        assert_eq!(summary.compliance_percentage(), 100.0);
    }

    #[test]
    fn test_compliance_dashboard_new() {
        let summary = ComplianceSummary {
            total_batches: 50,
            compliant_batches: 48,
            violations: 2,
            compliance_rate: 96.0,
            last_updated: chrono::Utc::now(),
        };
        let dashboard = ComplianceDashboard {
            summary,
            recent_violations: vec![],
            evidence_chains: vec![],
        };
        assert_eq!(dashboard.summary.compliance_percentage(), 96.0);
    }

    #[test]
    fn test_violation_item() {
        let violation = ViolationItem {
            batch_id: "BATCH-001".to_string(),
            rule_name: "TemperatureRange".to_string(),
            severity: "HIGH".to_string(),
            description: "Temperature exceeded limit".to_string(),
            timestamp: chrono::Utc::now(),
        };
        assert_eq!(violation.batch_id, "BATCH-001");
        assert_eq!(violation.severity, "HIGH");
    }

    #[test]
    fn test_evidence_chain_summary() {
        let chain = EvidenceChainSummary {
            batch_id: "BATCH-002".to_string(),
            chain_length: 5,
            is_complete: true,
            last_signature: Some("sig_abc123".to_string()),
        };
        assert!(chain.is_complete);
        assert_eq!(chain.chain_length, 5);
    }

    #[test]
    fn test_metric_value() {
        let metric = MetricValue {
            metric_type: MetricType::ComplianceRate,
            value: 99.95,
            unit: "percent".to_string(),
            timestamp: chrono::Utc::now(),
        };
        matches!(metric.metric_type, MetricType::ComplianceRate);
        assert!((metric.value - 99.95).abs() < 0.001);
    }
}
