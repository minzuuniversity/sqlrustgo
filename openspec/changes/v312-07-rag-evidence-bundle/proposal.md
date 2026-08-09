# Proposal: V312-07 RAG Evidence Bundle

## Why

Every GMP RAG answer must be traceable and verifiable. V312-07 ensures all answers are grounded in retrieved chunks with citation bundles, and fails closed when citations don't verify.

## What Changes

- `rag.rs`: New module — `Citation`, `CitationBundle`, `AnswerEnvelope`, `generate_rag_answer`, `rag_answer`
- `CitationBundle.compute_evidence_hash()`: SHA-256 of all citation content
- `AnswerEnvelope.is_grounded()`: verifies bundle + hash
- `fail_on_uncited`: when true, returns Err if answer can't be verified

## Capabilities

### New Capabilities

- `gmp-rag-evidence-bundle`: Every RAG answer carries a citation bundle with chunk_id, chunk_hash, content_snippet, evidence_hash
- `gmp-fail-closed`: Unverified answers return Err instead of fabricated content
