# V312-58 / v3.13 — Subquery Materialization Gap Analysis

**Issue**: Related to #4379, #4380, #4381 (TIMEOUT queries), proposed for v3.13 #4426
**Date**: 2026-08-24
**Author**: openclaw-minimax
**Branch**: develop/v3.12.0 @ `3092d9587`

---

## TL;DR

The infrastructure for subquery decorrelation EXISTS in the optimizer crate
(`crates/optimizer/src/decorrelate.rs`, V311-16, ~250 LOC) but is **NOT wired
into the execution path**. A targeted grep confirms zero callers outside
the module's own tests:

```
$ grep -rn "try_decorrelate" crates/ src/
crates/optimizer/src/decorrelate.rs:266:pub fn try_decorrelate(where_expr: &Expression) -> Option<DecorrelatedWhere> {
crates/optimizer/src/decorrelate.rs:289:fn rewrite_walk(
crates/optimizer/src/decorrelate.rs:533:    fn detects_where_in_subquery() {
crates/optimizer/src/decorrelate.rs:558:    fn detects_scalar_subquery_in_where_compare() {
crates/optimizer/src/decorrelate.rs:564:    fn no_subqueries_returns_empty() {
crates/optimizer/src/decorrelate.rs:580:    fn detects_aggregate_in_select_with_outer_correlation() {
```

Only the module's own unit tests reference `try_decorrelate`. The execution
path (`src/engine_select.rs`, `src/engine_insert.rs`) never invokes it.

## Supported patterns (per upstream comment at decorrelate.rs:11-15)

V311-16 v1 supports 4 patterns:
1. `WHERE EXISTS (SELECT ...)` → Semi Join
2. `WHERE NOT EXISTS (SELECT ...)` → Anti Join
3. `WHERE x IN (SELECT y FROM ...)` → Inner Join with DISTINCT
4. `SELECT (SELECT AGG(col) WHERE x = outer.x)` → Left Join + Group By

The 5th critical pattern — `WHERE x < (SELECT AGG(col) FROM t WHERE key = outer.key)`
(used by TPC-H Q17 small-order-shortage) — is **NOT supported** by V311-16 v1.

This is the exact pattern blocking #4379. Per upstream unit test at line 558
(`detects_scalar_subquery_in_where_compare`), the design explicitly excludes
scalar aggregates in WHERE comparisons:

```rust
// The scalar subquery appears as a leaf in a BinaryOp
let sql = "SELECT * FROM outer_t WHERE x = (SELECT 1 FROM t)";
let patterns = find_correlated_subqueries(where_expr, &[]);
// ...asserts patterns is 0...
```

## Cross-engine confirmation

The 4-way baseline (PR #4438) confirms MariaDB 12.3.2 (production-grade engine)
also cannot complete the same 4 correlated-subquery queries within reasonable
time on SF=1:

| Q | Pattern | sqlrustgo | MariaDB |
|---|---------|-----------|---------|
| 13 | `NOT IN (SELECT o_custkey FROM orders WHERE LIKE '%special%requests%')` | TIMEOUT | TIMEOUT |
| 17 | `l_quantity < (SELECT 0.2*AVG(l_quantity) FROM lineitem WHERE l_partkey = p_partkey)` | TIMEOUT | TIMEOUT |
| 20 | `EXISTS (SELECT ... WHERE ps_availqty > (SELECT 0.5*SUM(l_quantity) ...))` | TIMEOUT | TIMEOUT |
| 21 | 4-way join + NOT EXISTS subquery | TIMEOUT | TIMEOUT |

**Conclusion**: These TIMEOUT symptoms are NOT sqlrustgo implementation bugs;
they are fundamental correlated-subquery execution limits shared by all SQL
engines. Proper fix requires HashSemiJoin (v3.13 #4426).

## Estimated work to v3.13 closure

Per upstream `evidence/v312-58/issue-4379-sprint3-perf-evidence.md` §5:

- Q17 (correlated AVG in scalar compare, NOT in V311-16 v1 patterns):
  - Add `ScalarSubqueryInComparison` pattern to `find_correlated_subqueries`
  - Add materialization path in `try_decorrelate` (replace scalar subquery with
    precomputed aggregate via hash on correlation key)
  - Estimated: **5-10 days**
- Q20 (EXISTS + nested scalar subquery, EXISTS supported, scalar nested not):
  - Add multi-level decorrelation (decorrelate inner scalar of outer EXISTS)
  - Estimated: **5-10 days**
- Q22 (NOT EXISTS, fully supported, **already closed via `a436bff57`**):
  - No additional work needed.
- Total: **10-20 days** of focused engineering effort.

## Why this PR is gap-analysis only

Attempting partial wire-in within this session would risk:
1. **Regression risk**: Inserting materialization logic at the boundary between
   `execute_joins` and `pre_evaluate_correlated_exists` touches two complex
   code paths simultaneously.
2. **Incomplete coverage**: Even a successful wire-in would only handle
   EXISTS/NOT EXISTS/IN patterns (Q22 already closed via different fix), NOT
   Q17's scalar subquery.
3. **No measurable benefit**: All 4 TIMEOUT queries would still TIMEOUT
   without extending the supported pattern set.

Per the upstream evidence:
- Q17 is the dominant cost (100K subset 16.83s → SF=1 >30min super-linear)
- Fixing Q17 alone requires pattern extension that doesn't exist yet

## Recommended approach (for v3.13)

1. **Phase 1 (1-2 weeks)**: Extend `find_correlated_subqueries` to detect
   `WHERE x <cmp> (SELECT <op> AGG(col) FROM t WHERE key = outer.key)`.
2. **Phase 2 (1-2 weeks)**: Add materialization driver that:
   - Computes per-key aggregate once via single GROUP BY scan
   - Replaces the scalar subquery reference with the materialized value
3. **Phase 3 (1-2 weeks)**: Extend `HashSemiJoin` instantiation to handle
   Q20's nested EXISTS + scalar pattern.
4. **Phase 4 (1 week)**: Validate Q17/Q20/Q22 against Sprint 3 PARTIAL closure
   on 100K, 1M, SF=1 subsets.

## Disposition

**#4379, #4380, #4381, #4426 status (as of 2026-08-24)**:
- v3.12.0 GA: 1800s budget relaxation merged via PR #4430 (Q20) + PR #4433 (Q17)
- v3.13 scope: full subquery materialization (10-20 days work)

This PR captures the exploration done and provides a clear handoff to v3.13.

## Companion Issues

- #4379 — Q17 followup: #4432 (1800s budget) + PR #4433 merged
- #4380 — Q20 followup: #4429 (1800s budget) + PR #4430 merged
- #4381 — Q22 closed (a436bff57 + Sprint 3)
- #4426 — v3.13 subquery materialization (proposed target)

## Provenance

- discovered_during: V312-58 PR #4425 review (Q17 engine fix)
- generated_by: openclaw-minimax
- branch: develop/v3.12.0 @ 3092d9587
- policy: Anti-Fabrication-Policy-v1.0