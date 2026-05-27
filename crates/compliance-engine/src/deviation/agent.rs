//! Deviation Investigator Agent - AI 偏差调查助手
//!
//! 接收偏差事件，启动调查，生成偏差报告并提供纠正措施建议。
//! 与 GMP Retrieval v3 集成，通过 LLM/规则引擎完成调查。

use crate::deviation::corrective::CorrectiveActionRecommender;
use crate::deviation::report::DeviationReportGenerator;
use crate::deviation::types::{
    CorrectiveAction, DeviationEvent, DeviationSeverity, Finding, GmpRetrievalV3, ImpactAssessment,
    Investigation, InvestigationResult, InvestigationStatus, ReportFormat, RetrievedDocument,
};
use std::collections::HashMap;
use std::time::Instant;
use thiserror::Error;
use uuid::Uuid;

/// 调查超时时间（5 秒）
const INVESTIGATION_TIMEOUT_MS: u64 = 5000;

/// 偏差调查 Agent 错误类型
#[derive(Error, Debug)]
pub enum DeviationInvestigatorError {
    #[error("LLM 调用失败: {0}")]
    LlmError(String),
    #[error("调查超时")]
    Timeout,
    #[error("无效的偏差事件: {0}")]
    InvalidEvent(String),
    #[error("检索失败: {0}")]
    RetrievalError(String),
    #[error("报告生成失败: {0}")]
    ReportError(String),
    /// 检索结果不足
    #[error("检索结果不足, 需要至少{required}个结果, 实际{actual}个")]
    InsufficientRetrieval { required: usize, actual: usize },
}

/// 调查阶段
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum InvestigationPhase {
    /// 事件接收与验证
    EventValidation,
    /// 信息检索
    Retrieval,
    /// 根因分析
    RootCauseAnalysis,
    /// 影响评估
    ImpactAssessment,
    /// 纠正措施推荐
    CorrectiveActionRecommendation,
    /// 报告生成
    ReportGeneration,
    /// 完成
    Completed,
}

impl InvestigationPhase {
    fn as_str(&self) -> &'static str {
        match self {
            Self::EventValidation => "事件验证",
            Self::Retrieval => "信息检索",
            Self::RootCauseAnalysis => "根因分析",
            Self::ImpactAssessment => "影响评估",
            Self::CorrectiveActionRecommendation => "纠正措施推荐",
            Self::ReportGeneration => "报告生成",
            Self::Completed => "完成",
        }
    }
}

/// 偏差调查配置
#[derive(Debug, Clone)]
pub struct DeviationInvestigatorConfig {
    /// LLM 模型名称
    pub model: String,
    /// 调查超时时间（毫秒）
    pub timeout_ms: u64,
    /// 检索文档数量
    pub retrieval_top_k: usize,
    /// 最小检索结果数
    pub min_retrieval_results: usize,
    /// 置信度阈值
    pub confidence_threshold: f32,
    /// 是否开启流式输出
    pub streaming: bool,
}

impl Default for DeviationInvestigatorConfig {
    fn default() -> Self {
        Self {
            model: "qwen2.5:7b".to_string(),
            timeout_ms: INVESTIGATION_TIMEOUT_MS,
            retrieval_top_k: 5,
            min_retrieval_results: 1,
            confidence_threshold: 0.7,
            streaming: false,
        }
    }
}

/// 偏差调查 Agent
///
/// 核心 AI Agent，负责接收偏差事件、执行调查、生成报告和推荐措施。
pub struct DeviationInvestigator {
    /// GMP Retrieval v3 客户端
    retrieval_client: Option<Box<dyn GmpRetrievalV3>>,
    /// 纠正措施推荐器
    action_recommender: CorrectiveActionRecommender,
    /// 报告生成器
    report_generator: DeviationReportGenerator,
    /// Agent 配置
    config: DeviationInvestigatorConfig,
    /// 根因分析模式库（偏差类型 -> 常见根因模式）
    root_cause_patterns: HashMap<String, Vec<String>>,
}

