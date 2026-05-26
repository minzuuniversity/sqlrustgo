//! Compliance Judge Agent
//!
//! Core LLM-powered compliance judgment engine.

use super::audit::JudgmentAuditor;
use super::confidence::{ConfidenceCalculator, ResponseClarity};
use super::parser::RuleParser;
use super::types::{JudgmentRequest, JudgmentResult, JudgmentStatus, RuleEvidence};
use async_trait::async_trait;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::time::timeout;

/// Judgment timeout (500ms as per requirements)
const JUDGMENT_TIMEOUT_MS: u64 = 500;

/// Ollama client trait for LLM inference
#[async_trait]
pub trait OllamaClient: Send + Sync {
    async fn chat(&self, prompt: &str, model: &str) -> Result<OllamaResponse, OllamaError>;
}

/// Ollama response from chat completion
#[derive(Debug, Clone)]
pub struct OllamaResponse {
    pub content: String,
    pub logprob: Option<f32>,
    pub model: String,
}

/// Ollama errors
#[derive(Error, Debug)]
pub enum OllamaError {
    #[error("Network error: {0}")]
    Network(String),
    #[error("Model not found: {0}")]
    ModelNotFound(String),
    #[error("Timeout")]
    Timeout,
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

/// Compliance Judge Agent
pub struct ComplianceJudge {
    ollama_client: Box<dyn OllamaClient>,
    #[allow(dead_code)]
    rule_parser: RuleParser,
    confidence_calc: ConfidenceCalculator,
    auditor: JudgmentAuditor,
    model: String,
}

impl ComplianceJudge {
    pub fn new(ollama_client: Box<dyn OllamaClient>, model: &str) -> Self {
        Self {
            ollama_client,
            rule_parser: RuleParser::new(),
            confidence_calc: ConfidenceCalculator::new(),
            auditor: JudgmentAuditor::new(),
            model: model.to_string(),
        }
    }

    /// Make a compliance judgment with 500ms timeout
    pub async fn judge(&self, request: &JudgmentRequest) -> JudgmentResult {
        let start = Instant::now();

        // Execute judgment with timeout
        let result = timeout(
            Duration::from_millis(JUDGMENT_TIMEOUT_MS),
            self.execute_judgment(request),
        )
        .await;

        let latency_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(Ok(mut judgment)) => {
                judgment.latency_ms = latency_ms;
                judgment.model = Some(self.model.clone());
                judgment
            }
            Ok(Err(e)) => JudgmentResult::error(&request.operation_id, &e.to_string()),
            Err(_) => JudgmentResult::timeout(&request.operation_id),
        }
    }

    /// Execute the actual judgment logic
    async fn execute_judgment(
        &self,
        request: &JudgmentRequest,
    ) -> Result<JudgmentResult, OllamaError> {
        // Build prompt from request
        let prompt = self.build_prompt(request);

        // Call LLM
        let response = self.ollama_client.chat(&prompt, &self.model).await?;

        // Calculate confidence
        let clarity = ResponseClarity::from_response(&response.content);
        let confidence_score = self.confidence_calc.calculate(response.logprob, clarity, 0); // rule_citations = 0 for now

        // Parse response into judgment
        let judgment =
            self.parse_judgment_response(request, &response.content, confidence_score, clarity);

        // Record to audit log
        for evidence in &judgment.evidence {
            self.auditor.record(
                &request.operation_id,
                &evidence.rule_id,
                &format!("{:?}", judgment.status),
                judgment.confidence_score,
                Some(&self.model),
                judgment.latency_ms,
            );
        }

        Ok(judgment)
    }

    /// Build prompt for LLM
    fn build_prompt(&self, request: &JudgmentRequest) -> String {
        let lang = request.language.as_deref().unwrap_or("zh");
        let lang_instruction = match lang {
            "zh" => "请用中文回答。",
            "en" => "Please respond in English.",
            "ja" => "日本語でお答えください。",
            _ => "请用中文回答。",
        };

        format!(
            r#"你是 GMP 合规判断助手。请严格根据规则判断以下操作是否合规。

## 操作描述
{}

## 判断要求
1. 如果合规，返回 "COMPLIANT" 并说明理由
2. 如果不合规，返回 "NON_COMPLIANT" 并说明违反的规则和严重程度
3. 如果不确定，返回 "NEEDS_REVIEW"
4. 提供置信度评分 (0.0-1.0)

## 输出格式
STATUS: [COMPLIANT|NON_COMPLIANT|NEEDS_REVIEW]
CONFIDENCE: [0.0-1.0]
REASON: [简要说明]
{}

注意：只基于提供的操作描述进行判断，不要假设额外信息。"#,
            request.operation_description, lang_instruction
        )
    }

