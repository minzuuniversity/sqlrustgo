//! Hybrid search implementation

use crate::types::{Filter, QmdData, SearchResult};
use std::collections::HashMap;

/// RRF (Reciprocal Rank Fusion) configuration
#[derive(Debug, Clone)]
pub struct RrfConfig {
    /// RRF smoothing constant (default 60)
    pub k: f32,
    /// Vector channel weight (default 1.0)
    pub vector_weight: f32,
    /// Graph channel weight (default 1.0)
    pub graph_weight: f32,
    /// Text/FTS channel weight (default 1.0)
    pub text_weight: f32,
    /// Rule channel weight (default 2.0 - rules are higher priority)
    pub rule_weight: f32,
}

impl Default for RrfConfig {
    fn default() -> Self {
        Self {
            k: 60.0,
            vector_weight: 1.0,
            graph_weight: 1.0,
            text_weight: 1.0,
            rule_weight: 2.0,
        }
    }
}

/// Configuration for hybrid search
#[derive(Debug, Clone)]
pub struct HybridSearchConfig {
    /// Weight for vector search results (deprecated - use RRF)
    pub vector_weight: f32,
    /// Weight for graph search results (deprecated - use RRF)
    pub graph_weight: f32,
    /// Weight for text search results (deprecated - use RRF)
    pub text_weight: f32,
    /// Maximum results to return
    pub limit: usize,
    /// Enable reranking
    pub enable_rerank: bool,
    /// RRF configuration (if Some, use RRF instead of weighted sum)
    pub rrf: Option<RrfConfig>,
}

impl Default for HybridSearchConfig {
    fn default() -> Self {
        Self {
            vector_weight: 0.4,
            graph_weight: 0.3,
            text_weight: 0.3,
            limit: 10,
            enable_rerank: true,
            rrf: Some(RrfConfig::default()),
        }
    }
}

/// Hybrid query combining multiple search modalities
#[derive(Debug, Clone)]
pub struct HybridQuery {
    /// Vector for vector search
    pub vector: Option<Vec<f32>>,
    /// Graph pattern for graph search
    pub graph_pattern: Option<String>,
    /// Text query for text search
    pub text_query: Option<String>,
    /// Search limit
    pub limit: usize,
    /// Optional filters
    pub filters: Vec<Filter>,
}

impl HybridQuery {
    /// Create a new hybrid query with vector
    pub fn with_vector(vector: Vec<f32>, limit: usize) -> Self {
        Self {
            vector: Some(vector),
            graph_pattern: None,
            text_query: None,
            limit,
            filters: Vec::new(),
        }
    }

    /// Add graph pattern to query
    pub fn with_graph_pattern(mut self, pattern: String) -> Self {
        self.graph_pattern = Some(pattern);
        self
    }

    /// Add text query to query
    pub fn with_text_query(mut self, text: String) -> Self {
        self.text_query = Some(text);
        self
    }
}

/// Result of hybrid search
#[derive(Debug, Clone)]
pub struct HybridResult {
    /// Combined and reranked results
    pub results: Vec<HybridSearchResultItem>,
    /// Vector search results (unranked)
    pub vector_results: Vec<SearchResult>,
    /// Graph search results (unranked)
    pub graph_results: Vec<SearchResult>,
    /// Text search results (unranked)
    pub text_results: Vec<SearchResult>,
}

/// A single item in hybrid search results
#[derive(Debug, Clone)]
pub struct HybridSearchResultItem {
    /// Result ID
    pub id: String,
    /// Combined score
    pub score: f32,
    /// Vector score component
    pub vector_score: Option<f32>,
    /// Graph score component
    pub graph_score: Option<f32>,
    /// Text score component
    pub text_score: Option<f32>,
    /// Result data
    pub data: QmdData,
}

/// Hybrid searcher combining vector, graph, and text search
pub struct HybridSearcher {
    config: HybridSearchConfig,
}

impl HybridSearcher {
    /// Create a new hybrid searcher with default config
    pub fn new() -> Self {
        Self {
            config: HybridSearchConfig::default(),
        }
    }

    /// Create with custom config
    pub fn with_config(config: HybridSearchConfig) -> Self {
        Self { config }
    }

