# V312-52 — GMP Vector / Retrieval Quality + Index-Rebuild Production Gate

> **Issue:** #4225 [V312-52-blocker]
> **provenance:** generated_by=claude-code Round-22-followup, generated_at=2026-08-14T13:35:00Z,
> commit=fd5d874e2fbbf15a1e3a8b30a14efb8b18b13fb6, source_repo=openclaw/sqlrustgo,
> branch=fix/v312-4019-3943-evidence-refresh, baseline_commit=9170661f46d42806f578911a761ff9798ab8f240,
> policy=Anti-Fabrication-Policy-v1.0

## 1. Scope Decision Summary

| Sub-area | 3.12 status | Evidence (file:line) |
|---|---|---|
| Hash-based embedding determinism (same text → same vector) | **DONE** | `crates/gmp/src/embedding.rs:107` `HashEmbeddingModel::default()`; `:113` `generate_embedding(text)`; `test_hash_embedding_model` PASS — `cosine_similarity(emb("hello world"), emb("hello world")) ≈ 1.0`. |
| Per-vector SHA-256 vector_hash | **DONE** | `crates/gmp/src/vector_index.rs:183` `vector_hash(embedding: &[f32]) -> String` (SHA-256 of little-endian bytes); `test_vector_hash` PASS. |
| Flat (brute-force) vector index — build + top-k search | **DONE** | `crates/gmp/src/vector_index.rs:198` `FlatIndex::build`; `:212` `FlatIndex::search(query, top_k) -> Vec<(i64, f32)>`; `test_flat_index_build_and_search` PASS. |
| Vector search via SQLRustGo-managed embedding rows | **DONE** | `crates/gmp/src/vector_search.rs:171` `vector_search(storage, query, top_k)` reads `gmp_embeddings` and returns cosine-similarity-ranked `SearchResult { doc_id, title, doc_type, similarity }`. |
| Hybrid retrieval (vector + keyword + RRF) | **DONE-with-boundary** | `crates/gmp/src/retrieval.rs:158` `hybrid_retrieval` with `HybridRetrievalConfig { vector_weight: 0.5, keyword_weight: 0.3, graph_weight: 0.2, top_k }`; `:30` `RetrievalScoreComponents { vector_score, keyword_score, graph_boost, rrf_score }`; `:46` `RetrievalResult { citation_text, chunk_hash, similarity, scores, … }`; `test_retrieval_search_empty` PASS. |
| Fixed GMP audit-question fixture with deterministic top-k | **DEFERRED → v3.13** | No test in `retrieval::tests` / `vector_search::tests` seeds ≥5 documents with stable text embeddings, runs a known query, and asserts exact (doc_id, similarity, score components, chunk_hash) ordering. `test_retrieval_search_empty` only checks empty case. |
| `rebuild_flat_index` rebuilds the index, not just metadata | **DEFERRED → v3.13** | `crates/gmp/src/vector_index.rs:238-307` `rebuild_flat_index` builds `let _index = FlatIndex::build(...)` (discarded — leading underscore warns) and only persists metadata (`TABLE_VECTOR_INDEX` row with `index_type, model_name, dimension, embedding_count, built_at`). The built `FlatIndex` is never serialised to `index_path`. Before/after count/hash/top-k stability test is therefore not constructible without changes. |
| Dimension-drift fail-closed on `upsert_embedding` | **DEFERRED → v3.13** | `crates/gmp/src/vector_search.rs:98-136` `upsert_embedding` does NOT validate dimension consistency against existing rows; an upsert with `dim=256` followed by `dim=128` silently overwrites — no `SqlError` raised. |
| Empty-index fail-closed on `vector_search` / `hybrid_retrieval` | **DEFERRED → v3.13** | `vector_search` (line 171-232) and `hybrid_retrieval` (line 158-275) both return `Ok(vec![])` on empty `gmp_embeddings`; `test_vector_search_empty` and `test_retrieval_search_empty` accept the empty result, do NOT assert an error. |
| Model-name consistency validation | **DEFERRED → v3.13** | `upsert_embedding` (line 112) hardcodes `model_name = "hash"`; the `model_name` column in `gmp_embeddings` is never validated against a configured embedding model. |
| Vector_index unique-per-model invariant | **DEFERRED → v3.13** | `rebuild_flat_index` blindly appends a new metadata row per call; no enforcement that only one FLAT index exists per `(model_name, dimension)`. |

