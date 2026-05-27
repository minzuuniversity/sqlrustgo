//! 偏差调查助手的核心类型定义
//!
//! 包含偏差事件、调查、报告和纠正措施相关的所有类型。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 偏差事件严重级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeviationSeverity {
    /// 关键偏差 - 可能影响产品质量或患者安全
    Critical,
    /// 主要偏差 - 显著偏离已批准流程
    Major,
    /// 次要偏差 - 轻微偏离，影响有限
    Minor,
    /// 建议关注 - 潜在改进机会
    Advisory,
}

impl DeviationSeverity {
    /// 获取严重级别的字符串表示
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::Major => "major",
            Self::Minor => "minor",
            Self::Advisory => "advisory",
        }
    }

    /// 从字符串解析严重级别
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "critical" => Self::Critical,
            "major" => Self::Major,
            "minor" => Self::Minor,
            _ => Self::Advisory,
        }
    }
}

/// 偏差事件类型分类
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviationEventType {
    /// 温度偏差
    Temperature,
    /// 时间偏差
    Time,
    /// 操作步骤偏差
    Procedure,
    /// 设备偏差
    Equipment,
    /// 材料偏差
    Material,
    /// 环境偏差
    Environmental,
    /// 文档偏差
    Documentation,
    /// 其他偏差
    Other(String),
}

impl DeviationEventType {
    /// 获取事件类型字符串
    pub fn as_str(&self) -> &str {
        match self {
            Self::Temperature => "temperature",
            Self::Time => "time",
            Self::Procedure => "procedure",
            Self::Equipment => "equipment",
            Self::Material => "material",
            Self::Environmental => "environmental",
            Self::Documentation => "documentation",
            Self::Other(s) => s.as_str(),
        }
    }
}

/// 偏差事件 - 输入：AI Agent 接收的事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviationEvent {
    /// 唯一事件标识符
    pub event_id: String,
    /// 偏差类型
    pub event_type: DeviationEventType,
    /// 偏差描述（自然语言）
    pub description: String,
    /// 严重级别
    pub severity: DeviationSeverity,
    /// 事件发生时间
    pub occurred_at: DateTime<Utc>,
    /// 事件来源（操作员/系统/设备名）
    pub source: String,
    /// 相关产品名称
    pub product_name: Option<String>,
    /// 相关批次 ID
    pub batch_id: Option<String>,
    /// 相关设备 ID
    pub equipment_id: Option<String>,
    /// 额外元数据（JSON 格式）
    pub metadata: serde_json::Value,
}

impl DeviationEvent {
    /// 创建新的偏差事件
    pub fn new(
        event_id: &str,
        event_type: DeviationEventType,
        description: &str,
        severity: DeviationSeverity,
        source: &str,
    ) -> Self {
        Self {
            event_id: event_id.to_string(),
            event_type,
            description: description.to_string(),
            severity,
            occurred_at: Utc::now(),
            source: source.to_string(),
            product_name: None,
            batch_id: None,
            equipment_id: None,
            metadata: serde_json::json!({}),
        }
    }

    /// 设置批次信息
    pub fn with_batch(mut self, product_name: &str, batch_id: &str) -> Self {
        self.product_name = Some(product_name.to_string());
        self.batch_id = Some(batch_id.to_string());
        self
    }

    /// 设置设备信息
    pub fn with_equipment(mut self, equipment_id: &str) -> Self {
        self.equipment_id = Some(equipment_id.to_string());
        self
    }
}

/// 调查状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum InvestigationStatus {
    /// 待启动
    Pending,
    /// 调查进行中
    InProgress,
    /// 调查完成
    Completed,
    /// 调查已关闭
    Closed,
}

impl InvestigationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Closed => "closed",
        }
    }
}

/// 调查发现
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    /// 发现唯一标识
    pub id: String,
    /// 发现描述
    pub description: String,
    /// 支持证据
    pub evidence: Vec<String>,
    /// 关联的规则/流程编号
    pub related_rules: Vec<String>,
    /// 是否已确认为根因
    pub is_root_cause: bool,
}

