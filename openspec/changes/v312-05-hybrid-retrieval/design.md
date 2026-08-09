# Design: V312-05 Hybrid Retrieval

## Context

GMP audit questions need to retrieve specific evidence from the corpus. The retrieval system must:
1. Score documents by vector similarity (semantic matching)
2. Score by keyword overlap (exact term matching)
3. Boost documents with graph relations (network effect)
4. Fuse scores via RRF for unified ranking

## Decisions

### Decision: RRF with k=60

Standard IR practice: `rrf_score = sum(1 / (k + rank))`. Default k=60 works well for top-k retrieval with noisy signals.

### Decision: Weighted average as base score

Before RRF, we use weighted average: `0.5 * vector + 0.3 * keyword + 0.2 * graph`. RRF is applied to the ranked list as a re-ranking step.

### Decision: RetrievalResult includes chunk-level citation

Each result carries `chunk_id`, `chunk_hash`, and `citation_text` for direct RAG evidence citation without further lookup.

### Decision: Flat index for vector search

Reuses `FlatIndex` from V312-04 for fast cosine similarity computation over in-memory embeddings.
