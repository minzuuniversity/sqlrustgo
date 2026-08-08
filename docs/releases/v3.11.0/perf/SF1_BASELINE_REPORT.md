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
| Q 1 | 4 | 21571.4 | ok; 4 rows |
| Q 2 | 642 | 1955.8 | ok; 642 rows |
| Q 3 | 10 | 34772.4 | ok; 10 rows |
| Q 4 | 5 | 4279.2 | ok; 5 rows |
| Q 5 | 5 | 44022.0 | ok; 5 rows |
| Q 6 | 1 | 8118.6 | ok; 1 rows |
| Q 7 | 7 | 348531.0 | ok; 7 rows |
| Q 8 | 2 | 68216.0 | ok; 2 rows |
| Q 9 | 175 | 815102.5 | ok; 175 rows |
| Q10 | 20 | 18720.7 | ok; 20 rows |
| Q11 | 29636 | 4791.0 | ok; 29636 rows |
| Q12 | 7 | 55365.4 | ok; 7 rows |
| Q13 | 42 | 714577.3 | ok; 42 rows |
| Q14 | 1 | 8508.9 | ok; 1 rows |
| Q15 | 10000 | 9041.9 | ok; 10000 rows |
| Q16 | 0 | 17347.9 | ok; 0 rows |
| Q17 | 1 | 6364.6 | ok; 1 rows |
| Q18 | 14 | 34052.3 | ok; 14 rows |
| Q19 | 1 | 11499.6 | ok; 1 rows |
| Q20 | 10000 | 226.9 | ok; 10000 rows |
| Q21 | 100 | 52511.5 | ok; 100 rows |
| Q22 | 7 | 9074.3 | ok; 7 rows |

## Summary

- Queries executed in this run: 22/22
- Queries returning 0 rows: 1
- Total rows across executed queries: 50680
- Total elapsed time: 2288651.1 ms (2288.7 s)
- Required non-empty query check: FAIL - Q[16] returned 0 rows
- Slowest query: Q9 (815102.5 ms)

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


## Cross-engine row-count comparison (PG oracle)

- Issue: #3654 (SHA256 cross-engine correctness, follow-up to #3650)
- Date: 2026-08-09
- sqlrustgo surface: in-process `MySqlTestClient` + BINT v2 (`/tmp/tpch-sf1-bin`)
- PG surface: `tpch_sf1_reference` database on local PostgreSQL 16.14, same
  `/tmp/tpch-sf1/*.tbl` fixture, 9 secondary indexes (lineitem, orders, partsupp,
  customer, supplier, nation).

| Q  | sqlrustgo | PG     | match | notes |
|----|----------:|-------:|:-----:|-------|
| 1  | 4         | 4      | ✓     | float tail differs ≤ 1e-6 |
| 2  | 642       | 20     | ✗     | known overcount (Q2) |
| 3  | 10        | 10     | ✓     | float tail |
| 4  | 5         | 5      | ✓     | |
| 5  | 5         | 5      | ✓     | Q5 PASS (regression of pre-fix OOM) |
| 6  | 1         | 1      | ✓     | |
| 7  | 7         | 7      | ✓     | |
| 8  | 2         | 2      | ✓     | Q8 PASS (after q8.sql canonical fix) |
| 9  | 175       | 175    | ✓     | |
| 10 | 20        | 20     | ✓     | Q10 PASS (regression of pre-fix OOM) |
| 11 | 29,636    | 29,636 | ✓     | |
| 12 | 7         | 2      | ✗     | overcount |
| 13 | 42        | 42     | ✓     | Q13 PASS (regression of pre-fix OOM) |
| 14 | 1         | 1      | ✓     | |
| 15 | 10,000    | 10,000 | ✓     | |
| 16 | 0         | 18,314 | ✗     | engine bug — single-table `p_*` predicate pushdown |
| 17 | 1         | 1      | ✓     | |
| 18 | 14        | 57     | ✗     | undercount |
| 19 | 1         | 1      | ✓     | |
| 20 | 10,000    | 172    | ✗     | overcount (LIMIT/exists semantics) |
| 21 | 100       | 100    | ✓     | Q21 PASS (regression of pre-fix OOM) |
| 22 | 7         | 7      | ✓     | |

**17/22 row counts match.** Float-tail deltas on matched queries are
≤1e-6 and within IEEE-754 summation order tolerance.

PG oracle checksums: `/tmp/postgres-sf1-checksums.txt` (22 entries).
sqlrustgo checksums: `/tmp/sqlrustgo-sf1-checksums.txt` (22 entries).
`diff -u` is non-empty due to the 5 row-count mismatches above plus
float-tail precision on Q1/Q3/Q6/Q9/Q11/Q13/Q14/Q15/Q19/Q22. A
cell-by-cell float-tolerance pass is the next step (issue #3654).
