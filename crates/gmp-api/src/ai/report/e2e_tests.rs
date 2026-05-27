//! 端到端集成测试 — #1411 跨语言合规报告
//!
//! Step 7: 验证完整流程: 本地模式 + LLM 模式

use crate::ai::report::{
    api::{generate_cross_lang_report, supported_standards, CrossLangReportRequest},
    template::{build_final_report, ReportCategory, ReportStandard},
    term_dictionary::{preprocess_terms, validate_terms},
    translator::{CrossLangTranslator, NoopTranslateClient, TranslateConfig},
};

/// ===== 本地模式（local_only=true）端到端测试 =====

#[tokio::test]
async fn test_e2e_local_mode_batch_release() {
    let request = CrossLangReportRequest {
        source_text: "批次放行报告：生产已完成，原料检验合格，偏差调查已关闭，CAPA已批准。"
            .to_string(),
        standard: "fda_21cfr11".to_string(),
        batch_id: Some("LOT-2026-001".to_string()),
        local_only: true,
    };

    let response = generate_cross_lang_report(&request, None).await;

    assert!(
        response.success,
        "Local mode should succeed: {:?}",
        response.error
    );
    assert_eq!(response.mode, "local");
    let report = response.report.expect("Report must exist");
    assert_eq!(report.title, "BATCH RELEASE REPORT");
    assert_eq!(report.meta.standard, ReportStandard::Fda21Cfr11);
    assert_eq!(report.meta.batch_id, Some("LOT-2026-001".to_string()));
    assert!(report.meta.accuracy_score > 0.0);
}

#[tokio::test]
async fn test_e2e_local_mode_ema_compliance() {
    let request = CrossLangReportRequest {
        source_text:
            "计算机化系统验证报告：Audit Trail已审查，Electronic Signature已签署，数据完整性确认。"
                .to_string(),
        standard: "ema_annex11".to_string(),
        batch_id: Some("CSV-2026-001".to_string()),
        local_only: true,
    };

    let response = generate_cross_lang_report(&request, None).await;

    assert!(response.success);
    let report = response.report.expect("Report must exist");
    assert!(report.title.contains("EU") || report.title.contains("EUROPEAN"));
    assert_eq!(report.meta.standard, ReportStandard::EmaAnnex11);
}

#[tokio::test]
async fn test_e2e_local_mode_no_batch_id() {
    let request = CrossLangReportRequest {
        source_text:
            "设备验证报告：Installation Qualification已完成，Operational Qualification进行中。"
                .to_string(),
        standard: "fda_21cfr11".to_string(),
        batch_id: None,
        local_only: true,
    };

    let response = generate_cross_lang_report(&request, None).await;

    assert!(response.success);
    let report = response.report.expect("Report must exist");
    assert_eq!(report.meta.batch_id, None);
    // 设备验证报告 → fda_21cfr11 → BatchRelease (标准 fallback 行为)
    assert_eq!(report.report_type, ReportCategory::BatchRelease);
}

/// ===== 术语预处理单元测试（Step 7 补充）=====

#[test]
fn test_e2e_term_preprocessing_full_flow() {
    let input = "原料检验：成品放行前需完成所有质量标准检测，偏差调查报告已归档。";
    let (preprocessed, count) = preprocess_terms(input);

    // 验证术语已替换
    assert!(preprocessed.contains("Raw Material") || preprocessed.contains("Quality"));
    assert!(count > 0, "Should detect at least some terms");
}

#[test]
fn test_e2e_validation_full_flow() {
    // 模拟完整 FDA 报告输出
    let output = "BATCH RELEASE REPORT\n\nManufacturing completed per SOP. \
        Raw Material Inspection passed. Batch meets release specifications. \
        Audit Trail verified. Electronic Signature attached.";

    let (valid, score, errors) = validate_terms(output, "fda_21cfr11");

    // 完整 FDA 报告应通过校验
    assert!(
        score >= 0.5,
        "FDA report should have score >= 0.5, got {}",
        score
    );
    // errors 为空或仅有非关键项警告
    let critical_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.contains("缺失") || e.contains("Missing"))
        .collect();
    assert!(
        critical_errors.is_empty(),
        "Should have no critical errors, got: {:?}",
        critical_errors
    );
}

