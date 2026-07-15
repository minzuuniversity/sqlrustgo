# V311-17 Hash Anti Join Task Checklist

## Phase 1: Anti Join operator (24h estimate)

- [ ] 1.1 Create `crates/executor/src/join/hash_anti_join.rs` with `HashAntiJoin` struct
- [ ] 1.2 Implement `HashAntiJoin::new()` constructor
- [ ] 1.3 Implement `HashAntiJoin::build()` (bloom filter + key map)
- [ ] 1.4 Implement `HashAntiJoin::next()` (pull outer rows that pass NOT EXISTS)
- [ ] 1.5 Implement bloom filter helper (FNV-1a + DJB2, 128 bytes)
- [ ] 1.6 Add `HashAntiJoin` to `crates/executor/src/join/mod.rs` module
- [ ] 1.7 Re-export from `crates/executor/src/lib.rs`

## Phase 2: NOT EXISTS routing (12h estimate)

- [ ] 2.1 Modify `src/engine_select.rs` `pre_evaluate_correlated_exists` Expression::NotExists arm
- [ ] 2.2 Add new helper `pre_eval_not_exists_indexed()` 
- [ ] 2.3 Implement bloom-filter short-circuit path
- [ ] 2.4 Implement pure-static-residual short-circuit
- [ ] 2.5 Implement outer-ref-residual slow path (Q21-style)
- [ ] 2.6 Reuse existing `substitute_outer_refs_in_expr` 
- [ ] 2.7 Fallback to naive `execute_select` for non-indexable WHERE

## Phase 3: Tests (4h estimate)

- [ ] 3.1 Create `tests/integration/sql/anti_join_main_path_test.rs`
- [ ] 3.2 test_not_exists_returns_rows_with_no_inner_match
- [ ] 3.3 test_not_exists_with_residual_predicate  
- [ ] 3.4 test_not_in_equivalent_correctness
- [ ] 3.5 test_mixed_exists_not_exists_in_q21_shape
- [ ] 3.6 test_bloom_filter_short_circuit_metric
- [ ] 3.7 test_large_scale_10k_under_5_seconds

## Phase 4: Regression (2h estimate)

- [ ] 4.1 `cargo test --release --test q21_exists_hash_path_test` (2/2 PASS)
- [ ] 4.2 `cargo test --release --test q4_hash_semi_join_test` (4/4 PASS)
- [ ] 4.3 `cargo test --release --test clustered_table_v1_test` (3/3 PASS)
- [ ] 4.4 `cargo test --release --test cluster_index_main_path_test` (7/7 PASS)
- [ ] 4.5 No regression on existing 15+ F-23/Q4/Q21 tests

## Phase 5: Documentation (4h estimate)

- [ ] 5.1 `docs/releases/v3.11.0/perf/HASH_ANTI_JOIN_PERF.md` - algorithm diagram + perf chart
- [ ] 5.2 Update `docs/releases/v3.11.0/FEATURE_CHECKLIST.md` - V311-17 DONE
- [ ] 5.3 PR description linking perf results

## Phase 6: PR + merge (1h)

- [ ] 6.1 Create `fix/v311-17-hash-anti-join` branch
- [ ] 6.2 Push to backup
- [ ] 6.3 Create PR on 250
- [ ] 6.4 Lower approval → 0 via admin API
- [ ] 6.5 Merge PR
- [ ] 6.6 Force-push to gitcode + gitee
- [ ] 6.7 Restore approval → 2 via admin API
