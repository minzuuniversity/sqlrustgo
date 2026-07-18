//! Hybrid rerank module tests
//!
//! Tests: RerankConfig, ModeWeights, SearchMode, ScoredResult,
//!        RerankedItem, ScoreBreakdown, HybridReranker

use sqlrustgo_server::hybrid_rerank::{
    HybridReranker, ModeWeights, RerankConfig, RerankedItem, ScoreBreakdown, ScoredResult,
    SearchMode,
};

// ============ SearchMode tests ============

#[test]
fn test_search_mode_sql() {
    let mode = SearchMode::Sql;
    assert!(matches!(mode, SearchMode::Sql));
    assert_eq!(format!("{}", mode), "sql");
}

#[test]
fn test_search_mode_vector() {
    let mode = SearchMode::Vector;
    assert!(matches!(mode, SearchMode::Vector));
    assert_eq!(format!("{}", mode), "vector");
}

#[test]
fn test_search_mode_graph() {
    let mode = SearchMode::Graph;
    assert!(matches!(mode, SearchMode::Graph));
    assert_eq!(format!("{}", mode), "graph");
}

#[test]
fn test_search_mode_copy() {
    let mode = SearchMode::Sql;
    let copy = mode;
    assert_eq!(mode, copy);
}

#[test]
fn test_search_mode_hash() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(SearchMode::Sql);
    set.insert(SearchMode::Vector);
    set.insert(SearchMode::Graph);
    assert_eq!(set.len(), 3);
}

#[test]
fn test_search_mode_debug() {
    let mode = SearchMode::Vector;
    let debug = format!("{:?}", mode);
    assert!(debug.contains("Vector"));
}

// ============ RerankConfig tests ============

#[test]
fn test_rerank_config_default() {
    let cfg = RerankConfig::default();
    assert!(cfg.enabled);
    assert_eq!(cfg.algorithm, "rrf");
    assert_eq!(cfg.top_k, 10);
    assert!(cfg.min_score.is_none());
}

#[test]
fn test_rerank_config_custom() {
    let cfg = RerankConfig {
        enabled: false,
        algorithm: "linear".to_string(),
        mode_weights: ModeWeights::default(),
        top_k: 50,
        min_score: Some(0.5),
    };
    assert!(!cfg.enabled);
    assert_eq!(cfg.algorithm, "linear");
    assert_eq!(cfg.top_k, 50);
    assert_eq!(cfg.min_score, Some(0.5));
}

#[test]
fn test_rerank_config_clone() {
    let cfg = RerankConfig::default();
    let c = cfg.clone();
    assert_eq!(cfg.enabled, c.enabled);
    assert_eq!(cfg.algorithm, c.algorithm);
}

#[test]
fn test_rerank_config_debug() {
    let cfg = RerankConfig::default();
    let debug = format!("{:?}", cfg);
    assert!(debug.contains("RerankConfig"));
    assert!(debug.contains("rrf"));
}

// ============ ModeWeights tests ============

#[test]
fn test_mode_weights_default() {
    let mw = ModeWeights::default();
    assert_eq!(mw.sql, 1.0);
    assert_eq!(mw.vector, 1.0);
    assert_eq!(mw.graph, 1.0);
    assert_eq!(mw.multi_match_bonus, 0.1);
}

#[test]
fn test_mode_weights_custom() {
    let mw = ModeWeights {
        sql: 2.0,
        vector: 1.5,
        graph: 0.5,
        multi_match_bonus: 0.2,
    };
    assert_eq!(mw.sql, 2.0);
    assert_eq!(mw.vector, 1.5);
    assert_eq!(mw.graph, 0.5);
    assert_eq!(mw.multi_match_bonus, 0.2);
}

#[test]
fn test_mode_weights_clone() {
    let mw = ModeWeights::default();
    let c = mw.clone();
    assert_eq!(mw.sql, c.sql);
}

#[test]
fn test_mode_weights_debug() {
    let mw = ModeWeights::default();
    let debug = format!("{:?}", mw);
    assert!(debug.contains("ModeWeights"));
}

