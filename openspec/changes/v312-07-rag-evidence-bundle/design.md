# Design: V312-07 RAG Evidence Bundle

## Decisions

### Decision: SHA-256 evidence hash

`evidence_hash = SHA256(citations.map(|c| c.chunk_id|chunk_hash|content_snippet).join("|"))`. This makes it possible to verify that a given answer was generated from a specific set of citations.

### Decision: fail_on_uncited = true

When an answer cannot be verified against citations (empty corpus, hash mismatch), return `Err` instead of fabricating content. This is the "fail closed" requirement.

### Decision: Snippet extraction

Sentences are split on `.!?` boundaries. Window of N sentences is taken per citation, truncated to 200 chars.
