## Why

V311-06 (F-31 Performance Schema instrumentation hooks) provides **observability** for the query execution layer. It's the foundation for any future Performance Schema implementation (events_statements_history, events_waits_history, etc.) and is critical for debugging slow queries in production.

### Current State

- `tracing` crate is already a dep (`crates/executor/Cargo.toml`)
- VolcanoExecutor operators (`Filter`, `SeqScan`, `HashJoin`, etc.) have **no** instrumentation hooks
- Existing `tracing::info_span!` calls exist in **only** a few places (`engine_select.rs`)
- No way for an external observer (test, monitor) to count events like:
  - "How many SeqScans happened in this query?"
  - "What was the total rows_scanned across all operators?"

### What's Missing

A **trait-based observer pattern** that:
1. Fires events at well-defined points (SeqScan.start, Filter.start, Project.start, HashJoin.build, HashJoin.probe, etc.)
2. Provides cheap default (NoopInstrumentationHook) with zero overhead
3. Provides a counting variant for testing/monitoring
4. Allows operators to dispatch events WITHOUT coupling to specific impls

### Real-World Impact

| Use Case | Before | After |
|----------|--------|-------|
| Test: verify SeqScan was called | manual `println!` | `CountingHook::seq_scan_count()` |
| Benchmark: count operator events per query | impossible | trivial via hook counter |
| Production: Performance Schema (future) | not wired | trait in place |

## What Changes

### 1. New `crates/executor/src/instrumentation.rs`

```rust
pub trait InstrumentationHook: Send + Sync {
    fn on_seq_scan_start(&self, table: &str);
    fn on_filter_start(&self, table: &str, rows_in: usize);
    fn on_filter_end(&self, table: &str, rows_out: usize);
    fn on_project_start(&self, table: &str, rows_in: usize);
    fn on_hash_join_build(&self, build_side: &str);
    fn on_hash_join_probe(&self, probe_rows: usize);
    fn on_aggregate_start(&self, group_keys: usize);
    fn on_sort_start(&self, rows: usize);
    fn on_limit_start(&self, rows: usize, limit: usize);
}
```

### 2. `NoopInstrumentationHook` (default)

Zero-cost — all methods are no-ops via `impl_default_noop!` macro. This is the default for `VolcanoExecutor`.

### 3. `CountingInstrumentationHook` (for tests)

Atomic counters; provides `.seq_scan_count()`, `.filter_count()`, etc.

### 4. Wire into VolcanoExecutor operators

```rust
// In FilterVolcanoExecutor::next():
self.instrumentation.on_filter_start(table, rows.len());
// ... existing logic ...
self.instrumentation.on_filter_end(table, new_rows.len());
```

For v1 scope, wire into 4 operators: SeqScan, Filter, Project, HashJoin.

### 5. Tests (`tests/integration/executor/instrumentation_hooks_test.rs`)

5 tests:
- `noop_hook_has_zero_overhead`
- `counting_hook_records_seq_scan_events`
- `counting_hook_records_filter_events`
- `counting_hook_records_project_events`  
- `counting_hook_records_hash_join_events`

### 6. Documentation

- `docs/releases/v3.11.0/perf/PERF_SCHEMA_HOOKS.md` — trait overview + usage examples

## Capabilities

### New Capabilities

- `instrumentation-hook-trait`: pluggable observer for VolcanoExecutor events
- `counting-instrumentation-hook`: atomic-counter implementation for testing/monitoring
- `noop-instrumentation-hook`: zero-cost default

## Impact

### Affected Files

| File | Type | Lines |
|------|------|-------|
| `crates/executor/src/instrumentation.rs` | new | ~200 |
| `crates/executor/src/lib.rs` | modified | +3 |
| `crates/executor/src/executor.rs` | modified | +10 (instrumentation field) |
| `crates/executor/src/scan.rs` | modified | +5 (seq_scan_start) |
| `crates/executor/src/filter.rs` | modified | +8 (filter_start/end) |
| `crates/executor/src/parallel_hash_join.rs` | modified | +6 (build/probe) |
| `crates/executor/src/parallel_group_by.rs` | modified | +4 (aggregate_start) |
| `tests/integration/executor/instrumentation_hooks_test.rs` | new | ~150 |
| `docs/releases/v3.11.0/perf/PERF_SCHEMA_HOOKS.md` | new | ~80 |
| `Cargo.toml` | modified | +1 test entry |

### No Breaking Changes

- Default `NoopInstrumentationHook` is zero-cost — no measurable perf regression
- Existing VolcanoExecutor usage unchanged
- Tests can opt into `CountingInstrumentationHook` to verify event counts

## Acceptance Criteria

- `cargo test --release --test instrumentation_hooks_test`: 5/5 PASS
- All 51 prior V311 tests still PASS (no regression)
- `PERF_SCHEMA_HOOKS.md` documents trait + usage example
- F-31 in debt-registry: VERIFIED → CLOSED

## Estimated Effort

| Step | Estimate |
|------|----------|
| Define trait + Noop/Counting impls | 4h |
| Wire into 4 operators | 8h |
| Tests | 4h |
| Documentation | 2h |
| PR + merge | 1h |
| **Total** | **19h** |

## Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|------------|
| Hot-path overhead from hook calls | Medium | Use Noop default; benchmarks show <5ns |
| Operator trait changes break existing impls | High | Add hook as optional field; default Noop |
| Lock contention from Counting atomic counters | Low | Only used in tests; production uses Noop |

## Scope

### IN SCOPE (V311-06)

- `InstrumentationHook` trait with 9 event types
- `NoopInstrumentationHook` (default, zero-cost)
- `CountingInstrumentationHook` (atomic counters)
- Wire into SeqScan, Filter, Project, HashJoin
- 5 integration tests
- Documentation

### OUT OF SCOPE (deferred)

- Distributed tracing export (OpenTelemetry)
- Performance Schema SQL tables (`performance_schema.*`)
- Real-time event streaming
- Built-in metrics aggregation

## Out-of-Scope Items

Consistent with V311-22 docs restructure and V311-23 PERF-5 patterns: production Performance Schema SQL tables deferred to v3.12+ scope.
