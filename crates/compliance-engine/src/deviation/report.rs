//! 偏差报告生成模块
//!
//! 将偏差调查结果格式化为 Markdown 报告。
//! 支持标准 GMP 偏差报告格式。

use crate::deviation::types::{
    CorrectiveAction, DeviationEvent, DeviationReportMetadata, DeviationSeverity, Investigation,
    ReportFormat,
};
use chrono::Utc;

/// 偏差报告生成器
pub struct DeviationReportGenerator;

impl DeviationReportGenerator {
    /// 创建一个新的报告生成器
    pub fn new() -> Self {
        Self
    }

    /// 生成偏差报告（Markdown 格式）
    pub fn generate(
        &self,
        event: &DeviationEvent,
        investigation: &Investigation,
        corrective_actions: &[CorrectiveAction],
        format: ReportFormat,
    ) -> (String, DeviationReportMetadata) {
        let report_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();

        let content = match format {
            ReportFormat::Markdown | ReportFormat::Pdf => {
                self.generate_markdown(event, investigation, corrective_actions)
            }
            ReportFormat::Json => self.generate_json(event, investigation, corrective_actions),
        };

        let metadata = DeviationReportMetadata {
            report_id: report_id.clone(),
            title: format!(
                "{}偏差调查报告 - {}",
                event.event_type.as_str(),
                event.event_id
            ),
            generated_at: now,
            investigation_id: investigation.investigation_id.clone(),
            event_id: event.event_id.clone(),
            event_type: event.event_type.as_str().to_string(),
            severity: event.severity.as_str().to_string(),
            format,
            confidence: 0.85,
            generation_time_ms: 0,
        };

        (content, metadata)
    }

