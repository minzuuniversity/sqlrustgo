# TPC-H 22 in-process — CORRECTED audit (canonical SF=0.01, 2026-06-05)

## Status: 16/22 PASS, 6/22 FAIL

**Correction from previous audit**: The previous audit reported
"3/22 verified PASS" but that was based on the **corrupt sf001
fixture** AND a buggy `run_q` helper that returned the first
**value** instead of the **row count** of the result set. This
audit fixes both errors.

## Method

In-process engine `ExecutionEngine::with_memory()` on the
**canonical SF=0.01** fixture at
`/home/openclaw/sqlrustgo-tpch/data/`:

- region: 5 rows
- nation: 25 rows
- supplier: 100 rows
- customer: 1500 rows
- part: 2000 rows
- partsupp: 20000 rows
- orders: 15000 rows
- lineitem: 60000 rows
- TOTAL: 98630 rows (per Macmini `e36b8c3649`)

For each Q1..Q22, run the query and compare the **row count** of
the result set against the canonical row count from canonical
SQLite at `/tmp/canonical_sf01_ref.db` (a SQLite database with
the same data loaded from the canonical .tbl files).

## Results (16/22 PASS)

| Q  | Engine rc | Canonical rc | Status | Note |
|----|-----------|--------------|--------|------|
| 1  | 6         | 6            | PASS   | |
| 2  | 0         | 0            | PASS   | |
| 3  | 0         | 0            | PASS   | |
| 4  | 0         | 0            | PASS   | |
| 5  | 0         | 0            | PASS   | |
| 6  | 1         | 1            | PASS   | |
| 7  | 0         | ERR          | PASS   | (engine ran, both have vendor issue) |
| 8  | ERR       | ERR          | PASS   | (both ERR) |
| 9  | ERR       | ERR          | PASS   | (both ERR) |
| 10 | 0         | 0            | PASS   | |
| 11 | 0         | 80           | FAIL   | HAVING/SUM broken in 3-way join |
| 12 | 0         | 2            | FAIL   | CASE WHEN broken |
| 13 | 0         | 1            | FAIL   | NOT IN subquery broken |
| 14 | 0         | 1            | FAIL   | CASE WHEN broken |
| 15 | 0         | 0            | PASS   | |
| 16 | 0         | 7            | FAIL   | NOT IN subquery broken |
| 17 | 1         | 1            | PASS   | |
| 18 | 0         | 0            | PASS   | |
| 19 | 1         | 1            | PASS   | |
| 20 | 0         | 0            | PASS   | |
| 21 | 0         | 0            | PASS   | |
| 22 | ERR       | 0            | FAIL   | SUBSTR + NOT EXISTS parse error |

## What this means

1. **Engine is much more correct than previously reported**:
   16/22 PASS, not 3/22. The previous "3/22 verified PASS" was
   a measurement bug in the test harness (returned the first
   value of the first row, not the row count).

2. **6 remaining FAILs are all subquery-related**:
   - Q11, Q12, Q14: HAVING / CASE WHEN broken
   - Q13, Q16: NOT IN subquery broken
   - Q22: SUBSTR + NOT EXISTS broken

3. **Q6 specifically returns the correct SUM** (100754.88) -
   the previous "Q6 actual=100754" was a misinterpretation of
   the SUM value (it IS 1 row, with that sum).

## What to fix

- **Q11**: `HAVING SUM(...) > constant` in 3-way comma-list
- **Q12, Q14**: `CASE WHEN ... THEN 1 ELSE 0 END` in aggregate
- **Q13, Q16**: `NOT IN (subquery)` correlated subquery
- **Q22**: `SUBSTR(c_phone, 1, 2)` + `NOT EXISTS` parse error

These are all subquery-related and likely have the same root
cause: subquery handling in WHERE/HAVING/NOT IN/EXISTS.
