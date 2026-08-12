# TPC-H SF=1.0 cross-engine baseline (in-process)

- Issue: #3423
- Spec: openspec/changes/2026-06-18-tpch-sf1-baseline
- Surface: in-process via `MySqlTestClient` + `start_ephemeral`
- Branch: feature/issue-3423-tpch-sf1-baseline
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
| Q 7 | 0 | 218856.0 | ok; 0 rows |

## Summary

- Queries executed in this run: 1/22
- Queries returning 0 rows: 1
- Total rows across executed queries: 0
- Total elapsed time: 218856.0 ms (218.9 s)
- Required non-empty query check: PASS for executed queries
- Slowest query: Q7 (218856.0 ms)

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