/// 影响评估
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactAssessment {
    /// 产品质量影响
    pub product_quality: String,
    /// 患者安全影响
    pub patient_safety: Option<String>,
    /// 合规影响
    pub compliance_impact: String,
    /// 受影响批次范围
    pub affected_batches: Vec<String>,
    /// 影响严重程度（1-10）
    pub severity_score: u8,
    /// 综合分析
    pub summary: String,
}

/// 偏差调查
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Investigation {
    /// 调查唯一标识
    pub investigation_id: String,
    /// 关联的偏差事件 ID
    pub event_id: String,
    /// 调查状态
    pub status: InvestigationStatus,
    /// 调查标题
    pub title: String,
    /// 调查发现列表
    pub findings: Vec<Finding>,
    /// 根因分析
    pub root_cause: Option<String>,
    /// 影响评估
    pub impact: Option<ImpactAssessment>,
    /// 调查发起人
    pub initiated_by: String,
    /// 调查开始时间
    pub started_at: DateTime<Utc>,
    /// 调查完成时间
    pub completed_at: Option<DateTime<Utc>>,
    /// 调查耗时（秒）
    pub duration_seconds: Option<u64>,
}

impl Investigation {
    pub fn new(investigation_id: &str, event: &DeviationEvent, initiated_by: &str) -> Self {
        Self {
            investigation_id: investigation_id.to_string(),
            event_id: event.event_id.clone(),
            status: InvestigationStatus::Pending,
            title: format!("{}偏差调查 - {}", event.event_type.as_str(), event.event_id),
            findings: Vec::new(),
            root_cause: None,
            impact: None,
            initiated_by: initiated_by.to_string(),
            started_at: Utc::now(),
            completed_at: None,
            duration_seconds: None,
        }
    }

    /// 完成调查
    pub fn complete(&mut self) {
        self.status = InvestigationStatus::Completed;
        self.completed_at = Some(Utc::now());
        let dur = Utc::now() - self.started_at;
        self.duration_seconds = Some(dur.num_seconds() as u64);
    }
}

/// 纠正措施类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CorrectiveActionType {
    /// 立即纠正行动
    Immediate,
    /// 预防措施
    Preventive,
    /// 长期改进
    LongTerm,
}

impl CorrectiveActionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Immediate => "immediate",
            Self::Preventive => "preventive",
            Self::LongTerm => "long_term",
        }
    }
}

/// 纠正措施优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CorrectivePriority {
    Critical,
    High,
    Medium,
    Low,
}

impl CorrectivePriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }
}

/// 纠正措施建议
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectiveAction {
    /// 措施唯一标识
    pub id: String,
    /// 措施描述
    pub description: String,
    /// 措施类型
    pub action_type: CorrectiveActionType,
    /// 优先级
    pub priority: CorrectivePriority,
    /// 负责部门/人员
    pub owner: Option<String>,
    /// 完成期限
    pub deadline: Option<DateTime<Utc>>,
    /// 实施状态
    pub status: CorrectiveActionStatus,
    /// 有效性检查方法
    pub effectiveness_check: Option<String>,
    /// 预期效果
    pub expected_outcome: Option<String>,
}

/// 纠正措施状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CorrectiveActionStatus {
    Proposed,
    Approved,
    InProgress,
    Completed,
    Verified,
    Closed,
}

impl CorrectiveActionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Approved => "approved",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Verified => "verified",
            Self::Closed => "closed",
        }
    }
}

/// 偏差报告元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviationReportMetadata {
    /// 报告 ID
    pub report_id: String,
    /// 报告标题
    pub title: String,
    /// 报告生成时间
    pub generated_at: DateTime<Utc>,
    /// 调查 ID
    pub investigation_id: String,
    /// 关联事件 ID
    pub event_id: String,
    /// 事件类型
    pub event_type: String,
    /// 事件严重级别
    pub severity: String,
    /// 报告格式
    pub format: ReportFormat,
    /// 置信度评分
    pub confidence: f32,
    /// 生成耗时（毫秒）
    pub generation_time_ms: u64,
}

