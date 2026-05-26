//! LLM-powered Compliance Judge Agent
//!
//! Provides real-time compliance judgment using local LLM (Ollama).
//! Delivers judgments within 500ms latency requirement.

pub mod agent;
pub mod audit;
pub mod confidence;
pub mod parser;
pub mod types;

pub use agent::ComplianceJudge;
pub use audit::{AuditLogEntry, AuditLogWriter, JudgmentAuditor};
pub use confidence::ConfidenceCalculator;
pub use parser::RuleParser;
pub use types::{JudgmentRequest, JudgmentResult, JudgmentStatus};
