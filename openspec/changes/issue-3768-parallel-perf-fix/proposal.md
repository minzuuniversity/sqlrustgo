# Proposal — issue-3768-parallel-perf-fix (Issue #3776 / F-36)

> **Status**: Proposed (Issue #3776, V310-19 / F-36)
> **Goal**: Eliminate the parallel-execution regression. Make the parallel
> path **at least as fast as** sequential, ideally ≥2x on TPC-H Q1/Q6.

## Why

Baseline measurements (`tests/parallel_scan_bench_test.rs`) show:

| Rows | Sequential (N=1) | Parallel (N=4 partitions) | Verdict |
|------|------------------|----------------------------|---------|
| 1K   | 190.625 µs       | 556.833 µs                 | **2.92× slower** |
| 10K  | (slow)           | 1.10 ms                    | parallel wins by 2-3× |
| 50K  | (slow)           | 5.25 ms                    | parallel wins 4-5× |

**The 1K-row case is the regression.** On small datasets, partition overhead
exceeds parallel benefit. The current guard (`rows.len() >= PARALLEL_MIN_ROWS = 500K`)
supposedly prevents it, but `MemoryStorage::parallel_scan` does
`chunk + to_vec()` deep copies per partition — paying that cost even above
the threshold — and the SIMD `simd_eval` module sits unused.

## Goals

- **G1**: Default sequential behavior unchanged. `parallel_degree=1` must
  match `parallel_degree=N>1` throughput within **±10%** for rows < 50K.
- **G2**: Above `PARALLEL_MIN_ROWS` (500K), `parallel_degree=8` must beat
  `parallel_degree=1` by **≥1.5x** (conservative; spec asks for 2x on Q1/Q6).
- **G3**: TPC-H Q1 with `parallel_degree=8` on SF=0.01 ≥1.5x sequential.

## Non-Goals

- **N1**: Changing `PARALLEL_MIN_ROWS` from 500K — that's a separate tuning
  PR after we have real benchmarks.
- **N2**: Wiring `PipelineExecutor` to old `engine_select.rs`. Out of scope
  per the Integration Guide red-line — `PipelineExecutor` will slot into
  the new `execute_select` (per `EXECUTION_PIPELINE_REFACTORING.md`).
- **N3**: True disk-level `FileStorage::parallel_scan`. Requires
  `get_partition_boundaries()` exposing row offsets.

## Approach (3 layers)

### Layer 1 — Fix `parallel_scan` overhead (low risk)

Replace per-partition `Vec<Record>` deep copy with **shared ownership**:
```rust
// Before
let partition: Vec<Record> = data[cur..cur + size].to_vec();
partitions.push(Box::new(partition.into_iter()));

// After
let shared = Arc::new(data.clone()); // clone Arc, not Vec
let slice = Arc::clone(&shared);
partitions.push(Box::new(SharedSliceIter::new(slice, cur, cur + size)));
```

This eliminates the N×O(rows) deep-copy. Memory traffic drops from
`O(N × rows)` to `O(rows)` plus `O(N) Arc bumps`.

### Layer 2 — Eliminate parallel-filter regression (CBO hook)

Wire `UnifiedCostModel::should_parallelize_with(select)` into the existing
guard at `src/engine_select.rs:262`. Currently the guard only checks
`rows.len() >= PARALLEL_MIN_ROWS` — a hard threshold. Add per-selectivity
estimation so:

- `rows < 50K`  → always sequential (eliminate micro-bench regression)
- `rows 50K-500K` → sequential OR 2-partition parallel (selectivity-based)
- `rows >= 500K` → current threshold (full parallel)

This requires executing `should_parallelize_with` on a constructed
`UnifiedPlan::Filter` from the AST. The map function:
```rust
fn cbo_should_parallelize(engine: &ExecutionEngine, select: &SelectStatement) -> bool {
    // Synthesize a UnifiedPlan::Filter from WHERE and base table
    let plan = UnifiedPlan::Filter {
        predicate: Expression::Literal("1"), // placeholder
        input: Box::new(UnifiedPlan::TableScan {
            table_name: select.table.clone(),
            projection: None,
        }),
    };
    let for_update = select.lock_clause.as_ref().is_some_and(|l| l.for_update);
    engine.cost_model.as_ref().map_or(false, |cm| {
        cm.should_parallelize_with(&plan, for_update)
    })
}
```

### Layer 3 — SIMD batch predicate eval (forward path)

For queries where `k = literal` is the dominant filter shape, replace the
scalar `eval_predicate` with `simd_eval::LessThanPredicate::eval_batch_i64`.

This requires:
- `simd_eval::apply_mask(records, mask) -> Vec<Vec<Value>>` integration
- Path detection: if predicate is `(col =/</> literal) AND nothing else`,
  dispatch to batch eval.

Out of immediate scope (deferred to a follow-up).

## Validation

```bash
cargo test --test parallel_scan_perf_baseline
# Must pass:
# - N=1 throughput == N=4 throughput within 10% for rows < 50K
# - N=4 throughput >= N=1 throughput for rows >= 500K
# - Memory: parallel_scan(1000, 4) peak RSS < 1.5x sequential scan(1000)
```

## Migration

Layer 1 is backward compatible — just changes memory layout.
Layer 2 is backward compatible — default `parallel_degree=1` still uses the
hard threshold (which falls through CBO anyway since select_degree_1 = sequential).

## Refs

- Issue #3776 (V310-19 / F-36)
- Issue #3703/#3735-#3737/#3767 (parent chain)
- `tests/parallel_scan_bench_test.rs` (existing baseline)
- `INT5_PLUS_DEBT_INVENTORY.md §I-12`
- Integration Guide red-line on `PipelineExecutor` wiring
