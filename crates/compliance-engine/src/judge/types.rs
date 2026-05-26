//! Judgment types for LLM-powered compliance evaluation

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Judgment status enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum JudgmentStatus {
    /// Operation is compliant with rules
    Compliant,
    /// Operation violates compliance rules
    NonCompliant,
    /// Confidence below threshold, needs human review
    NeedsReview,
    /// Judgment timed out (>500ms)
    Timeout,
    /// Error during judgment
    Error,
}

/// Request for compliance judgment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgmentRequest {
    /// Unique operation identifier
    pub operation_id: String,
    /// Operation description to evaluate
    pub operation_description: String,
    /// Applicable rule IDs to evaluate against
    pub rule_ids: Vec<String>,
    /// Language code (e.g., "zh", "en", "ja")
    pub language: Option<String>,
    /// Whether to enable streaming response
    pub streaming: bool,
}

impl JudgmentRequest {
    pub fn new(operation_id: &str, operation_description: &str) -> Self {
        Self {
            operation_id: operation_id.to_string(),
            operation_description: operation_description.to_string(),
            rule_ids: Vec::new(),
            language: None,
            streaming: false,
        }
    }

    pub fn with_rules(mut self, rule_ids: Vec<String>) -> Self {
        self.rule_ids = rule_ids;
        self
    }

    pub fn with_language(mut self, lang: &str) -> Self {
        self.language = Some(lang.to_string());
        self
    }
}

/// Result of compliance judgment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgmentResult {
    /// Operation ID from request
    pub operation_id: String,
    /// Judgment status
    pub status: JudgmentStatus,
    /// Confidence score (0.0-1.0)
    pub confidence_score: f32,
    /// Human-readable explanation of judgment
    pub explanation: String,
    /// Relevant rule sections that influenced decision
    pub evidence: Vec<RuleEvidence>,
    /// Time taken for judgment in milliseconds
    pub latency_ms: u64,
    /// LLM model used
    pub model: Option<String>,
    /// Timestamp of judgment
    pub timestamp: DateTime<Utc>,
}

/// Evidence citation from a rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleEvidence {
    /// Rule ID
    pub rule_id: String,
    /// Rule text that was applied
    pub rule_text: String,
    /// Severity level if violated
    pub severity: Option<String>,
}

impl JudgmentResult {
    pub fn compliant(
        operation_id: &str,
        confidence_score: f32,
        explanation: String,
        latency_ms: u64,
    ) -> Self {
        Self {
            operation_id: operation_id.to_string(),
            status: JudgmentStatus::Compliant,
            confidence_score,
            explanation,
            evidence: Vec::new(),
            latency_ms,
            model: None,
            timestamp: Utc::now(),
        }
    }

    pub fn non_compliant(
        operation_id: &str,
        confidence_score: f32,
        explanation: String,
        evidence: Vec<RuleEvidence>,
        latency_ms: u64,
    ) -> Self {
        Self {
            operation_id: operation_id.to_string(),
            status: JudgmentStatus::NonCompliant,
            confidence_score,
            explanation,
            evidence,
            latency_ms,
            model: None,
            timestamp: Utc::now(),
        }
    }

    pub fn needs_review(operation_id: &str, confidence_score: f32, explanation: String) -> Self {
        Self {
            operation_id: operation_id.to_string(),
            status: JudgmentStatus::NeedsReview,
            confidence_score,
            explanation,
            evidence: Vec::new(),
            latency_ms: 0,
            model: None,
            timestamp: Utc::now(),
        }
    }

    pub fn timeout(operation_id: &str) -> Self {
        Self {
            operation_id: operation_id.to_string(),
            status: JudgmentStatus::Timeout,
            confidence_score: 0.0,
            explanation: "Judgment timed out after 500ms".to_string(),
            evidence: Vec::new(),
            latency_ms: 500,
            model: None,
            timestamp: Utc::now(),
        }
    }

    pub fn error(operation_id: &str, error_message: &str) -> Self {
        Self {
            operation_id: operation_id.to_string(),
            status: JudgmentStatus::Error,
            confidence_score: 0.0,
            explanation: error_message.to_string(),
            evidence: Vec::new(),
            latency_ms: 0,
            model: None,
            timestamp: Utc::now(),
        }
    }

    pub fn with_model(mut self, model: &str) -> Self {
        self.model = Some(model.to_string());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_judgment_request_new() {
        let req = JudgmentRequest::new("op-123", "User performed action X");
        assert_eq!(req.operation_id, "op-123");
        assert_eq!(req.operation_description, "User performed action X");
        assert!(req.rule_ids.is_empty());
        assert!(!req.streaming);
    }

    #[test]
    fn test_judgment_request_with_rules() {
        let req = JudgmentRequest::new("op-123", "action")
            .with_rules(vec!["rule-1".to_string(), "rule-2".to_string()]);
        assert_eq!(req.rule_ids.len(), 2);
    }

    #[test]
    fn test_judgment_result_compliant() {
        let result = JudgmentResult::compliant(
            "op-123",
            0.95,
            "Operation meets all compliance requirements".to_string(),
            150,
        );
        assert_eq!(result.status, JudgmentStatus::Compliant);
        assert!((result.confidence_score - 0.95).abs() < f32::EPSILON);
        assert_eq!(result.latency_ms, 150);
    }

    #[test]
    fn test_judgment_result_timeout() {
        let result = JudgmentResult::timeout("op-123");
        assert_eq!(result.status, JudgmentStatus::Timeout);
        assert_eq!(result.latency_ms, 500);
    }
}
