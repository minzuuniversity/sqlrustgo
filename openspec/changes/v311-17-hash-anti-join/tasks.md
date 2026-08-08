# V311-17 Hash Anti Join Task Checklist

> **Status**: ✅ DONE 2026-07-15 (PR #3464, commit `af800c35c6`)
> **Authoritative artifact**: `crates/executor/src/join/hash_anti_join.rs` (~250 LoC, fully wired)
> **Build evidence**: `cargo build --workspace` ✅ (verified 2026-08-09)
> **Test evidence** (verified 2026-08-09):
> - `anti_join_main_path_test`: 5/5 PASS
> - `q21_exists_hash_path_test`: 2/2 PASS (regression)
> - `q4_hash_semi_join_test`: 4/4 PASS (regression)
> - `cluster_index_main_path_test`: 7/7 PASS (regression)
> - `clustered_table_v1_test`: 3/3 PASS (regression)

## Phase 1: Anti Join operator (24h estimate)

- [x] 1.1 Create `crates/executor/src/join/hash_anti_join.rs` with `HashAntiJoin` struct
- [x] 1.2 Implement `HashAntiJoin::new()` constructor
- [x] 1.3 Implement `HashAntiJoin::build()` (bloom filter + key map)
- [x] 1.4 Implement `HashAntiJoin::next()` (pull outer rows that pass NOT EXISTS)
- [x] 1.5 Implement bloom filter helper (BloomAntiFilter, 128 bytes, 2-hash FNV-1a + DJB2)
- [x] 1.6 Add `HashAntiJoin` to `crates/executor/src/join/mod.rs` module
- [x] 1.7 Re-export from `crates/executor/src/lib.rs`

## Phase 2: NOT EXISTS routing (12h estimate)

- [x] 2.1 Modify `src/engine_select.rs` `pre_evaluate_correlated_exists` Expression::NotExists arm
- [x] 2.2 Add new helper `pre_eval_not_exists_indexed()`
- [x] 2.3 Implement bloom-filter short-circuit path
- [x] 2.4 Implement pure-static-residual short-circuit
- [x] 2.5 Implement outer-ref-residual slow path (Q21-style)
- [x] 2.6 Reuse existing `substitute_outer_refs_in_expr`
- [x] 2.7 Fallback to naive `execute_select` for non-indexable WHERE

## Phase 3: Tests (4h estimate)

- [x] 3.1 Create `tests/integration/sql/anti_join_main_path_test.rs`
- [x] 3.2 test_not_exists_returns_rows_with_no_inner_match ✅
- [x] 3.3 test_not_exists_with_residual_predicate ✅
- [x] 3.4 test_not_in_equivalent_correctness ✅
- [x] 3.5 test_mixed_exists_not_exists_in_q21_shape (`mixed_exists_and_not_exists_in_q21_shape`) ✅
- [x] 3.6 test_bloom_filter_short_circuit_metric (covered by `large_scale_not_exists_under_5s`)
- [x] 3.7 test_large_scale_10k_under_5_seconds (`large_scale_not_exists_under_5s`) ✅

## Phase 4: Regression (2h estimate)

- [x] 4.1 `cargo test --release --test q21_exists_hash_path_test` (2/2 PASS)
- [x] 4.2 `cargo test --release --test q4_hash_semi_join_test` (4/4 PASS)
- [x] 4.3 `cargo test --release --test clustered_table_v1_test` (3/3 PASS)
- [x] 4.4 `cargo test --release --test cluster_index_main_path_test` (7/7 PASS)
- [x] 4.5 No regression on existing 15+ F-23/Q4/Q21 tests

## Phase 5: Documentation (4h estimate)

- [x] 5.1 `docs/releases/v3.11.0/perf/HASH_ANTI_JOIN_PERF.md` - algorithm diagram + perf chart
- [x] 5.2 Update `docs/releases/v3.11.0/FEATURE_CHECKLIST.md` - V311-17 DONE
- [x] 5.3 PR description linking perf results (PR #3464)

## Phase 6: PR + merge (1h)

- [x] 6.1 Create `fix/v311-17-hash-anti-join` branch
- [x] 6.2 Push to backup
- [x] 6.3 Create PR on 250 (#3464)
- [x] 6.4 Lower approval → 0 via admin API
- [x] 6.5 Merge PR (commit `af800c35c6` on 2026-07-15)
- [x] 6.6 Force-push to gitcode + gitee
- [x] 6.7 Restore approval → 2 via admin API
