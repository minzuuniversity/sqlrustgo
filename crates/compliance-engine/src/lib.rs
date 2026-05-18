//! Compliance-as-Code Engine for SQLRustGo GMP

pub mod rule;
pub mod engine;
pub mod evaluator;
pub mod types;

pub use types::{
    ComplianceRule, RuleCondition, ViolationRecord, BatchRecord, Signature
};
pub use engine::ComplianceEngine;
pub use evaluator::RuleEvaluator;
pub use types::*;

#[derive(Debug, Clone)]
pub struct ComplianceCheckResult {
    pub rule_name: String,
    pub passed: bool,
    pub violations: Vec<ViolationRecord>,
    pub evidence: Vec<EvidenceItem>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl ComplianceCheckResult {
    pub fn is_compliant(&self) -> bool {
        self.passed && self.violations.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct EvidenceItem {
    pub item_type: String,
    pub description: String,
    pub hash: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationSeverity {
    Critical,
    Major,
    Minor,
    Advisory,
}

impl ViolationSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::Major => "major",
            Self::Minor => "minor",
            Self::Advisory => "advisory",
        }
    }
}
