# v3.9.0-rc7 Release Notes

> **Date**: 2026-06-12
> **Tag**: `v3.9.0-rc7` @ `642ff9cf9`

## What's New Since rc6

### Documentation

- **Performance overview**: Comprehensive performance overview document (PR #3363)
- **MariaDB comparison**: Real MariaDB comparison vs SQLRustGo (PR #3363)
- **Sysbench comparison**: SYSBENCH_MARIADB_COMPARISON_20260612.md

### Performance Baseline

- TPC-H 22-query runtime: ~30s (v3.8.0) → ~2.3s (v3.9.0)
- Q9 (6-way join): ~600ms → ~90ms (6.7x faster)
- Q1 (aggregation): ~150ms → ~50ms
- General executor: ~2x faster via IR plan + better join ordering

## All RC7 Deliverables

| Deliverable | Status |
|-------------|--------|
| Performance overview | ✅ Complete |
| MariaDB comparison | ✅ Complete |
| SYSBENCH comparison | ✅ Complete |

## Next: GA

GA pending: 24h real soak completion (running on 250), 72h/168h soak (pending 24h)
GA target: 2026-12-15