// ============ ScoredResult tests ============

#[test]
fn test_scored_result_new() {
    let result = ScoredResult::new("doc1".to_string(), SearchMode::Sql, 0.9);
    assert_eq!(result.id, "doc1");
    assert_eq!(result.score, 0.9);
    assert!(matches!(result.sources.as_slice(), [SearchMode::Sql]));
    assert_eq!(result.result_type, "document");
    assert!(result.metadata.is_empty());
}

#[test]
fn test_scored_result_new_vector() {
    let result = ScoredResult::new("vec1".to_string(), SearchMode::Vector, 0.8);
    assert_eq!(result.id, "vec1");
    assert!(matches!(result.sources.as_slice(), [SearchMode::Vector]));
}

#[test]
fn test_scored_result_new_graph() {
    let result = ScoredResult::new("graph1".to_string(), SearchMode::Graph, 0.7);
    assert_eq!(result.id, "graph1");
    assert!(matches!(result.sources.as_slice(), [SearchMode::Graph]));
}

#[test]
fn test_scored_result_add_mode_score() {
    let mut result = ScoredResult::new("doc1".to_string(), SearchMode::Sql, 0.9);
    result.add_mode_score(SearchMode::Vector, 0.8);
    result.add_mode_score(SearchMode::Graph, 0.7);
    assert_eq!(result.sources.len(), 3);
    assert_eq!(result.mode_scores.get(&SearchMode::Sql), Some(&0.9));
    assert_eq!(result.mode_scores.get(&SearchMode::Vector), Some(&0.8));
    assert_eq!(result.mode_scores.get(&SearchMode::Graph), Some(&0.7));
}

#[test]
fn test_scored_result_add_duplicate_mode() {
    let mut result = ScoredResult::new("doc1".to_string(), SearchMode::Sql, 0.9);
    result.add_mode_score(SearchMode::Sql, 0.95);
    assert_eq!(result.sources.len(), 1);
    assert_eq!(result.mode_scores.get(&SearchMode::Sql), Some(&0.95));
}

#[test]
fn test_scored_result_multi_match_bonus_single() {
    let result = ScoredResult::new("doc1".to_string(), SearchMode::Sql, 0.9);
    assert_eq!(result.multi_match_bonus(0.1), 0.0);
}

#[test]
fn test_scored_result_multi_match_bonus_double() {
    let mut result = ScoredResult::new("doc1".to_string(), SearchMode::Sql, 0.9);
    result.add_mode_score(SearchMode::Vector, 0.8);
    assert_eq!(result.multi_match_bonus(0.1), 0.1);
}

#[test]
fn test_scored_result_multi_match_bonus_triple() {
    let mut result = ScoredResult::new("doc1".to_string(), SearchMode::Sql, 0.9);
    result.add_mode_score(SearchMode::Vector, 0.8);
    result.add_mode_score(SearchMode::Graph, 0.7);
    assert_eq!(result.multi_match_bonus(0.1), 0.2);
}

#[test]
fn test_scored_result_multi_match_bonus_high_bonus() {
    let mut result = ScoredResult::new("doc1".to_string(), SearchMode::Sql, 0.9);
    result.add_mode_score(SearchMode::Vector, 0.8);
    assert_eq!(result.multi_match_bonus(0.5), 0.5);
}

#[test]
fn test_scored_result_clone() {
    let result = ScoredResult::new("doc1".to_string(), SearchMode::Sql, 0.9);
    let c = result.clone();
    assert_eq!(c.id, result.id);
    assert_eq!(c.score, result.score);
}

#[test]
fn test_scored_result_debug() {
    let result = ScoredResult::new("doc1".to_string(), SearchMode::Sql, 0.9);
    let debug = format!("{:?}", result);
    assert!(debug.contains("ScoredResult"));
    assert!(debug.contains("doc1"));
}