    /// 生成 Markdown 格式的偏差报告
    fn generate_markdown(
        &self,
        event: &DeviationEvent,
        investigation: &Investigation,
        corrective_actions: &[CorrectiveAction],
    ) -> String {
        let mut report = String::new();

        // 报告标题
        report.push_str(&format!(
            "# {}偏差调查报告\n\n",
            self.event_type_label(event.event_type.as_str())
        ));

        // 报告元信息
        report.push_str("## 1. 报告元信息\n\n");
        report.push_str(&format!("| 字段 | 内容 |\n|---|---|\n"));
        report.push_str(&format!("| 报告 ID | `{}` |\n", uuid::Uuid::new_v4()));
        report.push_str(&format!("| 事件 ID | `{}` |\n", event.event_id));
        report.push_str(&format!(
            "| 调查 ID | `{}` |\n",
            investigation.investigation_id
        ));
        report.push_str(&format!(
            "| 生成时间 | {} |\n",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ));
        report.push_str(&format!("| 事件类型 | {} |\n", event.event_type.as_str()));
        report.push_str(&format!(
            "| 严重级别 | **{}** |\n",
            self.severity_label(event.severity)
        ));
        report.push_str(&format!("| 发起人 | {} |\n", investigation.initiated_by));
        report.push_str(&format!(
            "| 调查状态 | {} |\n\n",
            investigation.status.as_str()
        ));

        // 偏差事件描述
        report.push_str("## 2. 偏差事件描述\n\n");
        report.push_str(&format!(
            "**时间**: {}\n\n",
            event.occurred_at.format("%Y-%m-%d %H:%M:%S UTC")
        ));
        report.push_str(&format!("**来源**: {}\n\n", event.source));
        report.push_str(&format!("**描述**:\n\n{}\n\n", event.description));

        if let Some(ref product) = event.product_name {
            report.push_str(&format!("**产品**: {}\n\n", product));
        }
        if let Some(ref batch) = event.batch_id {
            report.push_str(&format!("**批次**: {}\n\n", batch));
        }
        if let Some(ref equipment) = event.equipment_id {
            report.push_str(&format!("**设备**: {}\n\n", equipment));
        }

        // 调查发现
        report.push_str("## 3. 调查发现\n\n");

        if investigation.findings.is_empty() {
            report.push_str("（调查中，暂无发现）\n\n");
        } else {
            for (i, finding) in investigation.findings.iter().enumerate() {
                report.push_str(&format!("### 3.{} 发现 {}\n\n", i + 1, finding.id));
                report.push_str(&format!("**描述**: {}\n\n", finding.description));

                if !finding.evidence.is_empty() {
                    report.push_str("**证据**:\n\n");
                    for evidence in &finding.evidence {
                        report.push_str(&format!("- {}\n", evidence));
                    }
                    report.push_str("\n");
                }

                if !finding.related_rules.is_empty() {
                    report.push_str(&format!(
                        "**关联规则**: {}\n\n",
                        finding.related_rules.join(", ")
                    ));
                }

                if finding.is_root_cause {
                    report.push_str("> ⚠️ **根因标识**: 该发现已被确认为根本原因\n\n");
                }
            }
        }

        // 根因分析
        report.push_str("## 4. 根因分析\n\n");
        match investigation.root_cause {
            Some(ref rc) => {
                report.push_str(&format!("{}\n\n", rc));
            }
            None => {
                report.push_str("（根因分析进行中或未确定）\n\n");
            }
        }

        // 影响评估
        report.push_str("## 5. 影响评估\n\n");
        match investigation.impact {
            Some(ref impact) => {
                report.push_str(&format!("| 维度 | 评估 |\n|---|---|\n"));
                report.push_str(&format!("| 产品质量影响 | {} |\n", impact.product_quality));
                if let Some(ref safety) = impact.patient_safety {
                    report.push_str(&format!("| 患者安全影响 | {} |\n", safety));
                }
                report.push_str(&format!("| 合规影响 | {} |\n", impact.compliance_impact));
                report.push_str(&format!("| 影响评分 | {}/10 |\n", impact.severity_score));

                if !impact.affected_batches.is_empty() {
                    report.push_str(&format!(
                        "| 受影响批次 | {} |\n",
                        impact.affected_batches.join(", ")
                    ));
                }

                report.push_str(&format!("\n**综合评估**:\n\n{}\n\n", impact.summary));
            }
            None => {
                report.push_str("（影响评估进行中或未完成）\n\n");
            }
        }

        // 纠正与预防措施 (CAPA)
        report.push_str("## 6. 纠正与预防措施 (CAPA)\n\n");

        if corrective_actions.is_empty() {
            report.push_str("（暂无建议措施）\n\n");
        } else {
            let immediate: Vec<&CorrectiveAction> = corrective_actions
                .iter()
                .filter(|a| {
                    matches!(
                        a.action_type,
                        crate::deviation::types::CorrectiveActionType::Immediate
                    )
                })
                .collect();
            let preventive: Vec<&CorrectiveAction> = corrective_actions
                .iter()
                .filter(|a| {
                    matches!(
                        a.action_type,
                        crate::deviation::types::CorrectiveActionType::Preventive
                    )
                })
                .collect();
            let long_term: Vec<&CorrectiveAction> = corrective_actions
                .iter()
                .filter(|a| {
                    matches!(
                        a.action_type,
                        crate::deviation::types::CorrectiveActionType::LongTerm
                    )
                })
                .collect();

            if !immediate.is_empty() {
                report.push_str("### 6.1 立即纠正措施\n\n");
                self.write_action_table(&mut report, &immediate);
            }

            if !preventive.is_empty() {
                report.push_str("### 6.2 预防措施\n\n");
                self.write_action_table(&mut report, &preventive);
            }

            if !long_term.is_empty() {
                report.push_str("### 6.3 长期改进措施\n\n");
                self.write_action_table(&mut report, &long_term);
            }
        }

        // 附录
        report.push_str("---\n\n");
        report.push_str("## 7. 附录\n\n");
        report.push_str(&format!(
            "**调查耗时**: {:?}\n\n",
            investigation
                .duration_seconds
                .map_or("N/A".to_string(), |s| format!("{} 秒", s))
        ));
        report.push_str(&format!(
            "**报告生成时间**: {}\n\n",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        ));
        report.push_str("**签名栏**:\n\n");
        report.push_str("| 角色 | 姓名 | 日期 | 签名 |\n|---|---|---|---|\n");
        report.push_str("| 调查人 | | | |\n");
        report.push_str("| 审核人 | | | |\n");
        report.push_str("| 批准人 | | | |\n");

        report
    }

