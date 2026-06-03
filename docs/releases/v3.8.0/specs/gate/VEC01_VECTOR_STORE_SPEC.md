# VEC-01: Vector Store SQL Integration SPEC

**Issue**: #2990
**Status**: DRAFT (Phase 1 of 4)
**Estimated effort**: ~40h
**Target version**: v3.9.0 (Alpha integration)

## Background

`crates/vector/src/` contains HNSW, Flat, IVF, IVFPQ, parallel kNN, and
hybrid SQL-vector types. The unified storage layer has
`add_hnsw_vector_index` at `crates/unified-storage/src/unified_index.rs:177`.

**Gap**: no SQL surface for vector indexes. Users cannot run:

```sql
CREATE VECTOR INDEX idx ON items USING hnsw(embedding) WITH (dim=384, m=16);
SELECT id FROM items ORDER BY vector_distance(embedding, '[...]') LIMIT 10;
```

## Phase 1: SPEC (this document) [DONE in v3.8.0]

## Phase 2: Parser + AST [~8h]

1. Add `Token::Vector`, `Token::Distance` to `crates/parser/src/token.rs`
2. Add `CreateVectorIndexStatement` to `crates/parser/src/parser.rs`
3. Extend `Expression` with `VectorDistance { column, vector }`
4. Add a `USING hnsw(...)` clause to `CreateIndexStatement`
5. Tests: `crates/parser/tests/vector_index_test.rs`

## Phase 3: Planner + Executor [~16h]

1. Add `VectorIndexExec` and `VectorScanExec` to `crates/planner`
2. Wire into `ExecutionEngine::execute_show` for `SHOW VECTOR INDEXES`
3. Route `ORDER BY vector_distance(...) LIMIT N` through
   `crates/vector/src/hnsw.rs` for KNN search
4. Tests: `crates/executor/tests/vector_search_test.rs`

## Phase 4: Catalog + Storage [~12h]

1. Add vector index metadata to `sqlrustgo_catalog`
2. Persist index path + params in `crates/unified-storage`
3. Reuse `add_hnsw_vector_index` for index creation
4. Tests: `crates/vector/tests/integration_test.rs`

## Phase 5: e2e + docs [~4h]

1. Add e2e test that creates a vector index, inserts 10k rows,
   runs KNN search, asserts results match brute force
2. Document the SQL surface in
   `docs/USER_GUIDE/VECTOR_SEARCH.md`

## Acceptance Criteria

- All 5 phases complete
- `CREATE VECTOR INDEX ... USING hnsw(...)` parses and executes
- `ORDER BY vector_distance(...) LIMIT N` returns top-N neighbors
- e2e test passes (10k rows, 384-dim, KNN recall ≥ 0.95)

## Risks

- Multi-crate change (parser + planner + executor + storage).
  Mitigation: incremental merges per phase.
- Planner integration may conflict with INT-2 (parallel executor).
  Mitigation: coordinate on `ExecutionEngine::execute_physical` in
  Phase 3.

## Related

- `crates/vector/README.md` (current API)
- Issue #2990
- INT-2 (parallel executor) — shares `execute_physical` boundary
