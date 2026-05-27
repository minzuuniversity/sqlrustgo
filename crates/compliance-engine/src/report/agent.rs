//! Report Generator Agent
//!
//! Generates GMP reports from natural language queries.

use super::formatter::{JsonFormatter, MarkdownFormatter, ReportFormatter};
use super::intent::NoopIntentParser;
use super::types::{
    Metrics, ParsedIntent, ReportData, ReportFormat, ReportMetadata, ReportRequest, ReportResult,
    ReportType, TimeRange,
};
use async_trait::async_trait;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::time::timeout;

/// Report generation timeout
const GENERATION_TIMEOUT_MS: u64 = 3000;

/// Ollama client trait for LLM inference
#[async_trait]
pub trait ReportLlmClient: Send + Sync {
    async fn chat(
        &self,
        prompt: &str,
        model: &str,
    ) -> Result<crate::judge::agent::OllamaResponse, crate::judge::agent::OllamaError>;
}

/// Report Generator Agent
pub struct ReportGenerator {
    llm_client: Box<dyn ReportLlmClient>,
    model: String,
    markdown_formatter: MarkdownFormatter,
    json_formatter: JsonFormatter,
    intent_parser: NoopIntentParser,
}

impl ReportGenerator {
    /// Create a new ReportGenerator with the given LLM client
    pub fn new(llm_client: Box<dyn ReportLlmClient>, model: &str) -> Self {
        Self {
            llm_client,
            model: model.to_string(),
            markdown_formatter: MarkdownFormatter::new(),
            json_formatter: JsonFormatter::new(),
            intent_parser: NoopIntentParser::new(),
        }
    }

