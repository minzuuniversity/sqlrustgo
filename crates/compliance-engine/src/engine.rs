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
