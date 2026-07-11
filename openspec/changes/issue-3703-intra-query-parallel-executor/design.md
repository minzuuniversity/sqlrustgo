# Design — Issue #3703: Intra-Query Parallel Executor

> **For**: implementers + reviewers
> **Companion**: `proposal.md` (why + what), `specs/*.md` (what the system shall do)

---

## Context

### Current state (v3.9.0 GA → v3.10.0-alpha1)

- `crates/executor/src/parallel_executor.rs` (134 lines) — **trait shell only**
  - `trait ParallelExecutor` with 3 methods: `parallel_degree`, `set_parallel_degree`, `partition_scan`
  - `struct ParallelVolcanoExecutor { parallel_degree: usize }` — 0 production callers
- `crates/executor/src/parallel_vector_executor.rs` (757 lines) — **rayon-based vectorized parallel executor**
  - Full `PartitionInfo` + `partition()` infrastructure
  - Uses `rayon::prelude::*` — `par_iter()` + `reduce()` for parallel aggregation
  - **No caller in production path** (LocalExecutor never invokes it)
- `src/execution_engine.rs` (1498 lines) — top-level engine
  - `build_parallel_executor()` returns `ParallelVolcanoExecutor::new(self.parallel_degree)` (line ~225)
  - `set_parallel_degree(&mut self, degree: usize)` sets `parallel_degree = degree.max(1)`
  - **`parallel_degree` field exists but is never read by `execute_select`**

### v3.9.0 SOAK data (issue #3703 comment 69559)

- 3x sysbench × 20 client threads on M4
- Server CPU: 120% (= 1.2 cores saturated)
- RSS: 113 MB stable, WAL: 315 B bounded, FD: 34, err/s: 0.00
- Max throughput: ~1522 QPS / 87 TPS (mixed read-write)
- Verdict: single-threaded VolcanoExecutor is at 100% utilization

### Stakeholders

- **DBA / OLAP users**: want full-scan queries to finish in sub-second instead of 10s
- **OLTP users** (1 conn = 1 small query): zero impact (default N=1 preserves behavior)
- **Architecture/SRE**: concerned about correctness regression and tail latency

### Constraints