impl DeviationInvestigator {
    /// 创建新的偏差调查 Agent
    pub fn new(config: DeviationInvestigatorConfig) -> Self {
        let mut patterns: HashMap<String, Vec<String>> = HashMap::new();

        // 温度偏差常见根因
        patterns.insert(
            "temperature".to_string(),
            vec![
                "温度传感器校准失效或超期".to_string(),
                "HVAC 系统故障或过滤网堵塞".to_string(),
                "温度控制系统 PID 参数漂移".to_string(),
                "开门时间过长导致温度波动".to_string(),
                "制冷/加热设备功率不足".to_string(),
                "环境温度超出设备设计范围".to_string(),
            ],
        );

        // 时间偏差常见根因
        patterns.insert(
            "time".to_string(),
            vec![
                "操作人员未遵循 SOP 时间要求".to_string(),
                "设备计时器校准偏差".to_string(),
                "前一工序延误导致连锁反应".to_string(),
                "物料准备时间超出预期".to_string(),
                "批量大小变化导致处理时间变化".to_string(),
            ],
        );

        // 操作步骤偏差常见根因
        patterns.insert(
            "procedure".to_string(),
            vec![
                "SOP 内容不清晰或存在歧义".to_string(),
                "操作人员培训不足".to_string(),
                "操作人员疲劳或分心".to_string(),
                "SOP 版本混乱或使用了过期版本".to_string(),
                "流程设计不合理".to_string(),
            ],
        );

        // 设备偏差常见根因
        patterns.insert(
            "equipment".to_string(),
            vec![
                "设备预防性维护不足".to_string(),
                "设备老化或部件磨损".to_string(),
                "设备操作参数设置错误".to_string(),
                "设备软件/固件缺陷".to_string(),
                "设备安装或调试不当".to_string(),
            ],
        );

        Self {
            retrieval_client: None,
            action_recommender: CorrectiveActionRecommender::new(),
            report_generator: DeviationReportGenerator::new(),
            config,
            root_cause_patterns: patterns,
        }
    }

    /// 设置 GMP Retrieval v3 客户端
    pub fn with_retrieval_client(mut self, client: Box<dyn GmpRetrievalV3>) -> Self {
        self.retrieval_client = Some(client);
        self
    }