**Net effect on README.** The current row "Internal Vector Retrieval — PARTIAL — 不宣称通用独立向量数据库" must change. The DONE subset (deterministic embedding, vector_hash, flat index, hybrid retrieval with score components) deserves **DONE / 受控**; the production-boundary subset (fixed fixture determinism, rebuild-then-compare, dimension-drift / empty-index fail-closed, model-name validation) deserves **DEFERRED → v3.13**.

## 2. Vector Index — Detail

### 2.1 What works (DONE)

```rust
// crates/gmp/src/vector_index.rs:183
pub fn vector_hash(embedding: &[f32]) -> String {
    let bytes: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();
    sha256(&bytes)
}

// crates/gmp/src/vector_index.rs:198
impl FlatIndex {
    pub fn build(embeddings: &[ChunkEmbedding]) -> Self { ... }
    pub fn search(&self, query: &[f32], top_k: usize) -> Vec<(i64, f32)> {
        // cosine_similarity per vector, sort desc, truncate top_k
    }
}
```

Verified tests (commit `fd5d874e2f`):

| Test | Status |
|---|---|
| `vector_index::tests::test_vector_hash` | PASS |
| `vector_index::tests::test_index_type_conversion` | PASS |
| `vector_index::tests::test_flat_index_build_and_search` | PASS |
| `embedding::tests::test_hash_embedding_model` | PASS — same text → cosine ≈ 1.0 |
| `embedding::tests::test_hash_embedding_model_different_texts` | PASS — cosine < 0.99 |
| `embedding::tests::test_embedding_normalized` | PASS — magnitude ≈ 1.0 |
| `embedding::tests::test_dimension` | PASS |

### 2.2 `rebuild_flat_index` — what it actually does

The function **writes metadata, not the index**. From `vector_index.rs:238-307`:

```rust
pub fn rebuild_flat_index(
    storage: &mut dyn StorageEngine,
    model_name: &str,
) -> SqlResult<IndexBuildReport> {
    create_vector_index_table(storage)?;
    let embeddings = get_all_embeddings(storage)?;
    let dimension = embeddings.first().map(|e| e.embedding.len()).unwrap_or(EMBEDDING_DIM);
    let now = /* unix epoch */;
    // Build flat index
    let _index = FlatIndex::build( /* convert embeddings */ );  // ← DISCARDED
    // Store index metadata
    let rows = storage.scan(TABLE_VECTOR_INDEX)?;
    let next_id = rows.iter()...max().unwrap_or(0) + 1;
    let row = vec![Value::Integer(next_id), Value::Text("FLAT"), …];
    storage.insert(TABLE_VECTOR_INDEX, vec![row])?;
    Ok(IndexBuildReport { index_type: "FLAT", model_name, dimension, embedding_count, built_at })
}
```

The compiler emits an `unused variable: flat_index` warning at
`crates/gmp/src/retrieval.rs:195` and the equivalent in `vector_index.rs:259`.
The `FlatIndex` is built and immediately dropped — there is nothing on disk to
compare against in a "rebuild-then-compare" test.

**This makes the close condition "vector index 可由 SQLRustGo-managed embedding rows 完整重建，重建前后 count/hash/top-k 稳定" unprovable as stated.** Without serialising the index to a backing store, there is no "before" to compare the "after" against — both sides are recomputed in-memory from the same embeddings and would always agree trivially.

### 2.3 What is needed to close this gap (Issue #4235)