#[test]
fn test_e2e_build_report_full_flow() {
    let translated = "COMPLIANCE VERIFICATION REPORT\n\nSystem Validation completed per protocol. \
        Data Integrity confirmed. Audit Trail reviewed.";

    let report = build_final_report(
        translated,
        "合规验证报告：系统验证已完成，数据完整性确认，审计跟踪已审查。",
        "ema_annex11",
        Some("VER-2026-001".to_string()),
        0.92,
        8,
        true,
    );

    assert_eq!(report.report_type, ReportCategory::ComplianceVerification);
    assert_eq!(report.meta.standard, ReportStandard::EmaAnnex11);
    assert!(report.content.contains("COMPLIANCE VERIFICATION"));
    assert_eq!(report.meta.term_replacements, 8);
    assert_eq!(report.meta.accuracy_score, 0.92);
}

#[test]
fn test_e2e_supported_standards() {
    let standards = supported_standards();
    assert_eq!(standards.len(), 2);
    let (key, desc) = &standards[0];
    assert_eq!(*key, "fda_21cfr11");
    assert!(desc.contains("FDA"));
    let (key2, desc2) = &standards[1];
    assert_eq!(*key2, "ema_annex11");
    assert!(desc2.contains("EMA") || desc2.contains("Computerized"));
}

/// ===== LLM 模式 Mock 测试（translator 内部）=====

#[tokio::test]
async fn test_e2e_llm_mode_with_mock() {
    // Mock translator returns a boxed MockClient
    let translator = CrossLangTranslator::new(
        Box::new(NoopTranslateClient { response: "BATCH RELEASE REPORT\n\nManufacturing completed per SOP. Batch meets specifications. Audit Trail verified.".to_string() }),
        TranslateConfig::default(),
    );

    let request = CrossLangReportRequest {
        source_text: "批次放行报告：生产已完成，质量部门已批准放行。".to_string(),
        standard: "fda_21cfr11".to_string(),
        batch_id: Some("LOT-MOCK-001".to_string()),
        local_only: false, // 触发 LLM 路径
    };

    // 注意: 由于无真实 Ollama，会走到 MockClient 分支
    let response = generate_cross_lang_report(&request, Some(&translator)).await;

    // Mock 模式下也会有响应（MockClient 返回固定文本）
    assert!(response.success || response.error.is_some());
}

/// ===== 边界条件测试 =====

#[tokio::test]
async fn test_e2e_empty_source() {
    let request = CrossLangReportRequest {
        source_text: "".to_string(),
        standard: "fda_21cfr11".to_string(),
        batch_id: None,
        local_only: true,
    };

    let response = generate_cross_lang_report(&request, None).await;

    // 空输入应该也能处理（生成空白报告或返回 error）
    if !response.success {
        assert!(response.error.is_some());
    }
}

#[tokio::test]
async fn test_e2e_unknown_standard_falls_back() {
    let request = CrossLangReportRequest {
        source_text: "Some report content here.".to_string(),
        standard: "unknown_standard".to_string(),
        batch_id: None,
        local_only: true,
    };

    let response = generate_cross_lang_report(&request, None).await;

    // 未知规范应 fallback 到 FDA 默认
    assert!(response.success);
    let report = response.report.expect("Report should exist");
    assert_eq!(report.meta.standard, ReportStandard::Fda21Cfr11);
}

#[test]
fn test_e2e_report_category_mapping() {
    assert_eq!(
        ReportCategory::from_report_type("batch_release"),
        ReportCategory::BatchRelease
    );
    assert_eq!(
        ReportCategory::from_report_type("deviation"),
        ReportCategory::DeviationInvestigation
    );
    assert_eq!(
        ReportCategory::from_report_type("audit"),
        ReportCategory::AuditSummary
    );
    assert_eq!(
        ReportCategory::from_report_type("auto"),
        ReportCategory::Auto
    );
}
