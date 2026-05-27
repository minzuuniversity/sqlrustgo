use std::collections::HashMap;

use crate::bm25::Bm25Search;
use crate::query_rewriter::{FilterCondition, QueryIntent, RewrittenQuery};
use crate::reranker::RerankResult;

/// Retrieval source type for four-way fusion
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetrievalSource {
    /// Vector similarity search (embedding-based)
    Vector,
    /// Full-text search (BM25)
    Fts,
    /// Rule-based structured query
    Rule,
    /// Knowledge graph traversal
    Graph,
}

impl RetrievalSource {
    /// Get the weight for this source in fusion
    pub fn weight(&self) -> f64 {
        match self {
            RetrievalSource::Vector => 1.0,
            RetrievalSource::Fts => 0.8,
            RetrievalSource::Rule => 1.2,
            RetrievalSource::Graph => 0.9,
        }
    }
}

/// Unified search result with string doc_id for reranker compatibility
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub doc_id: String,
    pub score: f64,
    pub source: RetrievalSource,
}

impl SearchResult {
    pub fn new(doc_id: String, score: f64, source: RetrievalSource) -> Self {
        Self { doc_id, score, source }
    }

    /// Convert to RerankResult for reranking
    pub fn to_rerank_result(&self) -> RerankResult {
        RerankResult {
            doc_id: self.doc_id.clone(),
            score: self.score as f32,
        }
    }
}

pub struct RetrievalEngine {
    bm25: Option<Bm25Search>,
    documents: Vec<String>,
}

impl RetrievalEngine {
    /// Create a new empty retrieval engine
    pub fn new() -> Self {
        Self {
            bm25: None,
            documents: Vec::new(),
        }
    }

    /// Create engine with BM25 full-text search
    pub fn with_bm25(documents: Vec<String>) -> Self {
        Self {
            bm25: Some(Bm25Search::new(documents.clone())),
            documents,
        }
    }

    /// Get document by index
    pub fn get_doc(&self, idx: usize) -> Option<&String> {
        self.documents.get(idx)
    }

    /// Get all documents
    pub fn get_all_docs(&self) -> &[String] {
        &self.documents
    }

    /// Search using BM25 (FTS)
    pub fn search_bm25(&self, query: &str, top_k: usize) -> Vec<SearchResult> {
        match &self.bm25 {
            Some(search) => search
                .search(query, top_k)
                .into_iter()
                .map(|r| {
                    SearchResult::new(r.doc_id.to_string(), r.score, RetrievalSource::Fts)
                })
                .collect(),
            None => Vec::new(),
        }
    }

    /// Search using rule-based structured query
    pub fn search_rule(&self, query: &RewrittenQuery, top_k: usize) -> Vec<SearchResult> {
        let mut results: Vec<SearchResult> = self
            .documents
            .iter()
            .enumerate()
            .filter(|(_, doc)| {
                // Apply filters if present
                for filter in &query.filters {
                    if !self.matches_filter(doc, filter) {
                        return false;
                    }
                }
                true
            })
            .filter(|(_, doc)| {
                // Check if document contains any of the keywords
                let doc_lower = doc.to_lowercase();
                query.keywords.iter().any(|kw| doc_lower.contains(&kw.to_lowercase()))
            })
            .map(|(idx, doc)| {
                // Score based on keyword matches and intent
                let match_count = query
                    .keywords
                    .iter()
                    .filter(|kw| doc.to_lowercase().contains(&kw.to_lowercase()))
                    .count();
                let score = match query.intent {
                    QueryIntent::Search => match_count as f64,
                    QueryIntent::Filter => 2.0,
                    QueryIntent::Comparison => 1.5,
                    QueryIntent::Aggregation => 1.0,
                };
                SearchResult::new(idx.to_string(), score, RetrievalSource::Rule)
            })
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(top_k);
        results
    }

    /// Check if document matches a filter condition
    fn matches_filter(&self, doc: &str, filter: &FilterCondition) -> bool {
        let doc_lower = doc.to_lowercase();
        match filter.op.as_str() {
            "contains" => doc_lower.contains(&filter.value.to_string().to_lowercase()),
            "eq" => doc_lower.contains(&filter.value.to_string().to_lowercase()),
            _ => true, // Other operators not implemented yet
        }
    }

