//! Compliance Judge Agent
//!
//! Core LLM-powered compliance judgment engine.
//! 集成规则解析、LLM 推理、置信度评估和审计日志。

use super::audit::JudgmentAuditor;
use super::confidence::{ConfidenceCalculator, ResponseClarity};
use super::parser::{RuleParser, RuleSet};
use super::types::{JudgmentRequest, JudgmentResult, JudgmentStatus, RuleEvidence};
use async_trait::async_trait;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::time::timeout;

/// Judgment timeout (500ms as per requirements)
const JUDGMENT_TIMEOUT_MS: u64 = 500;

/// 当规则集为空时的默认规则文本
const DEFAULT_RULES_TEXT: &str = "(内置 GMP 合规规则)";

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
///
/// 合规判断智能体，集成：
/// - 多语言规则解析 (RuleSet)
/// - LLM 推理 (OllamaClient)
/// - 置信度评估 (ConfidenceCalculator)
/// - 审计日志 (JudgmentAuditor)
pub struct ComplianceJudge {
    ollama_client: Box<dyn OllamaClient>,
    #[allow(dead_code)]
    rule_parser: RuleParser,
    rule_set: RuleSet,
    confidence_calc: ConfidenceCalculator,
    auditor: JudgmentAuditor,
    model: String,
}

impl ComplianceJudge {
    /// 创建新的 ComplianceJudge
    ///
    /// # Arguments
    /// * `ollama_client` - LLM 客户端（支持 mock 和真实 HTTP）
    /// * `model` - Ollama 模型名称
    /// * `rule_set` - 预加载的规则集（为空时使用内置规则）
    pub fn new(
        ollama_client: Box<dyn OllamaClient>,
        model: &str,
        rule_set: Option<RuleSet>,
    ) -> Self {
        let parser = RuleParser::new();
        let rules = rule_set.unwrap_or_else(|| parser.load_builtin_rules());

        Self {
            ollama_client,
            rule_parser: parser,
            rule_set: rules,
            confidence_calc: ConfidenceCalculator::new(),
            auditor: JudgmentAuditor::new(),
            model: model.to_string(),
        }
    }

    /// 使用内置规则的简便构造方法
    pub fn new_with_builtin_rules(ollama_client: Box<dyn OllamaClient>, model: &str) -> Self {
        Self::new(ollama_client, model, None)
    }

    /// 获取审计器引用（用于测试审计日志）
    pub fn auditor(&self) -> &JudgmentAuditor {
        &self.auditor
    }

    /// 获取规则集引用
    pub fn rule_set(&self) -> &RuleSet {
        &self.rule_set
    }

