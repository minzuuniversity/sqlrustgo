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
