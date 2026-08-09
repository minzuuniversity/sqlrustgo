# GMP Embedding Provider

## CHANGED Requirements

### Requirement: Embeddings table has model metadata

The `gmp_embeddings` table contains `chunk_id`, `embedding`, `updated_at`, `model_name`, `dimension`, `vector_hash`.

#### Scenario: Embedding storage with full metadata
- **WHEN** `upsert_embedding(storage, chunk_id, embedding)` is called
- **THEN** the row stores model_name="hash", dimension=256, vector_hash=SHA256(bytes)

### Requirement: Vector hash is deterministic

Same vector always produces same hash.

#### Scenario: Vector hash consistency
- **WHEN** `vector_hash([1.0, 0.0, 0.0])` is called twice
- **THEN** both calls return the same 64-character hex string

### Requirement: Flat index search returns top-k by cosine similarity

#### Scenario: Flat index search
- **WHEN** a flat index is built with 2 vectors and searched with top_k=1
- **THEN** the most similar vector (highest cosine similarity) is returned

## ADDED Requirements

### Requirement: Index rebuild creates metadata record

`rebuild_flat_index` inserts a row into `gmp_vector_index` with index_type, model_name, dimension, embedding_count, built_at.

#### Scenario: Index rebuild
- **WHEN** `rebuild_flat_index(storage, "hash")` is called
- **THEN** `get_latest_index(storage)` returns a `VectorIndexMeta` with index_type="FLAT"

### Requirement: Ollama provider configurable

`OllamaConfig` has `base_url`, `model_name` fields. `ProviderFactory::create_ollama` constructs it from config.

#### Scenario: Ollama provider from config
- **WHEN** `ProviderFactory::create_ollama(&OllamaConfig::default())` is called
- **THEN** it returns an `OllamaEmbeddingProvider` using the configured base_url and model