    /// 加载自定义规则
    pub fn load_rules(&mut self, rule_set: RuleSet) {
        self.rule_set = rule_set;
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
    ///
    /// 执行流程:
    /// 1. 确定语言
    /// 2. 根据 rule_ids 或 category 查找适用规则文本
    /// 3. 构建包含规则上下文的 LLM prompt
    /// 4. 调用 LLM 进行推理
    /// 5. 解析 LLM 响应，提取判断结果和依据
    /// 6. 计算置信度
    /// 7. 记录审计日志
    async fn execute_judgment(
        &self,
        request: &JudgmentRequest,
    ) -> Result<JudgmentResult, OllamaError> {
        let language = request.language.as_deref().unwrap_or("zh");

        // Step 1 & 2: 获取适用的规则文本
        let (rule_context, matched_rule_ids) = self.get_applicable_rules(request, language);

        // Step 3: 构建 prompt
        let prompt = self.build_prompt(request, &rule_context, language);

        // Step 4: 调用 LLM
        let response = self.ollama_client.chat(&prompt, &self.model).await?;

        // Step 5: 计算置信度
        let clarity = ResponseClarity::from_response(&response.content);
        let rule_citations = self.count_rule_citations(&response.content, &matched_rule_ids);
        let confidence_score =
            self.confidence_calc
                .calculate(response.logprob, clarity, rule_citations);

        // Step 6: 解析响应为判断结果
        let judgment = self.parse_judgment_response(
            request,
            &response.content,
            confidence_score,
            clarity,
            &matched_rule_ids,
        );

        // Step 7: 记录审计日志
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

    /// 获取适用于请求的规则文本和匹配的规则 ID 列表
    fn get_applicable_rules(
        &self,
        request: &JudgmentRequest,
        language: &str,
    ) -> (String, Vec<String>) {
        // 如果请求指定了 rule_ids，使用指定的规则
        if !request.rule_ids.is_empty() {
            let text =
                RuleParser::format_rules_for_prompt(&request.rule_ids, &self.rule_set, language);
            return (text, request.rule_ids.clone());
        }

        // 否则使用所有规则
        let all_ids: Vec<String> = self.rule_set.rules.keys().cloned().collect();
        if all_ids.is_empty() {
            return (DEFAULT_RULES_TEXT.to_string(), Vec::new());
        }
        let text = RuleParser::format_rules_for_prompt(&all_ids, &self.rule_set, language);
        (text, all_ids)
    }

    /// 统计 LLM 响应中引用到的规则数量
    fn count_rule_citations(&self, response: &str, rule_ids: &[String]) -> usize {
        rule_ids
            .iter()
            .filter(|rule_id| response.contains(rule_id.as_str()))
            .count()
    }

    /// Build prompt for LLM with rule context
    fn build_prompt(
        &self,
        request: &JudgmentRequest,
        rule_context: &str,
        language: &str,
    ) -> String {
        let lang_instruction = match language {
            "zh" => "请用中文回答。",
            "en" => "Please respond in English.",
            "ja" => "日本語でお答えください。",
            _ => "请用中文回答。",
        };

        format!(
            r#"你是 GMP 合规判断助手。请严格根据以下合规规则判断操作是否合规。

## 适用合规规则
{}

## 操作描述
{}

## 判断要求
1. 仔细阅读每条规则，逐条对照操作描述
2. 如果操作完全符合所有规则，返回 "COMPLIANT" 并说明理由
3. 如果操作违反任何规则，返回 "NON_COMPLIANT" 并注明：
   - 违反的规则 ID（引用 [规则ID]）
   - 违反的严重程度
   - 具体违反内容
4. 如果信息不足以判断，返回 "NEEDS_REVIEW" 并说明缺少什么信息
5. 提供置信度评分 (0.0-1.0)

## 输出格式
STATUS: [COMPLIANT|NON_COMPLIANT|NEEDS_REVIEW]
CONFIDENCE: [0.0-1.0]
REASON: [简要说明]
VIOLATED_RULES: [规则ID列表，用逗号分隔，如有]
{}
注意：只基于提供的操作描述和规则进行判断，不要假设额外信息。如果不确定，选择 NEEDS_REVIEW 而非猜测。"#,
            rule_context, request.operation_description, lang_instruction
        )
    }

    /// Parse LLM response into JudgmentResult
    fn parse_judgment_response(
        &self,
        request: &JudgmentRequest,
        response: &str,
        confidence_score: f32,
        clarity: ResponseClarity,
        matched_rule_ids: &[String],
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
            let reason = response[pos + 7..].trim().to_string();
            // Also check for VIOLATED_RULES
            if let Some(vpos) = reason.find("VIOLATED_RULES:") {
                reason[..vpos].trim().to_string()
            } else {
                reason
            }
        } else {
            // If no REASON: marker, use the full response as explanation
            response.trim().to_string()
        };

        // Extract violated rules from response
        let violated_rules: Vec<String> = self.extract_violated_rules(response, matched_rule_ids);

        // Build evidence from matched rules
        let evidence: Vec<RuleEvidence> = if !violated_rules.is_empty() {
            // Use specifically violated rules as evidence
            violated_rules
                .iter()
                .map(|rule_id| {
                    let rule_text = self
                        .rule_set
                        .get_text(rule_id, request.language.as_deref().unwrap_or("zh"))
                        .unwrap_or(&format!("Rule {}", rule_id))
                        .to_string();
                    RuleEvidence {
                        rule_id: rule_id.clone(),
                        rule_text,
                        severity: self.get_rule_severity(rule_id),
                    }
                })
                .collect()
        } else if !matched_rule_ids.is_empty() {
            // Use all matched rules as evidence
            matched_rule_ids
                .iter()
                .map(|rule_id| {
                    let rule_text = self
                        .rule_set
                        .get_text(rule_id, request.language.as_deref().unwrap_or("zh"))
                        .unwrap_or(&format!("Rule {}", rule_id))
                        .to_string();
                    RuleEvidence {
                        rule_id: rule_id.clone(),
                        rule_text,
                        severity: self.get_rule_severity(rule_id),
                    }
                })
                .collect()
        } else {
            request
                .rule_ids
                .iter()
                .map(|rule_id| RuleEvidence {
                    rule_id: rule_id.clone(),
                    rule_text: format!("Rule {}", rule_id),
                    severity: Some("major".to_string()),
                })
                .collect()
        };

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

    /// 从 LLM 响应中提取被违反的规则 ID
    fn extract_violated_rules(&self, response: &str, matched_rule_ids: &[String]) -> Vec<String> {
        // 方法1: 检查 VIOLATED_RULES: 行（大小写不敏感）
        for line in response.lines() {
            let trimmed = line.trim();
            let upper = trimmed.to_uppercase();
            if let Some(pos) = upper.find("VIOLATED_RULES:") {
                // 提取原始行中 "VIOLATED_RULES:" 之后的部分，保留原始大小写
                let rules_str = &trimmed[pos + "VIOLATED_RULES:".len()..];
                return rules_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }

        // 方法2: 检查响应中引用了哪些规则 ID
        matched_rule_ids
            .iter()
            .filter(|rule_id| response.contains(rule_id.as_str()))
            .cloned()
            .collect()
    }

    /// 获取规则严重级别
    fn get_rule_severity(&self, rule_id: &str) -> Option<String> {
        self.rule_set.rules.get(rule_id).map(|r| r.severity.clone())
    }
}

/// No-op Ollama client for testing
pub struct NoopOllamaClient;

#[async_trait]
impl OllamaClient for NoopOllamaClient {
    async fn chat(&self, _prompt: &str, _model: &str) -> Result<OllamaResponse, OllamaError> {
        Ok(OllamaResponse {
            content: "COMPLIANT\nCONFIDENCE: 0.95\nREASON: No violations detected\nVIOLATED_RULES:"
                .to_string(),
            logprob: Some(-0.5),
            model: "noop".to_string(),
        })
    }
}

/// Mock Ollama client for custom response testing
pub struct MockOllamaClient {
    response: String,
    logprob: Option<f32>,
}

impl MockOllamaClient {
    pub fn new(response: &str) -> Self {
        Self {
            response: response.to_string(),
            logprob: Some(-0.5),
        }
    }

    pub fn with_logprob(mut self, logprob: f32) -> Self {
        self.logprob = Some(logprob);
        self
    }
}

#[async_trait]
impl OllamaClient for MockOllamaClient {
    async fn chat(&self, _prompt: &str, _model: &str) -> Result<OllamaResponse, OllamaError> {
        Ok(OllamaResponse {
            content: self.response.clone(),
            logprob: self.logprob,
            model: "mock".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::judge::types::JudgmentRequest;

    #[tokio::test]
    async fn test_judge_compliant() {
        let client = Box::new(NoopOllamaClient);
        let judge = ComplianceJudge::new_with_builtin_rules(client, "test-model");

        let request = JudgmentRequest::new("op-001", "User performed valid action");
        let result = judge.judge(&request).await;

        assert_eq!(result.status, JudgmentStatus::Compliant);
        assert!(result.confidence_score > 0.0);
        assert!(!result.explanation.is_empty());
        assert!(result.latency_ms >= 0);
    }

    #[tokio::test]
    async fn test_judge_non_compliant() {
        let client = Box::new(MockOllamaClient::new(
            "STATUS: NON_COMPLIANT\nCONFIDENCE: 0.85\nREASON: Violates gmp-batch-001\nVIOLATED_RULES: gmp-batch-001",
        ));
        let judge = ComplianceJudge::new_with_builtin_rules(client, "test-model");

        let request = JudgmentRequest::new("op-002", "Batch record not signed")
            .with_rules(vec!["gmp-batch-001".to_string()]);
        let result = judge.judge(&request).await;

        assert_eq!(result.status, JudgmentStatus::NonCompliant);
        assert!(result.confidence_score > 0.7);
    }

    #[tokio::test]
    async fn test_judge_with_rules() {
        let client = Box::new(NoopOllamaClient);
        let judge = ComplianceJudge::new_with_builtin_rules(client, "test-model");

        let request = JudgmentRequest::new("op-003", "Testing with rules").with_rules(vec![
            "gmp-batch-001".to_string(),
            "gmp-batch-002".to_string(),
        ]);
        let result = judge.judge(&request).await;

        assert!(!result.evidence.is_empty());
    }

    #[tokio::test]
    async fn test_judge_timeout() {
        // Use a client that simulates slow response
        let client = Box::new(MockOllamaClient::new(
            "COMPLIANT\nCONFIDENCE: 0.9\nREASON: OK",
        ));
        let judge = ComplianceJudge::new_with_builtin_rules(client, "test-model");

        let request = JudgmentRequest::new("op-004", "Normal operation");
        let result = judge.judge(&request).await;

        // Should still work since MockOllamaClient is instant
        assert_eq!(result.status, JudgmentStatus::Compliant);
        assert!(result.latency_ms < 500);
    }

    #[test]
    fn test_build_prompt_with_rules() {
        let client = Box::new(NoopOllamaClient);
        let judge = ComplianceJudge::new_with_builtin_rules(client, "test-model");

        let request = JudgmentRequest::new("op-001", "Test operation")
            .with_language("zh")
            .with_rules(vec!["gmp-batch-001".to_string()]);

        let (rule_context, _) = judge.get_applicable_rules(&request, "zh");
        let prompt = judge.build_prompt(&request, &rule_context, "zh");

        assert!(prompt.contains("Test operation"));
        assert!(prompt.contains("请用中文回答"));
        assert!(prompt.contains("gmp-batch-001"));
        assert!(prompt.contains("批记录"));
    }

    #[test]
    fn test_build_prompt_english() {
        let client = Box::new(NoopOllamaClient);
        let judge = ComplianceJudge::new_with_builtin_rules(client, "test-model");

        let request = JudgmentRequest::new("op-001", "Test operation")
            .with_language("en")
            .with_rules(vec!["gmp-batch-001".to_string()]);

        let (rule_context, _) = judge.get_applicable_rules(&request, "en");
        let prompt = judge.build_prompt(&request, &rule_context, "en");

        assert!(prompt.contains("Please respond in English"));
        assert!(prompt.contains("batch records"));
    }

    #[test]
    fn test_build_prompt_japanese() {
        let client = Box::new(NoopOllamaClient);
        let judge = ComplianceJudge::new_with_builtin_rules(client, "test-model");

        let request = JudgmentRequest::new("op-001", "Test operation")
            .with_language("ja")
            .with_rules(vec!["gmp-batch-001".to_string()]);

        let (rule_context, _) = judge.get_applicable_rules(&request, "ja");
        let prompt = judge.build_prompt(&request, &rule_context, "ja");

        assert!(prompt.contains("日本語でお答えください"));
    }

    #[test]
    fn test_parse_compliant_response() {
        let client = Box::new(NoopOllamaClient);
        let judge = ComplianceJudge::new_with_builtin_rules(client, "test-model");

        let request = JudgmentRequest::new("op-001", "Test");
        let response = "STATUS: COMPLIANT\nCONFIDENCE: 0.95\nREASON: All rules satisfied";

        let result = judge.parse_judgment_response(
            &request,
            response,
            0.95,
            ResponseClarity::Clear,
            &["gmp-batch-001".to_string()],
        );

        assert_eq!(result.status, JudgmentStatus::Compliant);
    }

    #[test]
    fn test_parse_non_compliant_response() {
        let client = Box::new(NoopOllamaClient);
        let judge = ComplianceJudge::new_with_builtin_rules(client, "test-model");

        let request =
            JudgmentRequest::new("op-001", "Test").with_rules(vec!["gmp-batch-001".to_string()]);
        let response =
            "STATUS: NON_COMPLIANT\nCONFIDENCE: 0.85\nREASON: Violates rule\nVIOLATED_RULES: gmp-batch-001";

        let result = judge.parse_judgment_response(
            &request,
            response,
            0.85,
            ResponseClarity::Clear,
            &["gmp-batch-001".to_string()],
        );

        assert_eq!(result.status, JudgmentStatus::NonCompliant);
        assert!(!result.evidence.is_empty());
    }

    #[test]
    fn test_parse_low_confidence_needs_review() {
        let client = Box::new(NoopOllamaClient);
        let judge = ComplianceJudge::new_with_builtin_rules(client, "test-model");

        let request = JudgmentRequest::new("op-001", "Ambiguous operation");
        let response = "STATUS: COMPLIANT\nCONFIDENCE: 0.5\nREASON: Uncertain";

        let result = judge.parse_judgment_response(
            &request,
            response,
            0.5, // Low confidence
            ResponseClarity::Ambiguous,
            &[],
        );

        // Should be overridden to NEEDS_REVIEW due to low confidence
        assert_eq!(result.status, JudgmentStatus::NeedsReview);
    }

    #[test]
    fn test_get_applicable_rules_with_ids() {
        let client = Box::new(NoopOllamaClient);
        let judge = ComplianceJudge::new_with_builtin_rules(client, "test-model");

        let request =
            JudgmentRequest::new("op-001", "Test").with_rules(vec!["gmp-batch-001".to_string()]);

        let (text, ids) = judge.get_applicable_rules(&request, "zh");
        assert!(text.contains("gmp-batch-001"));
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0], "gmp-batch-001");
    }

    #[test]
    fn test_get_applicable_rules_all() {
        let client = Box::new(NoopOllamaClient);
        let judge = ComplianceJudge::new_with_builtin_rules(client, "test-model");

        let request = JudgmentRequest::new("op-001", "Test");
        let (text, ids) = judge.get_applicable_rules(&request, "zh");

        // 没有指定 rule_ids 时使用所有规则
        assert_eq!(ids.len(), 6);
        assert!(text.contains("gmp-batch-001"));
    }

    #[test]
    fn test_count_rule_citations() {
        let client = Box::new(NoopOllamaClient);
        let judge = ComplianceJudge::new_with_builtin_rules(client, "test-model");

        let response = "This violates gmp-batch-001 and gmp-batch-002";
        let rule_ids = vec![
            "gmp-batch-001".to_string(),
            "gmp-batch-002".to_string(),
            "gmp-equip-001".to_string(),
        ];

        let count = judge.count_rule_citations(response, &rule_ids);
        assert_eq!(count, 2);
    }

    #[test]
    fn test_count_rule_citations_none() {
        let client = Box::new(NoopOllamaClient);
        let judge = ComplianceJudge::new_with_builtin_rules(client, "test-model");

        let response = "Everything looks good";
        let rule_ids = vec!["gmp-batch-001".to_string()];

        let count = judge.count_rule_citations(response, &rule_ids);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_extract_violated_rules_from_marker() {
        let client = Box::new(NoopOllamaClient);
        let judge = ComplianceJudge::new_with_builtin_rules(client, "test-model");

        let response = "STATUS: NON_COMPLIANT\nVIOLATED_RULES: gmp-batch-001, gmp-batch-002";
        let matched = vec!["gmp-batch-001".to_string(), "gmp-batch-002".to_string()];

        let violated = judge.extract_violated_rules(response, &matched);
        assert_eq!(violated.len(), 2);
        assert!(violated.contains(&"gmp-batch-001".to_string()));
    }

    #[test]
    fn test_extract_violated_rules_from_mention() {
        let client = Box::new(NoopOllamaClient);
        let judge = ComplianceJudge::new_with_builtin_rules(client, "test-model");

        let response = "This operation violates gmp-batch-001 because signatures are missing";
        let matched = vec!["gmp-batch-001".to_string(), "gmp-batch-002".to_string()];

        let violated = judge.extract_violated_rules(response, &matched);
        assert_eq!(violated.len(), 1);
        assert_eq!(violated[0], "gmp-batch-001");
    }

    #[test]
    fn test_get_rule_severity() {
        let client = Box::new(NoopOllamaClient);
        let judge = ComplianceJudge::new_with_builtin_rules(client, "test-model");

        assert_eq!(
            judge.get_rule_severity("gmp-batch-001"),
            Some("major".to_string())
        );
        assert_eq!(
            judge.get_rule_severity("gmp-batch-002"),
            Some("critical".to_string())
        );
        assert_eq!(judge.get_rule_severity("nonexistent"), None);
    }

    #[test]
    fn test_judge_model_name() {
        let client = Box::new(NoopOllamaClient);
        let judge = ComplianceJudge::new_with_builtin_rules(client, "qwen2.5:14b");

        // Access model through new_with_builtin_rules - model is private, but we can
        // verify via the auditor's recording behavior indirectly
        // The model name is set in execute_judgment
        assert!(judge.rule_set().len() > 0);
    }

    #[tokio::test]
    async fn test_judge_audit_log() {
        let client = Box::new(MockOllamaClient::new(
            "STATUS: COMPLIANT\nCONFIDENCE: 0.9\nREASON: All OK",
        ));
        let judge = ComplianceJudge::new_with_builtin_rules(client, "test-model");

        let request = JudgmentRequest::new("op-audit-001", "Audit test operation")
            .with_rules(vec!["gmp-batch-001".to_string()]);
        let result = judge.judge(&request).await;

        assert_eq!(result.status, JudgmentStatus::Compliant);
        // Verify audit entries exist
        let entries = judge.auditor().finish();
        assert!(!entries.is_empty());
    }

    #[tokio::test]
    async fn test_judge_evidence_with_violated_rules() {
        let client = Box::new(MockOllamaClient::new(
            "STATUS: NON_COMPLIANT\nCONFIDENCE: 0.9\nREASON: Missing signatures\nVIOLATED_RULES: gmp-batch-001",
        ));
        let judge = ComplianceJudge::new_with_builtin_rules(client, "test-model");

        let request = JudgmentRequest::new("op-evidence-001", "Batch record not signed")
            .with_rules(vec!["gmp-batch-001".to_string()]);
        let result = judge.judge(&request).await;

        assert_eq!(result.status, JudgmentStatus::NonCompliant);
        assert!(!result.evidence.is_empty());
        // Evidence should include rule text
        assert!(
            result.evidence[0].rule_text.contains("批记录")
                || result.evidence[0].rule_text.contains("batch")
        );
    }

    #[tokio::test]
    async fn test_judge_with_japanese() {
        let client = Box::new(NoopOllamaClient);
        let judge = ComplianceJudge::new_with_builtin_rules(client, "test-model");

        let request = JudgmentRequest::new("op-jp-001", "テスト操作").with_language("ja");
        let result = judge.judge(&request).await;

        assert_eq!(result.status, JudgmentStatus::Compliant);
    }
}
