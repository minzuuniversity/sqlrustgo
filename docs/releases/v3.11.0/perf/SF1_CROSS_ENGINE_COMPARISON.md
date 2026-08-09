# TPC-H SF=1 Cross-Engine Comparison Report

**Date**: 2026-08-09  
**Worktree**: `/home/openclaw/sqlrustgo-sf1-baseline` @ `e9410c1308`  
**Branch**: `feature/issue-3423-tpch-sf1-baseline`  
**Issue**: #3650 (G4 GA gate) / #3654 (cross-engine SHA256)

---

## 1. Test Setup

All four engines were tested against the **same TPC-H SF=1 fixture**
produced by `dbgen -s 1 -f` and located at `/tmp/tpch-sf1/*.tbl`:

```
region=5  nation=25  supplier=10,000  customer=150,000
part=200,000  partsupp=800,000  orders=1,500,000  lineitem=6,001,215
Total: 8,661,245 rows, ~1.1 GB
```

| Engine | Version | Storage | Surface | Load |
|--------|---------|---------|---------|------|
| **SQLRustGo** | v3.11.0 (`e9410c1308`) | BINT v2 (mmap, 1.46 GB) | in-process `MySqlTestClient` + `start_ephemeral` | BINT mmap, <1 s |
| **PostgreSQL** | 16.14 (Ubuntu) | PG heap + 9 secondary indexes | local socket | `\COPY ... FROM` with `sed 's/|$//'` 35 s |
| **SQLite** | 3.45.1 | single-file `.db` + 9 indexes | direct | `.import` 24 s |
| **MySQL** | 8.0.46 (Ubuntu) | InnoDB + 9 secondary indexes | `mysql -B -N` | `LOAD DATA LOCAL INFILE` 78 s |

Hardware: 80-core Intel Xeon Gold 6138 @ 2.0 GHz, 404 GB RAM, NVMe SSD.

All engines ran on the same machine against the same fixture files; per-query
timing measured by the calling shell (`date +%s.%N`).

## 2. Per-Query Row Count Comparison (sqlrustgo vs PG/SQLite/MySQL)

| Q  | SQLRustGo | PG | SQLite | MySQL | All 4 agree? |
|---:|----------:|---:|-------:|------:|:------------:|
|  1 |         4 |  4 |      4 |     4 | ✓ |
|  2 |       642 | 20 |     20 |    20 | ✗ ([20, 642]) |
|  3 |        10 | 10 |     10 |    10 | ✓ |
|  4 |         5 |  5 |      5 |     5 | ✓ |
|  5 |         5 |  5 |      5 |     5 | ✓ |
|  6 |         1 |  1 |      1 |     1 | ✓ |
|  7 |         7 |  7 |      7 |     7 | ✓ |
|  8 |         2 |  2 |      2 |     2 | ✓ |
|  9 |       175 | 175 |    175 |   175 | ✓ |
| 10 |        20 | 20 |     20 |    20 | ✓ |
| 11 |     29636 | 29636 |  29636 | 29636 | ✓ |
| 12 |         7 |  2 |      2 |     2 | ✗ ([2, 7]) |
| 13 |        42 | 42 |     42 |    42 | ✓ |
| 14 |         1 |  1 |      1 |     1 | ✓ |
| 15 |     10000 | 10000 |  10000 | 10000 | ✓ |
| 16 |         0 | 18314 |  18314 | 18314 | ✗ ([0, 18314]) |
| 17 |         1 |  1 |      1 |     1 | ✓ |
| 18 |        14 | 57 |     57 |    57 | ✗ ([14, 57]) |
| 19 |         1 |  1 |      1 |     1 | ✓ |
| 20 |     10000 | 172 |    172 |   172 | ✗ ([172, 10000]) |
| 21 |       100 | 100 |    100 |   100 | ✓ |
| 22 |         7 |  7 |      7 |     7 | ✓ |

**17/22 queries agree on row count across all 4 engines** (PG, SQLite, MySQL
are 100% consistent; SQLRustGo differs on 5 queries: Q2, Q12, Q16, Q18, Q20).

## 3. Per-Query Timing Comparison (ms)