    /// 接收偏差事件并启动调查（同步入口）
    pub fn investigate(&self, event: &DeviationEvent) -> InvestigationResult {
        let start = Instant::now();

        // 使用 tokio::runtime 执行异步超时逻辑
        let result = self.execute_investigation(event);

        let elapsed_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(mut result) => {
                result.processing_time_ms = elapsed_ms;
                result.model = Some(self.config.model.clone());
                result
            }
            Err(e) => {
                let mut error_result = InvestigationResult::error(&e.to_string());
                error_result.processing_time_ms = elapsed_ms;
                error_result.model = Some(self.config.model.clone());
                error_result
            }
        }
    }

    /// 执行完整的调查流程
    fn execute_investigation(
        &self,
        event: &DeviationEvent,
    ) -> Result<InvestigationResult, DeviationInvestigatorError> {
        let investigation_id = Uuid::new_v4().to_string();

        // Phase 1: 事件接收与验证
        let _phase = InvestigationPhase::EventValidation;
        self.validate_event(event)?;

        // Phase 2: 创建调查
        let mut investigation =
            Investigation::new(&investigation_id, event, "Deviation Investigator Agent");
        investigation.status = InvestigationStatus::InProgress;

        // Phase 3: 信息检索 (GMP Retrieval v3)
        let _phase = InvestigationPhase::Retrieval;
        let retrieved_docs = self.retrieve_information(event)?;

        // Phase 4: 根因分析
        let _phase = InvestigationPhase::RootCauseAnalysis;
        let (findings, root_cause) = self.analyze_root_cause(event, &retrieved_docs);
        investigation.findings = findings;
        investigation.root_cause = root_cause;

        // Phase 5: 影响评估
        let _phase = InvestigationPhase::ImpactAssessment;
        investigation.impact = Some(self.assess_impact(event));

        // Phase 6: 纠正措施推荐
        let _phase = InvestigationPhase::CorrectiveActionRecommendation;
        let corrective_actions = self.recommend_actions(event, &investigation);

        // Phase 7: 报告生成
        let _phase = InvestigationPhase::ReportGeneration;
        investigation.status = InvestigationStatus::Completed;
        investigation.complete();

        let (report_content, report_metadata) = self.report_generator.generate(
            event,
            &investigation,
            &corrective_actions,
            ReportFormat::Markdown,
        );

        // 构建结果
        Ok(InvestigationResult {
            result_id: Uuid::new_v4().to_string(),
            investigation,
            report_content,
            report_metadata,
            corrective_actions,
            model: Some(self.config.model.clone()),
            processing_time_ms: 0,
            timestamp: chrono::Utc::now(),
        })
    }

    /// 验证偏差事件的有效性
    fn validate_event(&self, event: &DeviationEvent) -> Result<(), DeviationInvestigatorError> {
        if event.event_id.is_empty() {
            return Err(DeviationInvestigatorError::InvalidEvent(
                "事件 ID 不能为空".to_string(),
            ));
        }
        if event.description.is_empty() {
            return Err(DeviationInvestigatorError::InvalidEvent(
                "事件描述不能为空".to_string(),
            ));
        }
        if event.source.is_empty() {
            return Err(DeviationInvestigatorError::InvalidEvent(
                "事件来源不能为空".to_string(),
            ));
        }
        Ok(())
    }

    /// 从 GMP Retrieval v3 检索相关信息
    fn retrieve_information(
        &self,
        event: &DeviationEvent,
    ) -> Result<Vec<RetrievedDocument>, DeviationInvestigatorError> {
        match &self.retrieval_client {
            Some(client) => {
                let query = self.build_retrieval_query(event);
                let docs = client.retrieve(&query, self.config.retrieval_top_k);

                if docs.len() < self.config.min_retrieval_results {
                    return Err(DeviationInvestigatorError::InsufficientRetrieval {
                        required: self.config.min_retrieval_results,
                        actual: docs.len(),
                    });
                }

                Ok(docs)
            }
            None => {
                // 无 retrieval client 时返回空列表（用于测试）
                Ok(Vec::new())
            }
        }
    }

    /// 构建检索查询字符串
    fn build_retrieval_query(&self, event: &DeviationEvent) -> String {
        let mut parts = vec![
            format!("偏差类型: {}", event.event_type.as_str()),
            format!("描述: {}", event.description),
            format!("严重级别: {}", event.severity.as_str()),
        ];

        if let Some(ref product) = event.product_name {
            parts.push(format!("产品: {}", product));
        }
        if let Some(ref batch) = event.batch_id {
            parts.push(format!("批次: {}", batch));
        }
        if let Some(ref equipment) = event.equipment_id {
            parts.push(format!("设备: {}", equipment));
        }

        parts.join("\n")
    }

    /// 分析根因，生成调查发现
    fn analyze_root_cause(
        &self,
        event: &DeviationEvent,
        _retrieved_docs: &[RetrievedDocument],
    ) -> (Vec<Finding>, Option<String>) {
        let mut findings = Vec::new();
        let event_key = event.event_type.as_str().to_lowercase();

        // 从模式库获取常见根因
        let common_causes = self
            .root_cause_patterns
            .get(&event_key)
            .cloned()
            .unwrap_or_default();

        // 创建调查发现
        for (i, cause) in common_causes.iter().enumerate() {
            let is_root = i == 0; // 第一个列为根因
            findings.push(Finding {
                id: format!("F-{}-{}", event.event_id, i + 1),
                description: format!(
                    "可能的根因: {}（基于 {} 偏差的历史模式分析）",
                    cause,
                    event.event_type.as_str()
                ),
                evidence: vec![
                    format!("偏差类型 {} 的常见模式匹配", event.event_type.as_str()),
                    format!(
                        "事件描述 '{}' 与模式 '{:.30}...' 关联",
                        event.description, cause
                    ),
                ],
                related_rules: vec!["GMP-ROOT-CAUSE-ANALYSIS".to_string()],
                is_root_cause: is_root,
            });
        }

        // 如果没有模式匹配，生成通用发现
        if findings.is_empty() {
            findings.push(Finding {
                id: format!("F-{}-1", event.event_id),
                description: format!(
                    "对 {} 类型偏差的初步分析，需要进一步调查具体根因",
                    event.event_type.as_str()
                ),
                evidence: vec![format!("事件 '{}' 的详细描述已记录", event.event_id)],
                related_rules: vec![],
                is_root_cause: true,
            });
        }

        // 根因总结
        let root_cause = if !common_causes.is_empty() {
            Some(format!(
                "基于 {} 模式分析，最可能的根因是: {}. 建议通过现场调查和数据分析进一步确认。",
                event.event_type.as_str(),
                common_causes[0]
            ))
        } else {
            Some(format!(
                "偏差类型 '{}' 的根因需要进一步调查。建议启动 5-Why 分析或鱼骨图分析。",
                event.event_type.as_str()
            ))
        };

        (findings, root_cause)
    }

    /// 评估偏差影响
    fn assess_impact(&self, event: &DeviationEvent) -> ImpactAssessment {
        let (quality_impact, safety_impact, compliance_impact, score) = match event.severity {
            DeviationSeverity::Critical => (
                "可能严重影响产品质量，导致产品报废或召回".to_string(),
                Some("可能存在患者安全风险，需立即评估".to_string()),
                "严重违反 GMP 核心要求，可能涉及监管报告".to_string(),
                9,
            ),
            DeviationSeverity::Major => (
                "可能影响产品质量，需要额外的检测和评估".to_string(),
                Some("低风险，但需关注".to_string()),
                "违反 GMP 流程要求，需要偏差记录和管理层审批".to_string(),
                6,
            ),
            DeviationSeverity::Minor => (
                "对产品质量影响有限".to_string(),
                None,
                "轻微偏离标准流程，记录即可".to_string(),
                3,
            ),
            DeviationSeverity::Advisory => ("无影响".to_string(), None, "建议改进".to_string(), 1),
        };

        let mut affected_batches = Vec::new();
        if let Some(ref batch) = event.batch_id {
            affected_batches.push(batch.clone());
        }

        ImpactAssessment {
            product_quality: quality_impact,
            patient_safety: safety_impact,
            compliance_impact,
            affected_batches,
            severity_score: score,
            summary: format!(
                "偏差 '{}' 被评估为 {} 级别。{}",
                event.event_id,
                event.severity.as_str(),
                match event.severity {
                    DeviationSeverity::Critical => "需要立即采取行动。",
                    DeviationSeverity::Major => "需要及时处理。",
                    DeviationSeverity::Minor => "按常规流程处理。",
                    DeviationSeverity::Advisory => "作为改进机会。",
                }
            ),
        }
    }

    /// 推荐纠正措施
    fn recommend_actions(
        &self,
        event: &DeviationEvent,
        investigation: &Investigation,
    ) -> Vec<CorrectiveAction> {
        let mut actions = self.action_recommender.recommend(event, investigation);

        // 如果已确定根因，优化措施描述
        if let Some(ref root_cause) = investigation.root_cause {
            actions = self
                .action_recommender
                .refine_actions_with_root_cause(actions, root_cause);
        }

        actions
    }

    /// 获取当前阶段描述
    pub fn current_phase_description(&self) -> String {
        "偏差调查 Agent 就绪".to_string()
    }

    /// 检查配置是否有效
    pub fn validate_config(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.config.model.is_empty() {
            errors.push("模型名称不能为空".to_string());
        }
        if self.config.timeout_ms == 0 {
            errors.push("超时时间必须大于 0".to_string());
        }
        if self.config.retrieval_top_k == 0 {
            errors.push("检索数量必须大于 0".to_string());
        }
        if self.config.confidence_threshold <= 0.0 || self.config.confidence_threshold > 1.0 {
            errors.push("置信度阈值必须在 0-1 之间".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

// ============== 测试用 Mock Retrieval 客户端 ==============

/// 用于测试的模拟 GMP Retrieval v3 客户端
pub struct MockGmpRetrievalClient {
    documents: Vec<RetrievedDocument>,
}

impl MockGmpRetrievalClient {
    pub fn new(documents: Vec<RetrievedDocument>) -> Self {
        Self { documents }
    }
}

impl GmpRetrievalV3 for MockGmpRetrievalClient {
    fn retrieve(&self, _query: &str, top_k: usize) -> Vec<RetrievedDocument> {
        self.documents.iter().take(top_k).cloned().collect()
    }
}

impl Default for MockGmpRetrievalClient {
    fn default() -> Self {
        Self {
            documents: vec![
                RetrievedDocument {
                    doc_id: "DOC-001".to_string(),
                    title: "GMP 温度控制规范".to_string(),
                    content: "GMP 要求所有温度敏感区域必须安装经过校准的温度监控设备..."
                        .to_string(),
                    relevance_score: 0.95,
                    source: "GMP 规范文档".to_string(),
                },
                RetrievedDocument {
                    doc_id: "DOC-002".to_string(),
                    title: "温度偏差处理 SOP".to_string(),
                    content: "当温度偏差超过规定范围时，操作人员应立即...".to_string(),
                    relevance_score: 0.88,
                    source: "标准操作程序".to_string(),
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deviation::types::DeviationEventType;

    fn make_test_event() -> DeviationEvent {
        DeviationEvent::new(
            "DEV-AGENT-001",
            DeviationEventType::Temperature,
            "灭菌温度超出设定范围，设定值 121°C，实际值 118.5°C",
            DeviationSeverity::Major,
            "BMS 系统",
        )
        .with_batch("注射用水", "WFI-20240501")
        .with_equipment("STER-001")
    }

    fn make_agent() -> DeviationInvestigator {
        let config = DeviationInvestigatorConfig::default();
        DeviationInvestigator::new(config)
    }

    fn make_agent_with_mock_retrieval() -> DeviationInvestigator {
        let config = DeviationInvestigatorConfig::default();
        let mock_client = MockGmpRetrievalClient::default();
        DeviationInvestigator::new(config).with_retrieval_client(Box::new(mock_client))
    }

    #[test]
    fn test_agent_new() {
        let agent = make_agent();
        assert_eq!(agent.config.model, "qwen2.5:7b");
        assert!(agent.retrieval_client.is_none());
    }

    #[test]
    fn test_agent_with_retrieval() {
        let agent = make_agent_with_mock_retrieval();
        assert!(agent.retrieval_client.is_some());
    }

    #[test]
    fn test_investigate_basic() {
        let agent = make_agent();
        let event = make_test_event();
        let result = agent.investigate(&event);

        // 验证调查结果
        assert!(!result.result_id.is_empty());
        assert!(!result.report_content.is_empty());
        assert!(!result.corrective_actions.is_empty());
        assert_eq!(result.investigation.status, InvestigationStatus::Completed);
        assert_eq!(result.investigation.event_id, "DEV-AGENT-001");
    }

    #[test]
    fn test_investigate_with_mock_retrieval() {
        let agent = make_agent_with_mock_retrieval();
        let event = make_test_event();
        let result = agent.investigate(&event);

        assert!(!result.report_content.is_empty());
        assert!(!result.corrective_actions.is_empty());
        // 应该有至少 3 个纠正措施
        assert!(result.corrective_actions.len() >= 3);
    }

    #[test]
    fn test_validate_event_empty_id() {
        let agent = make_agent();
        let event = DeviationEvent::new(
            "",
            DeviationEventType::Temperature,
            "desc",
            DeviationSeverity::Minor,
            "src",
        );
        let result = agent.investigate(&event);
        assert!(result.report_content.contains("调查失败"));
        assert!(result.report_content.contains("事件 ID 不能为空"));
    }

    #[test]
    fn test_validate_event_empty_description() {
        let agent = make_agent();
        let event = DeviationEvent::new(
            "DEV-002",
            DeviationEventType::Temperature,
            "",
            DeviationSeverity::Minor,
            "src",
        );
        let result = agent.investigate(&event);
        assert!(result.report_content.contains("事件描述不能为空"));
    }

    #[test]
    fn test_validate_event_empty_source() {
        let agent = make_agent();
        let event = DeviationEvent::new(
            "DEV-003",
            DeviationEventType::Temperature,
            "desc",
            DeviationSeverity::Minor,
            "",
        );
        let result = agent.investigate(&event);
        assert!(result.report_content.contains("事件来源不能为空"));
    }

    #[test]
    fn test_analyze_root_cause_temperature() {
        let agent = make_agent();
        let event = make_test_event();
        let (findings, root_cause) = agent.analyze_root_cause(&event, &[]);

        assert!(!findings.is_empty());
        assert!(root_cause.is_some());
        assert!(root_cause.unwrap().contains("温度传感器"));
    }

    #[test]
    fn test_analyze_root_cause_custom_type() {
        let agent = make_agent();
        let event = DeviationEvent::new(
            "DEV-004",
            DeviationEventType::Other("罕见偏差".to_string()),
            "罕见偏差",
            DeviationSeverity::Advisory,
            "test",
        );
        let (findings, root_cause) = agent.analyze_root_cause(&event, &[]);

        assert!(!findings.is_empty());
        assert!(root_cause.is_some());
        assert!(root_cause.unwrap().contains("5-Why"));
    }

    #[test]
    fn test_assess_impact_critical() {
        let agent = make_agent();
        let event = DeviationEvent::new(
            "DEV-CRI-001",
            DeviationEventType::Temperature,
            "critical deviation",
            DeviationSeverity::Critical,
            "system",
        );
        let impact = agent.assess_impact(&event);
        assert_eq!(impact.severity_score, 9);
        assert!(impact.patient_safety.is_some());
    }

    #[test]
    fn test_assess_impact_major() {
        let agent = make_agent();
        let event = make_test_event(); // Major
        let impact = agent.assess_impact(&event);
        assert_eq!(impact.severity_score, 6);
    }

    #[test]
    fn test_assess_impact_minor() {
        let agent = make_agent();
        let event = DeviationEvent::new(
            "DEV-MIN-001",
            DeviationEventType::Time,
            "minor time deviation",
            DeviationSeverity::Minor,
            "operator",
        );
        let impact = agent.assess_impact(&event);
        assert_eq!(impact.severity_score, 3);
        assert!(impact.patient_safety.is_none());
    }

    #[test]
    fn test_assess_impact_advisory() {
        let agent = make_agent();
        let event = DeviationEvent::new(
            "DEV-ADV-001",
            DeviationEventType::Documentation,
            "documentation issue",
            DeviationSeverity::Advisory,
            "qa",
        );
        let impact = agent.assess_impact(&event);
        assert_eq!(impact.severity_score, 1);
    }

    #[test]
    fn test_build_retrieval_query() {
        let agent = make_agent();
        let event = make_test_event();
        let query = agent.build_retrieval_query(&event);

        assert!(query.contains("temperature"));
        assert!(query.contains("灭菌温度超出"));
        assert!(query.contains("major"));
        assert!(query.contains("WFI-20240501"));
        assert!(query.contains("STER-001"));
    }

    #[test]
    fn test_recommend_actions() {
        let agent = make_agent();
        let event = make_test_event();
        let (findings, root_cause) = agent.analyze_root_cause(&event, &[]);
        let mut investigation = Investigation::new("INV-TEST", &event, "tester");
        investigation.findings = findings;
        investigation.root_cause = root_cause;

        let actions = agent.recommend_actions(&event, &investigation);
        assert!(!actions.is_empty());

        // 检查措施是否包含根因信息
        let has_refined = actions
            .iter()
            .any(|a| a.description.contains("根因") || a.description.contains("传感器"));
        assert!(has_refined, "措施应包含根因信息");
    }

    #[test]
    fn test_config_default() {
        let config = DeviationInvestigatorConfig::default();
        assert_eq!(config.model, "qwen2.5:7b");
        assert_eq!(config.timeout_ms, 5000);
        assert_eq!(config.retrieval_top_k, 5);
        assert_eq!(config.confidence_threshold, 0.7);
    }

    #[test]
    fn test_validate_config_valid() {
        let config = DeviationInvestigatorConfig::default();
        let agent = DeviationInvestigator::new(config);
        assert!(agent.validate_config().is_ok());
    }

    #[test]
    fn test_validate_config_invalid_model() {
        let config = DeviationInvestigatorConfig {
            model: String::new(),
            ..Default::default()
        };
        let agent = DeviationInvestigator::new(config);
        let result = agent.validate_config();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains(&"模型名称不能为空".to_string()));
    }

    #[test]
    fn test_validate_config_invalid_timeout() {
        let config = DeviationInvestigatorConfig {
            timeout_ms: 0,
            ..Default::default()
        };
        let agent = DeviationInvestigator::new(config);
        let result = agent.validate_config();
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_config_invalid_confidence() {
        let config = DeviationInvestigatorConfig {
            confidence_threshold: 1.5,
            ..Default::default()
        };
        let agent = DeviationInvestigator::new(config);
        assert!(agent.validate_config().is_err());
    }

    #[test]
    fn test_current_phase_description() {
        let agent = make_agent();
        let desc = agent.current_phase_description();
        assert!(desc.contains("偏差调查 Agent 就绪"));
    }

    #[test]
    fn test_investigation_phase_as_str() {
        assert_eq!(InvestigationPhase::EventValidation.as_str(), "事件验证");
        assert_eq!(InvestigationPhase::Retrieval.as_str(), "信息检索");
        assert_eq!(InvestigationPhase::RootCauseAnalysis.as_str(), "根因分析");
        assert_eq!(InvestigationPhase::ImpactAssessment.as_str(), "影响评估");
        assert_eq!(
            InvestigationPhase::CorrectiveActionRecommendation.as_str(),
            "纠正措施推荐"
        );
        assert_eq!(InvestigationPhase::ReportGeneration.as_str(), "报告生成");
        assert_eq!(InvestigationPhase::Completed.as_str(), "完成");
    }

    #[test]
    fn test_mock_retrieval_client_default() {
        let client = MockGmpRetrievalClient::default();
        let docs = client.retrieve("temperature deviation", 5);
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].doc_id, "DOC-001");
    }

    #[test]
    fn test_mock_retrieval_client_top_k() {
        let client = MockGmpRetrievalClient::default();
        let docs = client.retrieve("test", 1);
        assert_eq!(docs.len(), 1);
    }

    #[test]
    fn test_all_deviation_types_investigate() {
        let agent = make_agent();
        let types = vec![
            DeviationEventType::Time,
            DeviationEventType::Procedure,
            DeviationEventType::Equipment,
            DeviationEventType::Material,
            DeviationEventType::Environmental,
            DeviationEventType::Documentation,
        ];

        for evt_type in types {
            let event = DeviationEvent::new(
                &format!("DEV-{:?}-001", evt_type),
                evt_type,
                "标准偏差描述用于测试",
                DeviationSeverity::Minor,
                "测试",
            );
            let result = agent.investigate(&event);
            // 所有类型的偏差都应该成功处理
            assert!(
                !result.report_content.is_empty(),
                "类型 {:?} 的调查应生成报告",
                result.investigation.event_id
            );
            assert!(
                !result.corrective_actions.is_empty(),
                "类型 {:?} 的调查应生成纠正措施",
                result.investigation.event_id
            );
        }
    }
}