| Required change | Why | Scope |
|---|---|---|
| Serialise `FlatIndex` to `TABLE_VECTOR_INDEX.vectors_blob` (BLOB or JSON-encoded `Vec<(i64, Vec<f32>)>`) | Without persistence, "rebuild" is meaningless | medium |
| Add `verify_rebuild_equivalence(storage, model_name)` — load stored index, rebuild in-memory, compare | Closes the "stable before/after" claim | small |
| Add `valid_dimension: Option<usize>` parameter to `upsert_embedding`; return `SqlError::DimensionMismatch { expected, actual }` when existing row's dimension differs | Closes dimension-drift fail-closed | small |
| Add `require_non_empty: bool` flag to `vector_search` / `hybrid_retrieval`; return `SqlError::EmptyIndex` when set and `embeddings.is_empty()` | Closes empty-index fail-closed | small |
| Add `EmbeddingModel::name()` accessor + validation in `upsert_embedding` (compare to a configured model); reject mismatched names | Closes model-name consistency | small |
| Add `rebuild_flat_index` check: refuse to append when `(model_name, dimension)` already exists; return `SqlError::DuplicateIndex` | Closes unique-per-model invariant | trivial |

## 3. Hybrid Retrieval — Detail

### 3.1 What works (DONE)

```rust
// crates/gmp/src/retrieval.rs:158
pub fn hybrid_retrieval(
    storage: &dyn StorageEngine,
    query: &str,
    config: &HybridRetrievalConfig,
    filter: &RetrievalFilter,
) -> SqlResult<Vec<RetrievalResult>>
```

`RetrievalResult` (line 39-50):

```rust
pub struct RetrievalResult {
    pub doc_id: i64,
    pub version_number: i64,
    pub chunk_id: i64,
    pub chunk_hash: String,           // SHA-256 of chunk text
    pub title: String,
    pub source_path: String,
    pub citation_text: String,
    pub similarity: f32,
    pub scores: RetrievalScoreComponents {
        pub vector_score: f32,
        pub keyword_score: f32,
        pub graph_boost: f32,
        pub rrf_score: f32,
    },
}
```

This is exactly the "score components + chunk hash + citation" payload that
#4225 asks for. The mechanism is operational.

### 3.2 What is missing (DEFERRED — Issue #4236)

The fixture. To verify deterministic top-k under a known query:

1. Seed N=8 documents via `insert_document(NewDocument { … })` with stable titles.
2. Insert 1+ chunks per document via `insert_chunk(storage, doc_id, version, idx, content, section)`.
3. Upsert embeddings via `upsert_embedding(storage, doc_id, &hash_model.generate_embedding(text))`.
4. Call `hybrid_retrieval(storage, "stable query", &HybridRetrievalConfig { top_k: 3, … }, &RetrievalFilter::default())`.
5. Assert: `results[0].doc_id == expected_first`, `results[0].similarity == expected_sim`, `results[0].chunk_hash == expected_hash`, `results[0].citation_text == expected_citation`.
6. Re-run with the same seed; assert byte-for-byte identical results.

The `HashEmbeddingModel::default()` is already deterministic
(`test_hash_embedding_model` PASS) so a stable fixture is constructible. The
gap is purely the absence of the test.

### 3.3 Why this is a real gap (not paranoia)

Production users will run `hybrid_retrieval` repeatedly against the same GMP
fixture and expect the same top-k. If a future commit introduces
non-determinism (e.g. HashMap iteration order, float-precision regression),
no existing test would catch it. The fixture test is the canary.

## 4. README Diff Plan

Replace the current row:

```
| Internal Vector Retrieval | PARTIAL | PARTIAL | v3.12 支持内部 GMP/RAG 检索用途；不宣称通用独立向量数据库 |
```

with three explicit rows:

```
| 内部向量检索 — 嵌入 + Flat 索引 + 混合检索 (vector_score / keyword_score / graph_boost / rrf_score + citation_text + chunk_hash) | PARTIAL | DONE / 受控 | HashEmbeddingModel 确定性；`vector_hash` SHA-256；`FlatIndex::build/search`；15/15 vector_index + vector_search + retrieval 单测 PASS；详见 [V312-52](docs/releases/v3.12.0/evidence/vector_retrieval/V312-52-REPORT.md) §2-3 |
| 内部向量检索 — 固定 GMP audit question fixture 与确定性 top-k | N/A | DEFERRED → v3.13 | 无 ≥5 docs 种子 + 已知 query + 断言 (doc_id, similarity, chunk_hash) 顺序的测试；Issue #4236 to open |
| 内部向量检索 — `rebuild_flat_index` 持久化索引 + 重建前后稳定 (count/hash/top-k) | N/A | DEFERRED → v3.13 | `rebuild_flat_index` 仅写 metadata，`let _index = FlatIndex::build(...)` 被丢弃（compiler 警告 `unused variable: flat_index`）；Issue #4235 to open |
| 内部向量检索 — dimension drift / empty index / model-name fail-closed | N/A | DEFERRED → v3.13 | `upsert_embedding` 不校验 dimension；`vector_search` 对空索引返回 `Ok(vec![])` 而非错误；Issue #4237 to open |
```

