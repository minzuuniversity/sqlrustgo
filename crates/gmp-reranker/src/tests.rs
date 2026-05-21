//! Unit tests for gmp-reranker crate

use crate::{create_reranker, NoopReranker, OllamaReranker, Reranker};
use gmp_core::{ChannelType, ScoredChunk};
use std::sync::Arc;

fn make_test_chunk(chunk_id: i64, score: f32) -> ScoredChunk {
    ScoredChunk {
        chunk_id,
        doc_id: format!("doc-{}", chunk_id),
        content: format!("Test content for chunk {}", chunk_id),
        score,
        source: ChannelType::Vector,
    }
}

/// Test that Reranker trait is object-safe and can be used via trait object
#[tokio::test]
async fn test_reranker_trait_object() {
    let reranker: Arc<dyn Reranker> = Arc::new(NoopReranker);
    
    let chunks = vec![
        make_test_chunk(1, 0.9),
        make_test_chunk(2, 0.8),
        make_test_chunk(3, 0.7),
    ];
    
    let result = reranker.rerank("test query", chunks.clone(), 2).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 2);
}

/// Test NoopReranker::rerank() returns input unchanged (order preserved, just truncated to top_k)
#[tokio::test]
async fn test_noop_reranker_returns_input_unchanged() {
    let reranker = NoopReranker;
    
    let chunks = vec![
        make_test_chunk(1, 0.9),
        make_test_chunk(2, 0.8),
        make_test_chunk(3, 0.7),
    ];
    
    let result = reranker.rerank("test query", chunks.clone(), 10).await;
    assert!(result.is_ok());
    
    let reranked = result.unwrap();
    // Should return all chunks since top_k > len
    assert_eq!(reranked.len(), 3);
    // Order should be preserved
    assert_eq!(reranked[0].chunk_id, 1);
    assert_eq!(reranked[1].chunk_id, 2);
    assert_eq!(reranked[2].chunk_id, 3);
    // Scores unchanged
    assert_eq!(reranked[0].score, 0.9);
    assert_eq!(reranked[1].score, 0.8);
    assert_eq!(reranked[2].score, 0.7);
}

/// Test NoopReranker respects top_k limit
#[tokio::test]
async fn test_noop_reranker_respects_top_k() {
    let reranker = NoopReranker;
    
    let chunks = vec![
        make_test_chunk(1, 0.9),
        make_test_chunk(2, 0.8),
        make_test_chunk(3, 0.7),
        make_test_chunk(4, 0.6),
        make_test_chunk(5, 0.5),
    ];
    
    let result = reranker.rerank("test query", chunks, 2).await;
    assert!(result.is_ok());
    
    let reranked = result.unwrap();
    assert_eq!(reranked.len(), 2);
    assert_eq!(reranked[0].chunk_id, 1);
    assert_eq!(reranked[1].chunk_id, 2);
}

/// Test NoopReranker handles empty input
#[tokio::test]
async fn test_noop_reranker_empty_input() {
    let reranker = NoopReranker;
    
    let chunks: Vec<ScoredChunk> = vec![];
    
    let result = reranker.rerank("test query", chunks, 10).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
}

/// Test NoopReranker name
#[test]
fn test_noop_reranker_name() {
    let reranker = NoopReranker;
    assert_eq!(reranker.name(), "noop");
}

/// Test OllamaReranker handles empty candidates gracefully
#[tokio::test]
async fn test_ollama_reranker_empty_candidates() {
    let reranker = OllamaReranker::new(
        "http://localhost:11434".to_string(),
        "bge-reranker-v2-m3".to_string(),
    );
    
    let chunks: Vec<ScoredChunk> = vec![];
    
    let result = reranker.rerank("test query", chunks, 10).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
}

/// Test OllamaReranker returns original order when embed fails (no server)
#[tokio::test]
async fn test_ollama_reranker_embed_failure_returns_original() {
    // Use invalid URL - no server running
    let reranker = OllamaReranker::new(
        "http://localhost:19999".to_string(),  // Invalid port
        "bge-reranker-v2-m3".to_string(),
    );
    
    let chunks = vec![
        make_test_chunk(1, 0.9),
        make_test_chunk(2, 0.8),
    ];
    
    // Since Ollama is not available at that port, embed_text will fail
    // and rerank should return original chunks (truncated to top_k)
    let result = reranker.rerank("test query", chunks.clone(), 2).await;
    assert!(result.is_ok());
    
    // Should get chunks back (original order, truncated to top_k)
    let reranked = result.unwrap();
    assert_eq!(reranked.len(), 2);
    assert_eq!(reranked[0].chunk_id, 1);
    assert_eq!(reranked[1].chunk_id, 2);
}

/// Test OllamaReranker name
#[test]
fn test_ollama_reranker_name() {
    let reranker = OllamaReranker::new(
        "http://localhost:11434".to_string(),
        "bge-reranker-v2-m3".to_string(),
    );
    assert_eq!(reranker.name(), "ollama-bge-reranker-v2-m3");
}

/// Test create_reranker falls back to NoopReranker when Ollama unavailable
#[tokio::test]
async fn test_create_reranker_fallback_to_noop() {
    // Use invalid URL to ensure Ollama is not available
    let reranker = create_reranker("http://localhost:19999", "bge-reranker-v2-m3");
    
    // Should fall back to NoopReranker
    assert_eq!(reranker.name(), "noop");
    
    // Verify behavior via trait object
    let chunks = vec![
        make_test_chunk(1, 0.9),
        make_test_chunk(2, 0.8),
    ];
    
    let result = reranker.rerank("test query", chunks.clone(), 10).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 2);
}

/// Test create_reranker returns Arc<dyn Reranker> that works via trait object
#[tokio::test]
async fn test_create_reranker_returns_trait_object() {
    let reranker = create_reranker("http://localhost:19999", "bge-reranker-v2-m3");
    
    let chunks = vec![
        make_test_chunk(1, 0.9),
        make_test_chunk(2, 0.8),
        make_test_chunk(3, 0.7),
    ];
    
    let result = reranker.rerank("test query", chunks, 2).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 2);
}
