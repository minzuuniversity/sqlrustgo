//! Deviation Investigator Agent - AI 偏差调查助手
//!
//! 接收 GMP 偏差事件，自动启动调查、生成偏差报告并提供纠正措施建议。
//! 与 GMP Retrieval v3 集成，通过模式匹配和规则引擎完成调查。

pub mod agent;
pub mod corrective;
pub mod report;
pub mod types;

pub use agent::DeviationInvestigator;
pub use agent::DeviationInvestigatorConfig;
pub use agent::DeviationInvestigatorError;
pub use corrective::CorrectiveActionRecommender;
pub use report::DeviationReportGenerator;
pub use types::{
    CorrectiveAction, CorrectiveActionStatus, CorrectiveActionType, CorrectivePriority,
    DeviationEvent, DeviationEventType, DeviationReportMetadata, DeviationSeverity, Finding,
    GmpRetrievalV3, ImpactAssessment, Investigation, InvestigationResult, InvestigationStatus,
    ReportFormat, RetrievedDocument,
};
