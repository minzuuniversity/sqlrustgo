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

## Known Limitation (Sprint 5 PR)

The Sprint 5 PR shipped the call-site wiring but **rebuilds a fresh
`HashSemiJoin` per outer row** because `pre_evaluate_correlated_exists`
is `&self` while `HashSemiJoin::probe_outer_key` required `&mut self`.
Functional and call-site correctness were both verified by the new
tests, but the per-row rebuild was O(N_inner) per outer row — exactly
the work the pre-built index was supposed to amortise. The Sprint 5
PR intentionally did not claim TPC-H perf wins at SF=1.

## Resolved: interior mutability refactor (followup commit)

The followup commit on the same branch (`fix/v312-58-sprint5-hash-semi-join-call-site`)
addresses the limitation via the option-2 approach recommended above:
**interior mutability inside `HashSemiJoin`**.

### Changes

| File | Δ | Purpose |
|------|---|---------|
| `crates/executor/src/join/hash_semi_join.rs` | +39 / -13 | `probe_count` / `bloom_short_circuits` → `AtomicU64`; `matched_outer_keys: HashSet<Value>` → `parking_lot::Mutex<HashSet<Value>>`; `probe_outer_key` signature `&mut self` → `&self` (lock only on the `Matched` path; counters use `Relaxed` ordering — they are diagnostic metrics, not control flow); `matched_outer_count` stays `&self`, locks to read `.len()`; unit tests updated to `.load(Ordering::Relaxed)`. |
| `src/engine_select.rs` | +35 / -33 | `HashSemiJoinIndex.inner_rows: Vec<Vec<Value>>` → `HashSemiJoinIndex.hsj: HashSemiJoin` (the `from_select` instance is now kept, not discarded); `try_build_hash_semi_join_index_for_subq` retains the built instance; `probe_hash_semi_join` drops the per-row rebuild and now directly calls `idx.hsj.probe_outer_key(...)` (single-line probe). |

### Why interior mutability (option 2) over `Mutex<HashSemiJoin>` (option 1)

- **Zero lock on the `NotMatched` path** — the dominant case for an
  outer row whose probe key is absent from the inner key_index.
  `parking_lot::Mutex` wrapping the entire `HashSemiJoin` would lock
  on every probe.
- **Counter updates are wait-free** via `AtomicU64::fetch_add(Relaxed)`,
  which is sufficient because the counters are diagnostic metrics, not
  control-flow.
- **HashSet write is rare** — only on the `Matched` path, where we
  already paid the cost of the bucket lookup; the additional mutex
  acquisition is in the noise relative to the HashSet allocation.

### Result

`probe_hash_semi_join` is now O(1) per outer row, O(N_inner) once at
build time. No `&mut self` plumbing through
`pre_evaluate_correlated_exists` is required; the index holds a
long-lived `HashSemiJoin` instance and reuses it for every probe.

### Verification

- `cargo test --lib -p sqlrustgo-executor`: 699 / 699 passed
  (8 / 8 in `join::hash_semi_join::tests` after the unit-test updates).
- `cargo test --test issue_4444_hash_semi_join_call_site`: 2 / 2 green
  (Q4-mini single-equality EXISTS, plus the no-matches variant —
  call-site still reached, counter still advances).
- `cargo test --test issue_4444_nested_exists_regression`: 2 / 2 green
  (Q20 + Q4-shape unchanged).
- `cargo test --test issue_4374_regression`: 2 / 2 green.
- `cargo test --test issue_4443_materialization_regression`: 3 / 3 green.
- `cargo clippy --lib` on the touched symbols: 0 new warnings.

### Scope clarification

TPC-H Q4 proper carries a residual `l_commitdate < l_receiptdate`
predicate in the inner WHERE. The `HashSemiJoinIndex` constructor in
`HashSemiJoin::from_select` only models the single-equality shape; the
`try_build_hash_semi_join_index_for_subq` shape gate returns `None`
for residual-bearing subqueries, so the canonical Q4 falls through to
`SubqueryIndex` / `pre_eval_exists_subquery_fast` and is unaffected
by this refactor. The followup therefore does **not** claim a Q4 SF=1
wall-time win — see "Followups" for the residual-filter work that
would extend `HashSemiJoinIndex` to Q4-shaped queries.

## Resolved: residual filter re-evaluation (followup-2 commit)

