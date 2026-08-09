# Design: V312-03 GMP Corpus Ingestion

## Context

GMP corpus at `~/gmp-platform/gmp-md` has 1697 markdown files. Ingestion must:
1. Walk directory tree, filter `.md`/`.txt`
2. For each file: chunk, hash, store document+chunks+embeddings
3. Idempotent re-import via `source_hash`
4. Content change → new version

## Decisions

### Decision: SHA-256 for source_hash

Same as V312-02 schema — consistent hash function across the system.

### Decision: Re-ingestion is idempotent by source_hash

If a file with the same `source_hash` is ingested again, it is skipped. This prevents duplicate versions when the corpus is re-processed.

### Decision: Simple walkdir instead of external crate

Uses `fs::read_dir` recursion to avoid adding `walkdir` crate dependency.

## Schema

Corpus files → `gmp_documents` → `gmp_document_versions` → `gmp_chunks` → `gmp_embeddings`
