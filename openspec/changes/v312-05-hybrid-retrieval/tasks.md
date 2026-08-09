# V312-05 Hybrid Retrieval — Implementation Tasks

## 1. Implementation

- [x] 1.1 `retrieval.rs` — `HybridRetrievalConfig`, `RetrievalFilter`, `RetrievalResult`, `RetrievalScoreComponents`
- [x] 1.2 `retrieval.rs` — `hybrid_retrieval` combining vector/keyword/graph with weighted average
- [x] 1.3 `retrieval.rs` — `retrieval_search` (active-only convenience wrapper)
- [x] 1.4 `retrieval.rs` — `keyword_score`, `document_matches_filter`, `rrf_score`
- [x] 1.5 `retrieval.rs` — chunk-level citation extraction for RAG
- [x] 1.6 `lib.rs` — Added `pub mod vector_index;` and `pub mod retrieval;`

## 2. Tests

- [x] 2.1 `test_rrf_score` — RRF formula correct
- [x] 2.2 `test_keyword_score` — keyword matching works
- [x] 2.3 `test_keyword_score_no_match` — no match returns 0
- [x] 2.4 `test_document_matches_filter_type/status/effective_date`
- [x] 2.5 `test_retrieval_result_score_summary`
- [x] 2.6 `test_retrieval_search_empty`
- [x] 2.7 All 113 GMP tests pass

## 3. OpenSpec

- [x] 3.1 Create `v312-05-hybrid-retrieval` change
- [x] 3.2 Write `proposal.md`, `design.md`, spec, tasks
