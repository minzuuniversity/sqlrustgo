//! LLM 客户端 — 跨语言合规报告翻译引擎
//!
//! 封装 Ollama/ OpenAI ChatGPT 等 LLM 提供者

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// Re-export from compliance-engine's OllamaClient trait
pub use sqlrustgo_compliance_engine::judge::agent::OllamaClient as TranslateOllamaClient;
pub use sqlrustgo_compliance_engine::judge::agent::OllamaError;
pub use sqlrustgo_compliance_engine::judge::agent::OllamaResponse;

/// 翻译模型配置
#[derive(Debug, Clone)]
pub struct TranslateConfig {
    /// Ollama base URL
    pub ollama_url: String,
    /// 翻译模型名称
    pub model: String,
    /// 温度参数 (0.0-1.0)
    pub temperature: f32,
}

impl Default for TranslateConfig {
    fn default() -> Self {
        Self {
            ollama_url: "http://localhost:11434".to_string(),
            model: "qwen2.5:7b".to_string(),
            temperature: 0.1, // 低温度保证术语一致性
        }
    }
}

/// 翻译请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslateRequest {
    /// 中文输入文本
    pub source_text: String,
    /// 报告类型: "fda_21cfr11" | "ema_annex11"
    pub report_type: String,
    /// 批号 (可选，用于报告中)
    pub batch_id: Option<String>,
}

/// 翻译结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslateResult {
    /// 英文翻译结果
    pub translated_text: String,
    /// 术语替换数量
    pub term_replacements: usize,
    /// 准确率评分
    pub accuracy_score: f64,
    /// 是否通过校验
    pub validated: bool,
    /// 校验错误 (如有)
    pub validation_errors: Vec<String>,
}

/// 跨语言翻译引擎
pub struct CrossLangTranslator {
    llm_client: Box<dyn TranslateOllamaClient>,
    model: String,
    temperature: f32,
}

impl CrossLangTranslator {
    /// 创建翻译器
    pub fn new(llm_client: Box<dyn TranslateOllamaClient>, config: TranslateConfig) -> Self {
        Self {
            llm_client,
            model: config.model,
            temperature: config.temperature,
        }
    }

    /// 执行翻译 (端到端)
    pub async fn translate(&self, request: &TranslateRequest) -> TranslateResult {
        use crate::ai::report::term_dictionary::{
            calculate_accuracy_score, preprocess_terms, validate_terms,
        };

        // Step 1: 本地术语预处理
        let (preprocessed, term_count) = preprocess_terms(&request.source_text);

        // Step 2: LLM 翻译
        let llm_output = self
            .call_llm(&preprocessed, &request.report_type)
            .await
            .unwrap_or_else(|_| preprocessed.clone());

        // Step 3: 后处理 - 清理格式
        let cleaned = self.clean_output(&llm_output);

        // Step 4: 术语校验
        let (valid, accuracy, errors) = validate_terms(&cleaned, &request.report_type);

        // Step 5: 计算准确率
        let score = calculate_accuracy_score(&cleaned, errors.len());

        TranslateResult {
            translated_text: cleaned,
            term_replacements: term_count,
            accuracy_score: score,
            validated: valid,
            validation_errors: errors,
        }
    }

    /// 调用 LLM 翻译
    async fn call_llm(&self, preprocessed: &str, report_type: &str) -> Result<String, OllamaError> {
        let prompt = Self::build_translate_prompt(preprocessed, report_type);
        let response = self.llm_client.chat(&prompt, &self.model).await?;
        Ok(response.content)
    }

