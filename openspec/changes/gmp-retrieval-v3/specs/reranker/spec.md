# Reranker Specification

## Overview

LLM-based reranking to improve search result quality through relevance scoring.

## Functionality

### Core Features

1. **Cross-Encoder Scoring**
   - Score query-document pairs
   - Return relevance scores [0.0, 1.0]

2. **Reranking Pipeline**
   - Input: Initial candidate list (top-k * 3)
   - Output: Reranked list (top-k)

3. **Batch Processing**
   - Process multiple documents in single request
   - Max batch size: 50 documents

### API

```rust
pub struct RerankRequest {
    pub query: String,
    pub documents: Vec<String>,
    pub top_n: usize,
}

pub struct RerankResult {
    pub doc_id: usize,
    pub score: f32,
}

pub async fn rerank(
    client: &OllamaClient,
    request: &RerankRequest,
    model: &str,
) -> Result<Vec<RerankResult>, RerankerError>;
```

### Performance

- Reranking latency < 150ms for 20 documents
- Throughput: 100 docs/second

### Fallback

- Ollama unavailable → return original order with scores=1.0
- Timeout → return original order