#[test]
fn test_scored_result_with_metadata() {
    let mut result = ScoredResult::new("doc1".to_string(), SearchMode::Sql, 0.9);
    result
        .metadata
        .insert("author".to_string(), serde_json::json!("Alice"));
    assert_eq!(
        result.metadata.get("author"),
        Some(&serde_json::json!("Alice"))
    );
}

#[test]
fn test_scored_result_different_result_types() {
    let doc = ScoredResult::new("e1".to_string(), SearchMode::Sql, 0.9);
    assert_eq!(doc.result_type, "document");

    let mut entity = ScoredResult::new("e2".to_string(), SearchMode::Vector, 0.8);
    entity.result_type = "entity".to_string();
    assert_eq!(entity.result_type, "entity");
}

// ============ ScoreBreakdown tests ============

#[test]
fn test_score_breakdown_partial() {
    let breakdown = ScoreBreakdown {
        sql_score: Some(0.9),
        vector_score: Some(0.8),
        graph_score: None,
        multi_match_bonus: 0.1,
        final_score: 1.7,
    };
    assert!(breakdown.sql_score.is_some());
    assert!(breakdown.vector_score.is_some());
    assert!(breakdown.graph_score.is_none());
}

#[test]
fn test_score_breakdown_all_modes() {
    let breakdown = ScoreBreakdown {
        sql_score: Some(0.9),
        vector_score: Some(0.8),
        graph_score: Some(0.7),
        multi_match_bonus: 0.2,
        final_score: 2.6,
    };
    assert!(breakdown.sql_score.is_some());
    assert!(breakdown.graph_score.is_some());
}

#[test]
fn test_score_breakdown_clone() {
    let breakdown = ScoreBreakdown {
        sql_score: Some(0.9),
        vector_score: None,
        graph_score: Some(0.7),
        multi_match_bonus: 0.1,
        final_score: 1.7,
    };
    let c = breakdown.clone();
    assert_eq!(c.final_score, 1.7);
}

#[test]
fn test_score_breakdown_debug() {
    let breakdown = ScoreBreakdown {
        sql_score: Some(0.9),
        vector_score: None,
        graph_score: None,
        multi_match_bonus: 0.0,
        final_score: 0.9,
    };
    let debug = format!("{:?}", breakdown);
    assert!(debug.contains("ScoreBreakdown"));
}

// ============ RerankedItem tests ============

#[test]
fn test_reranked_item_basic() {
    let item = RerankedItem {
        id: "doc1".to_string(),
        combined_score: 1.8,
        source_modes: vec![SearchMode::Sql, SearchMode::Vector],
        result_type: "document".to_string(),
        score_breakdown: ScoreBreakdown {
            sql_score: Some(0.9),
            vector_score: Some(0.8),
            graph_score: None,
            multi_match_bonus: 0.1,
            final_score: 1.8,
        },
        metadata: None,
    };
    assert_eq!(item.id, "doc1");
    assert_eq!(item.combined_score, 1.8);
    assert_eq!(item.source_modes.len(), 2);
    assert!(item.metadata.is_none());
}

#[test]
fn test_reranked_item_with_metadata() {
    let mut meta = std::collections::HashMap::new();
    meta.insert("rank".to_string(), serde_json::json!(1));
    let item = RerankedItem {
        id: "doc1".to_string(),
        combined_score: 1.5,
        source_modes: vec![SearchMode::Sql],
        result_type: "entity".to_string(),
        score_breakdown: ScoreBreakdown {
            sql_score: Some(1.5),
            vector_score: None,
            graph_score: None,
            multi_match_bonus: 0.0,
            final_score: 1.5,
        },
        metadata: Some(meta),
    };
    assert!(item.metadata.is_some());
}

#[test]
fn test_reranked_item_clone() {
    let item = RerankedItem {
        id: "doc1".to_string(),
        combined_score: 1.0,
        source_modes: vec![SearchMode::Graph],
        result_type: "document".to_string(),
        score_breakdown: ScoreBreakdown {
            sql_score: None,
            vector_score: None,
            graph_score: Some(1.0),
            multi_match_bonus: 0.0,
            final_score: 1.0,
        },
        metadata: None,
    };
    let c = item.clone();
    assert_eq!(c.id, "doc1");
}

