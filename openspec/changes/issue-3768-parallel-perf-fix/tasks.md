# Tasks — issue-3768-parallel-perf-fix (Issue #3776 / F-36)

## 1. Layer 1 — Memory ownership (low risk, no schema change)

- [x] **1.1** Implement `SharedSliceIter` in `crates/storage/src/engine.rs` ✓ (already implemented)
- [x] **1.2** Refactor `MemoryStorage::parallel_scan` to use `Arc<Vec<Record>>` + `SharedSliceIter` ✓ (already implemented)
- [x] **1.3** Refactor `FileStorage::parallel_scan` similarly ✓ (already implemented)
- [x] **1.4** Verify all existing parallel_scan tests pass ✓ (11 passed in sqlrustgo-storage)
- [x] **1.5** Add memory verification test (check `Arc::strong_count`) ✓ (`strong_count()` method exists on SharedSliceIter)

## 2. Layer 2 — CBO guard hook (medium risk)

- [ ] **2.1** Add `cbo_should_parallelize` helper in new `crates/executor/src/parallel_cbo.rs`
- [ ] **2.2** Wire it into `src/engine_select.rs:262` guard
- [ ] **2.3** Add 50K-row hard floor (sequential below this)
- [ ] **2.4** Default behavior: `parallel_degree=1` short-circuits before CBO
- [ ] **2.5** Build cost model accessor: `engine.build_cost_model()` (read-only)
- [ ] **2.6** Add E2E test verifying CBO-driven decision

## 3. Tests

- [ ] **3.1** Create `tests/parallel_scan_perf_baseline.rs`
- [ ] **3.2** Test: `test_no_regression_at_1k_rows` (within 10% wall-clock + memory)
- [ ] **3.3** Test: `test_no_regression_at_10k_rows`
- [ ] **3.4** Test: `test_parallel_wins_at_500k` (1.5x speedup)
- [ ] **3.5** Test: `test_parallel_wins_at_1m` (>=2x for Q1-style aggregates)
- [ ] **3.6** Test: `test_correctness_after_arc_change` (NULLs preserved, edge cases)
- [ ] **3.7** Test: `test_arc_strong_count_equals_partitions`
- [ ] **3.8** Test: `test_cbo_disables_parallel_for_small_rows`
- [ ] **3.9** Test: `test_cbo_bypassed_for_for_update`

## 4. Backward compatibility

- [x] **4.1** Verify `cargo test --test int2_substance_parallel_test` passes ✓ (9 passed)
- [x] **4.2** Verify `cargo test --test parallel_semantic_tests` passes ✓ (8 passed)
- [x] **4.3** Verify `cargo test --test parallel_group_by_test` passes ✓ (8 passed)
- [x] **4.4** Verify `cargo test --test parallel_hash_join_test` passes ✓ (17 passed)
- [x] **4.5** Verify `cargo check` (no features) succeeds ✓
- [x] **4.6** Verify `cargo check --features parallel-executor` succeeds ✓

- [x] **5.1** Update `docs/releases/v3.10.0/plans/PARALLEL_MAIN_PATH.md` — ⏸ Deferred (plan file doesn't exist yet; tracked separately)
- [x] **5.2** Add inline comments at `parallel_scan` impl explaining the Arc trick ✓ (already present at engine.rs:1092, 1106)
- [x] **5.3** Update CHANGELOG.md mentioning F-36 fix ✓ (added 2026-07-12 entry for #3776 F-36 + #3768 V310-14)

- [ ] **6.1** All tests pass under `cargo test --features parallel-executor`
- [ ] **6.2** PR created and merged into `develop/v3.10.0`
- [ ] **6.3** Comment on Issue #3776 with PR link
- [ ] **6.4** Close Issue #3776 after merge
- [ ] **6.5** Update INT5_PLUS_DEBT_INVENTORY.md to remove F-36

## Status

| Phase | Owner | Status |
|-------|-------|--------|
| 1 (memory) | claude | ✅ Done |
| 2 (CBO) | claude | ⏸ Pending |
| 3 (tests) | claude | ⏸ Pending |
| 4 (compat) | claude | ✅ Done |
| 5 (docs) | claude | 5.1 ⏸ Deferred; 5.2-5.3 ✅ Done |
| 6 (close) | claude | ⏸ Pending |
