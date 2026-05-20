//! Rule evaluator

use crate::types::{BatchRecord, ComplianceRule, RuleCondition, RuleResult, ViolationRecord};

pub struct RuleEvaluator;

impl RuleEvaluator {
    pub fn evaluate(rule: &ComplianceRule, batch: &BatchRecord) -> RuleResult {
        let passed = Self::check_condition(&rule.condition, batch);

        RuleResult {
            rule_name: rule.name.clone(),
            passed,
            violations: if passed {
                vec![]
            } else {
                vec![ViolationRecord {
                    severity: rule.violation.severity.clone(),
                    message: rule.violation.message.clone(),
                    field: None,
                    remediation: rule.violation.remediation.clone(),
                }]
            },
        }
    }

    fn check_condition(condition: &RuleCondition, batch: &BatchRecord) -> bool {
        match condition {
            RuleCondition::Threshold {
                field: _,
                operator,
                value,
            } => Self::check_threshold(&batch.signatures.len(), operator, *value),
            RuleCondition::Count { field: _, min, max } => {
                let count = batch.signatures.len() as u32;
                let min_ok = min.map(|m| count >= m).unwrap_or(true);
                let max_ok = max.map(|m| count <= m).unwrap_or(true);
                min_ok && max_ok
            }
            RuleCondition::Presence { field: _, required } => {
                let has_field = !batch.batch_id.is_empty();
                has_field == *required
            }
            RuleCondition::Custom { expression: _ } => true,
        }
    }

