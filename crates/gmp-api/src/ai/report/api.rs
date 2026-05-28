//! API 端点 — 跨语言合规报告对外接口
//!
//! Step 6: 对外 API，封装完整翻译流程
//! HTTP 端点集成由 server crate 负责（routes.rs）

use serde::{Deserialize, Serialize};

// Re-export internal types for external use
pub use super::template::{
    build_final_report, FinalReport, ReportCategory, ReportMeta, ReportStandard,
};
pub use super::translator::{
    CrossLangTranslator, TranslateConfig, TranslateRequest, TranslateResult,
};

/// 跨语言报告生成请求（对外 API）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossLangReportRequest {
    /// 中文输入文本
    pub source_text: String,
    /// 报告规范: "fda_21cfr11" | "ema_annex11"
    pub standard: String,
    /// 批号 (可选)
    pub batch_id: Option<String>,
    /// 是否仅本地处理（跳过 LLM）
    pub local_only: bool,
}

impl Default for CrossLangReportRequest {
    fn default() -> Self {
        Self {
            source_text: String::new(),
            standard: "fda_21cfr11".to_string(),
            batch_id: None,
            local_only: true, // 默认本地模式（无 LLM）
        }
    }
}

/// 跨语言报告生成响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossLangReportResponse {
    /// 是否成功
    pub success: bool,
    /// 最终报告（JSON 序列化）
    pub report: Option<FinalReport>,
    /// 错误信息
    pub error: Option<String>,
    /// 处理模式
    pub mode: String,
}

impl CrossLangReportResponse {
    /// 成功响应
    pub fn ok(report: FinalReport, mode: &str) -> Self {
        Self {
            success: true,
            report: Some(report),
            error: None,
            mode: mode.to_string(),
        }
    }

    /// 失败响应
    pub fn fail(error: impl Into<String>) -> Self {
        Self {
            success: false,
            report: None,
            error: Some(error.into()),
            mode: "error".to_string(),
        }
    }
}

/// 生成跨语言合规报告（端到端）
///
/// 这是 #1411 的主入口函数，封装完整流程:
/// 1. 预处理: 术语本地替换
/// 2. 翻译: LLM 翻译（如有客户端）
/// 3. 后处理: 清理格式
/// 4. 校验: 术语验证
/// 5. 组装: FinalReport
///
/// # Arguments
/// * `request` - 跨语言报告请求
/// * `translator` - 翻译器（可为 None 则仅本地处理）
///
/// # Returns
/// 跨语言报告响应
pub async fn generate_cross_lang_report(
    request: &CrossLangReportRequest,
    translator: Option<&CrossLangTranslator>,
) -> CrossLangReportResponse {
    use super::term_dictionary::{preprocess_terms, validate_terms};

    // Step 1: 术语预处理
    let (preprocessed, term_count) = preprocess_terms(&request.source_text);

    // Step 2: 翻译
    let (translated, accuracy_score, validation_passed, _errors) = if request.local_only {
        // 本地模式：仅术语替换，无 LLM
        let cleaned = preprocessed.trim().to_string();
        let (_, accuracy, errs_vec) = validate_terms(&cleaned, &request.standard);
        (cleaned, accuracy, errs_vec.is_empty(), errs_vec)
    } else if let Some(t) = translator {
        // LLM 模式：完整翻译
        let result = t
            .translate(&TranslateRequest {
                source_text: request.source_text.clone(),
                report_type: request.standard.clone(),
                batch_id: request.batch_id.clone(),
            })
            .await;
        (
            result.translated_text,
            result.accuracy_score,
            result.validated,
            result.validation_errors,
        )
    } else {
        return CrossLangReportResponse::fail("No translator available and local_only=false");
    };

    // Step 3: 组装 FinalReport
    let report = build_final_report(
        &translated,
        &request.source_text,
        &request.standard,
        request.batch_id.clone(),
        accuracy_score,
        term_count,
        validation_passed,
    );

    let mode = if request.local_only { "local" } else { "llm" };

    CrossLangReportResponse::ok(report, mode)
}

/// 获取支持的报告规范列表
pub fn supported_standards() -> Vec<(&'static str, &'static str)> {
    vec![
        ("fda_21cfr11", "FDA 21 CFR Part 11 — Electronic Records"),
        ("ema_annex11", "EMA Annex 11 — Computerized Systems"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_local_only_report() {
        let request = CrossLangReportRequest {
            source_text: "批次放行报告：偏差调查报告已完成，CAPA 已批准，原料检验合格。"
                .to_string(),
            standard: "fda_21cfr11".to_string(),
            batch_id: Some("LOT-001".to_string()),
            local_only: true,
        };

        let response = generate_cross_lang_report(&request, None).await;

        assert!(
            response.success,
            "Response should be success: {:?}",
            response.error
        );
        let report = response.report.expect("Report should exist");
        assert_eq!(report.title, "BATCH RELEASE REPORT");
        assert_eq!(report.meta.standard, ReportStandard::Fda21Cfr11);
        assert_eq!(response.mode, "local");
    }

    #[test]
    fn test_supported_standards() {
        let standards = supported_standards();
        assert_eq!(standards.len(), 2);
        assert_eq!(standards[0].0, "fda_21cfr11");
        assert_eq!(standards[1].0, "ema_annex11");
    }

    #[test]
    fn test_cross_lang_report_request_default() {
        let req = CrossLangReportRequest::default();
        assert!(req.local_only);
        assert_eq!(req.standard, "fda_21cfr11");
        assert!(req.batch_id.is_none());
    }
}
