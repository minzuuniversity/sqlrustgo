//! Compliance Rule Parser
//!
//! Parses and validates GMP compliance rules from YAML/JSON format.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Rule severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

/// Supported languages for rules
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleLanguage {
    Chinese,
    English,
    Japanese,
    Korean,
}

/// Rule category
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleCategory {
    Documentation,
    Equipment,
    Personnel,
    Production,
    QualityControl,
    Storage,
    Validation,
    Other,
}

/// Compliance rule structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    pub text: String,
    pub severity: Severity,
    pub language: RuleLanguage,
    pub category: RuleCategory,
    pub version: Option<String>,
}

/// Errors for rule parsing and validation
#[derive(Error, Debug)]
pub enum RuleError {
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Missing required field: {0}")]
    MissingField(String),
}

/// Rule parser for YAML/JSON
pub struct RuleParser;

impl RuleParser {
    /// Parse a rule from JSON string
    pub fn parse_json(s: &str) -> Result<Rule, RuleError> {
        serde_json::from_str(s).map_err(|e| RuleError::ParseError(e.to_string()))
    }

    /// Parse multiple rules from JSON array
    pub fn parse_json_array(s: &str) -> Result<Vec<Rule>, RuleError> {
        serde_json::from_str(s).map_err(|e| RuleError::ParseError(e.to_string()))
    }

    /// Parse a rule from YAML string
    pub fn parse_yaml(s: &str) -> Result<Rule, RuleError> {
        serde_yaml::from_str(s).map_err(|e| RuleError::ParseError(e.to_string()))
    }

    /// Parse multiple rules from YAML
    pub fn parse_yaml_all(s: &str) -> Result<Vec<Rule>, RuleError> {
        serde_yaml::from_str(s).map_err(|e| RuleError::ParseError(e.to_string()))
    }
}

/// Rule validator for completeness checking
pub struct RuleValidator;

impl RuleValidator {
    /// Validate a rule has all required fields
    pub fn validate(rule: &Rule) -> Result<(), RuleError> {
        if rule.id.is_empty() {
            return Err(RuleError::ValidationError(
                "Rule id cannot be empty".to_string(),
            ));
        }
        if rule.text.is_empty() {
            return Err(RuleError::ValidationError(
                "Rule text cannot be empty".to_string(),
            ));
        }
        Ok(())
    }

    /// Validate a batch of rules
    pub fn validate_batch(rules: &[Rule]) -> Vec<Result<(), RuleError>> {
        rules.iter().map(Self::validate).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_rule() {
        let json = r#"{
            "id": "GMP-001",
            "text": "Temperature must be maintained between 2-8°C",
            "severity": "critical",
            "language": "english",
            "category": "storage"
        }"#;

        let rule = RuleParser::parse_json(json).unwrap();
        assert_eq!(rule.id, "GMP-001");
        assert_eq!(rule.severity, Severity::Critical);
    }

    #[test]
    fn test_validate_rule() {
        let rule = Rule {
            id: "GMP-001".to_string(),
            text: "Temperature must be maintained".to_string(),
            severity: Severity::Critical,
            language: RuleLanguage::English,
            category: RuleCategory::Storage,
            version: None,
        };

        assert!(RuleValidator::validate(&rule).is_ok());
    }

    #[test]
    fn test_validate_empty_id() {
        let rule = Rule {
            id: "".to_string(),
            text: "Temperature must be maintained".to_string(),
            severity: Severity::Critical,
            language: RuleLanguage::English,
            category: RuleCategory::Storage,
            version: None,
        };

        assert!(RuleValidator::validate(&rule).is_err());
    }
}