    /// Perform four-way fusion with weighted RRF
    pub fn fuse_results(&self, result_sets: Vec<Vec<SearchResult>>, k: usize) -> Vec<SearchResult> {
        let mut fusion = WeightedRrfFusion::new(k);

        for results in result_sets {
            for result in results {
                fusion.add_result(result);
            }
        }

        fusion.rank()
    }

    /// Search with filter support
    pub fn search_with_filter(
        &self,
        query: &RewrittenQuery,
        top_k: usize,
    ) -> Vec<SearchResult> {
        // Use rule-based search which supports filters
        self.search_rule(query, top_k)
    }
}

impl Default for RetrievalEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Weighted RRF Fusion for four-way retrieval
struct WeightedRrfFusion {
    k: usize,
    scores: HashMap<String, f64>,
    source_counts: HashMap<String, usize>,
    sources: HashMap<String, RetrievalSource>,
}

impl WeightedRrfFusion {
    fn new(k: usize) -> Self {
        Self {
            k,
            scores: HashMap::new(),
            source_counts: HashMap::new(),
            sources: HashMap::new(),
        }
    }

    fn add_result(&mut self, result: SearchResult) {
        let source_weight = result.source.weight();
        *self.scores.entry(result.doc_id.clone()).or_insert(0.0) +=
            result.score * source_weight;
        *self.source_counts.entry(result.doc_id.clone()).or_insert(0) += 1;
        self.sources
            .entry(result.doc_id.clone())
            .or_insert(result.source);
    }

    fn rank(&self) -> Vec<SearchResult> {
        let mut results: Vec<SearchResult> = self
            .scores
            .iter()
            .map(|(doc_id, &score)| {
                let votes = *self.source_counts.get(doc_id).unwrap_or(&1);
                let rrf_score = score + (1.0 / (self.k as f64 + votes as f64));
                let source = self.sources.get(doc_id).copied().unwrap_or(RetrievalSource::Fts);
                SearchResult::new(doc_id.clone(), rrf_score, source)
            })
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retrieval_source_weights() {
        assert_eq!(RetrievalSource::Vector.weight(), 1.0);
        assert_eq!(RetrievalSource::Fts.weight(), 0.8);
        assert_eq!(RetrievalSource::Rule.weight(), 1.2);
        assert_eq!(RetrievalSource::Graph.weight(), 0.9);
    }

    #[test]
    fn test_search_result_to_rerank() {
        let result = SearchResult::new("doc1".to_string(), 0.95, RetrievalSource::Vector);
        let rerank_result = result.to_rerank_result();
        assert_eq!(rerank_result.doc_id, "doc1");
        assert_eq!(rerank_result.score, 0.95);
    }

    #[test]
    fn test_engine_with_bm25() {
        let docs = vec![
            "The quick brown fox".to_string(),
            "The lazy dog".to_string(),
            "The quick rabbit".to_string(),
        ];
        let engine = RetrievalEngine::with_bm25(docs);
        let results = engine.search_bm25("quick", 2);
        assert!(!results.is_empty());
    }

    #[test]
    fn test_search_rule() {
        let docs = vec![
            "GMP偏差报告A".to_string(),
            "GMP偏差报告B".to_string(),
            "其他文档".to_string(),
        ];
        let engine = RetrievalEngine::with_bm25(docs);
        let query = RewrittenQuery {
            original: "偏差报告".to_string(),
            keywords: vec!["偏差".to_string(), "报告".to_string()],
            filters: vec![],
            intent: QueryIntent::Search,
        };
        let results = engine.search_rule(&query, 10);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_fuse_results() {
        let docs = vec![
            "文档1".to_string(),
            "文档2".to_string(),
            "文档3".to_string(),
        ];
        let engine = RetrievalEngine::with_bm25(docs);

        let results1 = vec![
            SearchResult::new("0".to_string(), 0.9, RetrievalSource::Vector),
            SearchResult::new("1".to_string(), 0.8, RetrievalSource::Vector),
        ];
        let results2 = vec![
            SearchResult::new("0".to_string(), 0.7, RetrievalSource::Fts),
            SearchResult::new("2".to_string(), 0.6, RetrievalSource::Fts),
        ];

        let fused = engine.fuse_results(vec![results1, results2], 60);
        assert_eq!(fused.len(), 3);
        // Doc 0 appears in both, should rank higher
        assert_eq!(fused[0].doc_id, "0");
    }
}
