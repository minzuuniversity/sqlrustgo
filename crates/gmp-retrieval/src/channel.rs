//! Four-channel retrieval: Rule, Vector, FTS, Graph

use crate::fusion::RrfFusion;
use crate::embedding_cache::CachedEmbeddingStore;
use anyhow::Result;
use gmp_core::{ChannelType, ScoredChunk};
use serde::{Deserialize, Serialize};

/// Agent-knowledge rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeRule {
    pub id: String,
    pub category: String,
    pub subcategory: String,
    pub description: String,
    pub requirement: String,
    pub keywords: Vec<String>,
}

/// Rule-based channel matcher
#[derive(Clone)]
pub struct RuleChannel {
    rules: Vec<KnowledgeRule>,
}

impl RuleChannel {
    /// Create from agent-knowledge YAML files
    pub fn from_yaml_files(agent_knowledge_dir: &str) -> Result<Self> {
        let mut rules = Vec::new();
        let path = std::path::Path::new(agent_knowledge_dir);

        if path.exists() {
            for entry in walkdir::WalkDir::new(path)
                .into_iter()
                .filter_map(Result::ok)
                .filter(|e| {
                    e.path()
                        .extension()
                        .is_some_and(|ext| ext == "yaml" || ext == "yml")
                })
            {
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    if let Ok(rule) = serde_yaml::from_str::<KnowledgeRule>(&content) {
                        rules.push(rule);
                    }
                }
            }
        }

        // Add built-in rules if none loaded
        if rules.is_empty() {
            rules = get_builtin_rules();
        }

        Ok(Self { rules })
    }

    /// Create with predefined rules
    pub fn new(rules: Vec<KnowledgeRule>) -> Self {
        Self { rules }
    }

    /// Search using rule matching
    pub fn search(&self, query: &str, top_k: usize) -> Vec<ScoredChunk> {
        let query_lower = query.to_lowercase();
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();

        let mut scored: Vec<(f32, ScoredChunk)> = self
            .rules
            .iter()
            .filter_map(|r| {
                let mut match_count = 0;

                // Check keywords
                for term in &query_terms {
                    for kw in &r.keywords {
                        if kw.to_lowercase().contains(term) || term.contains(&kw.to_lowercase()) {
                            match_count += 1;
                        }
                    }
                }

                // Check description
                if r.description.to_lowercase().contains(&query_lower)
                    || r.requirement.to_lowercase().contains(&query_lower)
                {
                    match_count += 2;
                }

                if match_count > 0 {
                    let score = match_count as f32 * 0.5;
                    Some((
                        score,
                        ScoredChunk {
                            chunk_id: 0,
                            doc_id: format!("rule-{}", r.id),
                            content: format!(
                                "【{}】{}\n\n要求: {}",
                                r.category, r.description, r.requirement
                            ),
                            score,
                            source: ChannelType::Rule,
                        },
                    ))
                } else {
                    None
                }
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.into_iter().take(top_k).map(|(_, c)| c).collect()
    }

    /// Get rule count
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }
}

