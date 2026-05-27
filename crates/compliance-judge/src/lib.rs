//! # Compliance Judge Agent
//!
//! LLM-powered real-time compliance judgment engine for GMP Platform.
//!
//! ## Architecture
//!
//! - [`ollama`] - Ollama LLM integration
//! - [`rules`] - Compliance rule parsing and validation
//! - [`confidence`] - Confidence scoring based on logprob analysis
//! - [`agent`] - Core compliance judge agent
//! - [`audit`] - Audit log with WAL persistence and hash chain verification

pub mod agent;
pub mod audit;
pub mod confidence;
pub mod ollama;
pub mod rules;

pub use agent::{ComplianceJudge, JudgmentRequest, JudgmentResult, JudgmentStatus};
pub use audit::{AuditLogEntry, AuditLogReader, AuditLogWriter};
pub use confidence::ConfidenceScore;
pub use ollama::{OllamaClient, OllamaClientImpl};
pub use rules::{Rule, RuleParser, RuleValidator};
