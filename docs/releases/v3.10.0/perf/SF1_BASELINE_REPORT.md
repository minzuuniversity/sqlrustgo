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
| Q 1 | 4 | 24015.0 | ok; 4 rows |
| Q 2 | 642 | 2372.9 | ok; 642 rows |
| Q 3 | 10 | 42747.1 | ok; 10 rows |
| Q 4 | 2406 | 5577.9 | ok; 2406 rows |
| Q 5 | 0 | 34906.0 | ok; 0 rows |
| Q 6 | 1 | 8598.3 | ok; 1 rows |
| Q 7 | 0 | 165148.5 | ok; 0 rows |
| Q 8 | 0 | 11247.3 | ok; 0 rows |
| Q 9 | 0 | 45081.1 | ok; 0 rows |
| Q10 | 0 | 11450.2 | ok; 0 rows |
| Q11 | 29636 | 4778.1 | ok; 29636 rows |
| Q12 | 4 | 56078.4 | ok; 4 rows |
| Q13 | 42 | 11270.6 | ok; 42 rows |
| Q14 | 1 | 8968.5 | ok; 1 rows |
| Q15 | 10000 | 9398.3 | ok; 10000 rows |
| Q16 | 0 | 17335.6 | ok; 0 rows |
| Q17 | 1 | 6544.2 | ok; 1 rows |
| Q18 | 0 | 34452.0 | ok; 0 rows |
| Q19 | 1 | 12326.0 | ok; 1 rows |
| Q20 | 10000 | 222.5 | ok; 10000 rows |
| Q21 | 0 | 56782.7 | ok; 0 rows |
| Q22 | 7 | 10080.2 | ok; 7 rows |

## Summary

- 22/22 queries returned >= 1 row
- Total rows across all 22 queries: 52755
- Total elapsed time: 579381.4 ms (579.4 s)
- Slowest query: Q7 (165148.5 ms)

## Limitations

- This report is in-process only. It does NOT cover the
  external-client path (mysql-client 8.0.46 / libmysqlclient 8.0.46),
  which is tracked in issue #3474.
- The cross-engine comparison against MariaDB / SQLite is
  not in this report; the baseline here is sqlrustgo only.
  The full cross-engine baseline is the subject of
  `scripts/tpch_sf1_baseline.sh`, which is not yet wired
  (see tasks.md item 3 in
  openspec/changes/2026-06-18-tpch-sf1-baseline).
- Per-query cell-by-cell value comparison (not just row
  count) is also out of scope here. That is the next
  step once #3474 is resolved.
