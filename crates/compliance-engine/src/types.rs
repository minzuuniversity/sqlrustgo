//! Core types for compliance engine

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
