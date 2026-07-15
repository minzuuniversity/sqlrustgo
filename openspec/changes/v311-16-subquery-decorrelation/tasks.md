# V311-16 Subquery Decorrelation Task Checklist

## Phase 1: Pattern Detection (8h estimate)

- [ ] 1.1 Create `crates/optimizer/src/decorrelate.rs` with `SubqueryPattern` enum
- [ ] 1.2 Implement `find_correlated_subqueries()` — walks Expression tree
- [ ] 1.3 Pattern: `ExistsSemi` for `WHERE EXISTS (SELECT ...)`
- [ ] 1.4 Pattern: `NotExistsAnti` for `WHERE NOT EXISTS (SELECT ...)`
- [ ] 1.5 Pattern: `InToInnerJoin` for `WHERE x IN (SELECT y FROM ...)`
- [ ] 1.6 Pattern: `ScalarAggGroupBy` for `SELECT (SELECT AGG(col) WHERE x = outer.x)`
- [ ] 1.7 Export from `crates/optimizer/src/lib.rs`

## Phase 2: Lifting Algorithm (16h estimate)

- [ ] 2.1 `try_decorrelate()` — main entry, returns true if changed
- [ ] 2.2 `lift_inner_select()` — extracts the inner SELECT body
- [ ] 2.3 `split_static_filter()` — separates static (inner-only) from correlated parts
- [ ] 2.4 `build_inline_view()` — wraps the lifted SELECT + static filter
- [ ] 2.5 `rewrite_to_join()` — replaces `Filter + Subquery` with `Join (Semi/Anti/Inner)`
- [ ] 2.6 Handle NOT IN NULL semantics (preserve 3-valued logic)

## Phase 3: Optimizer Chain Wiring (4h estimate)

- [ ] 3.1 Modify `crates/optimizer/src/query_planner.rs::optimize()` to call `try_decorrelate()`
- [ ] 3.2 Verify pass ordering: decorrelate BEFORE join_reorder
- [ ] 3.3 Decorrelated plan flows through predicate_pushdown → join_reorder → cost_aware
- [ ] 3.4 No-op when no subqueries detected

## Phase 4: Tests (8h estimate)

- [ ] 4.1 Create `tests/integration/optimizer/decorrelation_test.rs`
- [ ] 4.2 test_simple_exists_decorrelates
- [ ] 4.3 test_exists_with_residual_decorrelates
- [ ] 4.4 test_not_exists_decorrelates
- [ ] 4.5 test_in_subquery_decorrelates
- [ ] 4.6 test_correlated_scalar_subquery_decorrelates
- [ ] 4.7 test_non_correlated_subquery_unchanged
- [ ] 4.8 test_subquery_with_or_passthrough
- [ ] 4.9 test_tpc_h_q2_shape_decorrelation

## Phase 5: Regression + Bench (4h estimate)

- [ ] 5.1 `cargo test --release --test q4_hash_semi_join_test` (4/4)
- [ ] 5.2 `cargo test --release --test anti_join_main_path_test` (5/5)
- [ ] 5.3 `cargo test --release --test q21_exists_hash_path_test` (2/2)
- [ ] 5.4 `cargo test --release --test cluster_index_main_path_test` (7/7)
- [ ] 5.5 `cargo test --release --test clustered_table_v1_test` (3/3)
- [ ] 5.6 Add `[[test]]` entry for `decorrelation_test`

## Phase 6: Documentation (1h)

- [ ] 6.1 `docs/releases/v3.11.0/perf/SUBQUERY_DECORRELATION.md` (perf comparison + algorithm diagram)
- [ ] 6.2 Update `docs/releases/v3.11.0/FEATURE_CHECKLIST.md` — V311-16 → DONE
- [ ] 6.3 Update `docs/governance/debt/debt-registry.yaml` if relevant

## Phase 7: PR + merge (30min)

- [ ] 7.1 Branch `fix/v311-16-subquery-decorrelation`
- [ ] 7.2 Push to backup
- [ ] 7.3 Create PR
- [ ] 7.4 Lower approval → 0
- [ ] 7.5 Merge
- [ ] 7.6 Force-push to gitcode + gitee
- [ ] 7.7 Restore approval → 2

## Phase 8: Close Issue (NEW)

- [ ] 8.1 Search/create V311-16 issue tracker on 250
- [ ] 8.2 After merge, post completion summary  
- [ ] 8.3 Close the issue
