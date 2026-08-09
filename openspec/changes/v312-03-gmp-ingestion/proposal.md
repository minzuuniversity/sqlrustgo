# Proposal: V312-03 GMP Corpus Ingestion

## Why

The GMP corpus at `~/gmp-platform/gmp-md` (1697 markdown files, 9.3 MB) must be ingested into SQLRustGo's new GMP tables (from V312-02) to provide the RAG, graph, and evidence navigation features in V312-05 through V312-07.

## What Changes

- `ingestion.rs`: New module providing `ingest_file` and `ingest_corpus` functions.
- Each file is chunked, hashed (SHA-256), and stored with source_path preserved.
- Re-ingestion with identical content is idempotent (same `source_hash` skips re-import).
- Content changes produce a new document version.
- Ingestion report tracks document/chunk/embedding/skipped/failure counts.

## Capabilities

### New Capabilities

- `gmp-corpus-ingestion`: Batch import of filesystem corpus into GMP tables with idempotency and version tracking.
