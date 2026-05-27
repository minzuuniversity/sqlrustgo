//! Report types for Report Generator Agent

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Report type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ReportType {
    BatchSummary,
    Compliance,
    Equipment,
    AuditSummary,
    #[default]
    Auto,
}

impl std::str::FromStr for ReportType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "批次汇总" | "batch_summary" | "BatchSummary" => Ok(Self::BatchSummary),
            "合规报告" | "compliance" | "Compliance" => Ok(Self::Compliance),
            "设备报告" | "equipment" | "Equipment" => Ok(Self::Equipment),
            "审计摘要" | "audit_summary" | "AuditSummary" => Ok(Self::AuditSummary),
            _ => Ok(Self::Auto),
        }
    }
}

impl ReportType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::BatchSummary => "批次汇总",
            Self::Compliance => "合规报告",
            Self::Equipment => "设备报告",
            Self::AuditSummary => "审计摘要",
            Self::Auto => "自动",
        }
    }
}

/// Output format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ReportFormat {
    #[default]
    Markdown,
    Json,
    Pdf,
}

/// Time range for report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl TimeRange {
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        Self { start, end }
    }

    /// Parse from "YYYY-MM-DD" strings
    pub fn from_ymd(start: &str, end: &str) -> Result<Self, chrono::ParseError> {
        use chrono::NaiveDate;
        let start_date = NaiveDate::parse_from_str(start, "%Y-%m-%d")?;
        let end_date = NaiveDate::parse_from_str(end, "%Y-%m-%d")?;
        let start = start_date.and_hms_opt(0, 0, 0).unwrap();
        let end = end_date.and_hms_opt(23, 59, 59).unwrap();
        Ok(Self {
            start: DateTime::from_naive_utc_and_offset(start, Utc),
            end: DateTime::from_naive_utc_and_offset(end, Utc),
        })
    }
}

/// Query filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryFilter {
    pub field: String,
    pub operator: String,
    pub value: String,
}

/// Parsed intent from natural language
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedIntent {
    pub report_type: ReportType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_range: Option<TimeRange>,
    #[serde(default)]
    pub filters: Vec<QueryFilter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grouping: Option<String>,
    #[serde(default)]
    pub metrics: Vec<String>,
}

impl Default for ParsedIntent {
    fn default() -> Self {
        Self {
            report_type: ReportType::Auto,
            time_range: None,
            filters: Vec::new(),
            grouping: None,
            metrics: Vec::new(),
        }
    }
}

/// Report generation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportRequest {
    pub query: String,
    pub report_type: ReportType,
    pub format: ReportFormat,
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_range: Option<TimeRange>,
}

impl ReportRequest {
    pub fn new(query: &str) -> Self {
        Self {
            query: query.to_string(),
            report_type: ReportType::Auto,
            format: ReportFormat::default(),
            language: None,
            time_range: None,
        }
    }

    pub fn with_type(mut self, report_type: ReportType) -> Self {
        self.report_type = report_type;
        self
    }

    pub fn with_format(mut self, format: ReportFormat) -> Self {
        self.format = format;
        self
    }

    pub fn with_language(mut self, lang: &str) -> Self {
        self.language = Some(lang.to_string());
        self
    }

    pub fn with_time_range(mut self, time_range: TimeRange) -> Self {
        self.time_range = Some(time_range);
        self
    }
}

/// Metadata about the generated report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportMetadata {
    pub query: String,
    pub report_type: ReportType,
    pub format: ReportFormat,
    pub record_count: usize,
    pub confidence: f32,
    pub generation_time_ms: u64,
}

/// Report generation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportResult {
    pub report_id: String,
    pub title: String,
    pub content: String,
    pub data: Option<serde_json::Value>,
    pub metadata: ReportMetadata,
    pub generated_at: DateTime<Utc>,
}

impl ReportResult {
    pub fn new(
        report_id: String,
        title: String,
        content: String,
        metadata: ReportMetadata,
    ) -> Self {
        Self {
            report_id,
            title,
            content,
            data: None,
            metadata,
            generated_at: Utc::now(),
        }
    }

    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }
}

/// Report data (internal structure for formatting)
#[derive(Debug, Clone)]
pub struct ReportData<'a> {
    pub report_type: ReportType,
    pub time_range: Option<&'a TimeRange>,
    pub records: Vec<Record>,
    pub metrics: Metrics,
}

#[derive(Debug, Clone)]
pub struct Record {
    pub id: String,
    pub fields: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default)]
pub struct Metrics {
    pub total: usize,
    pub pass_rate: Option<f32>,
    pub fail_rate: Option<f32>,
    pub compliance_rate: Option<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_report_type_default() {
        assert_eq!(ReportType::default(), ReportType::Auto);
    }

    #[test]
    fn test_report_type_from_str() {
        assert_eq!("批次汇总".parse::<ReportType>().unwrap(), ReportType::BatchSummary);
        assert_eq!("compliance".parse::<ReportType>().unwrap(), ReportType::Compliance);
        assert_eq!("unknown".parse::<ReportType>().unwrap(), ReportType::Auto);
    }

    #[test]
    fn test_report_request_builder() {
        let req = ReportRequest::new("本周批次通过率")
            .with_type(ReportType::BatchSummary)
            .with_format(ReportFormat::Json)
            .with_language("zh");

        assert_eq!(req.query, "本周批次通过率");
        assert_eq!(req.report_type, ReportType::BatchSummary);
        assert_eq!(req.format, ReportFormat::Json);
        assert_eq!(req.language, Some("zh".to_string()));
    }

    #[test]
    fn test_time_range_parsing() {
        let range = TimeRange::from_ymd("2026-05-01", "2026-05-31").unwrap();
        assert!(range.start < range.end);
    }
}