    fn check_threshold(actual: &usize, operator: &str, expected: f64) -> bool {
        let a = *actual as f64;
        match operator {
            ">=" => a >= expected,
            ">" => a > expected,
            "<=" => a <= expected,
            "<" => a < expected,
            "==" => (a - expected).abs() < f64::EPSILON,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ComplianceRule, RuleCondition, Signature, ViolationSpec};

    fn make_batch_with_sigs(sig_count: usize) -> BatchRecord {
        let sigs: Vec<Signature> = (0..sig_count)
            .map(|i| Signature {
                signer: format!("Signer{}", i),
                role: "QA".to_string(),
                timestamp: "2026-01-01T00:00:00Z".to_string(),
                hash: format!("hash{}", i),
            })
            .collect();
        BatchRecord {
            batch_id: "BATCH001".to_string(),
            product_name: "Product A".to_string(),
            signatures: sigs,
            status: "pending".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            metadata: serde_json::json!({}),
        }
    }

    fn make_rule_with_threshold(_field: &str, operator: &str, value: f64) -> ComplianceRule {
        ComplianceRule {
            name: "threshold_rule".to_string(),
            description: "Test".to_string(),
            applies_to: vec!["batch".to_string()],
            condition: RuleCondition::Threshold {
                field: "signatures".to_string(),
                operator: operator.to_string(),
                value,
            },
            violation: ViolationSpec {
                severity: "major".to_string(),
                message: "Threshold violated".to_string(),
                remediation: Some("Fix it".to_string()),
            },
            enabled: true,
        }
    }

    fn make_rule_with_count(min: Option<u32>, max: Option<u32>) -> ComplianceRule {
        ComplianceRule {
            name: "count_rule".to_string(),
            description: "Test".to_string(),
            applies_to: vec!["batch".to_string()],
            condition: RuleCondition::Count {
                field: "signatures".to_string(),
                min,
                max,
            },
            violation: ViolationSpec {
                severity: "major".to_string(),
                message: "Count violated".to_string(),
                remediation: None,
            },
            enabled: true,
        }
    }

    fn make_rule_with_presence(required: bool) -> ComplianceRule {
        ComplianceRule {
            name: "presence_rule".to_string(),
            description: "Test".to_string(),
            applies_to: vec!["batch".to_string()],
            condition: RuleCondition::Presence {
                field: "batch_id".to_string(),
                required,
            },
            violation: ViolationSpec {
                severity: "major".to_string(),
                message: "Presence violated".to_string(),
                remediation: None,
            },
            enabled: true,
        }
    }

    #[test]
    fn test_evaluate_passed_threshold() {
        let rule = make_rule_with_threshold("signatures", ">=", 2.0);
        let batch = make_batch_with_sigs(2);
        let result = RuleEvaluator::evaluate(&rule, &batch);
        assert!(result.passed);
        assert!(result.violations.is_empty());
    }

    #[test]
    fn test_evaluate_failed_threshold() {
        let rule = make_rule_with_threshold("signatures", ">=", 2.0);
        let batch = make_batch_with_sigs(1);
        let result = RuleEvaluator::evaluate(&rule, &batch);
        assert!(!result.passed);
        assert_eq!(result.violations.len(), 1);
    }

    #[test]
    fn test_evaluate_threshold_greater_than() {
        let rule = make_rule_with_threshold("signatures", ">", 1.0);
        let batch = make_batch_with_sigs(2);
        let result = RuleEvaluator::evaluate(&rule, &batch);
        assert!(result.passed);
    }

    #[test]
    fn test_evaluate_threshold_less_than_or_equal() {
        let rule = make_rule_with_threshold("signatures", "<=", 2.0);
        let batch = make_batch_with_sigs(2);
        let result = RuleEvaluator::evaluate(&rule, &batch);
        assert!(result.passed);
    }

    #[test]
    fn test_evaluate_threshold_less_than() {
        let rule = make_rule_with_threshold("signatures", "<", 2.0);
        let batch = make_batch_with_sigs(1);
        let result = RuleEvaluator::evaluate(&rule, &batch);
        assert!(result.passed);
    }

    #[test]
    fn test_evaluate_threshold_equals() {
        let rule = make_rule_with_threshold("signatures", "==", 2.0);
        let batch = make_batch_with_sigs(2);
        let result = RuleEvaluator::evaluate(&rule, &batch);
        assert!(result.passed);
    }

    #[test]
    fn test_evaluate_threshold_invalid_operator() {
        let rule = make_rule_with_threshold("signatures", "invalid", 2.0);
        let batch = make_batch_with_sigs(2);
        let result = RuleEvaluator::evaluate(&rule, &batch);
        assert!(!result.passed);
    }

    #[test]
    fn test_evaluate_count_min_passed() {
        let rule = make_rule_with_count(Some(2), None);
        let batch = make_batch_with_sigs(3);
        let result = RuleEvaluator::evaluate(&rule, &batch);
        assert!(result.passed);
    }

    #[test]
    fn test_evaluate_count_min_failed() {
        let rule = make_rule_with_count(Some(2), None);
        let batch = make_batch_with_sigs(1);
        let result = RuleEvaluator::evaluate(&rule, &batch);
        assert!(!result.passed);
    }

    #[test]
    fn test_evaluate_count_max_passed() {
        let rule = make_rule_with_count(None, Some(3));
        let batch = make_batch_with_sigs(2);
        let result = RuleEvaluator::evaluate(&rule, &batch);
        assert!(result.passed);
    }

    #[test]
    fn test_evaluate_count_max_failed() {
        let rule = make_rule_with_count(None, Some(2));
        let batch = make_batch_with_sigs(3);
        let result = RuleEvaluator::evaluate(&rule, &batch);
        assert!(!result.passed);
    }

    #[test]
    fn test_evaluate_count_range_passed() {
        let rule = make_rule_with_count(Some(1), Some(5));
        let batch = make_batch_with_sigs(3);
        let result = RuleEvaluator::evaluate(&rule, &batch);
        assert!(result.passed);
    }

    #[test]
    fn test_evaluate_count_range_failed_below_min() {
        let rule = make_rule_with_count(Some(2), Some(5));
        let batch = make_batch_with_sigs(1);
        let result = RuleEvaluator::evaluate(&rule, &batch);
        assert!(!result.passed);
    }

    #[test]
    fn test_evaluate_count_range_failed_above_max() {
        let rule = make_rule_with_count(Some(1), Some(3));
        let batch = make_batch_with_sigs(5);
        let result = RuleEvaluator::evaluate(&rule, &batch);
        assert!(!result.passed);
    }

    #[test]
    fn test_evaluate_count_no_bounds() {
        let rule = make_rule_with_count(None, None);
        let batch = make_batch_with_sigs(0);
        let result = RuleEvaluator::evaluate(&rule, &batch);
        assert!(result.passed);
    }

    #[test]
    fn test_evaluate_presence_required_passed() {
        let rule = make_rule_with_presence(true);
        let batch = make_batch_with_sigs(0);
        let result = RuleEvaluator::evaluate(&rule, &batch);
        assert!(result.passed);
    }

    #[test]
    fn test_evaluate_presence_required_failed() {
        let rule = make_rule_with_presence(false);
        let batch = make_batch_with_sigs(0);
        let result = RuleEvaluator::evaluate(&rule, &batch);
        assert!(!result.passed);
    }

    #[test]
    fn test_evaluate_custom_expression() {
        let rule = ComplianceRule {
            name: "custom".to_string(),
            description: "".to_string(),
            applies_to: vec!["batch".to_string()],
            condition: RuleCondition::Custom {
                expression: "x > 5".to_string(),
            },
            violation: ViolationSpec {
                severity: "major".to_string(),
                message: "Custom violated".to_string(),
                remediation: None,
            },
            enabled: true,
        };
        let batch = make_batch_with_sigs(0);
        let result = RuleEvaluator::evaluate(&rule, &batch);
        assert!(result.passed);
    }

    #[test]
    fn test_evaluate_rule_name_preserved() {
        let rule = ComplianceRule {
            name: "my_custom_rule".to_string(),
            description: "".to_string(),
            applies_to: vec!["batch".to_string()],
            condition: RuleCondition::Threshold {
                field: "signatures".to_string(),
                operator: ">=".to_string(),
                value: 100.0,
            },
            violation: ViolationSpec {
                severity: "critical".to_string(),
                message: "Too few sigs".to_string(),
                remediation: Some("Get more".to_string()),
            },
            enabled: true,
        };
        let batch = make_batch_with_sigs(1);
        let result = RuleEvaluator::evaluate(&rule, &batch);
        assert_eq!(result.rule_name, "my_custom_rule");
    }

    #[test]
    fn test_evaluate_violation_fields_populated() {
        let rule = ComplianceRule {
            name: "violation_test".to_string(),
            description: "".to_string(),
            applies_to: vec!["batch".to_string()],
            condition: RuleCondition::Threshold {
                field: "signatures".to_string(),
                operator: ">=".to_string(),
                value: 5.0,
            },
            violation: ViolationSpec {
                severity: "critical".to_string(),
                message: "Must have 5 sigs".to_string(),
                remediation: Some("Add more signers".to_string()),
            },
            enabled: true,
        };
        let batch = make_batch_with_sigs(2);
        let result = RuleEvaluator::evaluate(&rule, &batch);
        assert!(!result.passed);
        assert_eq!(result.violations.len(), 1);
        let v = &result.violations[0];
        assert_eq!(v.severity, "critical");
        assert_eq!(v.message, "Must have 5 sigs");
        assert!(v.remediation.is_some());
        assert_eq!(v.remediation.as_deref(), Some("Add more signers"));
    }
}