    /// 写入措施表格
    fn write_action_table(&self, report: &mut String, actions: &[&CorrectiveAction]) {
        report.push_str("| ID | 描述 | 优先级 | 有效性检查 | 预期效果 |\n");
        report.push_str("|---|---|---|---|---|\n");
        for action in actions {
            let priority_label = match action.priority {
                crate::deviation::types::CorrectivePriority::Critical => "🔴 关键",
                crate::deviation::types::CorrectivePriority::High => "🟠 高",
                crate::deviation::types::CorrectivePriority::Medium => "🟡 中",
                crate::deviation::types::CorrectivePriority::Low => "🟢 低",
            };
            report.push_str(&format!(
                "| `{}` | {} | {} | {} | {} |\n",
                action.id,
                action.description,
                priority_label,
                action.effectiveness_check.as_deref().unwrap_or("-"),
                action.expected_outcome.as_deref().unwrap_or("-"),
            ));
        }
        report.push_str("\n");
    }

    /// 生成 JSON 格式的偏差报告
    fn generate_json(
        &self,
        event: &DeviationEvent,
        investigation: &Investigation,
        corrective_actions: &[CorrectiveAction],
    ) -> String {
        #[derive(serde::Serialize)]
        struct JsonReport<'a> {
            report_type: &'a str,
            event: &'a DeviationEvent,
            investigation: &'a Investigation,
            corrective_actions: &'a [CorrectiveAction],
            generated_at: String,
            report_id: String,
        }

        let report = JsonReport {
            report_type: "deviation_investigation",
            event,
            investigation,
            corrective_actions,
            generated_at: Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            report_id: uuid::Uuid::new_v4().to_string(),
        };

        serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
    }

    /// 获取事件类型的中文标签
    fn event_type_label(&self, event_type: &str) -> &str {
        match event_type {
            "temperature" => "温度",
            "time" => "时间",
            "procedure" => "操作步骤",
            "equipment" => "设备",
            "material" => "材料",
            "environmental" => "环境",
            "documentation" => "文档",
            _ => "偏差",
        }
    }

    /// 获取严重级别的中文标签
    fn severity_label(&self, severity: DeviationSeverity) -> &str {
        match severity {
            DeviationSeverity::Critical => "🔴 关键 (Critical)",
            DeviationSeverity::Major => "🟠 主要 (Major)",
            DeviationSeverity::Minor => "🟡 次要 (Minor)",
            DeviationSeverity::Advisory => "🟢 建议 (Advisory)",
        }
    }

    /// 验证报告内容是否完整
    pub fn validate_report(&self, content: &str) -> Vec<String> {
        let mut missing = Vec::new();

        let required_sections = [
            ("报告元信息", "## 1. 报告元信息"),
            ("偏差事件描述", "## 2. 偏差事件描述"),
            ("调查发现", "## 3. 调查发现"),
            ("根因分析", "## 4. 根因分析"),
            ("影响评估", "## 5. 影响评估"),
            ("纠正与预防措施", "## 6. 纠正与预防措施 (CAPA)"),
            ("附录", "## 7. 附录"),
        ];

        for &(name, marker) in &required_sections {
            if !content.contains(marker) {
                missing.push(format!("缺少必要章节: {}", name));
            }
        }

        // 检查是否包含签名栏
        if !content.contains("签名栏") {
            missing.push("缺少签名栏".to_string());
        }

        missing
    }
}

