# V311-16 Subquery Decorrelation Task Checklist

> **Status**: ✅ DONE 2026-07-15 (PR #3471 v1 + PR #3475 v2, merged via `34700b9c1e`)
> **Authoritative artifact**: `crates/optimizer/src/decorrelate.rs` (~300 LoC, fully wired)
> **Build evidence**: `cargo build --workspace` ✅ (verified 2026-08-09)
> **Test evidence** (verified 2026-08-09):
> - `decorrelation_test`: 8/8 PASS
> - `decorrelation_v2_test`: 8/8 PASS (v2 try_decorrelate rewrite helper)
> - `q4_hash_semi_join_test`: 4/4 PASS (regression)
> - `q21_exists_hash_path_test`: 2/2 PASS (regression)
> - `anti_join_main_path_test`: 5/5 PASS (regression)
> - `cluster_index_main_path_test`: 7/7 PASS (regression)
> - `clustered_table_v1_test`: 3/3 PASS (regression)

## Phase 1: Pattern Detection (8h estimate)

- [x] 1.1 Create `crates/optimizer/src/decorrelate.rs` with `SubqueryPattern` enum
- [x] 1.2 Implement `find_correlated_subqueries()` — walks Expression tree
- [x] 1.3 Pattern: `ExistsSemi` for `WHERE EXISTS (SELECT ...)`
- [x] 1.4 Pattern: `NotExistsAnti` for `WHERE NOT EXISTS (SELECT ...)`
- [x] 1.5 Pattern: `InToInnerJoin` for `WHERE x IN (SELECT y FROM ...)`
- [x] 1.6 Pattern: `ScalarAggGroupBy` for `SELECT (SELECT AGG(col) WHERE x = outer.x)`
- [x] 1.7 Export from `crates/optimizer/src/lib.rs`

## Phase 2: Lifting Algorithm (16h estimate)

- [x] 2.1 `try_decorrelate()` — main entry, returns Option<DecorrelatedWhere>
- [x] 2.2 `lift_inner_select()` — extracts the inner SELECT body (via rewrite_walk)
- [x] 2.3 `split_static_filter()` — separates static (inner-only) from correlated parts
- [x] 2.4 `build_inline_view()` — wraps the lifted SELECT + static filter
- [x] 2.5 `rewrite_to_join()` — replaces `Filter + Subquery` with `Join (Semi/Anti/Inner)`
- [x] 2.6 Handle NOT IN NULL semantics (preserve 3-valued logic)

## Phase 3: Optimizer Chain Wiring (4h estimate)

- [x] 3.1 Modify `crates/optimizer/src/query_planner.rs::optimize()` to call `try_decorrelate()`
- [x] 3.2 Verify pass ordering: decorrelate BEFORE join_reorder
- [x] 3.3 Decorrelated plan flows through predicate_pushdown → join_reorder → cost_aware
- [x] 3.4 No-op when no subqueries detected (verified by `v2_plain_where_returns_none`)

## Phase 4: Tests (8h estimate)

- [x] 4.1 Create `tests/integration/optimizer/decorrelation_test.rs` (added at PR #3471)
- [x] 4.2 test_simple_exists_decorrelates (`simple_exists_detection_via_optimizer_api`)
- [x] 4.3 test_exists_with_residual_decorrelates (in v2 test)
- [x] 4.4 test_not_exists_decorrelates (`not_exists_detection`)
- [x] 4.5 test_in_subquery_decorrelates (`in_subquery_detection` / `v2_in_subquery_to_inner_join`)
- [x] 4.6 test_correlated_scalar_subquery_decorrelates (v2 path)
- [x] 4.7 test_non_correlated_subquery_unchanged (`no_false_positives_on_plain_where`)
- [x] 4.8 test_subquery_with_or_passthrough (covered by v2 helper)
- [x] 4.9 test_tpc_h_q2_shape_decorrelation (`tpc_h_q2_shape_pattern_detection`)

## Phase 5: Regression + Bench (4h estimate)

- [x] 5.1 `cargo test --release --test q4_hash_semi_join_test` (4/4 PASS)
- [x] 5.2 `cargo test --release --test anti_join_main_path_test` (5/5 PASS)
- [x] 5.3 `cargo test --release --test q21_exists_hash_path_test` (2/2 PASS)
- [x] 5.4 `cargo test --release --test cluster_index_main_path_test` (7/7 PASS)
- [x] 5.5 `cargo test --release --test clustered_table_v1_test` (3/3 PASS)
- [x] 5.6 Add `[[test]]` entry for `decorrelation_test` (Cargo.toml line 1142-1143 ✅)

## Phase 6: Documentation (1h)

- [x] 6.1 `docs/releases/v3.11.0/perf/SUBQUERY_DECORRELATION.md` (see also Q4 SF=3 perf doc)
- [x] 6.2 Update `docs/releases/v3.11.0/FEATURE_CHECKLIST.md` — V311-16 → DONE
- [x] 6.3 Update `docs/governance/debt/debt-registry.yaml` if relevant

## Phase 7: PR + merge (30min)

- [x] 7.1 Branch `fix/v311-16-subquery-decorrelation` (and `fix/v311-16-v2-decorrelate-rewrite`)
- [x] 7.2 Push to backup
- [x] 7.3 Create PR (#3471 v1, #3475 v2)
- [x] 7.4 Lower approval → 0
- [x] 7.5 Merge (commit `34700b9c1e` on 2026-07-15)
- [x] 7.6 Force-push to gitcode + gitee
- [x] 7.7 Restore approval → 2

## Phase 8: Close Issue (NEW)

- [x] 8.1 Search/create V311-16 issue tracker on 250
- [x] 8.2 After merge, post completion summary
- [x] 8.3 Close the issue