    /// Combine and rerank results from multiple search modalities
    pub fn rerank(
        &self,
        vector_results: Vec<SearchResult>,
        graph_results: Vec<SearchResult>,
        text_results: Vec<SearchResult>,
    ) -> HybridResult {
        let mut combined: Vec<HybridSearchResultItem> = Vec::new();

        // Process vector results
        for r in &vector_results {
            combined.push(HybridSearchResultItem {
                id: r.id.clone(),
                score: r.score * self.config.vector_weight,
                vector_score: Some(r.score),
                graph_score: None,
                text_score: None,
                data: r.data.clone(),
            });
        }

        // Process graph results
        for r in &graph_results {
            // Check if already in combined (from vector)
            if let Some(existing) = combined.iter_mut().find(|x| x.id == r.id) {
                existing.score += r.score * self.config.graph_weight;
                existing.graph_score = Some(r.score);
            } else {
                combined.push(HybridSearchResultItem {
                    id: r.id.clone(),
                    score: r.score * self.config.graph_weight,
                    vector_score: None,
                    graph_score: Some(r.score),
                    text_score: None,
                    data: r.data.clone(),
                });
            }
        }

        // Process text results
        for r in &text_results {
            // Check if already in combined
            if let Some(existing) = combined.iter_mut().find(|x| x.id == r.id) {
                existing.score += r.score * self.config.text_weight;
                existing.text_score = Some(r.score);
            } else {
                combined.push(HybridSearchResultItem {
                    id: r.id.clone(),
                    score: r.score * self.config.text_weight,
                    vector_score: None,
                    graph_score: None,
                    text_score: Some(r.score),
                    data: r.data.clone(),
                });
            }
        }

        // Sort by combined score
        combined.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Apply limit
        combined.truncate(self.config.limit);

        HybridResult {
            results: combined,
            vector_results,
            graph_results,
            text_results,
        }
    }
}

/// YAML rule matcher for GMP compliance rules
#[derive(Debug, Clone)]
pub struct RuleMatcher {
    #[allow(dead_code)]
    rules: HashMap<String, Vec<(String, String)>>,
}

impl RuleMatcher {
    /// Create a new rule matcher (placeholder - loads from agent-knowledge/ in real impl)
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
        }
    }

    /// Match rules against query and return matched results
    pub fn match_rules(&self, _query: &str, _top_k: usize) -> Vec<SearchResult> {
        // TODO: Implement YAML rule loading from agent-knowledge/
        // For now, return empty results
        Vec::new()
    }

    /// Check if query has a direct rule match
    pub fn has_direct_match(&self, _query: &str) -> bool {
        // TODO: Implement actual rule matching
        false
    }
}

