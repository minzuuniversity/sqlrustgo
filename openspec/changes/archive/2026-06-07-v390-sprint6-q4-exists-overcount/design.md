## Context

Issue #3281 reports Q4 returns 5 distinct priority rows (correct after
Sprint 3 PR #3250) but the `count(*)` values are ~4x too large:
- PG (truth): 27, 23, 29, 23, 16 (sum=118)
- sqlrustgo: 107, 104, 98, 102, 84 (sum=495)

Sprint 1.5 cell-diff analysis (Sprint 4 doc) identified the cause:
`Expression::Exists(_) => true` in `src/engine_utils.rs:215` is a
conservative fallback that returns true for any correlated EXISTS. In
Q4's simplified form, the inner `SELECT * FROM lineitem WHERE
l_orderkey = o_orderkey AND l_commitdate < l_receiptdate` is a
correlated subquery — the conservative `=> true` causes every outer
`orders` row to pass the WHERE, but the `count(*)` over `orders` then
sums them up.

Wait, that's not quite right either — if `=> true`, every orders row
passes, and `count(*)` would be the total orders (150), not 5x of PG.
The actual observed behavior (107/104/98/102/84) is suggestive of a
Cartesian-product join between orders and lineitem: 4 lineitem per
order × 27/23/... orders = 108/92/... observed counts.

So the actual bug is more subtle: the EXISTS subquery is being
**joined** with the outer row rather than evaluated as a filter. This
suggests the correlated-subquery evaluator at `pre_eval_exists_subquery_fast`
(commit f5072d99f) is being bypassed, and the conservative fallback
at engine_utils.rs:215 is hit — but the conservative fallback
shouldn't cause a Cartesian product. The actual mechanism is likely:

1. The WHERE clause is being evaluated with the EXISTS expression
   un-substituted (still as `Expression::Exists(...)`)
2. The conservative `=> true` returns true for ALL orders
3. The 4x factor is something else — maybe the `count(*)` is being
   computed before the WHERE filter is applied, and 4 lineitem per
   order contributes 4x via the JOIN with lineitem (Q4's FROM clause
   has only `orders`, no lineitem, so this doesn't apply)

The truth will come from reading the actual code and running the repro.

## Goals / Non-Goals

**Goals:**

- Fix Q4 cell values to match PG: `[27, 23, 29, 23, 16]`.
- Add 3 minimal repro tests that fail pre-fix and pass post-fix.
- No regression on Q13/Q16/any other correlated-subquery query.

**Non-Goals:**

- Refactor the entire subquery evaluator (no architectural change).
- Add a B+ tree index on `lineitem.l_orderkey` (deferred; only if perf
  regresses the G5 gate).
- Fix the broader "v2 test 10/22" pass rate beyond Q4.

## Decisions

- **Decision 1: Real correlated-EXISTS evaluation via fast path** (if
  applicable) or via the conservative `=> true` replacement with an
  actual subquery execution.
- **Decision 2: 3-test scope** (minimal repro, Q4 simplified, Q13/Q16
  regression). Don't expand to all correlated-subquery cases in this
  PR.
- **Decision 3: Use the existing `pre_eval_exists_subquery_fast`**
  infrastructure (commit f5072d99f) — investigate why it's not being
  hit for Q4, then either route to it or fix the conservative fallback.
- **Decision 4: Touch minimum code** — likely 1 file
  (`src/engine_utils.rs` or wherever the conservative fallback lives).
  No refactor of `Expression::Exists` semantics.

## Risks / Trade-offs

- **Risk**: The real correlated-EXISTS evaluator may be slow for
  queries with many outer rows (Q4: 150 orders × 614 lineitem =
  92,100 row evaluations). Without an index, this is O(N×M).
  Mitigated by: G5 gate timeout is 120s for the full TPC-H run;
  Q4 alone with 150×614 should complete in seconds, well under 120s.
- **Risk**: Changing the conservative fallback may regress other
  queries that depend on the "always true" behavior. Mitigated by:
  run all 22 v2 tests + Q13/Q16 specifically to detect regressions.
- **Trade-off**: This is a 6-7h task per the issue estimate. Sprint 6
  may not have the time budget. Mitigate by: focus on the minimum
  change that fixes Q4 cell values, defer perf optimization.