impl Default for DeviationReportGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deviation::types::{
        CorrectiveActionStatus, CorrectiveActionType, CorrectivePriority, DeviationEvent,
        DeviationEventType, DeviationSeverity, Finding, ImpactAssessment, Investigation,
        InvestigationStatus,
    };

    fn make_test_event() -> DeviationEvent {
        DeviationEvent::new(
            "DEV-REPORT-001",
            DeviationEventType::Temperature,
            "灭菌温度超出设定范围，设定值 121°C，实际值 118.5°C，持续时间 5 分钟",
            DeviationSeverity::Major,
            "BMS 系统",
        )
        .with_batch("注射用水", "WFI-20240501")
        .with_equipment("STER-001")
    }

    fn make_test_investigation(event: &DeviationEvent) -> Investigation {
        let mut inv = Investigation::new("INV-REPORT-001", event, "QA 调查员");
        inv.status = InvestigationStatus::Completed;

        inv.findings = vec![
            Finding {
                id: "F-001".to_string(),
                description: "温度传感器校准过期，导致测量偏差".to_string(),
                evidence: vec![
                    "传感器校准记录显示上次校准日期为 2023-06-15".to_string(),
                    "校准周期为 6 个月，已超期 3 个月".to_string(),
                ],
                related_rules: vec!["GMP-003-温度校准".to_string()],
                is_root_cause: true,
            },
            Finding {
                id: "F-002".to_string(),
                description: "温度控制系统 PID 参数漂移".to_string(),
                evidence: vec!["控制系统日志显示输出功率波动".to_string()],
                related_rules: vec!["GMP-004-设备维护".to_string()],
                is_root_cause: false,
            },
        ];

        inv.root_cause =
            Some("温度传感器超期未校准，同时 PID 参数漂移导致控制精度下降".to_string());
        inv.impact = Some(ImpactAssessment {
            product_quality: "可能影响产品的灭菌效果，需要进行额外的微生物检测".to_string(),
            patient_safety: Some("低风险，但需评估无菌保证水平".to_string()),
            compliance_impact: "违反 GMP 温度控制要求（21 CFR 211.63）".to_string(),
            affected_batches: vec!["WFI-20240501".to_string(), "WFI-20240502".to_string()],
            severity_score: 7,
            summary: "该偏差对产品质量有潜在影响，需要立即采取纠正措施并加强监控".to_string(),
        });

        inv.complete();
        inv
    }

    fn make_test_actions(event: &DeviationEvent) -> Vec<CorrectiveAction> {
        vec![
            CorrectiveAction {
                id: format!("CA-1-{}", event.event_id),
                description: "重新校准温度传感器".to_string(),
                action_type: CorrectiveActionType::Immediate,
                priority: CorrectivePriority::Critical,
                owner: Some("维护团队".to_string()),
                deadline: None,
                status: CorrectiveActionStatus::Proposed,
                effectiveness_check: Some("校准后验证温度精度在 ±0.5°C".to_string()),
                expected_outcome: Some("温度测量恢复准确".to_string()),
            },
            CorrectiveAction {
                id: format!("CA-2-{}", event.event_id),
                description: "优化 PID 控制参数并升级控制固件".to_string(),
                action_type: CorrectiveActionType::Preventive,
                priority: CorrectivePriority::High,
                owner: Some("自控团队".to_string()),
                deadline: None,
                status: CorrectiveActionStatus::Proposed,
                effectiveness_check: Some("PID 参数验证确认温度波动 < 1°C".to_string()),
                expected_outcome: Some("温度控制稳定性提升".to_string()),
            },
            CorrectiveAction {
                id: format!("CA-3-{}", event.event_id),
                description: "实施温度设备预防性维护计划，纳入 CMMS 系统管理".to_string(),
                action_type: CorrectiveActionType::LongTerm,
                priority: CorrectivePriority::Medium,
                owner: Some("工程部".to_string()),
                deadline: None,
                status: CorrectiveActionStatus::Proposed,
                effectiveness_check: Some("PM 计划完成率 > 95%".to_string()),
                expected_outcome: Some("系统性降低设备偏差".to_string()),
            },
        ]
    }

    #[test]
    fn test_generate_markdown_report() {
        let generator = DeviationReportGenerator::new();
        let event = make_test_event();
        let investigation = make_test_investigation(&event);
        let actions = make_test_actions(&event);

        let (content, metadata) =
            generator.generate(&event, &investigation, &actions, ReportFormat::Markdown);

        // 验证基本内容
        assert!(!content.is_empty());
        assert!(content.contains("# 温度偏差调查报告"));
        assert!(content.contains("## 1. 报告元信息"));
        assert!(content.contains("## 2. 偏差事件描述"));
        assert!(content.contains("## 3. 调查发现"));
        assert!(content.contains("## 4. 根因分析"));
        assert!(content.contains("## 5. 影响评估"));
        assert!(content.contains("## 6. 纠正与预防措施 (CAPA)"));
        assert!(content.contains("## 7. 附录"));
        assert!(content.contains("签名栏"));

        // 验证事件信息
        assert!(content.contains("DEV-REPORT-001"));
        assert!(content.contains("灭菌温度超出设定范围"));
        assert!(content.contains("WFI-20240501"));
        assert!(content.contains("STER-001"));

        // 验证发现
        assert!(content.contains("F-001"));
        assert!(content.contains("温度传感器校准过期"));

        // 验证根因
        assert!(content.contains("温度传感器超期未校准"));

        // 验证措施
        assert!(content.contains("立即纠正措施"));
        assert!(content.contains("预防措施"));
        assert!(content.contains("长期改进措施"));

        // 验证元数据
        assert_eq!(metadata.severity, "major");
        assert_eq!(metadata.event_type, "temperature");
    }

    #[test]
    fn test_generate_json_report() {
        let generator = DeviationReportGenerator::new();
        let event = make_test_event();
        let investigation = make_test_investigation(&event);
        let actions = make_test_actions(&event);

        let (content, metadata) =
            generator.generate(&event, &investigation, &actions, ReportFormat::Json);

        // JSON 报告应包含关键结构
        assert!(content.contains("deviation_investigation"));
        assert!(content.contains("DEV-REPORT-001"));
        assert!(content.contains("temperature"));

        // 验证元数据
        assert_eq!(metadata.format, ReportFormat::Json);
    }

    #[test]
    fn test_empty_investigation_report() {
        let generator = DeviationReportGenerator::new();
        let event = DeviationEvent::new(
            "DEV-NO-001",
            DeviationEventType::Time,
            "时间偏差",
            DeviationSeverity::Minor,
            "操作员",
        );
        let investigation = Investigation::new("INV-NO-001", &event, "QA");
        let actions = Vec::new();

        let (content, _) =
            generator.generate(&event, &investigation, &actions, ReportFormat::Markdown);

        // 空的调查应生成合理的报告
        assert!(content.contains("暂无发现"));
        assert!(content.contains("暂无建议措施"));
        assert!(content.contains("根因分析进行中"));
    }

    #[test]
    fn test_validate_report_complete() {
        let generator = DeviationReportGenerator::new();
        let event = make_test_event();
        let investigation = make_test_investigation(&event);
        let actions = make_test_actions(&event);

        let (content, _) =
            generator.generate(&event, &investigation, &actions, ReportFormat::Markdown);

        let missing = generator.validate_report(&content);
        assert!(missing.is_empty(), "报告应完整无缺: {:?}", missing);
    }

    #[test]
    fn test_validate_report_missing_sections() {
        let generator = DeviationReportGenerator::new();

        let missing = generator.validate_report("## 1. 报告元信息\n...部分内容...");
        assert!(!missing.is_empty());
        assert!(missing.iter().any(|m| m.contains("偏差事件描述")));
    }

    #[test]
    fn test_event_type_label() {
        let generator = DeviationReportGenerator::new();
        assert_eq!(generator.event_type_label("temperature"), "温度");
        assert_eq!(generator.event_type_label("procedure"), "操作步骤");
        assert_eq!(generator.event_type_label("unknown_type"), "偏差");
    }

    #[test]
    fn test_severity_label() {
        let generator = DeviationReportGenerator::new();
        assert!(generator
            .severity_label(DeviationSeverity::Critical)
            .contains("Critical"));
        assert!(generator
            .severity_label(DeviationSeverity::Major)
            .contains("Major"));
    }

    #[test]
    fn test_default_impl() {
        let generator: DeviationReportGenerator = Default::default();
        let event = make_test_event();
        let investigation = make_test_investigation(&event);
        let actions = make_test_actions(&event);
        let (content, _) =
            generator.generate(&event, &investigation, &actions, ReportFormat::Markdown);
        assert!(!content.is_empty());
    }
}