impl Default for RuleMatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl HybridSearcher {
    /// Combine and rerank results using RRF (Reciprocal Rank Fusion)
    ///
    /// Replaces the old weighted sum approach with standard RRF formula:
    /// score = Σ w_c / (k + rank_c)
    ///
    /// Rule matches get boosted weight (default 2.0) for GMP compliance priority.
    pub fn rerank_rrf(
        &self,
        vector_results: Vec<SearchResult>,
        graph_results: Vec<SearchResult>,
        text_results: Vec<SearchResult>,
        rule_results: Vec<SearchResult>,
    ) -> HybridResult {
        let rrf_config = match &self.config.rrf {
            Some(cfg) => cfg.clone(),
            None => RrfConfig::default(),
        };

        // Convert SearchResults to ranked (doc_id, score) lists
        let ranked_vector: Vec<(String, f32)> = vector_results
            .iter()
            .enumerate()
            .map(|(i, r)| {
                (
                    r.id.clone(),
                    1.0 / (rrf_config.k + i as f32) * rrf_config.vector_weight,
                )
            })
            .collect();

        let ranked_graph: Vec<(String, f32)> = graph_results
            .iter()
            .enumerate()
            .map(|(i, r)| {
                (
                    r.id.clone(),
                    1.0 / (rrf_config.k + i as f32) * rrf_config.graph_weight,
                )
            })
            .collect();

        let ranked_text: Vec<(String, f32)> = text_results
            .iter()
            .enumerate()
            .map(|(i, r)| {
                (
                    r.id.clone(),
                    1.0 / (rrf_config.k + i as f32) * rrf_config.text_weight,
                )
            })
            .collect();

        let ranked_rules: Vec<(String, f32)> = rule_results
            .iter()
            .enumerate()
            .map(|(i, r)| {
                (
                    r.id.clone(),
                    1.0 / (rrf_config.k + i as f32) * rrf_config.rule_weight,
                )
            })
            .collect();

        // Build doc_id → all_scores map
        type ScoreEntry = (String, Option<f32>, Option<f32>, Option<f32>);
        let mut doc_scores: HashMap<String, Vec<ScoreEntry>> = HashMap::new();

        for r in &vector_results {
            doc_scores.entry(r.id.clone()).or_default().push((
                r.id.clone(),
                Some(r.score),
                None,
                None,
            ));
        }
        for r in &graph_results {
            doc_scores.entry(r.id.clone()).or_default();
            if let Some(entry) = doc_scores.get_mut(&r.id) {
                if let Some(e) = entry.iter_mut().find(|e| e.0 == r.id) {
                    e.2 = Some(r.score);
                } else {
                    entry.push((r.id.clone(), None, Some(r.score), None));
                }
            }
        }

        // Fused scores using RRF
        let mut fused: HashMap<String, f32> = HashMap::new();

        for (rank, (doc_id, _score)) in ranked_vector.iter().enumerate() {
            let rrf_score = rrf_config.vector_weight / (rrf_config.k + rank as f32);
            *fused.entry(doc_id.clone()).or_insert(0.0) += rrf_score;
        }
        for (rank, (doc_id, _score)) in ranked_graph.iter().enumerate() {
            let rrf_score = rrf_config.graph_weight / (rrf_config.k + rank as f32);
            *fused.entry(doc_id.clone()).or_insert(0.0) += rrf_score;
        }
        for (rank, (doc_id, _score)) in ranked_text.iter().enumerate() {
            let rrf_score = rrf_config.text_weight / (rrf_config.k + rank as f32);
            *fused.entry(doc_id.clone()).or_insert(0.0) += rrf_score;
        }
        for (rank, (doc_id, _score)) in ranked_rules.iter().enumerate() {
            let rrf_score = rrf_config.rule_weight / (rrf_config.k + rank as f32);
            *fused.entry(doc_id.clone()).or_insert(0.0) += rrf_score;
        }

        // Build results sorted by fused score
        let mut sorted: Vec<_> = fused.into_iter().collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        sorted.truncate(self.config.limit);

        // Map back to HybridSearchResultItem
        let results: Vec<_> = sorted
            .iter()
            .map(|(doc_id, score)| {
                let data = vector_results
                    .iter()
                    .find(|r| &r.id == doc_id)
                    .or_else(|| graph_results.iter().find(|r| &r.id == doc_id))
                    .or_else(|| text_results.iter().find(|r| &r.id == doc_id))
                    .or_else(|| rule_results.iter().find(|r| &r.id == doc_id))
                    .map(|r| r.data.clone())
                    .unwrap_or(QmdData {
                        id: doc_id.clone(),
                        data_type: crate::types::QmdDataType::Document,
                        vector: None,
                        graph: None,
                        text: None,
                        metadata: std::collections::HashMap::new(),
                        timestamp: 0,
                    });

                HybridSearchResultItem {
                    id: doc_id.clone(),
                    score: *score,
                    vector_score: vector_results
                        .iter()
                        .find(|r| &r.id == doc_id)
                        .map(|r| r.score),
                    graph_score: graph_results
                        .iter()
                        .find(|r| &r.id == doc_id)
                        .map(|r| r.score),
                    text_score: text_results
                        .iter()
                        .find(|r| &r.id == doc_id)
                        .map(|r| r.score),
                    data,
                }
            })
            .collect();

        HybridResult {
            results,
            vector_results,
            graph_results,
            text_results,
        }
    }
}

impl Default for HybridSearcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::QmdDataType;

    fn create_test_qmd_data(id: &str, vector: Vec<f32>) -> QmdData {
        QmdData {
            id: id.to_string(),
            data_type: QmdDataType::Vector,
            vector: Some(vector),
            graph: None,
            text: None,
            metadata: std::collections::HashMap::new(),
            timestamp: 0,
        }
    }

    #[test]
    fn test_hybrid_rerank() {
        let searcher = HybridSearcher::new();

        let vector_results = vec![SearchResult {
            id: "1".to_string(),
            score: 0.9,
            data: create_test_qmd_data("1", vec![0.1, 0.2]),
        }];

        let graph_results = vec![SearchResult {
            id: "2".to_string(),
            score: 0.8,
            data: create_test_qmd_data("2", vec![0.3, 0.4]),
        }];

        let text_results = vec![];

        let result = searcher.rerank(vector_results, graph_results, text_results);

        assert_eq!(result.results.len(), 2);
        assert_eq!(result.results[0].id, "1"); // 0.9 * 0.4 = 0.36
        assert_eq!(result.results[1].id, "2"); // 0.8 * 0.3 = 0.24
    }

    #[test]
    fn test_hybrid_combined_id() {
        let searcher = HybridSearcher::new();

        // Same ID appearing in both vector and graph
        let vector_results = vec![SearchResult {
            id: "1".to_string(),
            score: 0.9,
            data: create_test_qmd_data("1", vec![0.1, 0.2]),
        }];

        let graph_results = vec![SearchResult {
            id: "1".to_string(),
            score: 0.8,
            data: create_test_qmd_data("1", vec![0.1, 0.2]),
        }];

        let text_results = vec![];

        let result = searcher.rerank(vector_results, graph_results, text_results);

        assert_eq!(result.results.len(), 1);
        // Combined score: 0.9 * 0.4 + 0.8 * 0.3 = 0.36 + 0.24 = 0.60
        assert!((result.results[0].score - 0.60).abs() < 0.001);
    }
}
