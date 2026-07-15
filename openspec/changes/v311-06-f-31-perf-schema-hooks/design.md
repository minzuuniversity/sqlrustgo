## Context

The v3.11.0 release focuses on production observability alongside the prior V311-15/17 perf work. F-31 (Performance Schema hooks) is a **trait-based observer pattern** for VolcanoExecutor that provides the foundation for any future Performance Schema implementation.

### Current Limitations

1. VolcanoExecutor operators have no observer hooks — only a few `tracing::info_span!` calls in `engine_select.rs` (engine layer, not operator layer).
2. No way for tests to count "how many SeqScans happened in this query?"
3. No portable observer pattern — adding tracing later requires rippling edits.

### Target Architecture

```
VolcanoExecutor operator (Filter, SeqScan, etc.)
    │
    ├── instrumentation: Arc<dyn InstrumentationHook>  (default: NoopInstrumentationHook)
    │       │
    │       ├── on_filter_start(rows_in: usize)
    │       ├── on_filter_end(rows_out: usize)
    │       └── ... 8 more events
    │
    └── (existing logic)
```

Implementations:
- `NoopInstrumentationHook` — empty trait methods, zero-cost default
- `CountingInstrumentationHook` — atomic counters, for tests and benchmarks

## Goals / Non-Goals

**Goals:**
- Define `InstrumentationHook` trait with 9 event types
- Implement `NoopInstrumentationHook` and `CountingInstrumentationHook`
- Wire into 4 Volcano operators: SeqScan, Filter, Project, HashJoin (BasicJoin)
- Verify zero overhead via direct call in tests
- Provide PASSING tests demonstrating event counts

**Non-Goals:**
- Wiring into ALL operators (Aggregator, Sort, Limit deferred)
- Performance Schema SQL tables (`performance_schema.*`)
- OpenTelemetry export
- Distributed tracing

## Decisions

### Decision 1: Trait-based, not enum-based dispatch

**Choice**: Use `pub trait InstrumentationHook: Send + Sync` with `Arc<dyn InstrumentationHook>` on operators.

**Alternative considered**: Enum-dispatch with explicit match on each operator event. Rejected — adds a match for every event in every observer site; doesn't scale.

**Tradeoff**: One virtual call per event (negligible at 1-2 ns/call).

### Decision 2: Default to NoopInstrumentationHook

**Choice**: Every constructor of Volcano operators gets `NoopInstrumentationHook` by default. Switching to counting is opt-in.

**Alternative considered**: Make the trait mandatory. Rejected — pollutes every test that's not checking events.

### Decision 3: Atomic counters in CountingInstrumentationHook

**Choice**: `std::sync::atomic::AtomicU64` for each counter with `.fetch_add(1, Ordering::Relaxed)`.

**Why Relaxed**: We're just counting — no happens-before needed; relaxed is the cheapest.

### Decision 4: 9 event types covering major query phases

**Choice**: Cover:
- `on_seq_scan_start(table)`
- `on_filter_start(table, rows_in)` / `on_filter_end(table, rows_out)`
- `on_project_start(table, rows_in)` / `on_project_end(table, rows_out)`
- `on_hash_join_build(side)` / `on_hash_join_probe(rows)`
- `on_aggregate_start(groups)`
- `on_sort_start(rows)`

**Skipped for v2** (deferred to future PRs):
- `on_limit_start(rows, limit)` — trivial hook
- `on_parallel_*` — parallel variants

## Implementation Plan

### Phase 1: Trait + impls

```rust
// crates/executor/src/instrumentation.rs
pub trait InstrumentationHook: Send + Sync {
    fn on_seq_scan_start(&self, table: &str) {}
    fn on_filter_start(&self, table: &str, rows_in: usize) {}
    fn on_filter_end(&self, table: &str, rows_out: usize) {}
    fn on_project_start(&self, table: &str, rows_in: usize) {}
    fn on_project_end(&self, table: &str, rows_out: usize) {}
    fn on_hash_join_build(&self, side: &str) {}
    fn on_hash_join_probe(&self, rows: usize) {}
    fn on_aggregate_start(&self, groups: usize) {}
    fn on_sort_start(&self, rows: usize) {}
}

pub struct NoopInstrumentationHook;
impl InstrumentationHook for NoopInstrumentationHook {}

pub struct CountingInstrumentationHook { ... }
impl CountingInstrumentationHook {
    pub fn seq_scan_count(&self) -> u64 { ... }
    pub fn filter_count(&self) -> u64 { ... }
}
impl InstrumentationHook for CountingInstrumentationHook { ... }
```

### Phase 2: Wire into operators

In `crates/executor/src/seq_scan.rs`:
```rust
impl VolcanoExecutor for SeqScanVolcanoExecutor {
    fn next(&mut self) -> ... {
        self.instrumentation.on_seq_scan_start(&self.table);
        // existing logic
    }
}
```

Similar for Filter, Project, HashJoin.

### Phase 3: Tests

5 integration tests verifying event counts after typical queries.

### Phase 4: Documentation + PR

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Trait dispatch overhead | Atomic Relaxed is ~1 ns; Noop is 0 |
| Constructor changes break impls | Default to Noop; `with_instrumentation()` setter |
| CountingHook double-counts if same operator called twice | Hook impl is idempotent — accumulator pattern |

## Verification

- 5 new integration tests PASS
- 51 prior V311 tests still PASS (no regression)
- F-31 in debt-registry: VERIFIED → CLOSED
