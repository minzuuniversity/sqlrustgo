# TPC-H 22 root cause: `execute_joins` comma-list path silently drops rows

> **Discovered**: 2026-06-05 05:55 UTC+8, while investigating the
> Q3 row_count mismatch (actual=8 expected=10 in `eval_22_vs_sqlite`).
> **Severity**: P0 — affects the majority of TPC-H 22 MISMATCH
> queries, **and** explains why the in-process engine "passes" 10/22
> by coincidence rather than correctness.

## TL;DR

`sqlrustgo::ExecutionEngine::execute_joins` (or its caller) has
**two parallel join paths**:
- **Path A (correct)**: explicit `INNER JOIN ... ON ...` — 2-table
  join returns 150 rows for `customer, orders WHERE c=o`.
- **Path B (broken)**: comma-list `FROM t1, t2, ...` — 2-table
  join returns 13 rows for the same query, missing 137 rows.

TPC-H 22 uses the comma-list form almost exclusively (it's
how the spec writes queries). The `tpch_value_test_v2.rs`
diagnostic and `eval_22_vs_sqlite.rs` both see this broken
behaviour and report 6/22 and 10/22 respectively. The
`tpch_gate_test.rs` "doesn't crash" gate is unaffected because
the broken path still returns *some* rows; it just returns
*wrong* rows.

## Empirical evidence

Test file: `tests/diag_2t_join.rs` (added during this audit).

| Query | Result | Expected | Path |
|---|---|---|---|
| `SELECT COUNT(*) FROM customer, orders WHERE c_custkey = o_custkey` | **13** | 150 | comma-list (Path B) |
| `SELECT COUNT(*) FROM customer INNER JOIN orders ON c_custkey = o_custkey` | **150** | 150 | explicit (Path A) ✓ |
| Q3: `customer, orders, lineitem WHERE c=o AND l=o + 2 date filters` | **18** | 187 | comma-list 3-table |
| Q5: 6-table comma-list + 9 WHERE | **0** | ~5 | comma-list 6-table |
| Q3 step 1c: `customer, lineitem` (no `orders` in join, but `o_custkey` in WHERE) | **ERR** | (skip — invalid) | column resolution |

