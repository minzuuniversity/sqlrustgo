# Proposal: V312-05 Hybrid Retrieval

## Why

GMP audit queries need more than keyword or vector-only search. V312-05 combines vector similarity, keyword matching, and graph relation boosting with Reciprocal Rank Fusion (RRF) for unified ranking — providing precise RAG citations for audit evidence bundles (V312-07).

## What Changes

- `retrieval.rs`: New module — `hybrid_retrieval`, `retrieval_search`, `RetrievalFilter`, `RetrievalResult`, `RetrievalScoreComponents`
- `RetrievalResult` includes: doc_id, version_number, chunk_id, chunk_hash, source_path, citation_text, score breakdown (vector/keywork/graph/rrf)
- Filter support: doc_type, status, effective_date range, relation_types

## Capabilities

### New Capabilities

- `gmp-hybrid-retrieval`: Multi-signal retrieval (vector + keyword + graph) with RRF fusion and full citation provenance
