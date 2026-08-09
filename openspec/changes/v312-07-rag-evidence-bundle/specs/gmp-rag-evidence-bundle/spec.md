# GMP RAG Evidence Bundle

## ADDED Requirements

### Requirement: CitationBundle computes deterministic evidence hash

`compute_evidence_hash` returns SHA-256 of concatenated citation identifiers and content.

#### Scenario: Evidence hash determinism
- **WHEN** `compute_evidence_hash` is called twice with same citations
- **THEN** both calls return the same 64-character hex string

### Requirement: CitationBundle.verify checks hash integrity

`verify` returns `true` iff citations are non-empty and stored hash matches computed hash.

#### Scenario: Hash verification
- **WHEN** a CitationBundle is created and `verify()` is called
- **THEN** it returns `true` if the bundle has citations and hash matches

### Requirement: AnswerEnvelope.is_grounded requires verified bundle

`is_grounded` returns `true` only when `citation_bundle.verify()` is `true` and `grounded` is `true`.

#### Scenario: Grounded answer
- **WHEN** `AnswerEnvelope` has a verified bundle and `grounded = true`
- **THEN** `is_grounded()` returns `true`

### Requirement: Empty corpus returns Err (fail closed)

`rag_answer` on an empty corpus returns `Err("I could not find relevant evidence...")`.

#### Scenario: No evidence
- **WHEN** `rag_answer` is called on a storage with no documents
- **THEN** the returned result is `Err` with a message about no evidence found

### Requirement: Answer includes citation snippets

The generated answer text concatenates content snippets from all citations.

#### Scenario: Snippet concatenation
- **WHEN** citations have snippets ["First fact.", "Second fact."]
- **THEN** answer = "Based on the evidence: First fact. Second fact."