/// 报告格式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportFormat {
    Markdown,
    Json,
    Pdf,
}

impl ReportFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Markdown => "markdown",
            Self::Json => "json",
            Self::Pdf => "pdf",
        }
    }
}

/// 偏差调查结果（Agent 输出）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestigationResult {
    /// 调查结果唯一标识
    pub result_id: String,
    /// 关联的调查
    pub investigation: Investigation,
    /// 偏差报告内容（Markdown）
    pub report_content: String,
    /// 报告元数据
    pub report_metadata: DeviationReportMetadata,
    /// 推荐的纠正措施列表
    pub corrective_actions: Vec<CorrectiveAction>,
    /// LLM 模型名称
    pub model: Option<String>,
    /// 处理耗时
    pub processing_time_ms: u64,
    /// 结果时间戳
    pub timestamp: DateTime<Utc>,
}

/// GMP Retrieval v3 集成 trait
/// 允许 Agent 从 GMP 知识库检索相关信息
pub trait GmpRetrievalV3: Send + Sync {
    /// 检索与偏差相关的文档和规则
    fn retrieve(&self, query: &str, top_k: usize) -> Vec<RetrievedDocument>;
}

/// 检索到的文档
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievedDocument {
    /// 文档 ID
    pub doc_id: String,
    /// 文档标题
    pub title: String,
    /// 文档内容
    pub content: String,
    /// 相关性评分
    pub relevance_score: f32,
    /// 文档源
    pub source: String,
}

impl InvestigationResult {
    /// 创建错误结果
    pub fn error(error_message: &str) -> Self {
        let now = Utc::now();
        Self {
            result_id: uuid::Uuid::new_v4().to_string(),
            investigation: Investigation {
                investigation_id: String::new(),
                event_id: String::new(),
                status: InvestigationStatus::Closed,
                title: "Error".to_string(),
                findings: Vec::new(),
                root_cause: None,
                impact: None,
                initiated_by: String::new(),
                started_at: now,
                completed_at: Some(now),
                duration_seconds: Some(0),
            },
            report_content: format!("## 调查失败\n\n错误信息: {}\n", error_message),
            report_metadata: DeviationReportMetadata {
                report_id: uuid::Uuid::new_v4().to_string(),
                title: "Error Report".to_string(),
                generated_at: now,
                investigation_id: String::new(),
                event_id: String::new(),
                event_type: "unknown".to_string(),
                severity: "unknown".to_string(),
                format: ReportFormat::Markdown,
                confidence: 0.0,
                generation_time_ms: 0,
            },
            corrective_actions: Vec::new(),
            model: None,
            processing_time_ms: 0,
            timestamp: now,
        }
    }

