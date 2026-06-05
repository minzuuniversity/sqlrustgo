# TPC-H 22 audit: two-pass discrepancy (sf001 10/22 vs SF=0.01 6/22)

> **Discovered**: 2026-06-05 04:55 UTC+8, after PR #3125 merged
> **Context**: PR #3124 (`fix/v380-rc2-tpch-22-real`) merged earlier
> on the same day and reports a 6/22 row_count match on
> `tests/tpch_value_test_v2.rs`. My PR #3125 reports a 10/22
> row_count match on `tests/eval_22_vs_sqlite.rs`. **Both numbers
> are real; they just measure different fixtures.**

## TL;DR

The two audit runs use different fixtures and therefore produce
different "PASS" tallies. **Neither is wrong**; the audit numbers
are not directly comparable until you account for the fixture.

| Audit | Test file | Fixture | "PASS" tally | Method |
|---|---|---|---|---|
| My audit (PR #3125) | `tests/eval_22_vs_sqlite.rs` | SF=0.001 (614 lineitem) | **10/22** (4/22 non-coincidental) | In-process engine, row_count only |
| Macmini (PR #3124) | `tests/tpch_value_test_v2.rs` | SF=0.01 (60k lineitem) | **6/22** | In-process engine, row_count + first_3_rows |

The difference is driven by:
- **Fixture scale**: SF=0.01 has 100x more data; the result sets
  are bigger and the row_count deltas amplify (e.g. Q14 returned
  9 on sf001 and presumably a different number on SF=0.01).
- **Strictness**: `tpch_value_test_v2` also compares first_3_rows
  (string equality), while my audit only compared row_count.
  First-3-rows matching is harder because it's sensitive to
  column ordering, GROUP BY ordering, and type coercion of
  floats-to-ints.

## Why this matters

A future session that picks up TPC-H work might:
1. See PR #3124's "6/22" in the title and think that's the
   current truth.
2. See PR #3125's "10/22" in the title and think the
   current truth is different.
3. Run one of the two tests and observe something between
   6 and 10 PASS, depending on the machine state.

**The right framing**: the PASS count is fixture-dependent and
strictness-dependent, and the only meaningful number is the
**ratio of matched to total (0–1)**. Both 6/22 = 0.27 and
10/22 = 0.45 are far from 1.0, which is the real point:
**TPC-H 22 is not done.**

## How to reconcile in code

If a future fix wants to enforce a single number, the right
shape is a per-fixture, per-strictness matrix:

```
                       SF=0.001    SF=0.01     SF=1
row_count only         10/22       ?/22        ?/22
row_count + first_3    ?/22        6/22        ?/22
+ sum/avg check        ?/22        ?/22        ?/22
+ full row check       ?/22        ?/22        ?/22
```

A "done" PR should fill the whole matrix. Each cell is a
separate test. Currently the matrix has **2 cells filled** out
of 12. The thread's progress is real but small.

## Recommendation for the next session

When picking up Phase 5 (any of A/B/C/D from the SPEC §9):

1. **Pick a single canonical fixture for the PR's claims.**
   SF=0.001 (sf001) is the right default because the SQLite
   baseline JSONs are already computed for it.
2. **If the canonical fixture doesn't tell the right story
   (e.g. a query that needs more data to be meaningful), make
   the test explicit about which fixture it's using.**
3. **Cite both 6/22 and 10/22 numbers in the PR description**
   so a reviewer can place the new result in context. Don't
   claim "TPC-H 22 N/22" without saying which fixture.
4. **For Phase 5 Option D (gate tightening)**, the gate should
   be per-fixture, not a single number. Otherwise, the gate
   would oscillate between 6 and 10 depending on which test
   runs first.

## Quick reconciliation commands

```bash
# My audit (sf001):
cargo test --all-features --test eval_22_vs_sqlite -- --nocapture

# Macmini's audit (SF=0.01, requires ~/sqlrustgo-tpch/data):
ls ~/sqlrustgo-tpch/data/  # must exist with SF=0.01 .tbl files
cargo test --all-features --test tpch_value_test_v2 -- --nocapture
```

The two tests have different setup paths (`eval_22_vs_sqlite` reads
from `tests/data/tpch-sf001/` directly; `tpch_value_test_v2` reads
from `~/sqlrustgo-tpch/data/` via the `TPCH_DATA_DIR` env var). The
schemas are also slightly different (eval_22 uses INTEGER for all
numeric columns; tpch_value_test_v2 uses REAL for monetary columns).
That alone is enough to produce different results — the engine's
arithmetic path for `s_acctbal` may differ between INTEGER and REAL
schemas, especially with division.

## Bottom line

The 6/22 vs 10/22 discrepancy is **not a contradiction.** It's
two different audits of two different fixtures with two different
strictness levels. **Both audits agree that TPC-H 22 is not done.**
Pick one fixture, one strictness, gate on it, and ship. Phase 5
Option D + Option A are the right next moves.

Refs: #2977, #3124, #3125, `docs/discovery/2026-06-05-tpch-22-eval-full.md`,
`docs/discovery/2026-06-05-tpch-22-6pr-completion-correction.md`
