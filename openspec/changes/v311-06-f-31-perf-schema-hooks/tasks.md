# V311-06 Performance Schema Hooks Task Checklist

## Phase 1: Trait + impls (~4h)

- [ ] 1.1 Create `crates/executor/src/instrumentation.rs`
- [ ] 1.2 Define `pub trait InstrumentationHook: Send + Sync` with 9 default-implemented methods
- [ ] 1.3 Implement `pub struct NoopInstrumentationHook`
- [ ] 1.4 Implement `pub struct CountingInstrumentationHook` with `AtomicU64` counters
- [ ] 1.5 Add `CountingInstrumentationHook::seq_scan_count()`, `_filter_count()`, etc.
- [ ] 1.6 Export from `crates/executor/src/lib.rs`

## Phase 2: Wire into operators (~8h)

- [ ] 2.1 Add `instrumentation: Arc<dyn InstrumentationHook>` to VolcanoExecutor constructors
- [ ] 2.2 Wire `on_seq_scan_start` in `crates/executor/src/scan.rs`
- [ ] 2.3 Wire `on_filter_start/end` in `crates/executor/src/filter.rs`
- [ ] 2.4 Wire `on_project_start/end` in projection layer
- [ ] 2.5 Wire `on_hash_join_build/probe` in `parallel_hash_join.rs`
- [ ] 2.6 Add `with_instrumentation()` setter method on key operators

## Phase 3: Tests (~4h)

- [ ] 3.1 Create `tests/integration/executor/instrumentation_hooks_test.rs`
- [ ] 3.2 test_noop_hook_has_zero_overhead
- [ ] 3.3 test_counting_hook_records_seq_scan_events
- [ ] 3.4 test_counting_hook_records_filter_events
- [ ] 3.5 test_counting_hook_records_project_events
- [ ] 3.6 test_counting_hook_records_hash_join_events
- [ ] 3.7 Add `[[test]]` entry to `Cargo.toml`

## Phase 4: Regression + Docs (~2h)

- [ ] 4.1 Run all 51 prior V311 tests — all PASS
- [ ] 4.2 `docs/releases/v3.11.0/perf/PERF_SCHEMA_HOOKS.md` (trait overview, usage example)
- [ ] 4.3 Update `docs/governance/debt/debt-registry.yaml`: F-31 → CLOSED
- [ ] 4.4 Update `docs/releases/v3.11.0/FEATURE_CHECKLIST.md`: V311-06 → DONE

## Phase 5: PR + merge (~1h)

- [ ] 5.1 Branch `fix/v311-06-f-31-perf-schema-hooks`
- [ ] 5.2 Push to backup
- [ ] 5.3 Create PR on 250
- [ ] 5.4 Lower approval → 0
- [ ] 5.5 Merge
- [ ] 5.6 Force-push to gitcode + gitee
- [ ] 5.7 Restore approval → 2

## Phase 6: Issues (~5min)

- [ ] 6.1 Search/create V311-06 issue tracker on 250
- [ ] 6.2 Post completion comment
- [ ] 6.3 Close the issue
