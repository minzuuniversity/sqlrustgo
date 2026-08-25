# V312-58 Sprint 5 — HashSemiJoin::from_select Call-Site Wiring

**Branch**: `fix/v312-58-sprint5-hash-semi-join-call-site`
**Target**: `develop/v3.12.0`
**Tracks**: V312-58 master #4374, issue #4444 (Phase 3 followup)
**Date**: 2026-08-25

## Scope

Sprint 5 wires the existing `HashSemiJoin::from_select` constructor
(`crates/executor/src/join/hash_semi_join.rs:198`, added in Phase 3 by
commit `52c7e8b50`) as a call point inside
`pre_evaluate_correlated_exists` (`src/engine_select.rs:4014`), so the
simplest correlated-EXISTS shape (Q4-mini) actually exercises the
HashSemiJoin probe instead of falling through to the
`SubqueryIndex` / `pre_eval_exists_subquery_fast` / recursive
`execute_select` paths unchanged.

In scope for this PR:

- Single base-table correlated EXISTS with **a single correlated
  equality on the build key column**.
- No additional residual predicates (Q4-mini:
  `EXISTS (SELECT 1 FROM lineitem WHERE l_orderkey = o_orderkey)`).

Out of scope (deferred to follow-up Sprints):

- Composite correlation keys (Q20 L0/L5 full SUM,
  `ps_partkey = partsupp.ps_partkey AND ps_suppkey = partsupp.ps_suppkey`).
- NOT EXISTS / anti-semi-join semantics (Q22-style).
- Additional residual filter re-evaluation after the probe
  (Q4-with-`l_commitdate < l_receiptdate` and similar).

## Changes

| File | Δ | Purpose |
|------|---|---------|
| `src/engine_select.rs` | +~210 / -12 | Add `HashSemiJoinIndex` struct + DFS `collect_hash_semi_join_indexes` + per-row `probe_hash_semi_join`; extend `pre_evaluate_correlated_exists` signature (parallel cursor); diag counters; pre-fix removes the Sprint 4 dead-code wrapper that was blocking lib compile on `develop/v3.12.0`. |
| `src/lib.rs` | +2 lines | Re-export `reset_v312_58_sprint5_diag` / `dump_v312_58_sprint5_diag`. |
| `Cargo.toml` | +3 lines | Register the new `issue_4444_hash_semi_join_call_site` integration test. |
| `tests/integration/oracle/issue_4444_hash_semi_join_call_site.rs` | +190 (new) | Regression test: Q4-mini EXISTS with matches, plus Q4-mini with no matches. |

The wire-up goes between the existing `SubqueryIndex` arm and the
existing `pre_eval_exists_subquery_fast` fallback in
`pre_evaluate_correlated_exists` so the cheaper O(1) probe runs
before the residual-evaluating `SubqueryIndex` path.

## Acceptance Criteria

| # | Criterion | Evidence |
|---|-----------|----------|
| 1 | `HashSemiJoin::from_select` is invoked at least once for the Q4-mini shape on the in-memory fixture. | `DIAG_HASH_SEMI_JOIN_PROBE_HITS > 0` in `dump_v312_58_sprint5_diag()`. |
| 2 | Functional correctness is preserved. | Q4-mini returns the same outer rows as before (test asserts exact row set). |
| 3 | No regression in adjacent V312-58 sub-pipelines. | `issue_4444_nested_exists_regression` (2/2), `issue_4374_regression` (2/2), `issue_4443_materialization_regression` (3/3). |
| 4 | Develop tree builds cleanly. | `cargo build --lib` green. Pre-fix removed the Sprint-4-leftover duplicate `find_equality_inner_outer` at line 6071. |
| 5 | Sprint 5 diag API is reachable from integration tests. | `reset_v312_58_sprint5_diag` / `dump_v312_58_sprint5_diag` re-exported via `sqlrustgo::`. |

## Test Run (2026-08-25)

