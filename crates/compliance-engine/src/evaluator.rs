//! Rule evaluator

use crate::types::{BatchRecord, ComplianceRule, RuleCondition, RuleResult, ViolationRecord};
use crate::ViolationSeverity;

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
            RuleCondition::Threshold { field, operator, value } => {
                Self::check_threshold(&batch.signatures.len(), operator, *value)
            }
            RuleCondition::Count { field, min, max } => {
                let count = batch.signatures.len() as u32;
                let min_ok = min.map(|m| count >= m).unwrap_or(true);
                let max_ok = max.map(|m| count <= m).unwrap_or(true);
                min_ok && max_ok
            }
            RuleCondition::Presence { field, required } => {
                let has_field = !batch.batch_id.is_empty();
                has_field == *required
            }
            RuleCondition::Custom { expression: _ } => {
                true // Placeholder for custom expressions
            }
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