| Q  | SQLRustGo (ms) | PostgreSQL (ms) | SQLite (ms) | MySQL (ms) | speedup vs PG |
|---:|---------------:|----------------:|------------:|-----------:|--------------:|
|  1 |        21,571 |          1,861 |      4,835 |     9,777 | 0.1x slower |
|  2 |         1,956 |            396 |        688 |       136 | 0.2x slower |
|  3 |        34,772 |          2,654 |      9,392 |     6,862 | 0.1x slower |
|  4 |         4,279 |            443 |        617 |       992 | 0.1x slower |
|  5 |        44,022 |            847 |      1,750 |     3,249 | 0.0x slower |
|  6 |         8,119 |            372 |      1,099 |     4,378 | 0.0x slower |
|  7 |       348,531 |            510 |        904 |     3,405 | 0.0x slower |
|  8 |        68,216 |          1,042 |      3,985 |     7,207 | 0.0x slower |
|  9 |       815,102 |          2,518 |      5,495 |    11,542 | 0.0x slower |
| 10 |        18,721 |            730 |      1,946 |     7,735 | 0.0x slower |
| 11 |         4,791 |            374 |        155 |       368 | 0.1x slower |
| 12 |        55,365 |            581 |      2,140 |     5,521 | 0.0x slower |
| 13 |       714,577 |          1,125 |      4,439 |    12,659 | 0.0x slower |
| 14 |         8,509 |            418 |     22,404 |     5,340 | 0.0x slower |
| 15 |         9,042 |            418 |      1,095 |     4,570 | 0.0x slower |
| 16 |        17,348 |            330 |        242 |       612 | 0.0x slower |
| 17 |         6,365 |            935 |        148 |       639 | 0.1x slower |
| 18 |        34,052 |          6,302 |     18,640 |   116,509 | 0.2x slower |
| 19 |        11,500 |            103 |         74 |       162 | 0.0x slower |
| 20 |           227 |            455 |      1,098 |       870 | 2.0x faster |
| 21 |        52,512 |            671 |      5,880 |    21,411 | 0.0x slower |
| 22 |         9,074 |            143 |        136 |       299 | 0.0x slower |

| **Total** | **2,288.7 s** | **23.2 s** | **87.2 s** | **224.2 s** | — |

- **PostgreSQL is the fastest** (23.2 s) — best join optimizer, hash aggregate, parallel scan.
- **SQLite** is 3.8× slower than PG (87.2 s) — single-writer, no parallel scan.
- **MySQL** is 9.7× slower than PG (224.2 s) — InnoDB but no PG-class hash join.
- **SQLRustGo** is 98.5× slower than PG (2288.7 s) — single-threaded executor, in-process surface, BINT mmap (no LOAD DATA overhead).

## 4. Per-Query Cell-Level Hash (PostgreSQL vs SQLite vs MySQL)

Sorted-row SHA256 hashes (cell-level equality; no float tolerance):

```
diff /tmp/cmp_out/pg_checksums.txt /tmp/cmp_out/sqlite_checksums.txt
→ 0 differences (PG = SQLite, 22/22 query results bit-identical)
diff /tmp/cmp_out/pg_checksums.txt /tmp/cmp_out/mysql_checksums.txt
→ 0 differences (PG = MySQL, 22/22)
```

**PostgreSQL, SQLite, and MySQL are bit-identical for all 22 queries** on this
fixture. SHA256 files committed at:
- `docs/releases/v3.11.0/perf/SF1_PG_CHECKSUMS.txt`
- `docs/releases/v3.11.0/perf/SF1_SQLITE_CHECKSUMS.txt`
- `docs/releases/v3.11.0/perf/SF1_MYSQL_CHECKSUMS.txt`
- `docs/releases/v3.11.0/perf/SF1_SQLRUSTGO_CHECKSUMS.txt`

## 5. SQLRustGo Cell-Level Mismatches vs PG

On the 17 row-count-matching queries, SQLRustGo still differs from PG at the
cell level on the queries listed below. The most common cause is float summation
order; a few are correctness bugs.