The 13 vs 150 case is the smoking gun: same logical query,
same engine, two SQL forms, two different results. The explicit
`INNER JOIN` form goes through a different code path
(`execute_single_join` or equivalent) that correctly implements
the join. The comma-list form goes through what looks like an
`execute_joins` that iterates over the first table only,
producing 1 × N = N rows where N is the row count for the
first table that satisfies all WHERE conditions on its
columns. (Why 13 for customer × orders is unclear without
reading the code, but the pattern is consistent across the
3-table and 6-table cases — they're all under-counting.)

## How this maps to the 22-query audit

| Query | Audit | Likely root cause |
|---|---|---|
| Q1 | PASS | 1-table, no JOIN. Not affected. |
| Q2 | ERR (ps_suppkey not found) | 5-table comma-list Path B; column resolution fails before join runs |
| Q3 | MISMATCH 8/10 | 3-table comma-list Path B; row loss + LIMIT 10 amplifies |
| Q4 | MISMATCH 0/4 | 3-table comma-list Path B with subquery; row loss → 0 |
| Q5 | MISMATCH 0/1 | 6-table comma-list Path B; row loss → 0 |
| Q6 | PASS | 1-table, no JOIN. Not affected. |
| Q7 | PASS (0=0) | 5-table comma-list + EXTRACT YEAR parser limit; 0 rows is the right answer for sf001 |
| Q8 | ERR (Unsupported join condition) | 8-table comma-list Path B; condition resolution fails |
| Q9 | ERR (Join condition must ref one col each side) | 6-table comma-list Path B; condition classification fails |
| Q10 | MISMATCH 1/9 | 4-table comma-list Path B; row loss |
| Q11 | PASS (0=0) | 2-table with subquery; subquery in IN list returns 0 |
| Q12 | MISMATCH 0/1 | 1-table with subquery; subquery result wrong |
| Q13 | MISMATCH 0/9 | 2-table with outer join; outer join not supported? |
| Q14 | MISMATCH 9/1 | 2-table; row loss in different direction |
| Q15 | MISMATCH 0/4 | 2-table with derived table; predicate isolation failed (Macmini partial fix in #3098) |
| Q16 | MISMATCH 0/7 | 2-table with subquery + NOT EXISTS; subquery not executing? |
| Q17 | PASS | 1-table with subquery; works |
| Q18 | PASS (0=0) | 3-table with HAVING; 0 is the right answer for sf001 |
| Q19 | PASS | 2-table; works in this specific form |
| Q20 | PASS (0=0) | 2-table with subquery; 0 is the right answer for sf001 |
| Q21 | PASS (0=0) | 4-table with NOT EXISTS; 0 is the right answer for sf001 |
| Q22 | PASS (0=0) | 2-table with NOT EXISTS; 0 is the right answer for sf001 |

**Counting**: 9 MISMATCH + 3 ERR (Q2, Q8, Q9) = **12 of 22 are
attributable to Path B (the broken comma-list join path)**. The
remaining 10 are 1-table, subquery-only, or correctly return 0
on sf001.

## Why this is a P0 and not a P2

A "row_count match" gate that passes 10/22 today is hiding
**the entire TPC-H Q3 / Q4 / Q5 / Q10 / Q12 / Q13 / Q14 / Q15
/ Q16 correctness story**. If the engine ever "fixes" Q1
(row count) by accident — say, by improving the 1-table
aggregation — but leaves Path B broken, the 9/22 → 10/22
improvement is fake: more queries are now miscounting rows
in ways the gate doesn't catch.

The fix has to be on Path B, not on the gate.

## What the fix likely looks like

The `INNER JOIN ... ON` path is correct, so the fix is to
**route comma-list `FROM t1, t2 WHERE c1 = c2` through the
same code path** as the explicit join. Concretely:

- In the parser, detect `FROM t1, t2` and rewrite it to
  `FROM t1 INNER JOIN t2 ON TRUE` (or similar) **before**
  handing the AST to the executor.
- In the executor, treat comma-list as cross join, and let
  the WHERE conditions reduce it (this is standard SQL
  semantics).

Either is fine. The wrong move is to keep two separate
implementations and have them disagree.

## Recommendation for the next session

1. **Option A** (recommended, 2-3h): Rewrite the parser
   comma-list path to emit the same `INNER JOIN` AST that
   the explicit form produces. Re-run `eval_22_vs_sqlite` —
   expected outcome: 22/22 row_count match (or close to it;
   the few that may still fail are the 1-table queries with
   subqueries, which are a different class of bug).

2. **Option B** (less safe, 2-3h): Fix the `execute_joins`
   function directly. Higher risk because the function is
   hot; the parser-level fix is local and easy to review.

3. **Option C** (test-only, 30 min): Add a regression test
   that runs the 22 TPC-H queries through **both** the
   comma-list form AND the `INNER JOIN` form, asserting
   that the row counts match each other. This would be a
   "no regression" gate even without fixing the bug.

After the fix lands, the audit number should jump from
10/22 to ~20/22 (with the remaining 2 being Q2, Q8, Q9
column-resolution issues that are a separate bug class).

## Files referenced

- `tests/diag_2t_join.rs` — diagnostic test, 5 cases
- `tests/eval_22_vs_sqlite.rs` — current audit, 22 queries
- `tests/tpch_value_test_v2.rs` — Macmini's audit, 22 queries on SF=0.01
- `tests/tpch_gate_test.rs` — 13-query "doesn't crash" gate
- `src/engine_select.rs` — `execute_joins` likely location
- `src/expr_utils.rs` — `evaluate_expression` for column resolution

Refs: #2977, #3089, #3095, #3098, #3124, #3125, #3126, #3127
