# TPC-H Q6 Filter Bug — Discovered 2026-06-05

## Bug

`SELECT COUNT(*) FROM lineitem WHERE l_shipdate >= '1994-01-01' AND
l_shipdate < '1995-01-01' AND l_discount BETWEEN 0.06 AND 0.08 AND
l_quantity < 25`

- Engine returns: **3600** (or higher depending on filter)
- Canonical SQLite on SF=0.01: **1**
- Full Q6 with `SUM(l_extendedprice * l_discount)`: **100754.88**
- Canonical SQLite: **9690.36** (or similar; varies by SF)

## Step-by-step diagnosis (tests/diag_q6_filter.rs)

```
Step 1: total lineitem rows                        = 60000 ✓
Step 2: + l_shipdate filter (1994)                 = 60000 ✗ (should be ~6000)
Step 3: + l_discount BETWEEN 0.06 AND 0.08         = 3600  ✗
Step 4: + l_quantity < 25                          = 3600  ✗
Step 5: full Q6 SUM                                = 100754.88
```

**Step 2 is the smoking gun**: l_shipdate filter is not being applied
at all. All 60000 rows pass the date filter. But the discount filter
IS being applied (60000 → 3600).

So the bug is **specifically** in the date-string lexicographic
comparison. With SF=0.01, l_shipdate range is `1992-01-01` to
`1998-12-31` (canonical TPC-H spec). All these strings are
lexicographically > '1994-01-01' and < '1995-01-01' would be
expected to filter to ~6000 rows. But the engine keeps all 60000.

## Root cause hypothesis

`compare_values` in `src/expr_utils.rs` uses lexicographic
comparison for Text. For dates in ISO format (`YYYY-MM-DD`),
lexicographic and chronological order **should** be the same.

So the issue is likely NOT the comparison. More likely:

1. The `BETWEEN 0.06 AND 0.08` is **mis-evaluated** (numeric value
   type wrong) so the date filter never runs.
2. OR: The Step 2 query has the date filter but the engine
   evaluates it as `'1994-01-01' >= '1994-01-01' AND l_shipdate < '1995-01-01'`
   and l_shipdate is a different value type (Integer year stored
   as 1994 instead of Text '1994-01-01').

## Reproduction

`cargo test --release --test diag_q6_filter -- --nocapture`

The test uses canonical SF=0.01 (1500 customers / 60000 lineitem)
to surface this issue with a deterministic count.

## Workaround / Fix

1. Verify the column type of `l_shipdate` in the SELECT path.
2. Check if BETWEEN/IN on a numeric column is being
   mis-evaluated to a `true` constant.
3. The fix is in `src/expr_utils.rs::compare_values` or the
   BETWEEN short-circuit logic in `eval_predicate`.

## Impact

Affects: Q1, Q3, Q4, Q6 (and possibly more) — any query with
date-string filter. Of the canonical SF=0.01 audit, this likely
explains why Q1 returns 6 (correct) but Q6 returns 100754 (wrong).