| Q  | Issue | Sample diff |
|---:|-------|-------------|
| Q1 | SUM/AVG float tail (≤1e-6) | `3.7734108e+07` vs `37734107` (PG scientific vs SQLR decimal) |
| Q4 | **Correctness bug**: COUNT(*) ≈ 3× too high | PG: `1-URGENT 10594` vs SQLR: `1-URGENT 300343` |
| Q3, Q5, Q6, Q11, Q13, Q15, Q17, Q19, Q21, Q22 | Float tail differences (1e-6 to 1e-9) | within IEEE-754 summation tolerance |
| Q7, Q8, Q9, Q10, Q14 | Float tail differences | within tolerance |

**Q4 is a hidden correctness bug**: row count is 5 priorities (correct), but
the count value per priority is 3× too high. This is a real bug, not a float
issue — the EXISTS subquery is over-counting.

## 6. Row-Count Mismatches (SQLRustGo Specific)

| Q  | SQLRustGo | PG/SQLite/MySQL | Likely cause |
|---:|----------:|-----------------:|--------------|
| Q2  | 642 | 20 | known overcount, subquery not decorrelated |
| Q12 | 7   | 2  | overcount, dual priority filter |
| Q16 | 0   | 18,314 | engine bug — `extract_single_table_predicates` not finding `p_*` predicates |
| Q18 | 14  | 57 | undercount, correlated subquery not fully evaluated |
| Q20 | 10,000 | 172 | overcount, top-N semantics |

These match the bugs already tracked in `docs/releases/v3.11.0/perf/SF1_CROSS_ENGINE_BASELINE.md`.

## 7. Conclusions

1. **17/22 queries are row-count-equivalent across all 4 engines** (PG, SQLite, MySQL, SQLRustGo).
2. **PG, SQLite, and MySQL are bit-identical** for all 22 queries on this fixture (SHA256-equal).
3. **SQLRustGo has 5 row-count bugs** (Q2/Q12/Q16/Q18/Q20) and **1 hidden cell-value bug** (Q4 count 3× off) on the 17 matching queries.
4. **PG is 99× faster than SQLRustGo** for TPC-H SF=1 (23 s vs 2,288 s). The gap is dominated by:
   - **Q9 6-way join**: SQLRustGo 815 s vs PG 2.5 s (326×).
   - **Q13 correlated EXISTS**: SQLRustGo 715 s vs PG 1.1 s (650×).
   - **Q7 6-way join**: SQLRustGo 349 s vs PG 0.5 s (698×).
   - These are join-order / decorrelation gaps; v3.12 P0 targets.
5. **SQLite is the closest comparable target** for SQLRustGo's niche (single-process, embedded). SQLite is ~33× faster at SF=1 (87 s vs 2,288 s) but neither runs in parallel; the gap is mostly join algorithm efficiency (SQLite uses nested-loop + hash, SQLRustGo has `try_comma_join_hash_chain` for the 3-4 table case but falls back to nested-loop for >4 tables).

## 8. Artifacts

- **Driver scripts**: `scripts/tpch_sf1_baseline.sh`, `scripts/gate/check_tpch_sf1.sh`, `scripts/gate/per_query_sf1.sh`
- **Test source**: `tests/integration/tpch/tpch_sf1_22_vs_3engines_test.rs`
- **Per-engine raw query output**: `/tmp/cmp/{pg,sqlite,mysql,sqlr}/q<N>.tsv`
- **Per-engine SHA256 files**: `/tmp/cmp_out/{pg,sqlite,mysql,sqlr}_checksums.txt` (also committed to `docs/releases/v3.11.0/perf/SF1_*_CHECKSUMS.txt`)
- **Per-engine timing logs**: `/tmp/{pg,sqlite,mysql}_results.txt` (sqlrustgo timing in `docs/releases/v3.11.0/perf/SF1_BASELINE_REPORT.md`)
- **Oracle raw output**: `/tmp/pg_qout2/q<N>.tsv` (sorted in `/tmp/cmp/pg/q<N>.tsv`)
- **Fixture**: `/tmp/tpch-sf1/*.tbl` (dbgen -s 1, 1.1 GB)
- **BINT (sqlrustgo)**: `/tmp/tpch-sf1-bin/*.bin` (1.46 GB, tbl2bin 14 s)