# Proposal: V312-04 Embedding Provider

## Why

GMP embeddings need to be stored with full provenance: which model generated them, vector hash for integrity, and chunk-level references (not just document-level). V312-03's ingestion creates chunk embeddings; V312-04 adds the storage layer with index management.

## What Changes

- `vector_index.rs`: New module — `FlatIndex`, `VectorIndexMeta`, `rebuild_flat_index`, `vector_hash`
- `vector_search.rs`: Updated `upsert_embedding` to use `chunk_id` + store `model_name`, `dimension`, `vector_hash`
- `embedding.rs`: Updated `CREATE_EMBEDDINGS_TABLE` SQL with new columns
- `ingestion.rs`: Updated to pass `chunk_id` to `upsert_embedding`

## Capabilities

### New Capabilities

- `gmp-embedding-index-rebuild`: Rebuild flat index from stored embeddings with model metadata tracking
- `gmp-vector-hash`: SHA-256 hash of raw vector for integrity verification
