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

#[cfg(test)]
mod tests {
    use super::*;

    // =====================================================================
    // GmpDocument tests
    // =====================================================================

    #[test]
    fn test_gmp_document_creation() {
        let doc = GmpDocument {
            id: 1,
            doc_id: "GMP-001".to_string(),
            title: "偏差处理规程".to_string(),
            chapter: Some("第5章".to_string()),
            doc_type: Some("SOP".to_string()),
            department: Some("质量部".to_string()),
            status: "有效".to_string(),
            source_file: "/data/gmp-md/chapter5.md".to_string(),
            created_at: Some(1700000000),
            updated_at: Some(1700000000),
        };
        assert_eq!(doc.id, 1);
        assert_eq!(doc.doc_id, "GMP-001");
        assert_eq!(doc.title, "偏差处理规程");
        assert_eq!(doc.status, "有效");
    }

    #[test]
    fn test_gmp_document_clone() {
        let doc = GmpDocument {
            id: 1,
            doc_id: "GMP-001".to_string(),
            title: "测试文档".to_string(),
            chapter: None,
            doc_type: None,
            department: None,
            status: "有效".to_string(),
            source_file: "test.md".to_string(),
            created_at: None,
            updated_at: None,
        };
        let cloned = doc.clone();
        assert_eq!(cloned.id, doc.id);
        assert_eq!(cloned.doc_id, doc.doc_id);
    }

    // =====================================================================
    // GmpChunk tests
    // =====================================================================

    #[test]
    fn test_gmp_chunk_creation() {
        let chunk = GmpChunk {
            id: 1,
            doc_id: "GMP-001".to_string(),
            chunk_index: 0,
            content: "这是偏差处理规程的正文内容。".to_string(),
            token_count: 50,
        };
        assert_eq!(chunk.id, 1);
        assert_eq!(chunk.chunk_index, 0);
        assert!(chunk.content.contains("偏差"));
    }

    // =====================================================================
    // QueryIntent tests
    // =====================================================================

    #[test]
    fn test_query_intent_retrieve_sop() {
        let intent = QueryIntent::from_query("查找偏差处理SOP");
        assert_eq!(intent, QueryIntent::RetrieveSop);
    }

    #[test]
    fn test_query_intent_retrieve_sop_chinese() {
        let intent = QueryIntent::from_query("查找规程");
        assert_eq!(intent, QueryIntent::RetrieveSop);
    }

    #[test]
    fn test_query_intent_check_compliance() {
        let intent = QueryIntent::from_query("这样做是否合规");
        assert_eq!(intent, QueryIntent::CheckCompliance);
    }

    #[test]
    fn test_query_intent_compare_regulation() {
        let intent = QueryIntent::from_query("比较中国GMP和EU GMP的差异");
        assert_eq!(intent, QueryIntent::CompareRegulation);
    }

    #[test]
    fn test_query_intent_trace_process() {
        let intent = QueryIntent::from_query("偏差处理的流程是什么");
        assert_eq!(intent, QueryIntent::TraceProcess);
    }

    #[test]
    fn test_query_intent_classify_incident_deviation() {
        let intent = QueryIntent::from_query("偏差如何分级");
        assert_eq!(intent, QueryIntent::ClassifyIncident);
    }

    #[test]
    fn test_query_intent_classify_incident_change() {
        let intent = QueryIntent::from_query("变更控制的类型");
        assert_eq!(intent, QueryIntent::ClassifyIncident);
    }

    #[test]
    fn test_query_intent_generate_checklist() {
        // "清单" in query takes precedence (no "偏差" keyword)
        let intent = QueryIntent::from_query("生产设备检查清单");
        assert_eq!(intent, QueryIntent::GenerateChecklist);
    }

    #[test]
    fn test_query_intent_analyze_root_cause() {
        let intent = QueryIntent::from_query("根本原因分析");
        assert_eq!(intent, QueryIntent::AnalyzeRootCause);
    }

    #[test]
    fn test_query_intent_default() {
        // Unknown query falls back to RetrieveSop
        let intent = QueryIntent::from_query("GMP原料药管理要求");
        assert_eq!(intent, QueryIntent::RetrieveSop);
    }

    // =====================================================================
    // ScoredChunk tests
    // =====================================================================

    #[test]
    fn test_scored_chunk_creation() {
        let chunk = ScoredChunk {
            chunk_id: 1,
            doc_id: "GMP-001".to_string(),
            content: "偏差处理规程内容".to_string(),
            score: 0.95,
            source: ChannelType::Vector,
        };
        assert_eq!(chunk.score, 0.95);
        assert_eq!(chunk.source, ChannelType::Vector);
    }

    #[test]
    fn test_scored_chunk_clone() {
        let chunk = ScoredChunk {
            chunk_id: 1,
            doc_id: "GMP-001".to_string(),
            content: "测试内容".to_string(),
            score: 0.8,
            source: ChannelType::Fts,
        };
        let cloned = chunk.clone();
        assert_eq!(cloned.score, chunk.score);
        assert_eq!(cloned.source, chunk.source);
    }