/// Built-in GMP rules
fn get_builtin_rules() -> Vec<KnowledgeRule> {
    vec![
        KnowledgeRule {
            id: "DEV-001".into(),
            category: "偏差管理".into(),
            subcategory: "偏差分级".into(),
            description: "偏差分级标准".into(),
            requirement: "重大偏差应启动CAPA，一般偏差需记录和评审，微小偏差可记录可不评审".into(),
            keywords: vec![
                "偏差".into(),
                "分级".into(),
                "重大".into(),
                "一般".into(),
                "微小".into(),
            ],
        },
        KnowledgeRule {
            id: "DEV-002".into(),
            category: "偏差管理".into(),
            subcategory: "偏差处理".into(),
            description: "偏差处理时限要求".into(),
            requirement: "重大偏差应在24小时内通知质量部门，偏差调查报告应在30天内完成".into(),
            keywords: vec!["偏差".into(), "时限".into(), "24小时".into(), "30天".into()],
        },
        KnowledgeRule {
            id: "CAPA-001".into(),
            category: "CAPA管理".into(),
            subcategory: "CAPA触发".into(),
            description: "CAPA触发条件".into(),
            requirement: "偏差、投诉、审计发现、内审不符合项均应触发CAPA评估".into(),
            keywords: vec![
                "CAPA".into(),
                "触发".into(),
                "偏差".into(),
                "投诉".into(),
                "审计".into(),
            ],
        },
        KnowledgeRule {
            id: "SOP-001".into(),
            category: "文件管理".into(),
            subcategory: "SOP编写".into(),
            description: "SOP编写要求".into(),
            requirement: "SOP应包含目的、范围、职责、程序、记录、附录等要素，经审批后生效".into(),
            keywords: vec!["SOP".into(), "编写".into(), "规程".into()],
        },
        KnowledgeRule {
            id: "VAL-001".into(),
            category: "验证与确认".into(),
            subcategory: "设备确认".into(),
            description: "设备确认阶段".into(),
            requirement: "设备确认包括IQ（安装确认）、OQ（运行确认）、PQ（性能确认）三个阶段"
                .into(),
            keywords: vec![
                "验证".into(),
                "确认".into(),
                "IQ".into(),
                "OQ".into(),
                "PQ".into(),
                "设备".into(),
            ],
        },
    ]
}

/// Vector channel using embeddings + SQLite cosine similarity
pub struct VectorChannel {
    storage: std::sync::Arc<dyn gmp_storage::GmpStorage>,
    embedding_store: CachedEmbeddingStore,
}

impl VectorChannel {
    pub fn new(
        storage: std::sync::Arc<dyn gmp_storage::GmpStorage>,
        ollama_url: String,
        embedding_model: String,
    ) -> Self {
        Self {
            storage,
            embedding_store: CachedEmbeddingStore::new(ollama_url, embedding_model),
        }
    }

    /// Search using vector similarity (requires Ollama)
    pub async fn search(&self, query: &str, top_k: usize) -> Vec<ScoredChunk> {
        // Get query embedding from Ollama (with LRU cache)
        let embedding = match self.embedding_store.embed(query).await {
            Ok(e) => e,
            Err(_) => return vec![],
        };

        // Search vectors in storage
        let storage = self.storage.clone();
        match tokio::task::spawn_blocking(move || storage.search_vectors(&embedding, top_k)).await {
            Ok(Ok(results)) => results
                .into_iter()
                .map(|r| ScoredChunk {
                    chunk_id: r.chunk_id,
                    doc_id: r.doc_id,
                    content: r.content,
                    score: r.score as f32,
                    source: gmp_core::ChannelType::Vector,
                })
                .collect(),
            _ => vec![],
        }
    }
}

impl Default for VectorChannel {
    fn default() -> Self {
        unimplemented!("VectorChannel requires StorageEngine, use VectorChannel::new")
    }
}

/// FTS channel using BM25
#[derive(Clone)]
pub struct FtsChannel {
    storage: std::sync::Arc<dyn gmp_storage::GmpStorage>,
}

impl FtsChannel {
    pub fn new(storage: std::sync::Arc<dyn gmp_storage::GmpStorage>) -> Self {
        Self { storage }
    }

    /// Search using BM25 full-text search (now with real BM25 scores)
    pub fn search(&self, query: &str, top_k: usize) -> Vec<ScoredChunk> {
        match self.storage.search_fts_with_scores(query, top_k) {
            Ok(results) => results
                .into_iter()
                .map(|r| ScoredChunk {
                    chunk_id: r.chunk_id,
                    doc_id: r.doc_id,
                    content: r.content,
                    score: r.score as f32,
                    source: ChannelType::Fts,
                })
                .collect(),
            Err(_) => vec![],
        }
    }
}

/// Graph channel using Cypher queries (simplified)
#[derive(Clone)]
pub struct GraphChannel {
    rules: Vec<KnowledgeRule>,
}

impl GraphChannel {
    pub fn new(rules: Vec<KnowledgeRule>) -> Self {
        Self { rules }
    }

