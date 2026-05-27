//! Core types for compliance engine

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A compliance rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceRule {
    pub name: String,
    pub description: String,
    pub applies_to: Vec<String>,
    pub condition: RuleCondition,
    pub violation: ViolationSpec,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RuleCondition {
    Threshold {
        field: String,
        operator: String,
        value: f64,
    },
    Count {
        field: String,
        min: Option<u32>,
        max: Option<u32>,
    },
    Presence {
        field: String,
        required: bool,
    },
    Custom {
        expression: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViolationSpec {
    pub severity: String,
    pub message: String,
    pub remediation: Option<String>,
}

/// Batch record for compliance evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRecord {
    pub batch_id: String,
    pub product_name: String,
    pub signatures: Vec<Signature>,
    pub status: String,
    pub created_at: String,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signature {
    pub signer: String,
    pub role: String,
    pub timestamp: String,
    pub hash: String,
}

/// Result of rule evaluation
#[derive(Debug, Clone)]
pub struct RuleResult {
    pub rule_name: String,
    pub passed: bool,
    pub violations: Vec<ViolationRecord>,
}

#[derive(Debug, Clone)]
pub struct ViolationRecord {
    pub severity: String,
    pub message: String,
    pub field: Option<String>,
    pub remediation: Option<String>,
}

// ============================================================
// Audit Chain Summarizer types — SPEC v3.5.0 #1368
// ============================================================

/// Importance level for key events
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Importance {
    Critical,
    Major,
    Minor,
    Info,
}

impl Importance {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::Major => "major",
            Self::Minor => "minor",
            Self::Info => "info",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "critical" => Self::Critical,
            "major" => Self::Major,
            "minor" => Self::Minor,
            _ => Self::Info,
        }
    }
}

/// Alert severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Critical,
    Major,
    Minor,
    Info,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::Major => "major",
            Self::Minor => "minor",
            Self::Info => "info",
        }
    }
}

/// Alert type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertType {
    SecurityConcern,
    MissingSignature,
    SelfApproval,
    OffHoursBulkModification,
    ParameterOutOfRange,
    AccessAnomaly,
    CapaMissing,
}

impl AlertType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SecurityConcern => "security_concern",
            Self::MissingSignature => "missing_signature",
            Self::SelfApproval => "self_approval",
            Self::OffHoursBulkModification => "off_hours_bulk_modification",
            Self::ParameterOutOfRange => "parameter_out_of_range",
            Self::AccessAnomaly => "access_anomaly",
            Self::CapaMissing => "capa_missing",
        }
    }
}

/// A key event extracted from an audit chain entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyEvent {
    pub event_id: String,
    pub event_type: String,
    pub description: String,
    pub actor: String,
    pub timestamp: i64,
    pub importance: Importance,
    #[serde(default)]
    pub related_entities: Vec<String>,
}

impl KeyEvent {
    pub fn new(
        event_id: String,
        event_type: String,
        description: String,
        actor: String,
        timestamp: i64,
        importance: Importance,
    ) -> Self {
        Self {
            event_id,
            event_type,
            description,
            actor,
            timestamp,
            importance,
            related_entities: Vec::new(),
        }
    }
}

/// An alert generated from anomaly detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub alert_type: AlertType,
    pub severity: Severity,
    pub description: String,
    #[serde(default)]
    pub evidence: Vec<String>,
    pub recommendation: String,
}

impl Alert {
    pub fn new(
        alert_type: AlertType,
        severity: Severity,
        description: String,
        evidence: Vec<String>,
        recommendation: String,
    ) -> Self {
        Self {
            alert_type,
            severity,
            description,
            evidence,
            recommendation,
        }
    }

    /// Serialize to JSON matching the SPEC alert format
    pub fn to_spec_json(&self) -> serde_json::Value {
        serde_json::json!({
            "type": self.alert_type.as_str(),
            "severity": self.severity.as_str(),
            "description": self.description,
            "evidence": self.evidence,
            "recommendation": self.recommendation,
        })
    }
}

/// Chain health score components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainHealth {
    /// Overall health score 0-100
    pub score: f64,
    /// Completeness score 0-100
    pub completeness: f64,
    /// Consistency score 0-100
    pub consistency: f64,
    /// Timeliness score 0-100
    pub timeliness: f64,
    /// Authenticity score 0-100
    pub authenticity: f64,
}

/// Time range for a chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainTimeRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

