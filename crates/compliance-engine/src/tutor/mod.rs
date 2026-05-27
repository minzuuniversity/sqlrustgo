//! AI 合规培训助手（Tutor Agent）
//!
//! 新员工 AI 培训助手，GMP 规则问答系统。
//! 内置 GMP 法规知识库，支持自然语言问答和培训记录追踪。
//!
//! ## 模块结构
//! - `types` — 数据类型 (Regulation, KnowledgeEntry, TutorQuery, TutorResponse, TrainingRecord)
//! - `knowledge` — GMP 法规知识库 (21 CFR Part 210/211、EU GMP Annex 11 等核心法规)
//! - `agent` — 问答 Agent 主循环 (查询→知识检索→LLM推理→带引用回答)
//! - `tracker` — 培训记录追踪 (谁、何时、学了什么、正确率)

pub mod agent;
pub mod knowledge;
pub mod tracker;
pub mod types;

pub use agent::{LlmClient, TutorAgent, TutorAgentConfig, TutorAgentError};
pub use knowledge::{KnowledgeRetriever, KnowledgeStats};
pub use tracker::{TraineeStats, TrainingTracker};
pub use types::{
    KnowledgeEntry, Regulation, TrainingRecord, TrainingStatus, TutorQuery, TutorResponse,
};
