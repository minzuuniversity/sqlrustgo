//! Four-channel retrieval: Rule, Vector, FTS, Graph

use gmp_core::ScoredChunk;

/// Rule-based channel matcher
pub struct RuleChannel {
    rules: Vec<Rule>,
}

#[derive(Debug, Clone)]
struct Rule {
    id: String,
    pattern: String,
    #[allow(dead_code)]
    category: String,
    content: String,
    #[allow(dead_code)]
    weight: f32,
}

impl RuleChannel {
    /// Create a new rule channel with rules from YAML
    pub fn new(rules: Vec<(String, String, String)>) -> Self {
        let rules = rules
            .into_iter()
            .enumerate()
            .map(|(i, (pattern, category, content))| Rule {
                id: format!("rule-{:03}", i),
                pattern,
                category,
                content,
                weight: 2.0,
            })
            .collect();
        Self { rules }
    }

    /// Search using rule matching
    #[allow(dead_code)]
    pub fn search(&self, query: &str, top_k: usize) -> Vec<ScoredChunk> {
        let query_lower = query.to_lowercase();
        let mut scored: Vec<(f32, ScoredChunk)> = self
            .rules
            .iter()
            .filter(|r| r.pattern.to_lowercase().contains(&query_lower))
            .map(|r| {
                let score = if r.pattern.to_lowercase() == query_lower {
                    1.0
                } else {
                    0.5
                };
                (
                    score,
                    ScoredChunk {
                        chunk_id: 0,
                        doc_id: r.id.clone(),
                        content: r.content.clone(),
                        score,
                        source: gmp_core::ChannelType::Rule,
                    },
                )
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.into_iter().take(top_k).map(|(_, c)| c).collect()
    }
}

/// Vector channel using embeddings + HNSW
pub struct VectorChannel {
    #[allow(dead_code)]
    ollama_url: String,
    #[allow(dead_code)]
    embedding_model: String,
}

impl VectorChannel {
    pub fn new(ollama_url: String, embedding_model: String) -> Self {
        Self {
            ollama_url,
            embedding_model,
        }
    }

    /// Search using vector similarity
    #[allow(dead_code)]
    pub async fn search(&self, query: &str, top_k: usize) -> Vec<ScoredChunk> {
        let _ = (query, top_k);
        // Placeholder - will be implemented with actual Ollama embeddings
        vec![]
    }
}

impl Default for VectorChannel {
    fn default() -> Self {
        Self::new("http://localhost:11434".to_string(), "bge-m3".to_string())
    }
}

/// FTS channel using BM25
pub struct FtsChannel;

impl FtsChannel {
    pub fn new() -> Self {
        Self
    }

    /// Search using BM25 full-text search
    #[allow(dead_code)]
    pub fn search(&self, query: &str, top_k: usize) -> Vec<ScoredChunk> {
        let _ = (query, top_k);
        // Placeholder - will be implemented with actual BM25
        vec![]
    }
}

impl Default for FtsChannel {
    fn default() -> Self {
        Self::new()
    }
}

/// Graph channel using Cypher queries
pub struct GraphChannel;

impl GraphChannel {
    pub fn new() -> Self {
        Self
    }

    /// Search using graph traversal
    #[allow(dead_code)]
    pub fn search(&self, query: &str, top_k: usize) -> Vec<ScoredChunk> {
        let _ = (query, top_k);
        // Placeholder - will be implemented with actual Cypher
        vec![]
    }
}

impl Default for GraphChannel {
    fn default() -> Self {
        Self::new()
    }
}
