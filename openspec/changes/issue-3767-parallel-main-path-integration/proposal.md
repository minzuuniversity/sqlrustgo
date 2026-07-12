# Proposal — issue-3767-parallel-main-path-integration

> **Status**: Proposed (Issue #3767, V310-13 / I-12)
> **Goal**: Wire `ParallelVolcanoExecutor` into the production query execution path so that `--executor-parallelism=N>1` actually delivers N-way parallel scans/joins/aggregates end-to-end.

## Why

The 1762-line `ParallelVolcanoExecutor` implementation has been sitting in
`crates/executor/src/parallel_executor.rs` since v3.9.0, but the production
execution path never calls it. `INT5_PLUS_DEBT_INVENTORY.md §I-12` marks this as
"⚠️ PARTIAL (6/6 mock tests pass, but no main-path verification)".

As of #3703/#3735/#3736/#3737 work landed in v3.10.0:
- `StorageEngine::parallel_scan` (Memory + File) — **DONE**
- `ThreadPoolRegistry` + per-query explicit pools — **DONE**
- `PipelineExecutor` (storage + pool + filter) — **DONE**
- `parallel_group_by` (hash-partition + partial aggregates + merge) — **DONE**
- `parallel_hash_join` (partition + probe) — **DONE**
- `CancellationToken` + `ScopedCancellation` — **DONE**
- `UnifiedCostModel::should_parallelize_with(plan, for_update)` — **DONE**
- `LockClause` parser (FOR UPDATE / LOCK IN SHARE MODE) — **DONE**
- `engine_select.rs` parallel filter guard (parallel_degree > 1 + rows >= PARALLEL_MIN_ROWS + no correlated subquery + no FOR UPDATE) — **DONE**

What's missing per the issue body:
1. **`pub mod parallel_executor` declaration in `crates/executor/src/lib.rs`** — 
   ALREADY PRESENT (line 12), but the issue says it was missing. Verify it stays.
2. **`parallel_degree` configuration option** — exists via `--executor-parallelism`
   CLI flag and `SQLRUSTGO_EXECUTOR_PARALLELISM` env var, read at 
   `ExecutionEngine::new` time. `set_parallel_degree(N)` is a public API.
3. **End-to-end test verifying parallel path engages in production** —
   missing. INT-2 unit tests pass in isolation but no E2E test confirms the
   path engages when running a real query through `ExecutionEngine`.

This change proposes:
- A new E2E test that runs TPC-H Q1 with `parallel_degree=4` and verifies
  the parallel filter engages (via tracing span emission + result equivalence
  vs `parallel_degree=1`).
- Documentation update in `EXECUTION_PIPELINE_REFACTORING.md` or equivalent
  clarifying that the `PipelineExecutor` will slot into the new 
  `execute_select` pipeline (separately from the current guard-based parallel filter).

## Goals

- **G1**: Verify (with evidence) that setting `parallel_degree=4` actually 
  engages the parallel filter path in `engine_select.rs::execute_select`.
- **G2**: Add E2E test `tests/parallel_main_path_test.rs` covering Q1 with
  `parallel_degree ∈ {1, 2, 4, 8}` and asserting result equivalence.
- **G3**: Document the main-path integration status.

## Non-Goals

- **N1**: Wiring `PipelineExecutor` into old `engine_select.rs` — explicitly 
  OUT OF SCOPE per the Integration Guide red-line (would couple new pool-based
  path with old thread-per-connection model).
- **N2**: Tuning CBO selectivity thresholds.
- **N3**: True disk-level `FileStorage::parallel_scan` (requires
  `get_partition_boundaries()`).

## Migration Path

This change is **additive only**. Existing sequential path remains untouched.
The current guard-based parallel filter at `engine_select.rs:262` stays.

After this PR:
- `parallel_degree=1` (default) → existing sequential path
- `parallel_degree=N>1` → existing guard-based parallel filter path
- (Future work) `parallel_degree=N>1` + new `execute_select` → `PipelineExecutor`

## Refs

- Issue #3767, #3703, #3735/#3736/#3737
- `INT5_PLUS_DEBT_INVENTORY.md §I-12`
- `V310_DEVELOPMENT_PLAN.md`
- `openspec/changes/issue-3703-intra-query-parallel-executor/` (parent change)