    /// Parse LLM response into JudgmentResult
    fn parse_judgment_response(
        &self,
        request: &JudgmentRequest,
        response: &str,
        confidence_score: f32,
        clarity: ResponseClarity,
    ) -> JudgmentResult {
        let response_upper = response.to_uppercase();

        // Parse status
        let status =
            if response_upper.contains("NON_COMPLIANT") || response_upper.contains("不合规") {
                JudgmentStatus::NonCompliant
            } else if response_upper.contains("NEEDS_REVIEW") || response_upper.contains("需要审核")
            {
                JudgmentStatus::NeedsReview
            } else {
                JudgmentStatus::Compliant
            };

        // Determine final status based on confidence
        let final_status = if confidence_score < 0.7 || clarity == ResponseClarity::Contradictory {
            JudgmentStatus::NeedsReview
        } else {
            status
        };

        // Extract explanation (everything after REASON:)
        let explanation = if let Some(pos) = response.find("REASON:") {
            response[pos + 7..].trim().to_string()
        } else {
            response.trim().to_string()
        };

        // Build evidence from rules
        let evidence: Vec<RuleEvidence> = request
            .rule_ids
            .iter()
            .map(|rule_id| RuleEvidence {
                rule_id: rule_id.clone(),
                rule_text: format!("Rule {}", rule_id),
                severity: Some("major".to_string()),
            })
            .collect();

        JudgmentResult {
            operation_id: request.operation_id.clone(),
            status: final_status,
            confidence_score,
            explanation,
            evidence,
            latency_ms: 0,
            model: None,
            timestamp: chrono::Utc::now(),
        }
    }
}

/// No-op Ollama client for testing
pub struct NoopOllamaClient;

#[async_trait]
impl OllamaClient for NoopOllamaClient {
    async fn chat(&self, _prompt: &str, _model: &str) -> Result<OllamaResponse, OllamaError> {
        Ok(OllamaResponse {
            content: "COMPLIANT\nCONFIDENCE: 0.9\nREASON: No violations detected".to_string(),
            logprob: Some(-0.5),
            model: "noop".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_judge_compliant() {
        let client = Box::new(NoopOllamaClient);
        let judge = ComplianceJudge::new(client, "test-model");

        let request = JudgmentRequest::new("op-001", "User performed valid action");
        let result = judge.judge(&request).await;

        assert_eq!(result.status, JudgmentStatus::Compliant);
        assert!(result.confidence_score > 0.0);
        assert!(!result.explanation.is_empty());
    }

    #[test]
    fn test_build_prompt() {
        let client = Box::new(NoopOllamaClient);
        let judge = ComplianceJudge::new(client, "test-model");

        let request = JudgmentRequest::new("op-001", "Test operation").with_language("zh");

        let prompt = judge.build_prompt(&request);
        assert!(prompt.contains("Test operation"));
        assert!(prompt.contains("请用中文回答"));
    }

    #[test]
    fn test_parse_compliant_response() {
        let client = Box::new(NoopOllamaClient);
        let judge = ComplianceJudge::new(client, "test-model");

        let request = JudgmentRequest::new("op-001", "Test");
        let response = "COMPLIANT\nCONFIDENCE: 0.95\nREASON: All rules satisfied";

        let result =
            judge.parse_judgment_response(&request, response, 0.95, ResponseClarity::Clear);

        assert_eq!(result.status, JudgmentStatus::Compliant);
    }

    #[test]
    fn test_parse_non_compliant_response() {
        let client = Box::new(NoopOllamaClient);
        let judge = ComplianceJudge::new(client, "test-model");

        let request = JudgmentRequest::new("op-001", "Test").with_rules(vec!["rule-1".to_string()]);
        let response = "NON_COMPLIANT\nCONFIDENCE: 0.85\nREASON: Violates rule-1";

        let result =
            judge.parse_judgment_response(&request, response, 0.85, ResponseClarity::Clear);

        assert_eq!(result.status, JudgmentStatus::NonCompliant);
        assert!(!result.evidence.is_empty());
    }

    #[test]
    fn test_parse_low_confidence_needs_review() {
        let client = Box::new(NoopOllamaClient);
        let judge = ComplianceJudge::new(client, "test-model");

        let request = JudgmentRequest::new("op-001", "Ambiguous operation");
        let response = "COMPLIANT\nCONFIDENCE: 0.5\nREASON: Uncertain";

        let result = judge.parse_judgment_response(
            &request,
            response,
            0.5, // Low confidence
            ResponseClarity::Ambiguous,
        );

        // Should be overridden to NEEDS_REVIEW due to low confidence
        assert_eq!(result.status, JudgmentStatus::NeedsReview);
    }
}