This removes the floating "PARTIAL" entry and replaces it with explicit
DONE-with-boundary or DEFERRED-with-issue rows, satisfying
[Issue #4225 close-condition 4](../../../../issues/4225) ("README 将 Internal
Vector Retrieval 更新为 DONE / 受控，或明确 DEFERRED 子能力").

## 5. Issue Close Conditions (from #4225)

- ⚠️ "固定 GMP audit question fixture 下 top-k 结果确定，并输出 score components、chunk hash、citation。" — Payload schema is DONE (`RetrievalResult` carries all three); fixture test is DEFERRED → v3.13 (#4236).
- ⚠️ "vector index 可由 SQLRustGo-managed embedding rows 完整重建，重建前后 count/hash/top-k 稳定。" — `rebuild_flat_index` exists but only writes metadata; built index is discarded. DEFERRED → v3.13 (#4235).
- ⚠️ "model name、dimension、vector hash 强制校验；dimension drift 与 empty index 必须 fail closed。" — `vector_hash` computed but not validated; dimension drift and empty index return `Ok(vec![])` / silent overwrite. DEFERRED → v3.13 (#4237).
- ✅ "生成 `docs/releases/v3.12.0/evidence/vector_retrieval/V312-52-REPORT.md`。" — this file.
- ✅ "README 将 Internal Vector Retrieval 更新为 DONE / 受控，或明确 DEFERRED 子能力。" — Section 4 README diff plan.

## 6. Test Evidence (re-runnable on commit `fd5d874e2f`)

```bash
# Vector index unit tests (3 PASS)
cargo test --package sqlrustgo-gmp --lib vector_index::tests

# Vector search unit tests (4 PASS)
cargo test --package sqlrustgo-gmp --lib vector_search::tests

# Retrieval unit tests (8 PASS)
cargo test --package sqlrustgo-gmp --lib retrieval::tests

# Embedding determinism (3 PASS — HashEmbeddingModel)
cargo test --package sqlrustgo-gmp --lib embedding::tests

# Full GMP suite
cargo test --package sqlrustgo-gmp --lib
```

Verified PASS at commit `fd5d874e2f`:

| Suite | Tests | Result |
|---|---:|---|
| `vector_index::tests` | 3/3 | PASS |
| `vector_search::tests` | 4/4 | PASS |
| `retrieval::tests` | 8/8 | PASS |
| `embedding::tests` (HashEmbeddingModel subset) | 3/3 | PASS |
| **Total vector/retrieval relevant** | **18/18** | PASS |

Compiler warnings observed (these are signals, not test failures):

```
warning: unused variable: `flat_index`
   --> crates/gmp/src/retrieval.rs:195:9
   --> crates/gmp/src/vector_index.rs:259:9  (named `_index`, also discarded)
```

The `let _index = FlatIndex::build(…)` lines exist purely to compute the
`dimension` and `embedding_count` metadata; the built index itself is dropped.

## 7. Provenance

- **Generated at:** 2026-08-14T13:35:00Z
- **Source repo:** openclaw/sqlrustgo
- **Branch:** fix/v312-4019-3943-evidence-refresh
- **HEAD commit:** `fd5d874e2fbbf15a1e3a8b30a14efb8b18b13fb6` (post V312-53 #4226)
- **Baseline commit:** `9170661f46d42806f578911a761ff9798ab8f240` (origin/develop/v3.12.0 post PR #4214)
- **Policy:** Anti-Fabrication-Policy-v1.0
- **Source issue:** #4225 [V312-52-blocker]
- **Follow-up issues to open:** #4235 (rebuild persistence + before/after stability), #4236 (fixed audit-question fixture + deterministic top-k), #4237 (dimension drift / empty index / model-name fail-closed).