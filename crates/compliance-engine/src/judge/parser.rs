//! Compliance rule parser
//!
//! Parses compliance rules from YAML/JSON format.

use crate::types::{ComplianceRule, RuleCondition};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Parser errors
#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Invalid YAML format: {0}")]
    InvalidYaml(String),
    #[error("Invalid JSON format: {0}")]
    InvalidJson(String),
    #[error("Missing required field: {0}")]
    MissingField(String),
    #[error("Invalid field value: {0}")]
    InvalidValue(String),
}

/// Validation errors
#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("Rule incomplete: {0}")]
    Incomplete(String),
    #[error("Rule inconsistent: {0}")]
    Inconsistent(String),
}

/// Parsed compliance rule from YAML/JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedRule {
    pub id: String,
    pub text: String,
    pub severity: String,
    pub language: String,
    pub category: String,
}

/// Result of rule validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
}

impl ValidationResult {
    pub fn valid() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
        }
    }

    pub fn invalid(errors: Vec<String>) -> Self {
        Self {
            valid: false,
            errors,
        }
    }
}

/// Rule parser for YAML/JSON formats
pub struct RuleParser {
    strict_validation: bool,
}

impl Default for RuleParser {
    fn default() -> Self {
        Self {
            strict_validation: true,
        }
    }
}

impl RuleParser {
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse a rule from YAML string
    pub fn parse_yaml(&self, yaml_str: &str) -> Result<ParsedRule, ParseError> {
        serde_yaml::from_str(yaml_str).map_err(|e| ParseError::InvalidYaml(e.to_string()))
    }

    /// Parse a rule from JSON string
    pub fn parse_json(&self, json_str: &str) -> Result<ParsedRule, ParseError> {
        serde_json::from_str(json_str).map_err(|e| ParseError::InvalidJson(e.to_string()))
    }

    /// Validate a parsed rule
    pub fn validate(&self, rule: &ParsedRule) -> ValidationResult {
        let mut errors = Vec::new();

        if rule.id.is_empty() {
            errors.push("Rule ID cannot be empty".to_string());
        }

        if rule.text.is_empty() {
            errors.push("Rule text cannot be empty".to_string());
        }

        if rule.severity.is_empty() {
            errors.push("Severity cannot be empty".to_string());
        } else if !["critical", "major", "minor", "advisory"].contains(&rule.severity.as_str()) {
            errors.push(format!("Invalid severity: {}", rule.severity));
        }

        if rule.language.is_empty() {
            errors.push("Language cannot be empty".to_string());
        }

        if self.strict_validation && errors.is_empty() {
            // Check for consistency
            if rule.text.len() < 10 {
                errors.push("Rule text too short to be meaningful".to_string());
            }
        }

        if errors.is_empty() {
            ValidationResult::valid()
        } else {
            ValidationResult::invalid(errors)
        }
    }

    /// Convert ParsedRule to internal ComplianceRule
    pub fn to_compliance_rule(&self, parsed: &ParsedRule) -> ComplianceRule {
        ComplianceRule {
            name: parsed.id.clone(),
            description: parsed.text.clone(),
            applies_to: vec![parsed.category.clone()],
            condition: RuleCondition::Presence {
                field: "operation".to_string(),
                required: true,
            },
            violation: crate::types::ViolationSpec {
                severity: parsed.severity.clone(),
                message: parsed.text.clone(),
                remediation: None,
            },
            enabled: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_YAML_RULE: &str = r#"
id: "rule-001"
text: "All batch records must be reviewed and signed by a qualified person"
severity: "major"
language: "en"
category: "batch_record"
"#;

    const VALID_JSON_RULE: &str = r#"{
        "id": "rule-001",
        "text": "所有批记录必须由合格人员审核和签署",
        "severity": "major",
        "language": "zh",
        "category": "batch_record"
    }"#;

    #[test]
    fn test_parse_yaml_valid() {
        let parser = RuleParser::new();
        let result = parser.parse_yaml(VALID_YAML_RULE).unwrap();
        assert_eq!(result.id, "rule-001");
        assert!(result.text.contains("reviewed"));
        assert_eq!(result.severity, "major");
        assert_eq!(result.language, "en");
    }

    #[test]
    fn test_parse_json_valid() {
        let parser = RuleParser::new();
        let result = parser.parse_json(VALID_JSON_RULE).unwrap();
        assert_eq!(result.id, "rule-001");
        assert!(result.text.contains("批记录"));
        assert_eq!(result.language, "zh");
    }

    #[test]
    fn test_validate_valid() {
        let parser = RuleParser::new();
        let rule = parser.parse_yaml(VALID_YAML_RULE).unwrap();
        let result = parser.validate(&rule);
        assert!(result.valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validate_empty_id() {
        let parser = RuleParser::new();
        let rule = ParsedRule {
            id: "".to_string(),
            text: "Valid text".to_string(),
            severity: "major".to_string(),
            language: "en".to_string(),
            category: "test".to_string(),
        };
        let result = parser.validate(&rule);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("ID")));
    }

    #[test]
    fn test_validate_invalid_severity() {
        let parser = RuleParser::new();
        let rule = ParsedRule {
            id: "rule-001".to_string(),
            text: "Valid text".to_string(),
            severity: "invalid".to_string(),
            language: "en".to_string(),
            category: "test".to_string(),
        };
        let result = parser.validate(&rule);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.contains("severity")));
    }

    #[test]
    fn test_to_compliance_rule() {
        let parser = RuleParser::new();
        let parsed = parser.parse_yaml(VALID_YAML_RULE).unwrap();
        let rule = parser.to_compliance_rule(&parsed);
        assert_eq!(rule.name, "rule-001");
        assert_eq!(rule.violation.severity, "major");
    }
}
