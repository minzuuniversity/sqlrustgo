# Design — issue-3767-parallel-main-path-integration

## Architecture

The main-path integration has **3 layers**:

```
┌──────────────────────────────────────┐
│ Layer 1: CLI / env                    │
│ --executor-parallelism=N (WIRED ✓)    │
│ SQLRUSTGO_EXECUTOR_PARALLELISM (✓)    │
└──────────────────────────────────────┘
                ↓
┌──────────────────────────────────────┐
│ Layer 2: ExecutionEngine.parallel_degree │
│ Read at new() time; set_parallel_degree()│
│ Public API (WIRED ✓)                   │
└──────────────────────────────────────┘
                ↓
┌──────────────────────────────────────┐
│ Layer 3: execute_select guard          │
│ if parallel_degree > 1                 │
│    && rows.len() >= PARALLEL_MIN_ROWS  │
│    && no correlated subquery           │
│    && no FOR UPDATE / LOCK IN SHARE    │
│ then run filter via ParallelVolcanoExecutor (WIRED ✓) │
└──────────────────────────────────────┘
                ↓
┌──────────────────────────────────────┐
│ (Future) Layer 4: New execute_select │
│ Use PipelineExecutor (NOT WIRED)      │
│ Per Integration Guide red-line        │
└──────────────────────────────────────┘
```

## Current State (verified 2026-07-12)

| Layer | State | File | Verified |
|-------|-------|------|----------|
| 1 — CLI flag | ✅ Wired | `crates/mysql-server/src/main.rs:99-102` (clap derive) | ✓ |
| 1 — env var | ✅ Wired | `src/execution_engine.rs:154` (`SQLRUSTGO_EXECUTOR_PARALLELISM`) | ✓ |
| 2 — parallel_degree | ✅ Wired | `src/execution_engine.rs:106,212-217` (field, getter, setter) | ✓ |
| 3 — execute_select guard | ✅ Wired | `src/engine_select.rs:262-280` (gating logic) | ✓ |
| 3 — parallel_executor module | ✅ Declared | `crates/executor/src/lib.rs:12` (`pub mod parallel_executor`) | ✓ |
| 4 — PipelineExecutor | ⚠️ Standalone | `crates/executor/src/pipeline_executor.rs` (ready, not wired) | ✓ |

## What's needed

1. **E2E verification test** that proves the path engages in production.
   The current INT-2 tests (`tests/int2_substance_parallel_test.rs`) test
   `ParallelVolcanoExecutor::partition_scan` in isolation, not through the
   full `ExecutionEngine::execute_select` path.

2. **Tracing emission** in the guard so we can observe parallel engagement.
   Currently the guard calls `ParallelVolcanoExecutor::partition_scan` but
   doesn't emit a tracing span. Need to add `tracing::info_span!(...)` to 
   make path engagement observable in production.

3. **Test verifying the CLI flag actually plumbs through.**
   `tests/executor_parallelism_cli_test.rs` should:
   - Start `sqlrustgo-mysql-server` with `--executor-parallelism=4`
   - Verify env var propagation
   - Verify `parallel_degree=4` is reachable

## Code Layout

### `crates/executor/src/lib.rs` (verify)

```rust
pub mod parallel_executor; // line 12 — already present
```

### `src/engine_select.rs` (extend)

Add tracing to the guard at line 262:

```rust
let _parallel_guard = if self.parallel_degree > 1
    && rows.len() >= PARALLEL_MIN_ROWS
    && !select.where_clause.as_ref().is_some_and(where_expr_has_correlated_subquery)
    && select.lock_clause.is_none()
{
    let _span = tracing::info_span!(
        "parallel_filter_engaged",
        degree = self.parallel_degree,
        rows = rows.len(),
    );
    // ... existing parallel filter code ...
};
```

### Tests (add)

`tests/parallel_main_path_test.rs`:
- `test_execute_select_with_parallel_degree_4_engages_path`
- `test_execute_select_results_match_between_parallel_degrees`
- `test_parallel_query_with_tpch_q1_pattern`

## Refs

- `openspec/changes/issue-3703-intra-query-parallel-executor/` (parent)
- `src/engine_select.rs:262` (gating logic)
- `tests/int2_substance_parallel_test.rs` (existing parallel tests)
- `crates/executor/src/parallel_executor.rs` (1762-line impl)