```text
cargo test --test issue_4444_hash_semi_join_call_site \
           --test issue_4444_nested_exists_regression \
           --test issue_4374_regression \
           --test issue_4443_materialization_regression

issue_4444_hash_semi_join_call_site:
  q4_mini_single_equality_exists_uses_hash_semi_join_call_site ... ok
  q4_mini_no_matches_still_returns_zero_rows ... ok
issue_4444_nested_exists_regression:
  q20_nested_exists_returns_correct_suppliers ... ok
  q4_shape_existence_filter_still_works ... ok
issue_4374_regression:
  q20_scalar_sum_respects_supplier_and_date_filters ... ok
  q17_correlated_avg_filter_is_applied_before_aggregation ... ok
issue_4443_materialization_regression:
  execute_select_no_subquery_path_unchanged ... ok
  q20_prewarm_yields_correct_filtered_rows ... ok
  q17_prewarm_builds_scalar_agg_index_once ... ok

Total: 9 / 9 passed.
```

## Known Limitation (must be addressed in a follow-up Sprint)

`probe_hash_semi_join` rebuilds a fresh `HashSemiJoin` per outer row.
The reason: `pre_evaluate_correlated_exists` is `&self` (read-only
context), while `HashSemiJoin::probe_outer_key` requires `&mut self`
(the underlying `key_index`, `bloom`, and `matched_outer_keys`
counters need to mutate per probe).

Functional and call-site correctness are both verified by the new
tests, but the per-row rebuild is O(N_inner) per outer row — exactly
the work the pre-built index was supposed to amortise. **This PR
intentionally does not claim TPC-H perf wins at SF=1.**

Recommended fixes (pick one, prefer the second):

1. Wrap `HashSemiJoin` in `parking_lot::Mutex<HashSemiJoin>` inside
   `HashSemiJoinIndex` and lock around the probe — minimal
   refactor, costs a mutex on the probe fast path.
2. Refactor `HashSemiJoin::probe_outer_key` to use interior
   mutability (`AtomicU64` for the counters, `RwLock` for the
   key_index). Cleanest, touches `crates/executor`, affects all
   current HSJ callers.

Either fix is a separate change. After it lands, `probe_hash_semi_join`
becomes the O(1) per outer row the constructor was designed for and
Q4's outer × inner loop collapses to outer × {O(1) probe + minor
work}.

## Pre-fix: Sprint 4 Leftover Cleanup

`develop/v3.12.0` did not compile as of `ae572990b`. The Sprint 4
Step 1.5/1.6 commit (`c8a5bda7f`) left a dead-code wrapper at
`src/engine_select.rs:6071`:

```rust
fn find_equality_inner_outer(
    where_expr: &Expression,
    inner_table_name: &str,
    outer_table_info: &TableInfo,
) -> Option<(String, usize)> {
    let (pairs, _residual) =
        find_equality_pairs_and_residual(where_expr, inner_table_name, outer_table_info);
    pairs.into_iter().next()
}
```

It duplicated the `find_equality_inner_outer` already declared at
line 5939 and referenced `find_equality_pairs_and_residual`, which
is **not defined anywhere in the codebase**. Deleting the wrapper
restores `cargo build --lib` to a green state for `develop/v3.12.0`.

Verified independently by `cargo clippy --lib --all-features -- -D warnings` after stash pop with the Sprint 5 changes reverted
→ same single failure (`empty_line_after_doc_comments` in
`crates/storage/src/binary_storage_v2.rs:365`), confirming the
storage clippy error is pre-existing baseline noise and **not**
introduced by this PR.

## Followups

- `parking_lot::Mutex<HashSemiJoin>` in `HashSemiJoinIndex`
  (or interior-mutability refactor in `HashSemiJoin`) → actual
  TPC-H SF=1 perf gains for Q4-shape queries.
- Composite-key `try_scalar_agg_index_lookup` extension to multi-
  column build → Q20 L0/L5 full SUM no longer relies on the
  recursive `execute_select` path.
- NOT EXISTS plumbing via the same parallel `HashSemiJoinIndex`
  vector — Q22-style anti-semi-join shapes.