#[test]
fn test_reranked_item_debug() {
    let item = RerankedItem {
        id: "doc1".to_string(),
        combined_score: 2.0,
        source_modes: vec![SearchMode::Sql, SearchMode::Vector, SearchMode::Graph],
        result_type: "document".to_string(),
        score_breakdown: ScoreBreakdown {
            sql_score: Some(0.8),
            vector_score: Some(0.7),
            graph_score: Some(0.6),
            multi_match_bonus: 0.2,
            final_score: 2.0,
        },
        metadata: None,
    };
    let debug = format!("{:?}", item);
    assert!(debug.contains("RerankedItem"));
    assert!(debug.contains("doc1"));
}

// ============ HybridReranker tests ============

#[test]
fn test_hybrid_reranker_new() {
    let config = RerankConfig::default();
    let reranker = HybridReranker::new(config);
    let _ = reranker;
}

#[test]
fn test_hybrid_reranker_default_config() {
    let reranker = HybridReranker::default_config();
    let _ = reranker;
}

#[test]
fn test_hybrid_reranker_rerank_rrf() {
    let reranker = HybridReranker::default_config();
    let results = vec![
        ScoredResult::new("doc1".to_string(), SearchMode::Sql, 0.9),
        ScoredResult::new("doc2".to_string(), SearchMode::Vector, 0.8),
    ];
    let ranked = reranker.rerank(results);
    assert!(!ranked.is_empty());
}

#[test]
fn test_hybrid_reranker_rerank_linear() {
    let config = RerankConfig {
        enabled: true,
        algorithm: "linear".to_string(),
        mode_weights: ModeWeights::default(),
        top_k: 10,
        min_score: None,
    };
    let reranker = HybridReranker::new(config);
    let results = vec![
        ScoredResult::new("doc1".to_string(), SearchMode::Sql, 0.9),
        ScoredResult::new("doc2".to_string(), SearchMode::Vector, 0.8),
    ];
    let ranked = reranker.rerank(results);
    assert!(!ranked.is_empty());
}

#[test]
fn test_hybrid_reranker_rerank_composite() {
    let config = RerankConfig {
        enabled: true,
        algorithm: "composite".to_string(),
        mode_weights: ModeWeights::default(),
        top_k: 10,
        min_score: None,
    };
    let reranker = HybridReranker::new(config);
    let results = vec![ScoredResult::new("doc1".to_string(), SearchMode::Sql, 0.9)];
    let ranked = reranker.rerank(results);
    assert!(!ranked.is_empty());
}

#[test]
fn test_hybrid_reranker_rerank_unknown_algorithm() {
    let config = RerankConfig {
        enabled: true,
        algorithm: "unknown".to_string(),
        mode_weights: ModeWeights::default(),
        top_k: 10,
        min_score: None,
    };
    let reranker = HybridReranker::new(config);
    let results = vec![ScoredResult::new("doc1".to_string(), SearchMode::Sql, 0.9)];
    // Unknown algorithm falls back to RRF
    let ranked = reranker.rerank(results);
    assert!(!ranked.is_empty());
}

#[test]
fn test_hybrid_reranker_rerank_empty() {
    let reranker = HybridReranker::default_config();
    let results: Vec<ScoredResult> = vec![];
    let ranked = reranker.rerank(results);
    assert!(ranked.is_empty());
}

#[test]
fn test_hybrid_reranker_rerank_multi_source() {
    let reranker = HybridReranker::default_config();
    let mut r1 = ScoredResult::new("doc1".to_string(), SearchMode::Sql, 0.9);
    r1.add_mode_score(SearchMode::Vector, 0.8);
    r1.add_mode_score(SearchMode::Graph, 0.7);
    let results = vec![r1];
    let ranked = reranker.rerank(results);
    assert_eq!(ranked.len(), 1);
    assert_eq!(ranked[0].source_modes.len(), 3);
}