Followup-2 on the same V312-58 line addresses the residual-filter
limitation called out above. The `try_build_hash_semi_join_index_for_subq`
shape gate now accepts `Equality AND Residual` shapes where the
residual does **not** reference any outer-row column, and the residual
is applied to the inner rows at build time via
`crate::engine_utils::eval_predicate(residual, &row, &inner_table_info)`.
The pre-filtered inner rows are then fed into `HashSemiJoin::from_select`,
so per-outer-row probes stay O(1) — there is no probe-time work for
the residual.

### Changes

| File | Δ | Purpose |
|------|---|---------|
| `src/engine_select.rs` (`HashSemiJoinIndex` struct) | +18 lines | Add `residual: Option<Box<Expression>>` field; doc comment updated to describe the build-time application contract. |
| `src/engine_select.rs` (`try_build_hash_semi_join_index_for_subq`) | +60 lines | Split WHERE into `(equality, residual)` for `Equality AND Residual` shape; `mentions_outer` AST walk conservatively rejects residuals that reference outer-row columns (returns `None` so the caller falls through to `SubqueryIndex` / `pre_eval_exists_subquery_fast`); apply `eval_predicate` to filter inner rows before the HSJ sees them. |
| `tests/integration/tpch/q4_residual_filter_test.rs` | new (180 lines) | 3 regression tests: canonical TPC-H Q4 (`l_orderkey = o_orderkey AND l_commitdate < l_receiptdate`) exercises the residual-bearing HSJ call site; empty inner table; outer-ref-bearing residual correctly rejected by the shape gate. |
| `Cargo.toml` | +3 lines | Register the new `q4_residual_filter_test` integration test. |

### Scope limitations

- **Build-time application only**. A residual that references outer-row
  columns (`mentions_outer(residual) == true`) makes the shape gate
  return `None`; the query falls back to the existing SubqueryIndex /
  `pre_eval_exists_subquery_fast` path. Probe-time re-evaluation for
  outer-ref-bearing residuals is a separate, larger change tracked as
  a followup.
- **AND-chain only**. The shape gate parses `Equality AND Residual` (and
  the symmetric `Residual AND Equality`). Nested ANDs
  (`Equality AND (Residual1 AND Residual2)`) are not parsed — the
  second-level AND is treated as the residual expression and
  `mentions_outer` walks it recursively, which still rejects outer
  refs correctly, but nested-residual optimization (independent
  application of each conjunct) is a followup.
- **`OR`-chains, `NOT`, `LIKE`, `IN`, function calls** in the residual
  are accepted when `eval_predicate` can handle them (it does for all
  of these via `crate::expr_utils::evaluate_expression`).

### Verification (sequential, `--test-threads=1`)

- `cargo test --test q4_residual_filter_test -- --test-threads=1`:
  3 / 3 green.
  - `q4_real_residual_filter_uses_hash_semi_join_call_site` — canonical Q4
    shape: HSJ built (builds=1), probe reached (probe_hits>0), row count
    correct.
  - `q4_real_residual_filter_empty_inner_table` — empty lineitem:
    HSJ still built (builds=1) but probe returns NotMatched (probe_hits>0).
  - `q4_residual_with_outer_ref_falls_back_from_hsj` — `o_orderkey = o_orderkey`
    tautology triggers `mentions_outer` rejection: builds=0.
- `cargo test --test issue_4444_hash_semi_join_call_site --test issue_4444_nested_exists_regression --test issue_4374_regression --test issue_4443_materialization_regression --test q4_hash_semi_join_test -- --test-threads=1`:
  13 / 13 green (no regression in adjacent V312-58 / Q4 / HSJ pipelines).
- `cargo build --lib` — green.
- `cargo clippy --lib` — 0 new warnings on touched symbols.

### Result

Canonical TPC-H Q4 SF=1 (with `l_commitdate < l_receiptdate` residual)
now exercises the `HashSemiJoinIndex` call site instead of falling
through to `SubqueryIndex` / `pre_eval_exists_subquery_fast`. The
`probe_hash_semi_join` path stays O(1) per outer row (residual already
filtered at build time), so the same perf characteristics as the
single-equality Q4-mini path apply.

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

_(none — all V312-58 followups closed; remaining items are listed
  under "Out of Scope" below.)_

## Resolved: Nested-AND residual short-circuit (followup-4 commit)

Followup-4 on the same V312-58 line resolves the last remaining
followup item.

### What changed

The `find_eq_in_and_chain` walker in `try_build_hash_semi_join_index_for_subq`
already collected each non-equality conjunct into a `Vec<Expression>`
during followup-3, but the residual-leaves were then re-ANDed into
a single `Expression` for the `HashSemiJoinIndex.residual` field.
followup-4 keeps them as independent conjuncts by adding a new
field:

```rust
pub residual_conjuncts: Vec<Expression>,
```

and using short-circuit evaluation on both build and probe paths.

### Build-time short-circuit

In `try_build_hash_semi_join_index_for_subq`, the static-residual
filter is now:

```rust
inner_rows.into_iter().filter(|row| {
    residual_conjuncts.iter().all(|c| {
        engine_utils::eval_predicate(c, row, &inner_storage_info)
    })
}).collect()
```

For a residual `R1 AND R2 AND R3` with an inner row where `R1` is
false, `R2` and `R3` are not evaluated for that row. The
`iter().all(...)` short-circuits on the first `false`.

### Probe-time short-circuit

In `probe_hash_semi_join`, when the HSJ reports `Matched` and
`residual_conjuncts` is non-empty, the conjuncts are combined into
a single AND expression in encounter order and outer refs are
substituted once. `eval_predicate` then evaluates the combined
expression with its own short-circuit via the Rust `&&` operator.
For a residual `R1 AND R2 AND R3` with a matched inner row where
`R1` is false, `R2` and `R3` are not evaluated.

### Why combined-substitute beats per-conjunct substitute

A per-conjunct substitute + eval loop (N small AST walks + N small
evals) is more expensive than one combined substitute + one
recursive `eval_predicate` (one larger AST walk + one recursive
eval that short-circuits on the first false conjunct via Rust's
`&&`). The combined path keeps AST-walk cost constant in the
number of conjuncts; per-conjunct cost grows linearly. The
short-circuit property is preserved by `eval_predicate`'s own
`BinaryOp(_, "AND", _) &&` arm.

### Perf

In-process benchmark `q4_nested_and_short_circuit_test::q4_nested_and_static_residual_perf_scale_1k_orders_10k_lineitems`
(1000 orders + 10 000 lineitems, residual `l_commitdate < l_receiptdate AND l_receiptdate < '1995-01-01' AND l_shipmode != 'ZZ'`,
all three conjuncts static):

```
[perf] Q4 Nested-AND static residual (HSJ short-circuit): 1000 orders +
       10000 lineitems in 15.09 ms; builds=1, probe_hits=1000, rows=1000
```

For comparison, the single-conjunct static residual baseline
(`q4_residual_filter_scale_test`, followup-2) was 7.03 ms. The
3-conjunct path is ~2× slower than the single-conjunct path
because build-time pre-filter still runs `eval_predicate` three
times per inner row (once per conjunct) and probe-time combined
substitute walks a 3-conjunct AST instead of a 1-conjunct one.
Both numbers remain well below the 5 s threshold, and the
short-circuit property means that for residuals with a frequently
false leading conjunct, the third conjunct is never evaluated.

### Verification (sequential, `--test-threads=1`)

- `cargo test --test q4_nested_and_short_circuit_test -- --test-threads=1`: 2 / 2 green.
  - `q4_nested_and_static_residual_short_circuits_in_order` — Q4 with 3-conjunct residual: HSJ built (`builds=1`), probe reached, result row count correct.
  - `q4_nested_and_static_residual_perf_scale_1k_orders_10k_lineitems` — 10K lineitem scale, target < 10s (achieved 15.09 ms release).
- `cargo test --test q4_residual_filter_test -- --test-threads=1`: 3 / 3 green (no regression).
- `cargo test --test q4_residual_filter_scale_test -- --test-threads=1`: 1 / 1 green (no regression on the single-conjunct static-residual path).
- `cargo test --test q4_probe_time_residual_test -- --test-threads=1`: 3 / 3 green (no regression on the probe-time outer-ref path; that path still uses conjunct evaluation through the combined-substitute route).
- `cargo test --test q4_hash_semi_join_test`: 4 / 4 green (no regression on the SubqueryIndex path).
- `cargo test --test issue_4444_hash_semi_join_call_site --test issue_4444_nested_exists_regression --test issue_4374_regression --test issue_4443_materialization_regression`: 9 / 9 green.
- `cargo build --lib`: green.
- `cargo clippy --lib`: 0 new warnings on touched symbols.

## Out of Scope (still deferred after V312-58)

- Composite correlation keys (Q20 L0/L5 full SUM).
- NOT EXISTS / anti-semi-join semantics (Q22-style patterns).

## Resolved: probe-time residual re-evaluation for outer-ref residuals (followup-3 commit)

Followup-3 on the same V312-58 line completes the residual filter
work by handling outer-row-dependent residuals at probe time.

### What changed

