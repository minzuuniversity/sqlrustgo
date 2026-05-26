//! Report formatters

use super::types::{ReportData, ReportType, TimeRange};

/// Trait for report formatters
pub trait ReportFormatter {
    fn format(&self, data: &ReportData, title: &str, time_range: Option<&TimeRange>) -> String;
}

/// Markdown formatter
pub struct MarkdownFormatter;

impl MarkdownFormatter {
    pub fn new() -> Self {
        Self
    }

    fn format_batch_summary(
        &self,
        data: &ReportData,
        title: &str,
        time_range: Option<&TimeRange>,
    ) -> String {
        let mut s = String::new();
        s.push_str(&format!("# {}\n\n", title));

        if let Some(tr) = time_range {
            s.push_str(&format!(
                "**时间范围**: {} 至 {}\n\n",
                tr.start.format("%Y-%m-%d"),
                tr.end.format("%Y-%m-%d")
            ));
        }

        // Metrics section
        s.push_str("## 指标汇总\n\n");
        if let Some(rate) = data.metrics.pass_rate {
            s.push_str(&format!("- **通过率**: {:.1}%\n", rate * 100.0));
        }
        if let Some(rate) = data.metrics.fail_rate {
            s.push_str(&format!("- **不合格率**: {:.1}%\n", rate * 100.0));
        }
        s.push_str(&format!("- **总记录数**: {}\n\n", data.metrics.total));

        // Records table
        if !data.records.is_empty() {
            s.push_str("## 详细记录\n\n");
            s.push_str("| ID | ");
            let mut headers: Vec<&str> = Vec::new();
            if let Some(r) = data.records.first() {
                let mut fields: Vec<_> = r.fields.keys().collect();
                fields.sort();
                for field in &fields {
                    headers.push(*field);
                    s.push_str(&format!("{} | ", field));
                }
            }
            s.push_str("\n|");
            for _ in 0..=headers.len() {
                s.push_str("---|");
            }
            s.push('\n');

            for record in &data.records {
                s.push_str(&format!("| {} | ", record.id));
                for header in &headers {
                    s.push_str(&format!(
                        "{} | ",
                        record.fields.get(*header).unwrap_or(&"N/A".to_string())
                    ));
                }
                s.push('\n');
            }
        }

        s
    }

    fn format_compliance_report(
        &self,
        data: &ReportData,
        title: &str,
        time_range: Option<&TimeRange>,
    ) -> String {
        let mut s = String::new();
        s.push_str(&format!("# {}\n\n", title));

        if let Some(tr) = time_range {
            s.push_str(&format!(
                "**时间范围**: {} 至 {}\n\n",
                tr.start.format("%Y-%m-%d"),
                tr.end.format("%Y-%m-%d")
            ));
        }

        if let Some(rate) = data.metrics.compliance_rate {
            s.push_str(&format!("**合规率**: {:.1}%\n\n", rate * 100.0));
        }
        s.push_str(&format!("**总记录数**: {}\n\n", data.metrics.total));

        if !data.records.is_empty() {
            s.push_str("## 详细记录\n\n");
            s.push_str("| ID | 描述 | 严重程度 |\n|---|---|---|\n");
            for record in &data.records {
                let desc = record
                    .fields
                    .get("description")
                    .cloned()
                    .unwrap_or_default();
                let severity = record.fields.get("severity").cloned().unwrap_or_default();
                s.push_str(&format!("| {} | {} | {} |\n", record.id, desc, severity));
            }
        }

        s
    }

    fn format_equipment_report(
        &self,
        data: &ReportData,
        title: &str,
        time_range: Option<&TimeRange>,
    ) -> String {
        let mut s = String::new();
        s.push_str(&format!("# {}\n\n", title));

        if let Some(tr) = time_range {
            s.push_str(&format!(
                "**时间范围**: {} 至 {}\n\n",
                tr.start.format("%Y-%m-%d"),
                tr.end.format("%Y-%m-%d")
            ));
        }

        s.push_str(&format!("**设备总数**: {}\n\n", data.metrics.total));

        if !data.records.is_empty() {
            s.push_str("## 设备状态\n\n");
            s.push_str("| ID | 设备名称 | 状态 |\n|---|---|---|\n");
            for record in &data.records {
                let name = record.fields.get("name").cloned().unwrap_or_default();
                let status = record.fields.get("status").cloned().unwrap_or_default();
                s.push_str(&format!("| {} | {} | {} |\n", record.id, name, status));
            }
        }

        s
    }

