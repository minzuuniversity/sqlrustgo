# V312-04 Embedding Provider — Implementation Tasks

## 1. Implementation

- [x] 1.1 `vector_index.rs` — `FlatIndex`, `VectorIndexMeta`, `VectorIndexType`, `ChunkEmbedding`, `vector_hash`, `rebuild_flat_index`, `get_latest_index`
- [x] 1.2 `vector_search.rs` — Updated `create_embeddings_table` with `chunk_id`, `model_name`, `dimension`, `vector_hash` columns
- [x] 1.3 `vector_search.rs` — Updated `upsert_embedding` to store model metadata and vector hash
- [x] 1.4 `embedding.rs` — Updated `CREATE_EMBEDDINGS_TABLE` SQL with new columns
- [x] 1.5 `ingestion.rs` — Updated to pass `chunk_id` to `upsert_embedding`
- [x] 1.6 `lib.rs` — Added `pub mod vector_index; pub mod sql_api;` (was missing)

## 2. Tests

- [x] 2.1 `test_flat_index_build_and_search` — flat index search returns correct top-k
- [x] 2.2 `test_vector_hash` — vector hash is deterministic and 64-char hex
- [x] 2.3 `test_index_type_conversion` — VectorIndexType::from_str and as_str work
- [x] 2.4 All 105 GMP tests pass

## 3. OpenSpec

- [x] 3.1 Create `v312-04-embedding-provider` change
- [x] 3.2 Write `proposal.md`, `design.md`, spec, tasks