    /// 创建超时结果
    pub fn timeout() -> Self {
        let now = Utc::now();
        Self {
            result_id: uuid::Uuid::new_v4().to_string(),
            investigation: Investigation {
                investigation_id: String::new(),
                event_id: String::new(),
                status: InvestigationStatus::Pending,
                title: "Timeout".to_string(),
                findings: Vec::new(),
                root_cause: None,
                impact: None,
                initiated_by: String::new(),
                started_at: now,
                completed_at: None,
                duration_seconds: None,
            },
            report_content: "## 调查超时\n\n偏差调查超时，请重试或手动处理。\n".to_string(),
            report_metadata: DeviationReportMetadata {
                report_id: uuid::Uuid::new_v4().to_string(),
                title: "Timeout Report".to_string(),
                generated_at: now,
                investigation_id: String::new(),
                event_id: String::new(),
                event_type: "unknown".to_string(),
                severity: "unknown".to_string(),
                format: ReportFormat::Markdown,
                confidence: 0.0,
                generation_time_ms: 0,
            },
            corrective_actions: Vec::new(),
            model: None,
            processing_time_ms: 0,
            timestamp: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deviation_severity_as_str() {
        assert_eq!(DeviationSeverity::Critical.as_str(), "critical");
        assert_eq!(DeviationSeverity::Major.as_str(), "major");
        assert_eq!(DeviationSeverity::Minor.as_str(), "minor");
        assert_eq!(DeviationSeverity::Advisory.as_str(), "advisory");
    }

    #[test]
    fn test_deviation_severity_from_str() {
        assert_eq!(
            DeviationSeverity::from_str("critical"),
            DeviationSeverity::Critical
        );
        assert_eq!(
            DeviationSeverity::from_str("Major"),
            DeviationSeverity::Major
        );
        assert_eq!(
            DeviationSeverity::from_str("MINOR"),
            DeviationSeverity::Minor
        );
        assert_eq!(
            DeviationSeverity::from_str("unknown"),
            DeviationSeverity::Advisory
        );
    }

    #[test]
    fn test_deviation_event_type_as_str() {
        assert_eq!(DeviationEventType::Temperature.as_str(), "temperature");
        assert_eq!(
            DeviationEventType::Other("custom".to_string()).as_str(),
            "custom"
        );
    }

    #[test]
    fn test_deviation_event_new() {
        let event = DeviationEvent::new(
            "DEV-001",
            DeviationEventType::Temperature,
            "温度超出范围",
            DeviationSeverity::Major,
            "操作员A",
        );
        assert_eq!(event.event_id, "DEV-001");
        assert_eq!(event.description, "温度超出范围");
        assert_eq!(event.severity, DeviationSeverity::Major);
        assert_eq!(event.source, "操作员A");
        assert!(event.product_name.is_none());
    }

    #[test]
    fn test_deviation_event_with_batch() {
        let event = DeviationEvent::new(
            "DEV-002",
            DeviationEventType::Time,
            "时间超限",
            DeviationSeverity::Minor,
            "系统",
        )
        .with_batch("产品X", "BATCH-001");
        assert_eq!(event.product_name.unwrap(), "产品X");
        assert_eq!(event.batch_id.unwrap(), "BATCH-001");
    }

    #[test]
    fn test_deviation_event_with_equipment() {
        let event = DeviationEvent::new(
            "DEV-003",
            DeviationEventType::Equipment,
            "设备故障",
            DeviationSeverity::Critical,
            "设备监控",
        )
        .with_equipment("EQ-001");
        assert_eq!(event.equipment_id.unwrap(), "EQ-001");
    }

    #[test]
    fn test_investigation_new() {
        let event = DeviationEvent::new(
            "DEV-001",
            DeviationEventType::Temperature,
            "温度偏差",
            DeviationSeverity::Major,
            "操作员",
        );
        let inv = Investigation::new("INV-001", &event, "张三");
        assert_eq!(inv.investigation_id, "INV-001");
        assert_eq!(inv.event_id, "DEV-001");
        assert_eq!(inv.status, InvestigationStatus::Pending);
        assert_eq!(inv.initiated_by, "张三");
    }

    #[test]
    fn test_investigation_complete() {
        let event = DeviationEvent::new(
            "DEV-001",
            DeviationEventType::Temperature,
            "温度偏差",
            DeviationSeverity::Major,
            "操作员",
        );
        let mut inv = Investigation::new("INV-001", &event, "张三");
        assert_eq!(inv.status, InvestigationStatus::Pending);
        inv.complete();
        assert_eq!(inv.status, InvestigationStatus::Completed);
        assert!(inv.completed_at.is_some());
    }

    #[test]
    fn test_investigation_status_as_str() {
        assert_eq!(InvestigationStatus::Pending.as_str(), "pending");
        assert_eq!(InvestigationStatus::InProgress.as_str(), "in_progress");
        assert_eq!(InvestigationStatus::Completed.as_str(), "completed");
        assert_eq!(InvestigationStatus::Closed.as_str(), "closed");
    }

    #[test]
    fn test_corrective_action_type_as_str() {
        assert_eq!(CorrectiveActionType::Immediate.as_str(), "immediate");
        assert_eq!(CorrectiveActionType::Preventive.as_str(), "preventive");
        assert_eq!(CorrectiveActionType::LongTerm.as_str(), "long_term");
    }

    #[test]
    fn test_corrective_priority_as_str() {
        assert_eq!(CorrectivePriority::Critical.as_str(), "critical");
        assert_eq!(CorrectivePriority::High.as_str(), "high");
        assert_eq!(CorrectivePriority::Medium.as_str(), "medium");
        assert_eq!(CorrectivePriority::Low.as_str(), "low");
    }

    #[test]
    fn test_corrective_action_status_as_str() {
        assert_eq!(CorrectiveActionStatus::Proposed.as_str(), "proposed");
        assert_eq!(CorrectiveActionStatus::Approved.as_str(), "approved");
        assert_eq!(CorrectiveActionStatus::InProgress.as_str(), "in_progress");
        assert_eq!(CorrectiveActionStatus::Completed.as_str(), "completed");
        assert_eq!(CorrectiveActionStatus::Verified.as_str(), "verified");
        assert_eq!(CorrectiveActionStatus::Closed.as_str(), "closed");
    }

    #[test]
    fn test_investigation_result_error() {
        let result = InvestigationResult::error("LLM 调用失败");
        assert!(result.report_content.contains("调查失败"));
        assert!(result.report_content.contains("LLM 调用失败"));
        assert_eq!(result.report_metadata.confidence, 0.0);
    }

    #[test]
    fn test_investigation_result_timeout() {
        let result = InvestigationResult::timeout();
        assert!(result.report_content.contains("调查超时"));
        assert_eq!(result.report_metadata.confidence, 0.0);
    }

    #[test]
    fn test_finding_creation() {
        let finding = Finding {
            id: "F-001".to_string(),
            description: "温度传感器校准过期".to_string(),
            evidence: vec!["维护记录2024-03-01".to_string()],
            related_rules: vec!["GMP-001".to_string()],
            is_root_cause: true,
        };
        assert_eq!(finding.id, "F-001");
        assert!(finding.is_root_cause);
    }

    #[test]
    fn test_impact_assessment() {
        let impact = ImpactAssessment {
            product_quality: "可能影响产品稳定性".to_string(),
            patient_safety: Some("低风险".to_string()),
            compliance_impact: "违反GMP第X条".to_string(),
            affected_batches: vec!["BATCH-001".to_string()],
            severity_score: 7,
            summary: "需要立即采取措施".to_string(),
        };
        assert_eq!(impact.severity_score, 7);
        assert!(impact.patient_safety.is_some());
    }

    #[test]
    fn test_corrective_action_creation() {
        let action = CorrectiveAction {
            id: "CA-001".to_string(),
            description: "重新校准温度传感器".to_string(),
            action_type: CorrectiveActionType::Immediate,
            priority: CorrectivePriority::High,
            owner: Some("维护团队".to_string()),
            deadline: None,
            status: CorrectiveActionStatus::Proposed,
            effectiveness_check: Some("校准后验证温度精度在±0.5°C范围内".to_string()),
            expected_outcome: Some("温度恢复正常范围".to_string()),
        };
        assert_eq!(action.id, "CA-001");
        assert_eq!(action.priority, CorrectivePriority::High);
        assert_eq!(action.status, CorrectiveActionStatus::Proposed);
    }

    #[test]
    fn test_report_format_as_str() {
        assert_eq!(ReportFormat::Markdown.as_str(), "markdown");
        assert_eq!(ReportFormat::Json.as_str(), "json");
        assert_eq!(ReportFormat::Pdf.as_str(), "pdf");
    }

    #[test]
    fn test_deviation_severity_equality() {
        assert_eq!(DeviationSeverity::Critical, DeviationSeverity::Critical);
        assert_ne!(DeviationSeverity::Critical, DeviationSeverity::Major);
    }
}