The `try_build_hash_semi_join_index_for_subq` shape gate's
`mentions_outer(residual)` rejection was relaxed: residuals that
reference outer columns now stay in `HashSemiJoinIndex.residual`,
and every inner row (including those that would fail the residual)
is added to the HSJ bucket at build time. Probe-time work, in
`probe_hash_semi_join`, substitutes the outer row's values into
the residual using the existing
`crate::engine_utils::substitute_outer_refs_in_expr`, then
re-evaluates the substituted residual against every inner row
returned by `HashSemiJoin::get_inner_for_key(probe_value)`. The
probe verdict is `Matched` iff at least one inner row passes the
residual.

The recursive AND-tree walker (`find_eq_in_and_chain`) replaces
the previous 2-way-only `BinaryOp(_, "AND", _)` match, so nested
AND-trees like `(<eq> AND <r1>) AND <r2>` are also handled — the
first top-level `Equality(Identifier, Identifier)` anywhere in
the tree is the correlated equality, every other conjunct (any
depth) becomes the residual.

### Static vs outer-ref residual split

- **Static residual** (no outer reference): `try_build_hash_semi_join_index_for_subq`
  pre-filters inner rows at build time. Probe is a single O(1)
  `probe_outer_key` lookup.
- **Outer-ref residual**: every inner row stays in the HSJ. Probe
  performs the O(1) `probe_outer_key` lookup, then — only on
  `Matched` — iterates `get_inner_for_key` rows and
  `eval_predicate`s the substituted residual. The
  `NotMatched` path is unchanged (zero work after `probe_outer_key`).

### Perf

In-process benchmark `q4_probe_time_residual_test::q4_outer_ref_residual_perf_scale_1k_orders_10k_lineitems`
(1000 orders + 10 000 lineitems, residual `l_commitdate < l_receiptdate AND o_orderkey > 0`):

```
[perf] Q4 outer-ref residual (HSJ probe-time path): 1000 orders +
       10000 lineitems in 21.90 ms; builds=1, probe_hits=1000, rows=1000
```

This is ~3× slower than the static-residual path (7.03 ms in
`q4_residual_filter_scale_test`), which is expected: each
`Matched` outer row now walks its `get_inner_for_key` bucket
(here, ~10 rows on average) instead of returning immediately.

### Verification (sequential, `--test-threads=1`)

- `cargo test --test q4_probe_time_residual_test -- --test-threads=1`: 3 / 3 green.
  - `q4_outer_ref_residual_tautology_uses_probe_time_path` — canonical Q4 + `o_orderkey = o_orderkey` tautology residual: HSJ built (`builds=1`), probe reached, row count correct.
  - `q4_outer_ref_residual_substitutes_per_outer_row` — `o_orderkey > 0` residual: same shape, verifies the per-outer-row substitution.
  - `q4_outer_ref_residual_perf_scale_1k_orders_10k_lineitems` — 10K lineitem scale, target < 10s (achieved 21.9 ms).
- `cargo test --test q4_residual_filter_test -- --test-threads=1`: 3 / 3 green.
  - The stale `q4_residual_with_outer_ref_falls_back_from_hsj`
    (which expected `builds == 0` under the followup-2 contract)
    was renamed to `q4_residual_with_outer_ref_uses_probe_time_path`
    and updated to expect `builds == 1` and a correct result row
    count.
- `cargo test --test q4_residual_filter_scale_test -- --test-threads=1`: 1 / 1 green (no regression on the static-residual perf path).
- `cargo test --test issue_4444_hash_semi_join_call_site --test issue_4444_nested_exists_regression --test issue_4374_regression --test issue_4443_materialization_regression --test q4_hash_semi_join_test`: 13 / 13 green (no regression in adjacent V312-58 / Q4 / HSJ pipelines).
- `cargo build --lib`: green.
- `cargo clippy --lib`: 0 new warnings on touched symbols.

### Out of Scope (still deferred after this PR)

- **Nested-AND residual short-circuit optimization** (split
  `Equality AND (R1 AND R2)` into independent conjuncts and
  short-circuit on first false). Currently both conjuncts are
  re-ANDed into a single `Expression` and evaluated as one.
- Composite correlation keys (Q20 L0/L5 full SUM).
- NOT EXISTS / anti-semi-join semantics (Q22-style patterns).
- Composite-key `try_scalar_agg_index_lookup` extension to multi-
  column build → Q20 L0/L5 full SUM no longer relies on the
  recursive `execute_select` path.
- NOT EXISTS plumbing via the same parallel `HashSemiJoinIndex`
  vector — Q22-style anti-semi-join shapes.
