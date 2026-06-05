# TPC-H 22 re-audit at HEAD `424ecfe9b` — post PR #3119, #3124, #3125, #3126

> **Discovered**: 2026-06-05 05:05 UTC+8, immediately after PR #3126
> merged. I noticed `origin/develop/v3.8.0` had advanced to
> `424ecfe9b` with two new PRs (#3119 "Q2 explicit JOIN rewrite",
> #3124 "TPC-H 22/22 value assertion gate") that I had not
> accounted for in my PR #3125 audit. This doc re-runs the
> `eval_22_vs_sqlite` test at the **current** HEAD and compares
> to my PR #3125 baseline at `acc8d5a90`.

## TL;DR

The two-PR diff between `acc8d5a90` (my PR #3125 baseline) and
`424ecfe9b` (current HEAD) **did not change the headline 10/22
number**, but it did change the *category* of two queries:

| Query | At `acc8d5a90` (PR #3125 baseline) | At `424ecfe9b` (current HEAD) |
|---|---|---|
| Q2 | ERR (ps_suppkey not found) | **ERR (still)** |
| Q15 | ERR (Unsupported join condition) | **MISMATCH (0 rows, expected 4)** |
| (others) | unchanged | unchanged |

Final tallies at `424ecfe9b`:
- **MATCHED**: 10/22 (Q1, Q6, Q7=0, Q11=0, Q17, Q18=0, Q19, Q20=0, Q21=0, Q22=0)
- **MISMATCHED**: 9/22 (Q3, Q4, Q5, Q10, Q12, Q13, Q14, Q15, Q16)
- **ERROR**: 3/22 (Q2, Q8, Q9)
- **Wire**: 0/22 (server LOAD DATA EAGAIN still blocking)

## What PR #3119 (Macmini "Q2 explicit JOIN rewrite") actually fixed

Macmini's commit `6d324a375` modifies `tests/tpch_gate_test.rs`
to rewrite Q2's SQL from a 5-table comma-list join to an
explicit JOIN chain. The test then reports "TPC-H 22/22 PASS"
on the `tpch_gate_test` 13-query subset. The title "22/22 PASS"
is **only true on the 13-query `tpch_gate_test` subset** (which
excludes Q4, Q13-Q17, Q20-Q22).

This fix does **not** repair the underlying Q2 engine issue
(`find_join_key_index` cannot resolve `ps_suppkey` across the
5-table join chain). It just rewrites the test query to a
form that the engine can execute. **The 5-table comma-list
join that real-world TPC-H Q2 uses is still broken.**

My `eval_22_vs_sqlite` test uses the **unmodified** Q2 SQL from
`queries/q2.sql` (the real TPC-H form), so it still ERRs.

## What PR #3124 (Macmini `tpch_value_test_v2`) actually tests

PR #3124 adds a new test file `tests/tpch_value_test_v2.rs` that
runs all 22 queries and compares row_count + first_3_rows
against the SQLite baseline. On its fixture (SF=0.01, 60k
lineitem, not sf001 614), it reports **6/22 PASS**.

The test exists in the tree but is **not gating** (it doesn't
`assert!` on the result; the failure path is `eprintln!` and
return). The PR title "TPC-H 22/22 value assertion gate" is
therefore aspirational — the test is a diagnostic, not a gate.
(Compare with my `eval_22_vs_sqlite`, which also doesn't gate
on the result; same shape, different fixture.)

## Updated SPEC §9 cross-references

Phase 5 ABCDE options from my PR #3125 are still valid at
`424ecfe9b`. The **only** number that changed is Q15's
category (ERR → MISMATCH 0/4), which is consistent with
Macmini's claim that Q15 is "now parseable" — the SQL
parses, executes, but the row_count is 0 instead of 4
because the predicate pushdown into the derived table
isn't materialising rows correctly.

This is actually **better** than my PR #3125 audit showed:
Q15 has moved from "engine error" to "engine returns
wrong answer", which is fixable via predicate
materialisation, not a parser change.

## Updated recommendations for the next session

1. **The 10/22 number is robust to the latest 5 PRs.** The
   state of TPC-H 22 is the same: 10/22 row_count match,
   4/22 real issues (Q2 ERR, Q8 ERR, Q9 ERR, Q15 MISMATCH
   0/4), 8/22 silent-wrong (Q3, Q4, Q5, Q10, Q12, Q13,
   Q14, Q16).
2. **The next session should pick Phase 5 Option D first**
   (gate tightening) because the gate is now obviously
   missing: a test that produces a 16/22 MISMATCH+ERROR
   total today does not fail the build, so any future PR
   that makes the situation worse is uncaught.
3. **Q15 is fixable now** (Macmini's parse fix is in; only
   the predicate materialisation remains). This is a 1-2
   hour follow-up.
4. **Q2 is not fixable with the current engine.** The
   5-table comma-list join path needs Q15-style
   derived-table materialisation to extract the
   correlated subquery before the join chain. Macmini's
   "Q2 explicit JOIN rewrite" fixes the gate test but
   does not fix the engine. This is v3.9.0 work
   (Q2 hub-spoke reordering, per the v3.8.0 RC1
   closure report).

## Headline for Issue #2977

After PR #3119 / #3124 / #3125 / #3126 (all merged
2026-06-04 23:45 to 2026-06-05 04:55 UTC+8), the
**TPC-H 22 thread has converged to a stable state**:

- In-process: **10/22 row_count match** (4/22 non-coincidental)
- In-process engine errors: 3/22 (Q2, Q8, Q9)
- In-process silently wrong: 9/22 (Q3, Q4, Q5, Q10, Q12, Q13, Q14, Q15, Q16)
- Wire: 0/22 (server LOAD DATA EAGAIN blocks)
- Test coverage: 1 diagnostic (`eval_22_vs_sqlite`) + 1
  diagnostic (`tpch_value_test_v2`) + 6 hand-picked
  regression tests (`tpch_bug_regression_test`) + 1
  "doesn't crash" gate (`tpch_full_22_test`)

**Not done. Phase 5 ABCDE options from PR #3125 are the
correct next moves.**

Refs: #2977, #3116, #3119, #3124, #3125, #3126
