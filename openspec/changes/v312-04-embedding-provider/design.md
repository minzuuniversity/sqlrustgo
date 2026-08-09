# Design: V312-04 Embedding Provider

## Context

The `gmp_embeddings` table originally stored only `(doc_id, embedding, updated_at)`. V312-04 extends it to `(chunk_id, embedding, updated_at, model_name, dimension, vector_hash)`.

## Decisions

### Decision: chunk_id as primary key

Each chunk gets its own embedding row. `doc_id` is no longer the PK — `chunk_id` is. This enables chunk-granular RAG citations.

### Decision: vector_hash for integrity

`vector_hash = SHA256(bytes_of_float_vector)` lets callers verify embedding integrity without storing the raw bytes.

### Decision: Flat index in-memory

HNSW requires external crate (`hnswlib`/`usearch`). We stub `VectorIndexType::Hnsw` for future, implement `FlatIndex` now using `cosine_similarity` over in-memory vectors.

### Decision: model_name stored with each embedding

Allows auditing which model generated which embedding. Currently hardcoded to `"hash"` for the built-in `HashEmbeddingModel`.
