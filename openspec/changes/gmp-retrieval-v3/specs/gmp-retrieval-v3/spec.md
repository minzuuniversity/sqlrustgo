# GMP Retrieval v3 Specification

## Overview

Four-way fusion retrieval engine combining Rule-based, Vector, FTS (BM25), and Graph search.

## Functionality

### Core Features

1. **Multi-Source Retrieval**
   - Rule-based filtering (exact match on metadata)
   - Vector search via HNSW/IVF-PQ index
   - BM25 full-text search
   - Graph traversal (external Graph DB)

2. **RRF Fusion**
   - Reciprocal Rank Fusion with configurable k-factor (default: 60)
   - Weighted fusion support per source
   - Score normalization across sources

3. **Query Interface**
   - `search(query: &str, top_k: usize) -> Vec<SearchResult>`
   - `search_with_filter(query: &str, filter: Rule, top_k: usize) -> Vec<SearchResult>`

### Data Model

```rust
pub struct SearchResult {
    pub doc_id: String,
    pub score: f64,
    pub source: RetrievalSource,
    pub metadata: HashMap<String, String>,
}

pub enum RetrievalSource {
    Vector,
    Fts,
    Rule,
    Graph,
}
```

### Performance Requirements

- P99 latency < 200ms
- Recall > 90% (measured on GMP test corpus)
- Support 100+ concurrent queries

### Edge Cases

- Empty query → return recent documents
- No results from any source → return empty vector
- Ollama unavailable → fallback to BM25-only mode
- Graph DB unavailable → skip graph source, continue with others
