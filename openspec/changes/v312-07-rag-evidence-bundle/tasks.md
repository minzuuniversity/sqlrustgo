# V312-07 RAG Evidence Bundle — Implementation Tasks

## 1. Implementation

- [x] 1.1 `rag.rs` — `Citation`, `CitationBundle`, `AnswerEnvelope` with Serialize/Deserialize
- [x] 1.2 `rag.rs` — `CitationBundle.compute_evidence_hash` (SHA-256 of citation content)
- [x] 1.3 `rag.rs` — `CitationBundle.verify` (hash integrity check)
- [x] 1.4 `rag.rs` — `generate_rag_answer` with fail_on_uncited
- [x] 1.5 `rag.rs` — `rag_answer` convenience wrapper
- [x] 1.6 `rag.rs` — `extract_snippet` with sentence window and truncation
- [x] 1.7 `lib.rs` — Added `pub mod rag;`

## 2. Tests

- [x] 2.1 `test_citation_bundle_empty`, `test_citation_bundle_verify_empty`
- [x] 2.2 `test_answer_envelope_summary`, `test_answer_envelope_not_grounded`
- [x] 2.3 `test_extract_snippet_multiple_sentences/single_sentence/truncation`
- [x] 2.4 `test_rag_answer_empty_corpus`
- [x] 2.5 All 130 GMP tests pass

## 3. OpenSpec

- [x] 3.1 Create `v312-07-rag-evidence-bundle` change
- [x] 3.2 Write `proposal.md`, `design.md`, spec, tasks
