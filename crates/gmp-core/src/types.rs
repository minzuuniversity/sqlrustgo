//! GMP Core types

use serde::{Deserialize, Serialize};

/// A GMP document with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmpDocument {
    pub id: i64,
    pub doc_id: String,
    pub title: String,
    pub chapter: Option<String>,
    pub doc_type: Option<String>,
    pub department: Option<String>,
    pub status: String,
    pub source_file: String,
    pub created_at: Option<i64>,
    pub updated_at: Option<i64>,
}

/// A chunk of document content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmpChunk {
    pub id: i64,
    pub doc_id: String,
    pub chunk_index: i32,
    pub content: String,
    pub token_count: i32,
}

/// Query intent classification
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QueryIntent {
    /// Retrieve SOP document
    RetrieveSop,
    /// Check compliance (yes/no)
    CheckCompliance,
    /// Compare regulations across standards
    CompareRegulation,
    /// Trace process flow
    TraceProcess,
    /// Classify incident type (deviation/change/OOS)
    ClassifyIncident,
    /// Generate checklist
    GenerateChecklist,
    /// Analyze root cause
    AnalyzeRootCause,
}

impl QueryIntent {
    pub fn from_query(query: &str) -> Self {
        let q = query.to_lowercase();
        if q.contains("sop") || q.contains("规程") || q.contains("程序文件") {
            Self::RetrieveSop
        } else if q.contains("合规") || q.contains("是否") || q.contains("应该") {
            Self::CheckCompliance
        } else if q.contains("比较") || q.contains("对比") || q.contains("差异") {
            Self::CompareRegulation
        } else if q.contains("流程") || q.contains("步骤") || q.contains("先后") {
            Self::TraceProcess
        } else if q.contains("偏差")
            || q.contains("变更")
            || q.contains("oos")
            || q.contains("超标")
        {
            Self::ClassifyIncident
        } else if q.contains("清单") || q.contains("检查表") || q.contains(" checklist") {
            Self::GenerateChecklist
        } else if q.contains("原因") || q.contains("根因") || q.contains("分析") {
            Self::AnalyzeRootCause
        } else {
            Self::RetrieveSop
        }
    }
}

/// Scored document for retrieval results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredChunk {
    pub chunk_id: i64,
    pub doc_id: String,
    pub content: String,
    pub score: f32,
    pub source: ChannelType,
}

/// Channel type for retrieval
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChannelType {
    Rule,
    Vector,
    Fts,
    Graph,
}

impl std::fmt::Display for ChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChannelType::Rule => write!(f, "rule"),
            ChannelType::Vector => write!(f, "vector"),
            ChannelType::Fts => write!(f, "fts"),
            ChannelType::Graph => write!(f, "graph"),
        }
    }
}

/// Configuration for GMP retrieval system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmpConfig {
    pub ollama_url: String,
    pub ollama_model: String,
    pub embedding_model: String,
    pub reranker_model: String,
    pub llm_model: String,
    pub data_dir: String,
    pub hnsw_m: i32,
    pub hnsw_ef_construction: i32,
    pub rrf_k: f32,
}

impl Default for GmpConfig {
    fn default() -> Self {
        Self {
            ollama_url: "http://localhost:11434".to_string(),
            ollama_model: "qwen2.5:14b".to_string(),
            embedding_model: "bge-m3".to_string(),
            reranker_model: "bge-reranker-v2-m3".to_string(),
            llm_model: "qwen2.5:14b".to_string(),
            data_dir: "./data".to_string(),
            hnsw_m: 16,
            hnsw_ef_construction: 200,
            rrf_k: 60.0,
        }
    }
}