/// Complete audit chain summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditSummary {
    pub chain_id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub time_range: ChainTimeRange,
    pub total_events: usize,
    #[serde(default)]
    pub key_events: Vec<KeyEvent>,
    pub anomaly_score: f64,
    #[serde(default)]
    pub alerts: Vec<Alert>,
    pub executive_summary: String,
    pub detailed_summary: String,
    pub generated_at: i64,
}

/// Audit entry as loaded from storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub entry_id: String,
    pub chain_id: String,
    pub event_type: String,
    pub description: String,
    pub actor: String,
    pub timestamp: i64,
    pub fields: std::collections::HashMap<String, String>,
}

/// Query for batch summarizing multiple chains
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditQuery {
    pub entity_type: Option<String>,
    pub time_range: Option<ChainTimeRange>,
    pub min_anomaly_score: Option<f64>,
    pub limit: Option<usize>,
}

impl Default for AuditQuery {
    fn default() -> Self {
        Self {
            entity_type: None,
            time_range: None,
            min_anomaly_score: None,
            limit: Some(100),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ComplianceRule tests
    mod compliance_rule_tests {
        use super::*;

        #[test]
        fn test_compliance_rule_creation() {
            let rule = ComplianceRule {
                name: "test_rule".to_string(),
                description: "Test description".to_string(),
                applies_to: vec!["batch".to_string()],
                condition: RuleCondition::Threshold {
                    field: "signatures".to_string(),
                    operator: ">=".to_string(),
                    value: 1.0,
                },
                violation: ViolationSpec {
                    severity: "major".to_string(),
                    message: "Test violation".to_string(),
                    remediation: Some("Fix it".to_string()),
                },
                enabled: true,
            };
            assert_eq!(rule.name, "test_rule");
            assert!(rule.enabled);
        }

        #[test]
        fn test_compliance_rule_clone() {
            let rule = ComplianceRule {
                name: "clone_test".to_string(),
                description: "Clone test".to_string(),
                applies_to: vec!["product".to_string()],
                condition: RuleCondition::Count {
                    field: "signatures".to_string(),
                    min: Some(1),
                    max: None,
                },
                violation: ViolationSpec {
                    severity: "minor".to_string(),
                    message: "Clone violation".to_string(),
                    remediation: None,
                },
                enabled: false,
            };
            let cloned = rule.clone();
            assert_eq!(cloned.name, rule.name);
            assert_eq!(cloned.applies_to, rule.applies_to);
        }
    }

    // RuleCondition tests
    mod rule_condition_tests {
        use super::*;

        #[test]
        fn test_threshold_condition() {
            let cond = RuleCondition::Threshold {
                field: "count".to_string(),
                operator: ">=".to_string(),
                value: 5.0,
            };
            match cond {
                RuleCondition::Threshold {
                    field,
                    operator,
                    value,
                } => {
                    assert_eq!(field, "count");
                    assert_eq!(operator, ">=");
                    assert_eq!(value, 5.0);
                }
                _ => panic!("Expected Threshold variant"),
            }
        }

        #[test]
        fn test_count_condition() {
            let cond = RuleCondition::Count {
                field: "items".to_string(),
                min: Some(1),
                max: Some(10),
            };
            match cond {
                RuleCondition::Count { field, min, max } => {
                    assert_eq!(field, "items");
                    assert_eq!(min, Some(1));
                    assert_eq!(max, Some(10));
                }
                _ => panic!("Expected Count variant"),
            }
        }

        #[test]
        fn test_presence_condition_required() {
            let cond = RuleCondition::Presence {
                field: "batch_id".to_string(),
                required: true,
            };
            match cond {
                RuleCondition::Presence { field, required } => {
                    assert_eq!(field, "batch_id");
                    assert!(required);
                }
                _ => panic!("Expected Presence variant"),
            }
        }

        #[test]
        fn test_presence_condition_not_required() {
            let cond = RuleCondition::Presence {
                field: "optional_field".to_string(),
                required: false,
            };
            match cond {
                RuleCondition::Presence { field, required } => {
                    assert_eq!(field, "optional_field");
                    assert!(!required);
                }
                _ => panic!("Expected Presence variant"),
            }
        }

        #[test]
        fn test_custom_condition() {
            let cond = RuleCondition::Custom {
                expression: "x > 5".to_string(),
            };
            match cond {
                RuleCondition::Custom { expression } => {
                    assert_eq!(expression, "x > 5");
                }
                _ => panic!("Expected Custom variant"),
            }
        }
    }

    // ViolationSpec tests
    mod violation_spec_tests {
        use super::*;

        #[test]
        fn test_violation_spec_with_remediation() {
            let spec = ViolationSpec {
                severity: "critical".to_string(),
                message: "Critical violation".to_string(),
                remediation: Some("Apply fix".to_string()),
            };
            assert_eq!(spec.severity, "critical");
            assert!(spec.remediation.is_some());
        }

        #[test]
        fn test_violation_spec_without_remediation() {
            let spec = ViolationSpec {
                severity: "minor".to_string(),
                message: "Minor issue".to_string(),
                remediation: None,
            };
            assert_eq!(spec.severity, "minor");
            assert!(spec.remediation.is_none());
        }
    }

    // BatchRecord tests
    mod batch_record_tests {
        use super::*;

        #[test]
        fn test_batch_record_creation() {
            let record = BatchRecord {
                batch_id: "BATCH001".to_string(),
                product_name: "TestProduct".to_string(),
                signatures: vec![],
                status: "pending".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                metadata: serde_json::json!({}),
            };
            assert_eq!(record.batch_id, "BATCH001");
            assert_eq!(record.product_name, "TestProduct");
            assert!(record.signatures.is_empty());
        }

        #[test]
        fn test_batch_record_with_signatures() {
            let record = BatchRecord {
                batch_id: "BATCH002".to_string(),
                product_name: "Product2".to_string(),
                signatures: vec![
                    Signature {
                        signer: "Alice".to_string(),
                        role: "approver".to_string(),
                        timestamp: "2024-01-01T00:00:00Z".to_string(),
                        hash: "abc123".to_string(),
                    },
                    Signature {
                        signer: "Bob".to_string(),
                        role: "reviewer".to_string(),
                        timestamp: "2024-01-02T00:00:00Z".to_string(),
                        hash: "def456".to_string(),
                    },
                ],
                status: "approved".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                metadata: serde_json::json!({"key": "value"}),
            };
            assert_eq!(record.signatures.len(), 2);
            assert_eq!(record.signatures[0].signer, "Alice");
            assert_eq!(record.signatures[1].signer, "Bob");
        }
    }

    // Signature tests
    mod signature_tests {
        use super::*;

        #[test]
        fn test_signature_creation() {
            let sig = Signature {
                signer: "TestSigner".to_string(),
                role: "admin".to_string(),
                timestamp: "2024-01-01T12:00:00Z".to_string(),
                hash: "hash123".to_string(),
            };
            assert_eq!(sig.signer, "TestSigner");
            assert_eq!(sig.role, "admin");
            assert_eq!(sig.hash, "hash123");
        }

        #[test]
        fn test_signature_clone() {
            let sig = Signature {
                signer: "Cloner".to_string(),
                role: "user".to_string(),
                timestamp: "2024-01-01T12:00:00Z".to_string(),
                hash: "clone_hash".to_string(),
            };
            let cloned = sig.clone();
            assert_eq!(cloned.signer, sig.signer);
            assert_eq!(cloned.hash, sig.hash);
        }
    }

    // RuleResult tests
    mod rule_result_tests {
        use super::*;

        #[test]
        fn test_rule_result_passed() {
            let result = RuleResult {
                rule_name: "passing_rule".to_string(),
                passed: true,
                violations: vec![],
            };
            assert!(result.passed);
            assert!(result.violations.is_empty());
        }

        #[test]
        fn test_rule_result_failed() {
            let result = RuleResult {
                rule_name: "failing_rule".to_string(),
                passed: false,
                violations: vec![ViolationRecord {
                    severity: "major".to_string(),
                    message: "Violation occurred".to_string(),
                    field: Some("field1".to_string()),
                    remediation: Some("Fix field1".to_string()),
                }],
            };
            assert!(!result.passed);
            assert_eq!(result.violations.len(), 1);
            assert_eq!(result.violations[0].severity, "major");
        }
    }

    // ViolationRecord tests
    mod violation_record_tests {
        use super::*;

        #[test]
        fn test_violation_record_full() {
            let record = ViolationRecord {
                severity: "critical".to_string(),
                message: "Critical error".to_string(),
                field: Some("serial_number".to_string()),
                remediation: Some("Replace component".to_string()),
            };
            assert_eq!(record.severity, "critical");
            assert!(record.field.is_some());
            assert!(record.remediation.is_some());
        }

        #[test]
        fn test_violation_record_minimal() {
            let record = ViolationRecord {
                severity: "minor".to_string(),
                message: "Minor issue".to_string(),
                field: None,
                remediation: None,
            };
            assert_eq!(record.severity, "minor");
            assert!(record.field.is_none());
            assert!(record.remediation.is_none());
        }
    }
}