- **Volcano compatibility**: keep existing operators (Scan, Filter, Project, Agg, HashJoin) intact
- **Zero regression**: N=1 must be bit-exact identical to current behavior
- **Feature gate**: `--features parallel-executor` gates the rayon + multi-thread path (default build unchanged)
- **Storage thread safety**: rely on G13-OLTP-2 fix (PR #3680 parking_lot RwLock)

---

## Goals / Non-Goals

### Goals

1. Wire `ParallelVolcanoExecutor` into `LocalExecutor::execute_select()` so N=1=N, N>1 splits the input row stream
2. Add `--executor-parallelism=N` CLI flag (default 1) propagated through `ExecutionEngine::set_parallel_degree()`
3. Parallelize Scan → Filter → Project → Agg pipeline (Phase 1, this change)
4. Maintain 22/22 TPC-H cell-level consistency between N=1 and N=4
5. Keep all 25 lib tests + integration tests passing under both N=1 and N=4

### Non-Goals

- ❌ Full Volcano rewrite (push-based / morsel-driven) — 3-4 months effort, separate change
- ❌ Parallel hash join (Phase 2, separate change)
- ❌ Parallel GROUP BY (Phase 3, separate change)
- ❌ Modifying ServerThreadPool (P2-1, separate change)
- ❌ Changing WAL/storage layer (G13 already fixed RW lock contention)
- ❌ Auto-tuning `N` from CPU count — explicit CLI flag only in this change

---

## Decisions

### Decision 1: Pipeline model over full Volcano rewrite

**Choice**: Keep the existing Volcano `Next()` interface. Only parallelize at the **Scan boundary** — split rows into N partitions, run N independent Volcano pipelines (Scan→Filter→Project→Agg) on each partition, merge on main thread.

**Rationale**:
- Volcano's `Next()`-at-a-time model is hard to parallelize (every operator blocks on upstream)
- A full rewrite to push-based / morsel-driven would take 3-4 months
- Pipeline approach keeps existing operators intact, parallelizes only the I/O + heavy-compute stages
- Scan+Filter+Project typically consume ~80% of CPU for analytic queries

**Alternatives considered**:
- **Morsel-driven parallelism** (HyPer model): rejected — requires full Volcano rewrite
- **Operator-level thread pools** (each operator on its own thread): rejected — high contention, complex back-pressure
- **Hand-rolled thread pool**: rejected — `rayon` already in `parallel_vector_executor.rs`, no new dep

### Decision 2: Default N=1, opt-in N>1

**Choice**: `--executor-parallelism=1` is the default. N>1 only active with explicit flag or config.

**Rationale**:
- Zero regression risk for existing users
- N=1 path is bit-exact identical to current execution
- Allows incremental rollout: opt-in testing → benchmarking → A/B → default flip later

**Alternatives considered**:
- **Auto-detect N from num_cpus**: rejected — adds surprise for OLTP users; revisit in v3.11
- **Always parallel**: rejected — violates zero-regression contract

### Decision 3: Reuse existing `parallel_vector_executor.rs` infrastructure

**Choice**: Use the `PartitionInfo` + rayon `par_iter()` machinery already in `parallel_vector_executor.rs` (757 lines). Wrap it in a new `LocalExecutor::execute_select_parallel()` method.

**Rationale**:
- 757 lines of partition + parallel aggregation logic already exists and is tested
- Avoids duplicating rayon work-stealing setup, partition algorithms
- `VectorizedSeqScanExecutor` integration already in place

**Alternatives considered**:
- **Write new partition from scratch**: rejected — duplicate 700+ lines of tested code
- **Skip vectorization, use raw row Vec**: rejected — vectorized path is faster; keep it

### Decision 4: Feature gate `--features parallel-executor`

**Choice**: The N>1 path is gated behind `#[cfg(feature = "parallel-executor")]`. Default build: 0 bytes added, N forced to 1.

**Rationale**:
- Production binaries (e.g. `sqlrustgo-mysql-server`) can compile with feature off for zero footprint
- Test binaries enable feature to exercise the parallel path
- Matches v3.9.0 pattern of optional cargo features

**Alternatives considered**:
- **Always compile, runtime N=1**: rejected — adds rayon dep to default build, increases binary size ~300KB
- **Separate binary**: rejected — operational complexity, two binaries to maintain

### Decision 5: Main thread merges partial results

**Choice**: After N parallel pipelines complete, main thread does:
- Filter+Project: `Vec::extend()` all partial rows in order
- Aggregation: `HashMap` merge (or use existing `multi_way_hash_chain`)

**Rationale**:
- Simple, correct, easy to test
- Merge is O(partial_rows), fast relative to scan time
- Avoids implementing a parallel merge step (deferred to Phase 3 GROUP BY)

**Alternatives considered**:
- **Tree-merge with N threads**: rejected — only useful when partial result is also large; Scan-bound queries don't need it
- **Streaming merge via channel**: rejected — adds complexity without benefit at this scale

---

## Implementation Sketch

### 1. CLI flag (`crates/cli/src/main.rs`)

```rust
#[derive(Parser)]
struct Cli {
    /// Intra-query executor parallelism (1=sequential default, N=parallel workers)
    #[arg(long, default_value = "1", env = "SQLRUSTGO_EXECUTOR_PARALLELISM")]
    executor_parallelism: usize,
    // ... existing flags
}
```

### 2. ExecutionEngine wiring (`src/execution_engine.rs`)

```rust
// Already exists (line ~225):
pub fn build_parallel_executor(&self) -> ParallelVolcanoExecutor {
    ParallelVolcanoExecutor::new(self.parallel_degree)
}

// Already exists (line ~221):
pub fn set_parallel_degree(&mut self, degree: usize) {
    self.parallel_degree = degree.max(1);
}
```

No new changes to `execution_engine.rs` — wiring is in place. Only `execute_select` needs to call it.

### 3. LocalExecutor parallel path (new method, `crates/executor/src/local_executor.rs`)

```rust
impl LocalExecutor {
    /// Execute SELECT with intra-query parallelism.
    /// Only called when `parallel_degree > 1` AND `feature = "parallel-executor"`.
    pub fn execute_select_parallel(
        &mut self,
        stmt: &SelectStatement,
        degree: usize,
    ) -> SqlResult<ExecutorResult> {
        // 1. Get base row stream from storage
        let base_rows = self.scan_base_rows(stmt)?;

        // 2. Partition using ParallelVolcanoExecutor::partition_rows
        let partitions = ParallelVolcanoExecutor::new(degree).partition_rows(base_rows, degree);

        // 3. Run Filter+Project+Agg on each partition in parallel (rayon::par_iter)
        let partials: Vec<ExecutorResult> = partitions
            .par_iter()
            .map(|partition| self.run_volcano_pipeline(partition, stmt))
            .collect::<SqlResult<Vec<_>>>()?;

        // 4. Merge partials on main thread
        Ok(self.merge_partials(partials))
    }
}
```

### 4. Test plan

- **Unit tests**: `parallel_scan_partitions_evenly`, `parallel_scan_partitions_remainder`, `parallel_scan_empty_table`
- **Integration**: `parallel_executor_n1_eq_n4` (cell-level diff = 0 across 22 TPC-H queries)
- **Bench**: `parallel_bench_q1_sf01` (assert N=8 P99 < N=1 P99 / 1.5)
- **Regression**: full `cargo test --all-features --lib` (25 tests) + TPC-H 22/22

---

## Risks / Trade-offs

### [R1] StorageEngine contention under parallel scan (MEDIUM)

**Risk**: N threads each calling `storage.scan(table)` simultaneously may contend on `parking_lot::RwLock`.

**Mitigation**:
- G13-OLTP-2 fix (PR #3680) already made lock fair-FIFO
- Storage scan is read-only (RwLock read mode allows N concurrent readers)
- Benchmark with N=8 must complete without >5% throughput regression vs N=1 on mixed RW workload

### [R2] Partial result ordering not preserved (LOW)

**Risk**: Parallel pipelines produce rows in partition order, not original row order. ORDER BY queries may break.

**Mitigation**:
- Parallel path only triggers for non-ORDER-BY queries (check `stmt.order_by.is_none()`)
- For ORDER BY: fall back to sequential `execute_select`
- Add integration test: `parallel_preserves_order_by_falls_back`

### [R3] Rayon work-stealing overhead on small tables (LOW)

**Risk**: For tables < `PARALLEL_MIN_ROWS` (100K), rayon thread setup overhead exceeds parallel speedup.

**Mitigation**:
- `parallel_vector_executor.rs` already has `PARALLEL_MIN_ROWS` threshold
- Below threshold: fall back to sequential
- Verify in bench: N=8 on 1K-row table = N=1 baseline ± 5%

### [R4] Feature flag sprawl (LOW)

**Risk**: `--features parallel-executor` requires careful CI management (test both with and without).

**Mitigation**:
- Add to `check_alpha_v3.10.0.sh` A1_BUILD a `--features parallel-executor` matrix
- Update `check_integration_gate.sh` to run with both feature states
- Document in `Cargo.toml` feature list

### [R5] Memory amplification (LOW)

**Risk**: N partitions in memory simultaneously = N × single-partition size.

**Mitigation**:
- For SF=0.1 TPC-H: lineitem is ~60K rows × ~50 bytes = 3MB per partition × 8 = 24MB peak
- Well within 113MB baseline RSS
- No action needed; verify in bench

---

## Rollout Plan

1. **Implementation** (this change, ~1 week):
   - Add CLI flag
   - Add `execute_select_parallel()` to `LocalExecutor`
   - Wire feature gate
   - Tests + bench
2. **Phase 2 (separate change)**: Parallel hash join
3. **Phase 3 (separate change)**: Parallel GROUP BY
4. **Default flip** (v3.11+): Change default `--executor-parallelism=auto` to `num_cpus`

---

## Refs

- `proposal.md` — capabilities + acceptance criteria
- `specs/intra-query-parallel-scan/spec.md` — scan parallel behavior
- `specs/intra-query-parallel-hash-join/spec.md` — Phase 2 spec (placeholder)
- `specs/intra-query-parallel-group-by/spec.md` — Phase 3 spec (placeholder)
- `specs/cli-executor-parallelism-flag/spec.md` — CLI flag behavior
- `crates/executor/src/parallel_executor.rs` (existing shell)
- `crates/executor/src/parallel_vector_executor.rs` (existing rayon impl)
- `src/execution_engine.rs:225` (existing build_parallel_executor)
- Issue #3703 + comment 69559
