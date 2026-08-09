# GMP Hybrid Retrieval

## ADDED Requirements

### Requirement: Hybrid retrieval combines vector, keyword, and graph signals

`hybrid_retrieval` returns results scored by `vector_weight * vector_score + keyword_weight * keyword_score + graph_weight * graph_boost`.

#### Scenario: Retrieval with all signals
- **WHEN** `hybrid_retrieval` is called with default weights (0.5, 0.3, 0.2)
- **THEN** each result's `similarity` field reflects the weighted combination

### Requirement: RetrievalResult includes chunk-level citation

Each `RetrievalResult` contains `chunk_id`, `chunk_hash`, `citation_text` for direct RAG citation.

#### Scenario: Citation extraction
- **WHEN** `hybrid_retrieval` returns results for a document with chunks
- **THEN** `citation_text` is the text of the first chunk

### Requirement: Retrieval filter by status

`RetrievalFilter.statuses` filters results to matching document statuses.

#### Scenario: Active-only retrieval
- **WHEN** `RetrievalFilter.statuses = ["ACTIVE"]`
- **THEN** only documents with `DocStatus::Active` are returned

### Requirement: Retrieval filter by effective date range

`RetrievalFilter.effective_date_from/to` filters by document effective date.

#### Scenario: Date range filter
- **WHEN** `effective_date_from = 20250000` and `effective_date_to = 20300000`
- **THEN** documents with `effective_date` outside that range are excluded

### Requirement: Keyword score is fraction of query terms found in title or doc_type

#### Scenario: Keyword score computation
- **WHEN** query = "rust programming" and title = "Rust Guide"
- **THEN** keyword_score = 0.5 (1 of 2 terms match: "rust" in title, "programming" not found)

### Requirement: Score breakdown in result

`RetrievalResult.scores` contains `vector_score`, `keyword_score`, `graph_boost`, `rrf_score`.

#### Scenario: Score transparency
- **WHEN** a result is returned
- **THEN** `result.score_summary()` returns a string like "v0.60/k0.30/g0.10/rrf0.75"
