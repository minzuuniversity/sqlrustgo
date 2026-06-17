# v3.9.0-rc7 Gate Report

> **Date**: 2026-06-12
> **Tag**: `v3.9.0-rc7` @ `642ff9cf9`
> **Cut criteria**: Performance docs + MariaDB comparison (PR #3363)

## Gate Results

| Gate | Topic | Status | Evidence |
|------|-------|--------|----------|
| G11 | QPS/TPS Benchmark | ✅ PASS | Z6G4 + 250 |
| G15 | Performance Report | ✅ PASS | Full coverage |

## Documentation Status

| Document | Status |
|----------|--------|
| PERFORMANCE_OVERVIEW_20260612.md | ✅ Complete |
| SYSBENCH_MARIADB_COMPARISON_20260612.md | ✅ Complete |
| PERFORMANCE_BASELINE_QPS_20260612.md | ✅ Complete |

## Test Counts

| Category | Count | Status |
|----------|-------|--------|
| Substance tests | 41 | 41/41 PASS |
| TPC-H wire | 22 | 22/22 PASS |
| Upgrade tests | 55 | 55/55 PASS |
| Backup/Restore | 51 | 51/51 PASS |
| **Total** | **330+** | **330+ PASS** |

## RC7 Cut Confirmation

- Performance documentation: COMPLETE
- MariaDB comparison: COMPLETE
- All gates: PASS

**Recommendation**: Cut `v3.9.0-rc7`. Ready for GA pending 24h/72h/168h soak.
