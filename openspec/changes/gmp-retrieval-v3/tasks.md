## 1. Ollama Vectorizer

- [x] 1.1 Create `ollama_vectorizer.rs` module in `gmp-retrieval`
- [x] 1.2 Implement `OllamaEmbeddingClient` struct with async embedding generation
- [x] 1.3 Add LRU cache for embeddings (10,000 entries)
- [x] 1.4 Support `nomic-embed-text` and `mxbai-embed-large` models
- [x] 1.5 Write unit tests with mock Ollama responses

## 2. Query Rewriter

- [x] 2.1 Create `query_rewriter.rs` module in `gmp-retrieval`
- [x] 2.2 Implement `RewrittenQuery` and `QueryIntent` types
- [x] 2.3 Implement prompt template for Chinese query rewriting
- [x] 2.4 Implement JSON response parsing with fallback
- [x] 2.5 Add 500ms timeout with fallback to original query
- [x] 2.6 Write unit tests

## 3. Reranker

- [x] 3.1 Create `reranker.rs` module in `gmp-retrieval`
- [x] 3.2 Implement `RerankRequest` and `RerankResult` types
- [x] 3.3 Implement cross-encoder scoring via Ollama
- [x] 3.4 Implement batch processing (max 50 docs)
- [x] 3.5 Add fallback for unavailable Ollama
- [x] 3.6 Write unit tests

## 4. GMP Retrieval v3 Core

- [x] 4.1 Extend `RetrievalEngine` with four-way fusion
- [x] 4.2 Implement `RetrievalSource` enum (Vector/Fts/Rule/Graph)
- [x] 4.3 Implement weighted RRF fusion with configurable k
- [x] 4.4 Add `search_with_filter` method
- [x] 4.5 Integrate vectorizer for embedding queries
- [x] 4.6 Integrate query rewriter for natural language input
- [x] 4.7 Integrate reranker in post-processing
- [x] 4.8 Write integration tests

## 5. GMP Platform Integration

- [x] 5.1 Add `retrieval` module to `sqlrustgo-gmp` crate
- [x] 5.2 Create API endpoint `POST /api/v1/retrieval/search`
- [x] 5.3 Create API endpoint `POST /api/v1/retrieval/embed`
- [x] 5.4 Write API integration tests
- [x] 5.5 Update `gmp-api` exports

## 6. Performance & Quality

- [ ] 6.1 Benchmark P99 latency target (< 200ms)
- [ ] 6.2 Measure recall on GMP test corpus (> 90%)
- [ ] 6.3 Add stress test for 100+ concurrent queries
- [ ] 6.4 Run full test suite and verify