    fn format_audit_summary(
        &self,
        data: &ReportData,
        title: &str,
        time_range: Option<&TimeRange>,
    ) -> String {
        let mut s = String::new();
        s.push_str(&format!("# {}\n\n", title));

        if let Some(tr) = time_range {
            s.push_str(&format!(
                "**时间范围**: {} 至 {}\n\n",
                tr.start.format("%Y-%m-%d"),
                tr.end.format("%Y-%m-%d")
            ));
        }

        s.push_str(&format!("**审计记录数**: {}\n\n", data.metrics.total));

        if !data.records.is_empty() {
            s.push_str("## 操作摘要\n\n");
            s.push_str("| 时间 | 操作者 | 操作类型 | 详情 |\n|---|---|---|---|\n");
            for record in &data.records {
                let time = record.fields.get("timestamp").cloned().unwrap_or_default();
                let user = record.fields.get("user").cloned().unwrap_or_default();
                let action = record.fields.get("action").cloned().unwrap_or_default();
                let detail = record.fields.get("detail").cloned().unwrap_or_default();
                s.push_str(&format!(
                    "| {} | {} | {} | {} |\n",
                    time, user, action, detail
                ));
            }
        }

        s
    }
}

impl Default for MarkdownFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl ReportFormatter for MarkdownFormatter {
    fn format(&self, data: &ReportData, title: &str, time_range: Option<&TimeRange>) -> String {
        match data.report_type {
            ReportType::BatchSummary => self.format_batch_summary(data, title, time_range),
            ReportType::Compliance => self.format_compliance_report(data, title, time_range),
            ReportType::Equipment => self.format_equipment_report(data, title, time_range),
            ReportType::AuditSummary => self.format_audit_summary(data, title, time_range),
            ReportType::Auto => self.format_batch_summary(data, title, time_range),
        }
    }
}

/// JSON formatter
pub struct JsonFormatter;

impl JsonFormatter {
    pub fn new() -> Self {
        Self
    }
}

impl Default for JsonFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl ReportFormatter for JsonFormatter {
    fn format(&self, data: &ReportData, title: &str, time_range: Option<&TimeRange>) -> String {
        let mut obj = serde_json::Map::new();
        obj.insert(
            "title".to_string(),
            serde_json::Value::String(title.to_string()),
        );

        if let Some(tr) = time_range {
            let mut range = serde_json::Map::new();
            range.insert(
                "start".to_string(),
                serde_json::Value::String(tr.start.to_rfc3339()),
            );
            range.insert(
                "end".to_string(),
                serde_json::Value::String(tr.end.to_rfc3339()),
            );
            obj.insert("time_range".to_string(), serde_json::Value::Object(range));
        }

        let mut metrics = serde_json::Map::new();
        metrics.insert("total".to_string(), serde_json::json!(data.metrics.total));
        if let Some(rate) = data.metrics.pass_rate {
            metrics.insert("pass_rate".to_string(), serde_json::json!(rate));
        }
        if let Some(rate) = data.metrics.compliance_rate {
            metrics.insert("compliance_rate".to_string(), serde_json::json!(rate));
        }
        obj.insert("metrics".to_string(), serde_json::Value::Object(metrics));

        let records: Vec<_> = data
            .records
            .iter()
            .map(|r| {
                let mut rec = serde_json::Map::new();
                rec.insert("id".to_string(), serde_json::Value::String(r.id.clone()));
                rec.insert("fields".to_string(), serde_json::json!(r.fields));
                serde_json::Value::Object(rec)
            })
            .collect();
        obj.insert("records".to_string(), serde_json::Value::Array(records));

        serde_json::to_string_pretty(&obj).unwrap_or_else(|_| "{}".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::types::{Metrics, Record};
    use std::collections::HashMap;

    fn make_test_data() -> ReportData<'static> {
        let mut fields1 = HashMap::new();
        fields1.insert("name".to_string(), "Batch-001".to_string());
        fields1.insert("status".to_string(), "PASS".to_string());
        fields1.insert("pass_rate".to_string(), "98.5".to_string());

        let mut fields2 = HashMap::new();
        fields2.insert("name".to_string(), "Batch-002".to_string());
        fields2.insert("status".to_string(), "FAIL".to_string());
        fields2.insert("pass_rate".to_string(), "95.2".to_string());

        ReportData {
            report_type: ReportType::BatchSummary,
            time_range: None,
            records: vec![
                Record {
                    id: "1".to_string(),
                    fields: fields1,
                },
                Record {
                    id: "2".to_string(),
                    fields: fields2,
                },
            ],
            metrics: Metrics {
                total: 2,
                pass_rate: Some(0.967),
                fail_rate: Some(0.033),
                compliance_rate: None,
            },
        }
    }

    #[test]
    fn test_markdown_formatter_batch_summary() {
        let formatter = MarkdownFormatter::new();
        let data = make_test_data();
        let output = formatter.format(&data, "本周批次汇总", None);

        assert!(output.contains("# 本周批次汇总"));
        assert!(output.contains("通过率"));
        assert!(output.contains("Batch-001"));
        assert!(output.contains("PASS"));
    }

    #[test]
    fn test_json_formatter() {
        let formatter = JsonFormatter::new();
        let data = make_test_data();
        let output = formatter.format(&data, "本周批次汇总", None);

        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["title"], "本周批次汇总");
        assert_eq!(parsed["metrics"]["total"], 2);
    }
}
