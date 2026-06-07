## Why

Issue #3281 reports TPC-H Q4 returns the correct 5 distinct priority rows,
but the `count(*)` values are 4x too large:
- PG (truth): 27, 23, 29, 23, 16 (sum=118)
- sqlrustgo: 107, 104, 98, 102, 84 (sum=495)

Root cause: `Expression::Exists(_) => true` in `src/engine_utils.rs:215` is
the conservative fallback for correlated EXISTS subqueries. This makes
**every** outer row pass the EXISTS filter, but Q4's `count(*)` aggregates
those rows. In the SF=0.001 simplified fixture, each order has 4 lineitem
rows, and almost all satisfy `l_commitdate < l_receiptdate`, so a real
EXISTS would return true for those orders — but the count should reflect
the number of orders that match, not the number of (order, lineitem) pairs.

Wait — actually the 4x factor suggests the conservative fallback is
counting outer orders 4 times (once per lineitem) because the EXISTS is
being evaluated in a Cartesian-product context. This is a correlated
subquery implementation bug, not a pure "always true" bug.

Sprint 3 introduced a real correlated-EXISTS evaluator
(`pre_eval_exists_subquery_fast` in commit f5072d99f) but the conservative
fallback path is still being hit for Q4's specific shape. Either the fast
path is not being triggered, or there's a Cartesian-product bug in the
outer-row fanout.

This change fixes the conservative fallback to perform a real EXISTS
check (or routes Q4 to the fast path), then verifies Q4 cell values match
PG: 27/23/29/23/16 (sum=118).

## What Changes

- **Engine** (`src/engine_utils.rs`): Replace `Expression::Exists(_) => true`
  with a real correlated-subquery evaluator that:
  - Executes the inner SELECT against the outer row's column values
  - Returns true only if the inner SELECT returns ≥ 1 row
  - Has a fast path for `EXISTS(SELECT * FROM t WHERE key = outer.key)`
    using a storage index lookup (avoid N² row × row scan)
- **Reproduction test** (`tests/repro_3281_q4_exists_test.rs`): 3 tests:
  1. Minimal: `SELECT * FROM outer WHERE EXISTS(SELECT 1 FROM inner WHERE inner.x = outer.x)` returns the correct count of outer rows that have at least one matching inner.
  2. TPC-H Q4 simplified form: `SELECT o_orderpriority, count(*) FROM orders WHERE o_orderdate >= '1993-07-01' AND o_orderdate < '1993-10-01' AND EXISTS(SELECT * FROM lineitem WHERE l_orderkey = o_orderkey AND l_commitdate < l_receiptdate) GROUP BY o_orderpriority ORDER BY o_orderpriority` against SF=0.001 returns 5 rows with counts matching PG.
  3. Regression: Q13/Q16 (`IN (SELECT ...)` / `NOT IN (SELECT ...)`) still pass.

## Capabilities

### New Capabilities

- `correlated-exists-subquery-evaluation`: The executor MUST evaluate
  `EXISTS(correlated_subquery)` by executing the inner query with the
  outer row's column values bound, returning true iff the inner query
  returns ≥ 1 row. The current `=> true` conservative fallback
  produces wrong results for any query that combines `EXISTS` with
  `count(*)` over the outer table.

### Modified Capabilities

- None (the new capability is the only change required).

## Impact

- **Code**: `src/engine_utils.rs` (replace `Exists(_) => true` with real
  eval), `tests/repro_3281_q4_exists_test.rs` (new), `Cargo.toml`
  (register test).
- **APIs**: No public API change. `evaluate_expression` may take a
  different execution path for `Expression::Exists`.
- **Dependencies**: None.
- **TPC-H**: Q4 cell values flip from `[107, 104, 98, 102, 84]` (wrong)
  to `[27, 23, 29, 23, 16]` (PG truth). Side benefits: any query that
  uses correlated EXISTS with `count(*)` will now be correct.
- **Performance**: Q4 with 150 orders × 614 lineitem is 92,100 row
  evaluations. The conservative `=> true` is O(N) (no subquery). The
  real eval is O(N × M) without an index. If profiling shows this
  regresses wall-clock below the G5 gate (120s), add a
  `lineitem.l_orderkey` B+ tree index. (Not part of this PR; the
  acceptance criteria is correctness, not perf.)
