//! Intent parsing using LLM

use super::types::{ParsedIntent, ReportType, TimeRange};
use crate::judge::agent::OllamaClient;
use chrono::{DateTime, Utc};
use serde::Deserialize;

/// Intent parser errors
#[derive(Debug, thiserror::Error)]
pub enum IntentError {
    #[error("LLM error: {0}")]
    Llm(String),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
}

/// LLM-powered intent parser
pub struct IntentParser {
    ollama_client: Box<dyn OllamaClient>,
    model: String,
}

impl IntentParser {
    pub fn new(ollama_client: Box<dyn OllamaClient>, model: &str) -> Self {
        Self {
            ollama_client,
            model: model.to_string(),
        }
    }

    /// Parse natural language query into structured intent
    pub async fn parse(&self, query: &str) -> Result<ParsedIntent, IntentError> {
        let prompt = Self::build_prompt(query);

        let response = self
            .ollama_client
            .chat(&prompt, &self.model)
            .await
            .map_err(|e| IntentError::Llm(e.to_string()))?;

        self.parse_response(&response.content)
    }

    fn build_prompt(query: &str) -> String {
        let template = r#"你是一个 GMP 报表助手。请从用户查询中提取报表参数。

用户查询：{QUERY}

请严格输出以下 JSON 格式，不要包含任何其他内容：
{{
  "report_type": "批次汇总|合规报告|设备报告|审计摘要|自动",
  "time_range": {{"start": "YYYY-MM-DD", "end": "YYYY-MM-DD"}} | null,
  "filters": [{{"field": "字段名", "operator": "eq|gt|lt", "value": "值"}}],
  "grouping": "day|week|month|null",
  "metrics": ["指标1", "指标2"]
}}

注意：
- report_type 必须从 ["批次汇总", "合规报告", "设备报告", "审计摘要", "自动"] 中选择
- 如果无法确定时间范围，设为 null
- metrics 可以为空数组 []"#;

        template.replace("{QUERY}", query)
    }

    fn parse_response(&self, response: &str) -> Result<ParsedIntent, IntentError> {
        let json_str = Self::extract_json(response)?;

        #[derive(Deserialize)]
        struct RawIntent {
            report_type: String,
            time_range: Option<RawTimeRange>,
            filters: Vec<RawFilter>,
            grouping: Option<String>,
            metrics: Vec<String>,
        }

        #[derive(Deserialize)]
        struct RawTimeRange {
            start: String,
            end: String,
        }

        #[derive(Deserialize)]
        struct RawFilter {
            field: String,
            operator: String,
            value: String,
        }

        let raw: RawIntent = serde_json::from_str(json_str)?;

        let report_type = raw.report_type.parse::<ReportType>().unwrap_or_default();

        let time_range = raw.time_range.map(|tr| {
            let start = DateTime::parse_from_rfc3339(&format!("{}T00:00:00Z", tr.start))
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());
            let end = DateTime::parse_from_rfc3339(&format!("{}T23:59:59Z", tr.end))
                .map(|dt| dt.with_timezone(&Utc))
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

        Ok(ParsedIntent {
            report_type,
            time_range,
            filters,
            grouping: raw.grouping.filter(|g| g != "null"),
            metrics: raw.metrics,
        })
    }

    fn extract_json(response: &str) -> Result<&str, IntentError> {
        let trimmed = response.trim();
        if trimmed.starts_with('{') {
            let mut depth = 0;
            for (i, c) in trimmed.char_indices() {
                match c {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            return Ok(&trimmed[..=i]);
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(trimmed)
    }
}

/// No-op intent parser for testing (heuristic-based)
pub struct NoopIntentParser;

impl NoopIntentParser {
    pub fn new() -> Self {
        Self
    }

    pub fn parse_sync(&self, query: &str) -> ParsedIntent {
        let query_lower = query.to_lowercase();
        let report_type = if query_lower.contains("批次")
            || query_lower.contains("通过率")
            || query_lower.contains("合格")
        {
            ReportType::BatchSummary
        } else if query_lower.contains("偏差")
            || query_lower.contains("合规")
            || query_lower.contains("CAPA")
            || query_lower.contains("纠正")
        {
            ReportType::Compliance
        } else if query_lower.contains("设备")
            || query_lower.contains("维护")
            || query_lower.contains("OEE")
        {
            ReportType::Equipment
        } else if query_lower.contains("审计") || query_lower.contains("操作记录") {
            ReportType::AuditSummary
        } else {
            ReportType::Auto
        };

        ParsedIntent {
            report_type,
            time_range: None,
            filters: Vec::new(),
            grouping: None,
            metrics: Vec::new(),
        }
    }
}

impl Default for NoopIntentParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noop_heuristic_batch() {
        let parser = NoopIntentParser::new();
        let intent = parser.parse_sync("本周批次通过率是多少");
        assert_eq!(intent.report_type, ReportType::BatchSummary);
    }

    #[test]
    fn test_noop_heuristic_compliance() {
        let parser = NoopIntentParser::new();
        let intent = parser.parse_sync("上月偏差汇总");
        assert_eq!(intent.report_type, ReportType::Compliance);
    }

    #[test]
    fn test_noop_heuristic_equipment() {
        let parser = NoopIntentParser::new();
        let intent = parser.parse_sync("设备维护状态");
        assert_eq!(intent.report_type, ReportType::Equipment);
    }

    #[test]
    fn test_noop_heuristic_audit() {
        let parser = NoopIntentParser::new();
        let intent = parser.parse_sync("审计链摘要");
        assert_eq!(intent.report_type, ReportType::AuditSummary);
    }
}