    /// 构建翻译 prompt
    fn build_translate_prompt(source: &str, report_type: &str) -> String {
        let style_guide = match report_type {
            "fda_21cfr11" => {
                r#"FDA 21 CFR Part 11 风格指南:
- 使用正式商务英语
- 关键术语: Batch (批次), Deviation (偏差), CAPA (纠正预防措施)
- 电子记录相关术语: Electronic Signature, Audit Trail, Data Integrity
- 固定开头: "BATCH RELEASE REPORT"
- 段落结构: 概述 → 详情 → 结论"#
            }
            "ema_annex11" => {
                r#"EMA Annex 11 计算机化系统验证风格指南:
- 使用正式监管文档英语
- 关键术语: Computerized System (计算机化系统), Validation (验证)
- 数据完整性术语: Data Integrity, Audit Trail, Traceability
- 固定开头: "COMPLIANCE VERIFICATION REPORT"
- 段落结构: 范围 → 系统描述 → 验证结果 → 结论"#
            }
            _ => "Standard GMP compliance report style",
        };

        format!(
            r#"翻译以下中文 GMP 报告为英文。严格遵循风格指南。

=== 风格指南 ===
{}

=== 源文本 ===
{}

=== 要求 ===
1. 只输出翻译后的英文，不要包含任何解释、备注或格式符号
2. GMP 术语已预替换为英文（如 Batch, Deviation）
3. 确保语法正确、适合监管提交
4. 保持专业、正式的监管文档语气

=== 翻译 ==="#,
            style_guide, source
        )
    }

    /// 清理 LLM 输出
    fn clean_output(&self, output: &str) -> String {
        let trimmed = output.trim();

        // 移除可能的 markdown 代码块标记
        let cleaned: String = if trimmed.starts_with("```") {
            trimmed
                .trim_start_matches(|c: char| c == '`' || c.is_whitespace())
                .trim_end_matches('`')
                .trim()
                .to_string()
        } else {
            trimmed.to_string()
        };

        // 移除 "Translation:" 等前缀
        let cleaned = cleaned
            .strip_prefix("Translation:")
            .unwrap_or(&cleaned)
            .trim()
            .to_string();

        cleaned
    }
}

/// 测试用 Noop LLM 客户端
pub struct NoopTranslateClient {
    pub response: String,
}

#[async_trait]
impl TranslateOllamaClient for NoopTranslateClient {
    async fn chat(&self, _prompt: &str, _model: &str) -> Result<OllamaResponse, OllamaError> {
        Ok(OllamaResponse {
            content: self.response.clone(),
            logprob: Some(-0.3),
            model: "noop".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::report::term_dictionary::{preprocess_terms, validate_terms};

    fn make_mock_translator(response: &str) -> CrossLangTranslator {
        let client = Box::new(NoopTranslateClient {
            response: response.to_string(),
        });
        CrossLangTranslator::new(client, TranslateConfig::default())
    }

    #[tokio::test]
    async fn test_translate_fda_report() {
        let translator = make_mock_translator(
            "BATCH RELEASE REPORT\n\nManufacturing completed per SOP. \
            Quality tests passed. Batch meets release specifications. \
            Electronic Signature attached. Audit Trail verified. Data Integrity confirmed.",
        );

        let result = translator
            .translate(&TranslateRequest {
                source_text: "偏差调查报告：原料批次号 ABC123，检验偏差，根因分析已完成"
                    .to_string(),
                report_type: "fda_21cfr11".to_string(),
                batch_id: Some("ABC123".to_string()),
            })
            .await;

        assert!(result.validated, "Errors: {:?}", result.validation_errors);
        assert!(result.term_replacements >= 3);
        assert!(result.accuracy_score >= 0.8);
    }

    #[tokio::test]
    async fn test_preprocess_then_translate() {
        // 模拟完整的翻译流程
        let (preprocessed, count) =
            preprocess_terms("批次放行报告：偏差调查报告已完成，CAPA 已批准，原料检验合格");
        assert!(count >= 3);
        assert!(preprocessed.contains("Batch"));
        assert!(preprocessed.contains("Deviation"));
        assert!(preprocessed.contains("CAPA"));
    }

    #[test]
    fn test_validate_fda_output() {
        let output = "Batch Release Report. Manufacturing completed per SOP. Audit Trail verified. Electronic Signature attached. Data Integrity confirmed.";
        let (valid, score, errors) = validate_terms(output, "fda_21cfr11");
        assert!(errors.is_empty(), "Errors: {:?}", errors);
        assert!(score >= 0.5);
    }
}