    #[test]
    fn test_scored_chunk_ordering() {
        let mut chunks = [
            ScoredChunk {
                chunk_id: 1,
                doc_id: "A".into(),
                content: "".into(),
                score: 0.5,
                source: ChannelType::Rule,
            },
            ScoredChunk {
                chunk_id: 2,
                doc_id: "B".into(),
                content: "".into(),
                score: 0.9,
                source: ChannelType::Rule,
            },
            ScoredChunk {
                chunk_id: 3,
                doc_id: "C".into(),
                content: "".into(),
                score: 0.7,
                source: ChannelType::Rule,
            },
        ];
        chunks.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        assert_eq!(chunks[0].doc_id, "B");
        assert_eq!(chunks[1].doc_id, "C");
        assert_eq!(chunks[2].doc_id, "A");
    }

    // =====================================================================
    // ChannelType tests
    // =====================================================================

    #[test]
    fn test_channel_type_all_variants() {
        assert_eq!(format!("{}", ChannelType::Rule), "rule");
        assert_eq!(format!("{}", ChannelType::Vector), "vector");
        assert_eq!(format!("{}", ChannelType::Fts), "fts");
        assert_eq!(format!("{}", ChannelType::Graph), "graph");
    }

    #[test]
    fn test_channel_type_equality() {
        assert_eq!(ChannelType::Rule, ChannelType::Rule);
        assert_ne!(ChannelType::Rule, ChannelType::Vector);
        assert_ne!(ChannelType::Fts, ChannelType::Graph);
    }

    #[test]
    fn test_channel_type_copy() {
        let ct = ChannelType::Vector;
        let ct2 = ct;
        assert_eq!(ct, ct2);
    }

    // =====================================================================
    // GmpConfig tests
    // =====================================================================

    #[test]
    fn test_gmp_config_default() {
        let config = GmpConfig::default();
        assert_eq!(config.ollama_url, "http://localhost:11434");
        assert_eq!(config.ollama_model, "qwen2.5:14b");
        assert_eq!(config.embedding_model, "bge-m3");
        assert_eq!(config.reranker_model, "bge-reranker-v2-m3");
        assert_eq!(config.rrf_k, 60.0);
        assert_eq!(config.hnsw_m, 16);
    }

    #[test]
    fn test_gmp_config_custom() {
        let config = GmpConfig {
            ollama_url: "http://custom:11434".to_string(),
            ollama_model: "custom-model".to_string(),
            embedding_model: "custom-embedding".to_string(),
            reranker_model: "custom-reranker".to_string(),
            llm_model: "custom-llm".to_string(),
            data_dir: "/tmp/test".to_string(),
            hnsw_m: 8,
            hnsw_ef_construction: 100,
            rrf_k: 30.0,
        };
        assert_eq!(config.ollama_url, "http://custom:11434");
        assert_eq!(config.rrf_k, 30.0);
        assert_eq!(config.hnsw_m, 8);
    }

    // =====================================================================
    // Serialization tests
    // =====================================================================

    #[test]
    fn test_channel_type_serde() {
        let ct = ChannelType::Vector;
        let encoded = serde_json::to_string(&ct).unwrap();
        let decoded: ChannelType = serde_json::from_str(&encoded).unwrap();
        assert_eq!(ct, decoded);
    }

    #[test]
    fn test_scored_chunk_serde() {
        let chunk = ScoredChunk {
            chunk_id: 42,
            doc_id: "TEST-001".into(),
            content: "测试内容".into(),
            score: 0.876,
            source: ChannelType::Rule,
        };
        let encoded = serde_json::to_string(&chunk).unwrap();
        let decoded: ScoredChunk = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.chunk_id, 42);
        assert_eq!(decoded.score, 0.876);
    }

    #[test]
    fn test_query_intent_serde() {
        let intent = QueryIntent::ClassifyIncident;
        let encoded = serde_json::to_string(&intent).unwrap();
        let decoded: QueryIntent = serde_json::from_str(&encoded).unwrap();
        assert_eq!(intent, decoded);
    }

    // =====================================================================
    // Edge cases
    // =====================================================================

    #[test]
    fn test_empty_doc_id() {
        let chunk = ScoredChunk {
            chunk_id: 0,
            doc_id: "".to_string(),
            content: "内容".to_string(),
            score: 0.0,
            source: ChannelType::Fts,
        };
        assert_eq!(chunk.doc_id, "");
        assert_eq!(chunk.score, 0.0);
    }

    #[test]
    fn test_very_long_content() {
        let long_content = "x".repeat(10000);
        let chunk = GmpChunk {
            id: 1,
            doc_id: "TEST".to_string(),
            chunk_index: 0,
            content: long_content.clone(),
            token_count: 5000,
        };
        assert_eq!(chunk.content.len(), 10000);
    }

    #[test]
    fn test_all_query_intents_covered() {
        let test_cases = vec![
            ("SOP文件", QueryIntent::RetrieveSop),
            ("是否合规", QueryIntent::CheckCompliance),
            ("比较差异", QueryIntent::CompareRegulation),
            ("流程步骤", QueryIntent::TraceProcess),
            ("偏差分级", QueryIntent::ClassifyIncident),
            ("调查清单", QueryIntent::GenerateChecklist),
            ("根因分析", QueryIntent::AnalyzeRootCause),
        ];
        for (query, expected) in test_cases {
            assert_eq!(
                QueryIntent::from_query(query),
                expected,
                "Failed for query: {}",
                query
            );
        }
    }
}
