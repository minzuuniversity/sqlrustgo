## Context

The `AdaptiveHashIndex` (AHI) was implemented in V311-02 v1 as a storage-layer
data structure with `record_access`, `lookup`, and `invalidate_*` operations.
It sits in `crates/storage/src/adaptive_hash_index.rs` and is fully tested.

But the main engine path uses `storage.scan(table)` which returns ALL rows
regardless of any WHERE filter. AHI acceleration requires a *different* path —
one that does only a point-lookup, not a full scan.

V311-02 v2 wires a hook into the existing WHERE pk = ? filter to call
`record_access()` even when a full scan happens. This is the simplest
integration: the AHI becomes a metric on real workload data without
changing performance characteristics.

## Goals / Non-Goals

**Goals:**
- `ExecutionEngine` exposes `Arc<AdaptiveHashIndex>` via `ahi()` accessor
- WHERE pk = ? queries call `record_access` for every matching row
- AHI hit_rate stat is non-zero after 50+ repeated same-key queries
- F-24 debt-registry entry moves from VERIFIED → CLOSED
- Documentation explains the integration architecture

**Non-Goals:**
- Bypass the full scan on AHI hit (would require PageLocation-aware scan, v3.12+)
- Disk-persistent PageLocation tracking
- Secondary-index AHI

## Decisions

### Decision 1: Add AHI as `Arc<RwLock<AdaptiveHashIndex>>` on the engine

**Choice**: Following V311-01 v1 pattern (which added clustered_tables), expose an `Arc<AdaptiveHashIndex>` directly on `ExecutionEngine`.

**Alternative considered**: A `parking_lot::Mutex<Option<Arc<AdaptiveHashIndex>>>` allowing users to disable AHI by setting None. Rejected — too complex for v2; AHI is opt-out via not using it.

### Decision 2: Hook in WHERE pk = ? post-filter, NOT in scan

**Choice**: In `engine_select.rs::filter_partitions_parallel` (or equivalent), after filtering, for each row that has a matching PK column, call `ahi.record_access(table, pk_bytes, page_id=0, offset=0)`. We use `(0, 0)` as conceptual page locations because MemoryStorage doesn't have a real page system.

**Why this approach**:
- No semantic change to scan/filter correctness
- AHI receives realistic access patterns to track
- `lookup()` for warm queries becomes meaningful even if scan still happens (the AHI test path already exercises this)

**Alternative considered**: Add a new `lookup_by_pk(table, pk_value) -> Option<Record>` method to MemoryStorage and have engine use it. Rejected — too invasive for v2; partial benefit.

### Decision 3: v2 adds the hook but doesn't change query semantics

**Choice**: AHI is observability + warm-cache tracking, NOT a query accelerator. v2 surfaces the metric and warms the cache; v3.12+ can wire actual PageLocation-based lookups.

This matches the V311-01 v1 vs V311-01 v2 distinction:
- v1: production-API surface (storage crate)
- v2: production integration (engine crate)

### Decision 4: Statistics surface remains testable from external code

**Choice**: Tests can directly call `engine.ahi().total_lookups()` to assert behavior. The integration test verifies that production code paths increment these counters.

## Implementation Plan

### Phase 1: `ExecutionEngine::ahi()` accessor

```rust
// In ExecutionEngine:
pub fn ahi(&self) -> &Arc<sqlrustgo_storage::AdaptiveHashIndex> { &self.ahi }

// In engine_builder.rs constructors:
ahi: Arc::new(sqlrustgo_storage::AdaptiveHashIndex::new()),
```

### Phase 2: Wire record_access calls in WHERE filter

In `src/engine_select.rs` after `let mut rows = storage.scan(...)?`:

```rust
let ahi = self.ahi();
// For each outer row that survives the WHERE filter, record the access
for row in &rows {
    if let Some(pk_col) = primary_key_column_index {
        if let Some(pk_val) = row.get(pk_col) {
            let pk_bytes = pk_val.to_string().into_bytes();
            ahi.record_access("orders", &pk_bytes, 0, 0);
        }
    }
}
```

### Phase 3: Tests

Create `tests/integration/storage/adaptive_hash_main_path_test.rs` with 5 tests:

1. **pk_lookup_records_ahi_access** — single WHERE pk = N increments `ahi.total_lookups()` (if we surface lookups here) or `ahi.size()` (cache size grows)
2. **pk_lookup_promotes_after_threshold** — 17+ queries on same key → `ahi.size() == 1`
3. **ahi_hit_rate_strictly_increases** — repeated same-key → hit_rate > 0
4. **invalidate_table_clears_ahi** — DROP TABLE removes AHI entries for that table
5. **point_lookup_recurring_queries_benefit** — back-to-back same-key → metric growth across iterations

### Phase 4: Documentation

`docs/releases/v3.11.0/perf/ADAPTIVE_HASH_INDEX.md` — algorithm overview + v1+v2 integration diagram + benchmark.

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Engine construction breakage (constructor changes) | Run full regression test suite |
| `record_access` overhead per query | Test bench to verify sub-microsecond overhead |
| AHI memory unbounded if invalidation isn't called | Document manual invalidation |
| PageLocation semantics for MemoryStorage don't exist | Use `(0, 0)` placeholder; AHI promotion is by page_id only |

## Verification

- 5 new integration tests PASS
- All 25 V311-15/17/01/16 tests still PASS
- F-24 status: VERIFIED → CLOSED in debt-registry
