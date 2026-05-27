//! Compliance-as-Code Engine for SQLRustGo GMP

pub mod engine;
pub mod evaluator;
pub mod judge;
pub mod report;
pub mod rule;
pub mod summarizer;
pub mod types;

pub use engine::ComplianceEngine;
pub use evaluator::RuleEvaluator;
pub use judge::types::RuleEvidence;
pub use judge::{
    AuditLogEntry, AuditLogWriter, ComplianceJudge, ConfidenceCalculator, JudgmentAuditor,
    JudgmentRequest, JudgmentResult, JudgmentStatus, RuleParser,
};
pub use types::*;
pub use types::{BatchRecord, ComplianceRule, RuleCondition, Signature, ViolationRecord};

#[derive(Debug, Clone)]
pub struct ComplianceCheckResult {
    pub rule_name: String,
    pub passed: bool,
    pub violations: Vec<ViolationRecord>,
    pub evidence: Vec<EvidenceItem>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl ComplianceCheckResult {
    pub fn is_compliant(&self) -> bool {
        self.passed && self.violations.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct EvidenceItem {
    pub item_type: String,
    pub description: String,
    pub hash: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationSeverity {
    Critical,
    Major,
    Minor,
    Advisory,
}

impl ViolationSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::Major => "major",
            Self::Minor => "minor",
            Self::Advisory => "advisory",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ComplianceCheckResult tests
    mod compliance_check_result_tests {
        use super::*;

        #[test]
        fn test_is_compliant_true() {
            let result = ComplianceCheckResult {
                rule_name: "test_rule".to_string(),
                passed: true,
                violations: vec![],
                evidence: vec![],
                timestamp: chrono::Utc::now(),
            };
            assert!(result.is_compliant());
        }

        #[test]
        fn test_is_compliant_false_violations() {
            let result = ComplianceCheckResult {
                rule_name: "test_rule".to_string(),
                passed: false,
                violations: vec![ViolationRecord {
                    severity: "major".to_string(),
                    message: "Test violation".to_string(),
                    field: None,
                    remediation: None,
                }],
                evidence: vec![],
                timestamp: chrono::Utc::now(),
            };
            assert!(!result.is_compliant());
        }

        #[test]
        fn test_is_compliant_false_not_passed() {
            let result = ComplianceCheckResult {
                rule_name: "test_rule".to_string(),
                passed: false,
                violations: vec![],
                evidence: vec![],
                timestamp: chrono::Utc::now(),
            };
            assert!(!result.is_compliant());
        }
    }

    // EvidenceItem tests
    mod evidence_item_tests {
        use super::*;

        #[test]
        fn test_evidence_item_creation() {
            let item = EvidenceItem {
                item_type: "signature".to_string(),
                description: "Test signature".to_string(),
                hash: Some("abc123".to_string()),
                timestamp: chrono::Utc::now(),
            };
            assert_eq!(item.item_type, "signature");
            assert_eq!(item.description, "Test signature");
            assert!(item.hash.is_some());
        }

        #[test]
        fn test_evidence_item_clone() {
            let item = EvidenceItem {
                item_type: "workflow".to_string(),
                description: "Test workflow".to_string(),
                hash: None,
                timestamp: chrono::Utc::now(),
            };
            let cloned = item.clone();
            assert_eq!(cloned.item_type, item.item_type);
            assert_eq!(cloned.description, item.description);
        }
    }

    // ViolationSeverity tests
    mod violation_severity_tests {
        use super::*;

        #[test]
        fn test_as_str_critical() {
            assert_eq!(ViolationSeverity::Critical.as_str(), "critical");
        }

        #[test]
        fn test_as_str_major() {
            assert_eq!(ViolationSeverity::Major.as_str(), "major");
        }

        #[test]
        fn test_as_str_minor() {
            assert_eq!(ViolationSeverity::Minor.as_str(), "minor");
        }

        #[test]
        fn test_as_str_advisory() {
            assert_eq!(ViolationSeverity::Advisory.as_str(), "advisory");
        }

        #[test]
        fn test_violation_severity_equality() {
            assert_eq!(ViolationSeverity::Critical, ViolationSeverity::Critical);
            assert_ne!(ViolationSeverity::Critical, ViolationSeverity::Major);
        }
    }
}
