use chrono::Utc;
use sqlrustgo_compliance_engine::{ComplianceCheckResult, EvidenceItem, ViolationSeverity};

#[test]
fn test_compliance_check_result_is_compliant() {
    let result = ComplianceCheckResult {
        rule_name: "21CFR11".to_string(),
        passed: true,
        violations: vec![],
        evidence: vec![],
        timestamp: Utc::now(),
    };
    assert!(result.is_compliant());
}

#[test]
fn test_compliance_check_result_not_compliant_with_violations() {
    let result = ComplianceCheckResult {
        rule_name: "21CFR11".to_string(),
        passed: false,
        violations: vec![],
        evidence: vec![],
        timestamp: Utc::now(),
    };
    assert!(!result.is_compliant());
}

#[test]
fn test_violation_severity_as_str() {
    assert_eq!(ViolationSeverity::Critical.as_str(), "critical");
    assert_eq!(ViolationSeverity::Major.as_str(), "major");
    assert_eq!(ViolationSeverity::Minor.as_str(), "minor");
    assert_eq!(ViolationSeverity::Advisory.as_str(), "advisory");
}

#[test]
fn test_evidence_item() {
    let item = EvidenceItem {
        item_type: "signature".to_string(),
        description: "Operator signature".to_string(),
        hash: Some("abc123".to_string()),
        timestamp: Utc::now(),
    };
    assert_eq!(item.item_type, "signature");
    assert!(item.hash.is_some());
}
