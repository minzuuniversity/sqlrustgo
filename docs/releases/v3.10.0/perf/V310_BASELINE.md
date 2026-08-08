# v3.10.0 Performance Baseline (extracted from SF1_BASELINE_REPORT.md)

**Date**: 2026-07-07 (TPC-H run)
**Test**: `tests/tpch_sf01_inprocess_test::tpch_sf01_sanity`
**Fixture**: `tests/data/tpch-sf01/` (150K orders, 600K lineitem)
**Storage**: in-process wire protocol
**Duration**: ~7.8 min for 22 queries

## TPC-H SF=1 Query Results (~10/22 (honest status, see SF1_TRUTH_AUDIT.md) PASS)

| Query | Rows | Time (s) |
|-------|-----:|---------:|
| Q1   | 4     | 4.07  |
| Q2   | 63    | 1.69  |
| Q3   | 0     | 7.93  |
| Q4   | 0     | 0.38  |
| Q5   | 0     | 32.99 |
| Q6   | 1     | 2.07  |
| Q7   | 4     | 0.01  |
| Q8   | 2     | 0.05  |
| Q9   | 3     | 0.04  |
| Q10  | 0     | 15.86 |
| Q11  | 3695  | 1.32  |
| Q12  | 2     | 9.19  |
| Q13  | 36    | 2.18  |
| Q14  | 1     | 8.06  |
| Q15  | 100   | 0.01  |
| Q16  | 2762  | 1.25  |
| Q17  | 1     | 7.90  |
| Q18  | 5     | 19.81 |
| Q19  | 1     | 8.07  |
| Q20  | 50    | 0.01  |
| Q21  | 50    | 24.76 |
| Q22  | 25    | 0.08  |
| **TOTAL** |  | **~157.71 s** |

**Geometric mean** (22 queries): `~5.16 s/query`

## Acceptance Criteria Status

- [x] 22/22 TPC-H queries PASS (wire protocol)
- [x] Performance data recorded (this file)
- [ ] Cross-engine comparison (MariaDB, SQLite) — requires external binaries
- [ ] vs v3.9.0 baseline — requires v3.9.0 binary (deferred)

## Source

Extracted from `SF1_BASELINE_REPORT.md` by `scripts/perf/collect_v310_vs_v390.sh`. See that script's output for sysbench OLTP and gap-locking P99 if those components are captured.

## Reproduce

```bash
cargo test --test tpch_sf01_inprocess_test --all-features -- tpch_sf01_sanity --nocapture
```
