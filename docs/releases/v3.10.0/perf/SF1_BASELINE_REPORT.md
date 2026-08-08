# TPC-H SF=1.0 cross-engine baseline (in-process)

- Issue: #3423
- Spec: openspec/changes/2026-06-18-tpch-sf1-baseline
- Surface: in-process via `MySqlTestClient` + `start_ephemeral`
- Branch: fix/wire-deprecate-eof-partial
- Commit: see `git log` on the branch
- External-client follow-up: issue #3474 (out of scope here)

## Setup

- Fixture path: `/tmp/tpch-sf1`
- Generation tool: `dbgen -s 1 -f` (TPC-H dbgen, official)
- Row counts (verified at fixture load time):
  - region: 5 rows (expected ~5)
  - nation: 25 rows (expected ~25)
  - supplier: 10000 rows (expected ~10000)
  - customer: 150000 rows (expected ~150000)
  - part: 200000 rows (expected ~200000)
  - partsupp: 800000 rows (expected ~800000)
  - orders: 1500000 rows (expected ~1500000)
  - lineitem: 6001215 rows (expected ~6001215)

## Per-query results (sqlrustgo only — in-process surface)

| Q | rows | elapsed (ms) | notes |
|---|------|---------------|-------|
| Q 1 | 4 | 24923.0 | ok; 4 rows |
| Q 2 | 642 | 2774.9 | ok; 642 rows |
| Q 3 | 10 | 22362.9 | ok; 10 rows |
| Q 4 | 577704 | 14691.2 | ok; 577704 rows |
| Q 5 | 0 | 27475.9 | ok; 0 rows |
| Q 6 | 1 | 8868.9 | ok; 1 rows |
| Q 7 | 854 | 64194.7 | ok; 854 rows |
| Q 8 | 0 | 11503.0 | ok; 0 rows |
| Q 9 | 1403 | 89858.6 | ok; 1403 rows |
| Q10 | 0 | 11870.8 | ok; 0 rows |
| Q11 | 29636 | 4935.5 | ok; 29636 rows |
| Q12 | 7 | 22778.5 | ok; 7 rows |
| Q13 | 0 | 4619.5 | ok; 0 rows |
| Q14 | 1 | 9067.4 | ok; 1 rows |
| Q15 | 10000 | 9539.2 | ok; 10000 rows |
| Q16 | 0 | 17629.6 | ok; 0 rows |
| Q17 | 1 | 6671.1 | ok; 1 rows |
| Q18 | 1 | 17266.5 | ok; 1 rows |
| Q19 | 1 | 12266.5 | ok; 1 rows |
| Q20 | 10000 | 224.5 | ok; 10000 rows |
| Q21 | 100 | 35813.8 | ok; 100 rows |
| Q22 | 7 | 10817.3 | ok; 7 rows |

## Summary

- 22/22 queries completed without execution failure
- 17/22 queries returned at least one row; 5 returned zero rows and require cross-engine correctness review
- Total rows across all 22 queries: 630372
- Total elapsed time: 430153.4 ms (430.2 s)
- Slowest query: Q9 (89858.6 ms)

## Limitations

- This report is in-process only. It does NOT cover the
  external-client path (mysql-client 8.0.46 / libmysqlclient 8.0.46),
  which is tracked in issue #3474.
- The cross-engine comparison against MariaDB / SQLite is not in this report; the baseline here is sqlrustgo only.
- The full cross-engine baseline is the subject of `scripts/tpch_sf1_baseline.sh`, which is not yet wired (see tasks.md item 3 in `openspec/changes/2026-06-18-tpch-sf1-baseline`).
- Per-query cell-by-cell value comparison (not just row count) is also out of scope here. This report is execution evidence, not a GA correctness claim.
