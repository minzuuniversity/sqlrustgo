# INT-2: I-12 Parallel Executor Integration SPEC

**Issue**: #2986
**Status**: DRAFT (Phase 1 of 3)
**Owner**: Execution WG
**Estimated effort**: ~30h (split across 3 phases)
**Target version**: v3.9.0 (RC integration)

## Background

`ParallelVolcanoExecutor` exists at `crates/executor/src/parallel_executor.rs`
(1762 lines, plan-aware wrapper around `VolcanoExecutor`). It supports
`SeqScan`, `HashJoin`, `Aggregate`, `Filter`, `Projection` and dispatches
via `RayonTaskScheduler`.

**Current state**: The file is **not declared in `crates/executor/src/lib.rs`**
(verified 2026-06-04) — it is dead code at the crate level. The canonical
`ExecutionEngine` in `src/execution_engine.rs` never consults it.

**Gap**: 30h of integration work — see Phase breakdown below.

## Phase 1: SPEC (this document) [DONE in v3.8.0]

Document the integration design. No code changes.

## Phase 2: Mod-Tree Activation + Public API [~4h]

Make `ParallelVolcanoExecutor` usable:

1. Add `pub mod parallel_executor;` to `crates/executor/src/lib.rs`
2. Re-export `ParallelVolcanoExecutor`, `RayonTaskScheduler`,
   `ParallelExecutor` trait
3. Add a `parallel_degree: usize` field to
   `crates/executor/src/session_config.rs` (default 1 = serial)
4. Add a config getter `parallel_degree() -> usize` on
   `ExecutionEngine` (default: 1)

Verification: `cargo build` passes, `cargo test -p sqlrustgo-executor --lib`
still passes (the mod tree addition alone does not break anything).

## Phase 3: ExecutionEngine Integration [~16h]

Wire `ParallelVolcanoExecutor` into `ExecutionEngine::execute`:

1. In `ExecutionEngine::execute`, after the AST dispatch, if
   `self.parallel_degree > 1`, route to a new
   `ExecutionEngine::execute_physical` method
2. `execute_physical` builds a physical plan via `sqlrustgo_planner`
   and invokes `ParallelVolcanoExecutor::execute_parallel`
3. The result is converted back to the same `ExecutorResult` shape
4. Fall back to the current path (legacy `engine_select.rs`) when
   `parallel_degree == 1` so behavior is identical to today

Verification:
- `cargo test -p sqlrustgo-executor --lib` passes (no regression)
- New test: `parallel_degree=4` produces same rows as `parallel_degree=1`
  on a 4-table TPC-H Q-style query

## Phase 4: Coverage + Benchmarks [~10h]

1. Add `tests/parallel_executor_scaling_test.rs`:
   - 1, 4, 16, 64 threads
   - Measure throughput on a 1M-row scan + aggregate
2. Add `bench/benches/parallel_executor_bench.rs`:
   - Compare serial vs parallel on a TPC-H Q-style query
3. Update `docs/benchmarks/` with v3.9.0 numbers

## Acceptance Criteria for Issue #2986 close

- Phases 1, 2, 3, 4 all complete
- `bash scripts/gate/check_execution_semantics.sh` still PASS
- TPC-H Q9 (or similar 4-way join) shows speedup at 4 threads
- All existing tests pass (no regression)

## Risks

- **Mod-tree activation may surface "dead code" warnings** that block
  `cargo build`. Mitigation: keep `#[allow(dead_code)]` on internal helpers
  until each is exercised by tests.
- **Result-set ordering**: parallel execution may reorder rows. Mitigation:
  sort at the boundary, or document that order is not preserved when
  `parallel_degree > 1`.
- **Session config migration**: existing clients assume serial execution.
  Mitigation: default `parallel_degree = 1` for v3.8.0 GA, opt-in for
  v3.9.0.

## Related

- I-12 SPEC (original): `docs/releases/v3.0.0/specs/debt/I-12_parallel_executor.md`
- Issue: #2986
- F-09 deep fixes follow-up (parallel_vector_executor.rs)
