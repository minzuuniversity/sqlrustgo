//! Compliance Judge Agent
//!
//! Core agent that evaluates operations against GMP compliance rules using LLM.

use crate::audit::AuditLogWriter;
use crate::confidence::{ConfidenceCalculator, ConfidenceScore};
use crate::ollama::{ChatMessage, ChatRequest, ChatResponse, OllamaClient, OllamaError};
use crate::rules::Rule;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tokio::time::{timeout, Duration};

/// Judgment status enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JudgmentStatus {
    Compliant,
    NonCompliant,
    NeedsReview,
    Timeout,
}

/// Judgment request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgmentRequest {
    /// Operation description to evaluate
    pub operation: String,
    /// Context information
    pub context: Option<String>,
    /// Rules to evaluate against
    pub rules: Vec<Rule>,
    /// Enable streaming response
    pub stream: bool,
}

/// Judgment result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgmentResult {
    /// Judgment status
    pub status: JudgmentStatus,
    /// Reason for judgment
    pub reason: String,
    /// Confidence score
    pub confidence: ConfidenceScore,
    /// Evidence supporting the judgment
    pub evidence: Vec<String>,
    /// Whether timeout occurred
    pub timed_out: bool,
}

/// Errors for compliance judgment
#[derive(Error, Debug)]
pub enum JudgmentError {
    #[error("Ollama error: {0}")]
    OllamaError(#[from] OllamaError),
    #[error("Timeout exceeded")]
    Timeout,
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}

/// Compliance judge agent
pub struct ComplianceJudge {
    client: Arc<dyn OllamaClient>,
    default_model: String,
    timeout_ms: u64,
    audit_writer: Option<AuditLogWriter>,
}

impl ComplianceJudge {
    pub fn new(client: Arc<dyn OllamaClient>) -> Self {
        Self {
            client,
            default_model: "llama3".to_string(),
            timeout_ms: 500,
            audit_writer: None,
        }
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.default_model = model;
        self
    }

    pub fn with_timeout(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
        self
    }

    pub fn with_audit(mut self, writer: AuditLogWriter) -> Self {
        self.audit_writer = Some(writer);
        self
    }

    /// Make a compliance judgment with timeout
    pub async fn judge(&self, request: JudgmentRequest) -> Result<JudgmentResult, JudgmentError> {
        let prompt = self.construct_prompt(&request);
        let chat_request = ChatRequest {
            model: self.default_model.clone(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: prompt,
            }],
            stream: false,
        };

        let result = timeout(
            Duration::from_millis(self.timeout_ms),
            self.client.chat(chat_request),
        )
        .await;

        match result {
            Ok(Ok(response)) => {
                let judgment = self.parse_response(response, &request.rules)?;
                Ok(judgment)
            }
            Ok(Err(e)) => Err(JudgmentError::OllamaError(e)),
            Err(_) => Ok(self.create_timeout_result()),
        }
    }

    /// Construct LLM prompt from request
    fn construct_prompt(&self, request: &JudgmentRequest) -> String {
        let rules_text: Vec<String> = request
            .rules
            .iter()
            .map(|r| format!("- [{}] {:?}: {}", r.id, r.severity, r.text))
            .collect();

        let context = request
            .context
            .as_ref()
            .map(|c| format!("\nContext: {}", c))
            .unwrap_or_default();

        format!(
            r#"You are a GMP compliance auditor. Evaluate the following operation for compliance.

Operation: {}

Rules to check:
{}

{}

Respond in JSON format with:
{{"status": "COMPLIANT" or "NON_COMPLIANT" or "NEEDS_REVIEW", "reason": "...", "evidence": ["..."]}}"#,
            request.operation,
            rules_text.join("\n"),
            context,
        )
    }

