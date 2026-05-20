//! Compliance engine - loads and manages rules

use crate::types::ComplianceRule;
use std::collections::HashMap;

pub struct ComplianceEngine {
    rules: HashMap<String, ComplianceRule>,
}

impl ComplianceEngine {
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
        }
    }

    pub fn load_rule(&mut self, rule: ComplianceRule) {
        self.rules.insert(rule.name.clone(), rule);
    }

    pub fn load_rules(&mut self, rules: Vec<ComplianceRule>) {
        for rule in rules {
            self.load_rule(rule);
        }
    }

    pub fn get_rule(&self, name: &str) -> Option<&ComplianceRule> {
        self.rules.get(name)
    }

    pub fn rules_for(&self, entity_type: &str) -> Vec<&ComplianceRule> {
        self.rules
            .values()
            .filter(|r| r.applies_to.iter().any(|t| t == entity_type))
            .collect()
    }

    pub fn enabled_rules(&self) -> Vec<&ComplianceRule> {
        self.rules.values().filter(|r| r.enabled).collect()
    }

    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
}

impl Default for ComplianceEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ComplianceRule, RuleCondition, ViolationSpec};

    fn make_rule(name: &str, applies_to: Vec<&str>) -> ComplianceRule {
        ComplianceRule {
            name: name.to_string(),
            description: "Test rule".to_string(),
            applies_to: applies_to.into_iter().map(String::from).collect(),
            condition: RuleCondition::Threshold {
                field: "signatures".to_string(),
                operator: ">=".to_string(),
                value: 2.0,
            },
            violation: ViolationSpec {
                severity: "major".to_string(),
                message: "Violation".to_string(),
                remediation: None,
            },
            enabled: true,
        }
    }

    #[test]
    fn test_engine_new() {
        let engine = ComplianceEngine::new();
        assert_eq!(engine.rule_count(), 0);
    }

    #[test]
    fn test_engine_load_rule() {
        let mut engine = ComplianceEngine::new();
        let rule = make_rule("dual_sig", vec!["batch"]);
        engine.load_rule(rule);
        assert_eq!(engine.rule_count(), 1);
        assert!(engine.get_rule("dual_sig").is_some());
    }

    #[test]
    fn test_engine_load_rules() {
        let mut engine = ComplianceEngine::new();
        let rules = vec![
            make_rule("rule1", vec!["batch"]),
            make_rule("rule2", vec!["record"]),
        ];
        engine.load_rules(rules);
        assert_eq!(engine.rule_count(), 2);
    }

    #[test]
    fn test_engine_get_rule_not_found() {
        let engine = ComplianceEngine::new();
        assert!(engine.get_rule("nonexistent").is_none());
    }

    #[test]
    fn test_engine_rules_for() {
        let mut engine = ComplianceEngine::new();
        engine.load_rule(make_rule("rule1", vec!["batch", "record"]));
        engine.load_rule(make_rule("rule2", vec!["batch"]));
        engine.load_rule(make_rule("rule3", vec!["record"]));

        let batch_rules = engine.rules_for("batch");
        assert_eq!(batch_rules.len(), 2);

        let record_rules = engine.rules_for("record");
        assert_eq!(record_rules.len(), 2);

        let other_rules = engine.rules_for("unknown");
        assert!(other_rules.is_empty());
    }

    #[test]
    fn test_engine_enabled_rules() {
        let mut engine = ComplianceEngine::new();
        let mut rule1 = make_rule("rule1", vec!["batch"]);
        rule1.enabled = true;
        let mut rule2 = make_rule("rule2", vec!["batch"]);
        rule2.enabled = false;
        engine.load_rule(rule1);
        engine.load_rule(rule2);

        let enabled = engine.enabled_rules();
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].name, "rule1");
    }

    #[test]
    fn test_engine_default() {
        let engine = ComplianceEngine::default();
        assert_eq!(engine.rule_count(), 0);
    }
}
