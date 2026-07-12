# Design — issue-3768-parallel-perf-fix (Issue #3776 / F-36)

## Layers (recap from proposal)

```
┌─────────────────────────────────────────────────────────────┐
│ Layer 1: parallel_scan memory ownership (low risk)         │
│  Replace to_vec() deep copy with Arc<Vec<Record>>           │
│  Impact: 2.9× regression → ≤1× for small datasets           │
└─────────────────────────────────────────────────────────────┘
                ↓
┌─────────────────────────────────────────────────────────────┐
│ Layer 2: CBO integration into guard                       │
│  src/engine_select.rs:262 — add cbo_should_parallelize() │
│  Wrap current rows.len() >= 500K check with CBO call      │
│  Impact: adaptive parallelism based on selectivity         │
└─────────────────────────────────────────────────────────────┘
                ↓
┌─────────────────────────────────────────────────────────────┐
│ Layer 3: SIMD batch predicate eval (deferred)              │
│  simd_eval → filter path                                  │
│  Risk: high, requires careful dispatch                     │
└─────────────────────────────────────────────────────────────┘
```

## Layer 1: Shared-Ownership Iterators

### Current code (`crates/storage/src/engine.rs:1074`)

```rust
fn parallel_scan(&self, table: &str, num_partitions: usize) ->
    SqlResult<Vec<Box<dyn Iterator<Item = Record> + Send>>>
{
    let data = self.tables.get(table).ok_or(...)?;
    let total = data.len();
    let num_partitions = num_partitions.min(total);
    let base = total / num_partitions;
    let rem = total % num_partitions;
    let mut partitions = Vec::with_capacity(num_partitions);
    let mut cur = 0;
    for i in 0..num_partitions {
        let size = if i < rem { base + 1 } else { base };
        if size > 0 {
            // PROBLEM: deep copy of each partition
            let partition: Vec<Record> = data[cur..cur + size].to_vec();
            partitions.push(Box::new(partition.into_iter()));
        }
        cur += size;
    }
    Ok(partitions)
}
```

### Proposed code

```rust
fn parallel_scan(&self, table: &str, num_partitions: usize) ->
    SqlResult<Vec<Box<dyn Iterator<Item = Record> + Send>>>
{
    let data = self.tables.get(table).ok_or(...)?;
    let total = data.len();
    if total == 0 || num_partitions == 0 {
        return Ok(vec![]);
    }
    let num_partitions = num_partitions.min(total);
    let base = total / num_partitions;
    let rem = total % num_partitions;
    let shared: Arc<Vec<Record>> = Arc::new(data.clone());
    let mut partitions = Vec::with_capacity(num_partitions);
    let mut cur = 0;
    for i in 0..num_partitions {
        let size = if i < rem { base + 1 } else { base };
        if size > 0 {
            let part = Arc::clone(&shared);
            partitions.push(Box::new(
                SharedSliceIter::new(part, cur, cur + size)
            ));
        }
        cur += size;
    }
    Ok(partitions)
}

struct SharedSliceIter {
    data: Arc<Vec<Record>>,
    pos: usize,
    end: usize,
}

impl Iterator for SharedSliceIter {
    type Item = Record;
    fn next(&mut self) -> Option<Self::Item> {
        if self.pos < self.end {
            let r = self.data[self.pos].clone();
            self.pos += 1;
            Some(r)
        } else {
            None
        }
    }
}
```

**Key change**: One `Arc<Vec<Record>>` clone (just bumps refcount) instead of
N deep copies of the full Vec. Each worker still clones each row when calling
next (cheap row copy), but the Vec clone is gone.

### Verification

- Memory: peak RSS should be O(rows + N×Arc) instead of O(N×rows).
- Correctness: `extract_i64_column` and similar tests should still pass
  (rows are clones but values are identical).

## Layer 2: CBO Hook in Guard

### Current logic at `src/engine_select.rs:262`

```rust
let _parallel_guard = if self.parallel_degree > 1
    && rows.len() >= PARALLEL_MIN_ROWS
    && !select.where_clause.as_ref()
        .is_some_and(where_expr_has_correlated_subquery)
    && select.lock_clause.is_none()
{ ... parallel filter ... }
```

### Proposed

```rust
let should_run_parallel = if self.parallel_degree > 1
    && !select.where_clause.as_ref()
        .is_some_and(where_expr_has_correlated_subquery)
    && select.lock_clause.is_none()
{
    // Delegate the row-count / selectivity decision to CBO.
    // CBO returns false for FOR UPDATE / unknown stats / low rows.
    let for_update = select.lock_clause.as_ref()
        .is_some_and(|l| l.for_update);
    cbo_should_parallelize(self, select, rows.len(), for_update)
} else {
    false
};

let _parallel_guard = if should_run_parallel {
    // ... existing parallel filter code ...
};
```

The `cbo_should_parallelize` function lives in a new file
`crates/executor/src/parallel_cbo.rs`:
```rust
use sqlrustgo_optimizer::unified_cost::UnifiedCostModel;
use sqlrustgo_optimizer::unified_plan::UnifiedPlan;
use sqlrustgo_parser::Expression;

pub fn cbo_should_parallelize<S: StorageEngine>(
    engine: &ExecutionEngine<S>,
    select: &SelectStatement,
    rows: usize,
    for_update: bool,
) -> bool {
    if for_update {
        return false;
    }
    if rows < 50_000 {
        return false; // < 50K always sequential
    }

    // Synthesize a plan for CBO
    let plan = UnifiedPlan::Filter {
        predicate: Expression::Literal("1".to_string()),
        input: Box::new(UnifiedPlan::TableScan {
            table_name: select.table.clone(),
            projection: None,
        }),
    };

    // Use engine's cost model
    engine.build_cost_model().should_parallelize_with(&plan, for_update)
}
```

This delegates to the CBO and respects **selectivity** — even if rows
≥ 50K, if the selectivity is very high (most rows pass), parallel overhead
doesn't help.

### Backward compat

`parallel_degree=1` short-circuits before this is consulted. Default behavior
unchanged for users not setting the env var/CLI flag.

## Layer 3: SIMD batch eval — deferred

Per the proposal, this is out of immediate scope.

## Risk Assessment

| Layer | Risk | Mitigation |
|-------|------|------------|
| Layer 1 (Arc iter) | Low | Existing tests cover correctness; perf assertions validate no regression |
| Layer 2 (CBO hook) | Medium | Hard threshold (50K) above as safety floor; CBO can only return false |
| Layer 3 (SIMD) | High | Deferred |

## Test Plan

`tests/parallel_scan_perf_baseline.rs`:
1. `test_no_regression_at_1k_rows` — N=1 == N=4 within 10%
2. `test_parallel_wins_above_500k` — N=8 ≥ 1.5× N=1
3. `test_memory_no_quadruple` — RSS(parallel_scan(1M, 4)) < 4× RSS(scan(1M))
4. `test_correctness_preserved_after_arc_change` — same test as before but with shared iter

## Refs

- Issue #3776 (V310-19, F-36)
- Issue #3703/#3735-#3737/#3767 (parent chain)
- `tests/parallel_scan_bench_test.rs` (existing baseline)
- `crates/storage/src/engine.rs:1074-1103` (current `parallel_scan`)
- `src/engine_select.rs:262` (current guard)
- `crates/executor/src/parallel_group_by.rs` (similar Arc-shared approach reference)
- `crates/executor/src/simd_eval.rs` (Layer 3 SIMD modules)
