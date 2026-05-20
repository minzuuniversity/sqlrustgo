//! Compliance rule definitions

use crate::types::{ComplianceRule, RuleCondition, ViolationSpec};

impl ComplianceRule {
    pub fn new(name: &str, applies_to: Vec<&str>, condition: RuleCondition) -> Self {
        Self {
            name: name.to_string(),
            description: String::new(),
            applies_to: applies_to.into_iter().map(String::from).collect(),
            condition,
            violation: ViolationSpec {
                severity: "major".to_string(),
                message: String::new(),
                remediation: None,
            },
            enabled: true,
        }
    }

    pub fn dual_signature_required(applies_to: &[&str]) -> Self {
        Self::new(
            "dual_signature_required",
            applies_to.into(),
            RuleCondition::Threshold {
                field: "signatures".to_string(),
                operator: ">=".to_string(),
                value: 2.0,
            },
        )
    }

    pub fn with_severity(mut self, severity: &str) -> Self {
        self.violation.severity = severity.to_string();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::RuleCondition;

    #[test]
    fn test_compliance_rule_new() {
        let rule = ComplianceRule::new(
            "test_rule",
            vec!["batch"],
            RuleCondition::Presence {
                field: "batch_id".to_string(),
                required: true,
            },
        );
        assert_eq!(rule.name, "test_rule");
        assert_eq!(rule.applies_to, vec!["batch"]);
        assert!(rule.enabled);
        assert_eq!(rule.violation.severity, "major");
    }

    #[test]
    fn test_compliance_rule_dual_signature_required() {
        let rule = ComplianceRule::dual_signature_required(&["batch", "record"]);
        assert_eq!(rule.name, "dual_signature_required");
        assert_eq!(rule.applies_to, vec!["batch", "record"]);
        match rule.condition {
            RuleCondition::Threshold {
                field,
                operator,
                value,
            } => {
                assert_eq!(field, "signatures");
                assert_eq!(operator, ">=");
                assert_eq!(value, 2.0);
            }
            _ => panic!("Expected Threshold condition"),
        }
    }

    #[test]
    fn test_compliance_rule_with_severity() {
        let rule = ComplianceRule::new(
            "critical_rule",
            vec!["batch"],
            RuleCondition::Presence {
                field: "batch_id".to_string(),
                required: true,
            },
        )
        .with_severity("critical");
        assert_eq!(rule.violation.severity, "critical");
    }

    #[test]
    fn test_compliance_rule_with_severity_chained() {
        let rule = ComplianceRule::new(
            "rule1",
            vec!["batch"],
            RuleCondition::Count {
                field: "items".to_string(),
                min: Some(1),
                max: None,
            },
        )
        .with_severity("minor");
        assert_eq!(rule.violation.severity, "minor");
        assert!(rule.enabled);
    }

    #[test]
    fn test_compliance_rule_new_applies_to_multiple() {
        let rule = ComplianceRule::new(
            "multi_rule",
            vec!["batch", "record", "product"],
            RuleCondition::Threshold {
                field: "count".to_string(),
                operator: ">".to_string(),
                value: 0.0,
            },
        );
        assert_eq!(rule.applies_to.len(), 3);
        assert!(rule.applies_to.contains(&"batch".to_string()));
        assert!(rule.applies_to.contains(&"record".to_string()));
        assert!(rule.applies_to.contains(&"product".to_string()));
    }
}