    /// Generate a report from a natural language request
    pub async fn generate(&self, request: &ReportRequest) -> ReportResult {
        let start = Instant::now();

        let result = timeout(
            Duration::from_millis(GENERATION_TIMEOUT_MS),
            self.generate_internal(request),
        )
        .await;

        let elapsed_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(Ok(mut result)) => {
                result.metadata.generation_time_ms = elapsed_ms;
                result
            }
            Ok(Err(e)) => self.make_error_result(&e.to_string(), elapsed_ms),
            Err(_) => self.make_timeout_result(elapsed_ms),
        }
    }

    async fn generate_internal(
        &self,
        request: &ReportRequest,
    ) -> Result<ReportResult, ReportError> {
        // Step 1: Parse intent
        let intent = self.parse_intent(&request.query).await;

        // Step 2: Determine report type
        let report_type = if request.report_type == ReportType::Auto {
            intent.report_type
        } else {
            request.report_type
        };

        // Step 3: Determine time range
        let time_range = request.time_range.clone().or(intent.time_range.clone());

        // Step 4: Retrieve data (mock for now)
        let data = self.retrieve_data(&intent, &time_range).await;

        // Step 5: Build title
        let title = self.build_title(&request.query, report_type);

        // Step 6: Format output
        let (content, json_data) = self.format_output(
            &data,
            report_type,
            request.format,
            &title,
            time_range.as_ref(),
        );

        let record_count = data.records.len();

        let metadata = ReportMetadata {
            query: request.query.clone(),
            report_type,
            format: request.format,
            record_count,
            confidence: 0.85,
            generation_time_ms: 0,
        };

        Ok(
            ReportResult::new(uuid::Uuid::new_v4().to_string(), title, content, metadata)
                .with_data(json_data.unwrap_or_default()),
        )
    }

    async fn parse_intent(&self, query: &str) -> ParsedIntent {
        let response = self
            .llm_client
            .chat(&Self::build_intent_prompt(query), &self.model)
            .await;

        match response {
            Ok(resp) => self.parse_intent_response(&resp.content).await,
            Err(_) => self.intent_parser.parse_sync(query),
        }
    }

    fn build_intent_prompt(query: &str) -> String {
        format!(
            r#"你是一个 GMP 报表助手。请从用户查询中提取报表参数。

用户查询：{}

请严格输出以下 JSON 格式，不要包含任何其他内容：
{{
  "report_type": "批次汇总|合规报告|设备报告|审计摘要|自动",
  "time_range": {{"start": "YYYY-MM-DD", "end": "YYYY-MM-DD"}} | null,
  "filters": [{{"field": "字段名", "operator": "eq|gt|lt", "value": "值"}}],
  "grouping": "day|week|month|null",
  "metrics": ["指标1", "指标2"]
}}"#,
            query
        )
    }

    async fn parse_intent_response(&self, response: &str) -> ParsedIntent {
        let json_str = Self::extract_json(response);

        #[derive(serde::Deserialize)]
        struct RawIntent {
            report_type: String,
            time_range: Option<RawTr>,
            filters: Vec<RawFilter>,
            grouping: Option<String>,
            metrics: Vec<String>,
        }

        #[derive(serde::Deserialize)]
        struct RawTr {
            start: String,
            end: String,
        }

        #[derive(serde::Deserialize)]
        struct RawFilter {
            field: String,
            operator: String,
            value: String,
        }

        match serde_json::from_str::<RawIntent>(json_str) {
            Ok(raw) => {
                let report_type = raw.report_type.parse::<ReportType>().unwrap_or_default();
                let time_range = raw.time_range.map(|tr| {
                    use chrono::{DateTime, NaiveDateTime, Utc};
                    let start = NaiveDateTime::parse_from_str(&tr.start, "%Y-%m-%d")
                        .map(|nd| DateTime::from_naive_utc_and_offset(nd, Utc))
                        .unwrap_or_else(|_| Utc::now());
                    let end = NaiveDateTime::parse_from_str(&tr.end, "%Y-%m-%d")
                        .map(|nd| DateTime::from_naive_utc_and_offset(nd, Utc))
                        .unwrap_or_else(|_| Utc::now());
                    TimeRange { start, end }
                });
                let filters = raw
                    .filters
                    .into_iter()
                    .map(|f| super::types::QueryFilter {
                        field: f.field,
                        operator: f.operator,
                        value: f.value,
                    })
                    .collect();
                ParsedIntent {
                    report_type,
                    time_range,
                    filters,
                    grouping: raw.grouping.filter(|g| g != "null"),
                    metrics: raw.metrics,
                }
            }
            Err(_) => self.intent_parser.parse_sync(response),
        }
    }

    fn extract_json(response: &str) -> &str {
        let trimmed = response.trim();
        if trimmed.starts_with('{') {
            let mut depth = 0;
            for (i, c) in trimmed.char_indices() {
                match c {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            return &trimmed[..=i];
                        }
                    }
                    _ => {}
                }
            }
        }
        trimmed
    }

    async fn retrieve_data<'a>(
        &self,
        _intent: &ParsedIntent,
        _time_range: &'a Option<TimeRange>,
    ) -> ReportData<'a> {
        // TODO: Integrate with GMPRetrievalV3
        ReportData {
            report_type: _intent.report_type,
            time_range: _time_range.as_ref(),
            records: Vec::new(),
            metrics: Metrics::default(),
        }
    }

    fn build_title(&self, query: &str, report_type: ReportType) -> String {
        if !query.is_empty() {
            format!("报告: {}", query)
        } else {
            report_type.as_str().to_string()
        }
    }

    fn format_output(
        &self,
        data: &ReportData,
        _report_type: ReportType,
        format: ReportFormat,
        title: &str,
        time_range: Option<&TimeRange>,
    ) -> (String, Option<serde_json::Value>) {
        match format {
            ReportFormat::Markdown => {
                let content = self.markdown_formatter.format(data, title, time_range);
                (content, None)
            }
            ReportFormat::Json => {
                let json_str = self.json_formatter.format(data, title, time_range);
                let json_data: serde_json::Value =
                    serde_json::from_str(&json_str).ok().unwrap_or_default();
                (json_str, Some(json_data))
            }
            ReportFormat::Pdf => {
                let content = self.markdown_formatter.format(data, title, time_range);
                (content, None)
            }
        }
    }

    fn make_error_result(&self, error: &str, elapsed_ms: u64) -> ReportResult {
        ReportResult {
            report_id: uuid::Uuid::new_v4().to_string(),
            title: "Error".to_string(),
            content: format!("Report generation failed: {}", error),
            data: None,
            metadata: ReportMetadata {
                query: String::new(),
                report_type: ReportType::Auto,
                format: ReportFormat::Markdown,
                record_count: 0,
                confidence: 0.0,
                generation_time_ms: elapsed_ms,
            },
            generated_at: chrono::Utc::now(),
        }
    }

    fn make_timeout_result(&self, elapsed_ms: u64) -> ReportResult {
        ReportResult {
            report_id: uuid::Uuid::new_v4().to_string(),
            title: "Timeout".to_string(),
            content: "Report generation timed out after 3s".to_string(),
            data: None,
            metadata: ReportMetadata {
                query: String::new(),
                report_type: ReportType::Auto,
                format: ReportFormat::Markdown,
                record_count: 0,
                confidence: 0.0,
                generation_time_ms: elapsed_ms,
            },
            generated_at: chrono::Utc::now(),
        }
    }
}

