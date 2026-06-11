# Sprint 5 v6 + v7 + v8 Merge Report — TPC-H 22/22 PASS on SF=0.1

**Date**: 2026-06-09
**Branch**: `release/v3.9.0-q21-merge` (merged into `develop/v3.9.0`)
**Gate**: `cargo test --test tpch_sf01_22_vs_sqlite` — **22/22 PASS, 0 FAIL, 0 SKIP**

## Summary

Five bugs across Sprint 5 v6 + v7 + v8 fixed the TPC-H Q1-Q22 coverage gap. All 22 queries now match the cross-engine Truth Source (SQLite for 19, MariaDB for 3 EXTRACT queries) on row counts when run in-process against the 60K-lineitem SF=0.1 fixture.

## Bug Fixes

### Bug A (v6): Qualified outer ref substitution
- **File**: `src/engine_utils.rs`
- **Symptom**: Q21 EXISTS subquery timed out (>11min) on SF=0.1
- **Root cause**: `substitute_outer_refs_in_expr` only handled UNQUALIFIED outer identifiers. Q21 uses `l2.l_orderkey = l1.l_orderkey` (qualified). The qualified outer ref was never substituted, so the WHERE was non-indexable.
- **Fix**: post-pass in `substitute_outer_refs_in_select` that substitutes qualified outer refs whose qualifier is NOT one of the subquery's own.

### Bug B (v6): 3-way AND index detection
- **File**: `src/engine_select.rs`
- **Symptom**: Q21 l3 NOT EXISTS subquery not optimized
- **Root cause**: index detector only matched 2-way AND. Q21 l3 has 3-way AND: `l3.l_orderkey = X AND l3.l_suppkey <> Y AND l3.l_receiptdate > l3.l_commitdate`.
- **Fix**: flatten top-level AND, pick first `inner_col = literal` conjunct.

### Bug C (v7): Derived-table alias propagation
- **File**: `crates/parser/src/parser.rs`
- **Symptom**: Q15 returned 0 rows instead of 91
- **Root cause**: derived table's alias (`revenue`) stored in `DERIVED_ALIASES` but not passed to `find_join_predicate`. ON clause was dropped, falling back to `on=Literal("true")` cartesian.
- **Fix**: when t starts with `__subq_` and has no inline alias, look up the alias from `DERIVED_ALIASES`.

### Bug D (v7): PRIMARY KEY inline shifted type indices
- **File**: `tests/tpch_sf01_22_vs_sqlite.rs`
- **Symptom**: Q5/Q15/Q16 returned 0 rows due to wrong data types
- **Root cause**: supplier DDL `s_suppkey INTEGER PRIMARY KEY` (inline PK) was filtered whole, shifting the type slot for s_suppkey to s_name (TEXT). Loader stored s_suppkey as Text instead of Integer.
- **Fix**: strip inline `PRIMARY KEY` modifier instead of filtering the whole definition.

### Bug E (v8): BinaryOp-over-aggregate re-projection
- **File**: `src/engine_select.rs`
- **Symptom**: Q8 returned 932.71 (the first SUM operand) instead of 1.0 (the ratio)
- **Root cause**: re-projection only looked up column by alias, returning the first aggregate value and dropping the rest of the expression.
- **Fix**: detect BinaryOp-over-Aggregate columns; re-evaluate the full expression against the row's aggregate values via `evaluate_expression` + `agg_schema`.

## Cross-engine Baseline (v8)

| Query | Truth Source | Reason |
|-------|--------------|--------|
| Q1-Q6, Q10-Q22 | SQLite 3.51 | Standard SQL-92, no exotic features |
| Q7, Q8, Q9 | MariaDB 12.3.2 | Use `EXTRACT(YEAR FROM ...)` which SQLite does not support |

The baseline script (`scripts/tpch_sf01_baseline.sh`) auto-routes Q7-9 to MariaDB.

## Sprint 5 Final Score

| Sprint | Tests | Pass | Fail | Skip | Key change |
|--------|-------|------|------|------|------------|
| v6 | 19/19 | 16 | 0 | 3 (EXTRACT) | Q21 perf + baseline shell counter fix |
| v7 | 19/19 | 16 | 0 | 3 (EXTRACT) | + Q5/Q15/Q16 derived-table + loader type fix |
| v8 | **22/22** | **22** | **0** | **0** | + Q7-Q9 EXTRACT + Q8 re-projection |

## Verification

```
$ cargo test --test tpch_sf01_22_vs_sqlite -- --nocapture
=== Loading SF=0.1 fixture from tests/data/tpch-sf01 ===
  region: 5 rows, nation: 25 rows, supplier: 100 rows, customer: 1500 rows,
  part: 2000 rows, partsupp: 8000 rows, orders: 15000 rows, lineitem: 60000 rows
=== Running 22 TPC-H queries vs SQLite baseline ===
  Q 1: PASS (rc=6, expected=6) in 168ms       Q12: PASS (rc=2, expected=2) in 27s
  Q 2: PASS (rc=0, expected=0) in 7s         Q13: PASS (rc=22, expected=22) in 1.6s
  Q 3: PASS (rc=10, expected=10) in 650ms    Q14: PASS (rc=1, expected=1) in 1.2s
  Q 4: PASS (rc=5, expected=5) in 130ms      Q15: PASS (rc=91, expected=91) in 77ms
  Q 5: PASS (rc=1, expected=1) in 297ms      Q16: PASS (rc=282, expected=282) in 58ms
  Q 6: PASS (rc=1, expected=1) in 82ms       Q17: PASS (rc=1, expected=1) in 94s
  Q 7: PASS (rc=3, expected=3) in 397ms      Q18: PASS (rc=100, expected=100) in 789ms
  Q 8: PASS (rc=1, expected=1) in 6.6s       Q19: PASS (rc=1, expected=1) in 339ms
  Q 9: PASS (rc=0, expected=0) in 890ms      Q20: PASS (rc=0, expected=0) in 5.8ms
  Q10: PASS (rc=20, expected=20) in 828ms    Q21: PASS (rc=0, expected=0) in 633s
  Q11: PASS (rc=0, expected=0) in 44ms       Q22: PASS (rc=0, expected=0) in 5.4s
=== Summary: pass=22 fail=0 skip=0 ===
```

## Next Steps (Sprint 5 v9 / v3.9.0 GA)

- Push branch to Gitea 252/250 when servers recover
- Update CHANGELOG.md, RELEASE_NOTES for v3.9.0
- Tag v3.9.0-rc1 with this 22/22 result
- Wire-protocol test (`cargo test --test tpch_22_queries_wire_test`) on SF=0.001 fixture: 1/1 PASS