    /// Parse LLM response into judgment result
    fn parse_response(
        &self,
        response: ChatResponse,
        _rules: &[Rule],
    ) -> Result<JudgmentResult, JudgmentError> {
        let content = &response.message.content;

        // Try to parse as JSON
        let json_result: Result<serde_json::Value, _> = serde_json::from_str(content);

        match json_result {
            Ok(json) => {
                let status_str = json["status"].as_str().unwrap_or("NEEDS_REVIEW");
                let reason = json["reason"]
                    .as_str()
                    .unwrap_or("No reason provided")
                    .to_string();
                let evidence: Vec<String> = json["evidence"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();

                let status = match status_str.to_uppercase().as_str() {
                    "COMPLIANT" => JudgmentStatus::Compliant,
                    "NON_COMPLIANT" => JudgmentStatus::NonCompliant,
                    _ => JudgmentStatus::NeedsReview,
                };

                let confidence = response
                    .logprob
                    .map(ConfidenceCalculator::from_logprob)
                    .unwrap_or_else(|| ConfidenceCalculator::from_text_analysis(&reason));

                Ok(JudgmentResult {
                    status,
                    reason,
                    confidence,
                    evidence,
                    timed_out: false,
                })
            }
            Err(_) => {
                // Fallback: try to extract status from text
                let content_upper = content.to_uppercase();
                let status = if content_upper.contains("COMPLIANT")
                    && !content_upper.contains("NON_COMPLIANT")
                {
                    JudgmentStatus::Compliant
                } else if content_upper.contains("NON_COMPLIANT") {
                    JudgmentStatus::NonCompliant
                } else {
                    JudgmentStatus::NeedsReview
                };

                let confidence = ConfidenceCalculator::from_text_analysis(content);

                Ok(JudgmentResult {
                    status,
                    reason: content.clone(),
                    confidence,
                    evidence: vec![],
                    timed_out: false,
                })
            }
        }
    }

    /// Create timeout result
    fn create_timeout_result(&self) -> JudgmentResult {
        JudgmentResult {
            status: JudgmentStatus::Timeout,
            reason: "Judgment timed out after 500ms".to_string(),
            confidence: ConfidenceScore {
                score: 0.0,
                explanation: "No confidence - timeout occurred".to_string(),
                logprob: None,
            },
            evidence: vec![],
            timed_out: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::confidence::ConfidenceScore;
    use crate::ollama::{ChatMessage, ChatResponse};
    use crate::rules::{Rule, RuleCategory, RuleLanguage, Severity};
    use async_trait::async_trait;

    struct MockOllamaClient;

    #[async_trait]
    impl OllamaClient for MockOllamaClient {
        async fn chat(&self, _request: ChatRequest) -> Result<ChatResponse, OllamaError> {
            Ok(ChatResponse {
                model: "mock".to_string(),
                message: ChatMessage {
                    role: "assistant".to_string(),
                    content: r#"{"status": "COMPLIANT", "reason": "All rules satisfied", "evidence": ["Temperature within range"]}"#.to_string(),
                },
                logprob: Some(-0.05),
            })
        }

        async fn chat_stream(
            &self,
            _request: ChatRequest,
        ) -> Result<
            tokio::sync::mpsc::Receiver<Result<crate::ollama::StreamChunk, OllamaError>>,
            OllamaError,
        > {
            let (tx, rx) = tokio::sync::mpsc::channel(1);
            Ok(rx)
        }
    }

    #[tokio::test]
    async fn test_judge_compliant() {
        let client = Arc::new(MockOllamaClient);
        let judge = ComplianceJudge::new(client);

        let request = JudgmentRequest {
            operation: "Store vaccine at 5°C".to_string(),
            context: Some("Refrigerated storage area A".to_string()),
            rules: vec![Rule {
                id: "GMP-001".to_string(),
                text: "Temperature must be 2-8°C".to_string(),
                severity: Severity::Critical,
                language: RuleLanguage::English,
                category: RuleCategory::Storage,
                version: None,
            }],
            stream: false,
        };

        let result = judge.judge(request).await.unwrap();
        assert_eq!(result.status, JudgmentStatus::Compliant);
        assert!(!result.timed_out);
    }
}