/// Report generation errors
#[derive(Debug, Error)]
pub enum ReportError {
    #[error("LLM error: {0}")]
    Llm(String),
    #[error("Timeout")]
    Timeout,
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}

// ============== Mock LLM Client for Testing ==============

/// Mock LLM client for testing
pub struct MockReportLlmClient {
    response: String,
}

impl MockReportLlmClient {
    pub fn new(response: &str) -> Self {
        Self {
            response: response.to_string(),
        }
    }
}

#[async_trait]
impl ReportLlmClient for MockReportLlmClient {
    async fn chat(
        &self,
        _prompt: &str,
        _model: &str,
    ) -> Result<crate::judge::agent::OllamaResponse, crate::judge::agent::OllamaError> {
        Ok(crate::judge::agent::OllamaResponse {
            content: self.response.clone(),
            logprob: Some(-0.3),
            model: "mock".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_request() -> ReportRequest {
        ReportRequest::new("本周批次通过率是多少")
            .with_type(ReportType::BatchSummary)
            .with_format(ReportFormat::Markdown)
    }

    fn make_mock_generator() -> ReportGenerator {
        let mock_response = r#"{
            "report_type": "批次汇总",
            "time_range": null,
            "filters": [],
            "grouping": null,
            "metrics": ["通过率", "合格率"]
        }"#;
        let client = Box::new(MockReportLlmClient::new(mock_response));
        ReportGenerator::new(client, "test-model")
    }

    #[tokio::test]
    async fn test_generate_batch_summary() {
        let generator = make_mock_generator();
        let request = make_test_request();

        let result = generator.generate(&request).await;

        assert!(!result.report_id.is_empty());
        assert!(result.content.contains("报告"));
        assert_eq!(result.metadata.report_type, ReportType::BatchSummary);
        assert!(result.metadata.generation_time_ms < GENERATION_TIMEOUT_MS + 500);
    }

    #[tokio::test]
    async fn test_generate_json_format() {
        let generator = make_mock_generator();
        let mut request = make_test_request();
        request.format = ReportFormat::Json;

        let result = generator.generate(&request).await;

        assert!(result.data.is_some());
    }

    #[test]
    fn test_build_title() {
        let client = Box::new(MockReportLlmClient::new("{}"));
        let generator = ReportGenerator::new(client, "test");
        let title = generator.build_title("本周批次通过率", ReportType::BatchSummary);
        assert!(title.contains("本周批次通过率"));
    }

    #[tokio::test]
    async fn test_error_result() {
        let generator = make_mock_generator();
        let error_result = generator.make_error_result("test error", 100);
        assert!(error_result.content.contains("test error"));
    }
}
