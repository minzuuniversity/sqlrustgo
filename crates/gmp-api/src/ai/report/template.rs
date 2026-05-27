//! GMP 报告模板 — 跨语言合规报告最终输出
//!
//! Step 5: 将翻译结果转换为符合 FDA/EMA 规范的最终报告

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 最终输出报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalReport {
    /// 报告 ID
    pub id: String,
    /// 报告类型
    pub report_type: ReportCategory,
    /// 英文标题
    pub title: String,
    /// 英文正文
    pub content: String,
    /// 源中文文本
    pub source_text: String,
    /// 翻译元数据
    pub meta: ReportMeta,
}

/// 报告分类
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReportCategory {
    /// 批次放行报告
    BatchRelease,
    /// 合规验证报告
    ComplianceVerification,
    /// 偏差调查报告
    DeviationInvestigation,
    /// 设备验证报告
    EquipmentValidation,
    /// 审计摘要
    AuditSummary,
    /// 自动检测
    Auto,
}

impl ReportCategory {
    pub fn from_report_type(report_type: &str) -> Self {
        match report_type.to_lowercase().as_str() {
            "fda_21cfr11" | "batch_release" => ReportCategory::BatchRelease,
            "ema_annex11" | "compliance_verification" => ReportCategory::ComplianceVerification,
            "deviation_investigation" | "deviation" => ReportCategory::DeviationInvestigation,
            "equipment_validation" | "equipment" => ReportCategory::EquipmentValidation,
            "audit_summary" | "audit" => ReportCategory::AuditSummary,
            _ => ReportCategory::Auto,
        }
    }

    /// FDA 21 CFR Part 11 固定标题
    pub fn fda_title(&self) -> &'static str {
        match self {
            ReportCategory::BatchRelease => "BATCH RELEASE REPORT",
            ReportCategory::ComplianceVerification => "COMPLIANCE VERIFICATION REPORT",
            ReportCategory::DeviationInvestigation => "DEVIATION INVESTIGATION REPORT",
            ReportCategory::EquipmentValidation => "EQUIPMENT VALIDATION REPORT",
            ReportCategory::AuditSummary => "AUDIT SUMMARY REPORT",
            ReportCategory::Auto => "GMP COMPLIANCE REPORT",
        }
    }

    /// EMA Annex 11 固定标题
    pub fn ema_title(&self) -> &'static str {
        match self {
            ReportCategory::BatchRelease => "BATCH RELEASE — EUROPEAN UNION",
            ReportCategory::ComplianceVerification => "COMPLIANCE VERIFICATION — EU ANNEX 11",
            ReportCategory::DeviationInvestigation => "DEVIATION INVESTIGATION — EU GMP",
            ReportCategory::EquipmentValidation => "COMPUTERIZED SYSTEM QUALIFICATION",
            ReportCategory::AuditSummary => "AUDIT SUMMARY — EU GMP GUIDELINES",
            ReportCategory::Auto => "GMP COMPLIANCE REPORT — EUROPEAN UNION",
        }
    }
}

/// 报告元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportMeta {
    /// 翻译准确率 (0.0-1.0)
    pub accuracy_score: f64,
    /// 术语替换数量
    pub term_replacements: usize,
    /// 校验是否通过
    pub validation_passed: bool,
    /// 报告规范
    pub standard: ReportStandard,
    /// 批号 (可选)
    pub batch_id: Option<String>,
    /// 生成时间
    pub generated_at: DateTime<Utc>,
}

/// 报告规范
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReportStandard {
    Fda21Cfr11,
    EmaAnnex11,
}

impl ReportStandard {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "fda_21cfr11" => ReportStandard::Fda21Cfr11,
            "ema_annex11" => ReportStandard::EmaAnnex11,
            _ => ReportStandard::Fda21Cfr11,
        }
    }
}

/// 将翻译结果转换为最终报告
pub fn build_final_report(
    translated: &str,
    source: &str,
    report_type: &str,
    batch_id: Option<String>,
    accuracy_score: f64,
    term_replacements: usize,
    validation_passed: bool,
) -> FinalReport {
    let category = ReportCategory::from_report_type(report_type);
    let standard = ReportStandard::from_str(report_type);
    let title = match standard {
        ReportStandard::Fda21Cfr11 => category.fda_title(),
        ReportStandard::EmaAnnex11 => category.ema_title(),
    };

    let meta = ReportMeta {
        accuracy_score,
        term_replacements,
        validation_passed,
        standard,
        batch_id,
        generated_at: Utc::now(),
    };

    FinalReport {
        id: uuid::Uuid::new_v4().to_string(),
        report_type: category,
        title: title.to_string(),
        content: translated.to_string(),
        source_text: source.to_string(),
        meta,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_report_category_fda_titles() {
        assert_eq!(
            ReportCategory::BatchRelease.fda_title(),
            "BATCH RELEASE REPORT"
        );
        assert_eq!(
            ReportCategory::ComplianceVerification.ema_title(),
            "COMPLIANCE VERIFICATION — EU ANNEX 11"
        );
    }

    #[test]
    fn test_build_final_report() {
        let report = build_final_report(
            "BATCH RELEASE REPORT\n\nManufacturing completed per SOP.",
            "批次放行报告：生产已完成。",
            "fda_21cfr11",
            Some("LOT-001".to_string()),
            0.95,
            5,
            true,
        );

        assert_eq!(report.title, "BATCH RELEASE REPORT");
        assert_eq!(report.report_type, ReportCategory::BatchRelease);
        assert_eq!(report.meta.standard, ReportStandard::Fda21Cfr11);
        assert!(report.meta.validation_passed);
        assert_eq!(report.meta.batch_id, Some("LOT-001".to_string()));
    }

    #[test]
    fn test_report_standard_from_str() {
        assert_eq!(
            ReportStandard::from_str("fda_21cfr11"),
            ReportStandard::Fda21Cfr11
        );
        assert_eq!(
            ReportStandard::from_str("EMA_ANNEX11"),
            ReportStandard::EmaAnnex11
        );
        assert_eq!(
            ReportStandard::from_str("unknown"),
            ReportStandard::Fda21Cfr11
        );
    }
}