    /// Search using graph traversal
    pub fn search(&self, query: &str, top_k: usize) -> Vec<ScoredChunk> {
        let query_lower = query.to_lowercase();

        // Simple entity-based search
        let entities = extract_entities(query);

        let mut results = Vec::new();
        for rule in &self.rules {
            let mut score = 0.0;

            // Check category match
            if rule.category.to_lowercase().contains(&query_lower) {
                score += 2.0;
            }

            // Check entity matches
            for entity in &entities {
                if rule
                    .keywords
                    .iter()
                    .any(|kw| kw.to_lowercase().contains(entity))
                    || rule.description.to_lowercase().contains(entity)
                {
                    score += 1.0;
                }
            }

            if score > 0.0 {
                results.push(ScoredChunk {
                    chunk_id: 0,
                    doc_id: format!("graph-{}", rule.id),
                    content: format!(
                        "【{}】{}\n\n要求: {}",
                        rule.category, rule.description, rule.requirement
                    ),
                    score,
                    source: ChannelType::Graph,
                });
            }
        }

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.into_iter().take(top_k).collect()
    }
}

/// Extract entities from query
fn extract_entities(query: &str) -> Vec<String> {
    let mut entities = Vec::new();
    let keywords = [
        "偏差", "CAPA", "SOP", "变更", "验证", "设备", "工艺", "清洁", "偏差", "投诉", "审计",
    ];

    for kw in &keywords {
        if query.contains(kw) {
            entities.push(kw.to_string());
        }
    }

    entities
}

/// Full four-channel retrieval engine
pub struct FourChannelEngine {
    pub rule: RuleChannel,
    pub vector: VectorChannel,
    pub fts: FtsChannel,
    pub graph: GraphChannel,
    pub fusion: RrfFusion,
}

impl FourChannelEngine {
    pub fn new(
        storage: std::sync::Arc<dyn gmp_storage::GmpStorage>,
        agent_knowledge_dir: Option<&str>,
    ) -> Self {
        let rules = if let Some(dir) = agent_knowledge_dir {
            RuleChannel::from_yaml_files(dir)
                .map(|c| c.rules)
                .unwrap_or_else(|_| get_builtin_rules())
        } else {
            get_builtin_rules()
        };

        let storage_for_fts = storage.clone();
        let storage_for_vector = storage;
        Self {
            rule: RuleChannel::new(rules.clone()),
            vector: VectorChannel::new(
                storage_for_vector,
                "http://localhost:11434".to_string(),
                "bge-m3:latest".to_string(),
            ),
            fts: FtsChannel::new(storage_for_fts),
            graph: GraphChannel::new(rules),
            fusion: RrfFusion::default(),
        }
    }

    /// Search all channels in parallel and fuse results
    pub async fn search(&self, query: &str, top_k: usize) -> Vec<ScoredChunk> {
        let query_str = query.to_string();

        // Run all four channels in parallel using tokio::join!
        // Vector channel is async; Rule/FTS/Graph are sync → use spawn_blocking
        let (rule_res, vector_res, fts_res, graph_res) = tokio::join!(
            tokio::task::spawn_blocking({
                let rule = self.rule.clone();
                let q = query_str.clone();
                move || rule.search(&q, 10)
            }),
            self.vector.search(&query_str, 30),
            tokio::task::spawn_blocking({
                let fts = self.fts.clone();
                let q = query_str.clone();
                move || fts.search(&q, 30)
            }),
            tokio::task::spawn_blocking({
                let graph = self.graph.clone();
                let q = query_str.clone();
                move || graph.search(&q, 10)
            }),
        );

        let rule_results = rule_res.unwrap_or_default();
        let vector_results = vector_res;
        let fts_results = fts_res.unwrap_or_default();
        let graph_results = graph_res.unwrap_or_default();

        // RRF fusion with weights: Rule=2.0, others=1.0
        let channel_results = vec![
            (rule_results, 2.0),
            (vector_results, 1.0),
            (fts_results, 1.0),
            (graph_results, 1.0),
        ];

        let fused = self.fusion.fuse(channel_results);
        fused.into_iter().take(top_k).collect()
    }
}
