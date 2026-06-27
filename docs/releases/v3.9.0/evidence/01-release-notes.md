# 01 - Release Notes

See [`../RELEASE_NOTES.md`](../RELEASE_NOTES.md) for full details.

## Summary

**v3.9.0 is a production-readiness release focused on three things**:

1. **TPC-H 22/22** — all 22 TPC-H queries pass in-process, all 22 pass wire-protocol, 21/22 match cell-level vs SQLite
2. **Q13 subquery fix** — `NOT IN (subquery_with_LIKE)` is tri-valued-logic correct
3. **Q9 6x faster** — 6-way join now 90ms (was 600ms) via hash-join pre-filter pushdown

## New Features
- TPC-H 22/22 SHA-256 baseline
- Backup/Restore/PITR (G6)
- Savepoint (G5)
- INT-2 Parallel Executor (G2)
- INT-3 Single Expression (G3)
- Time Travel + Hash Chain (G10)
- QPS/TPS Benchmark (G11)
- Cross-version upgrade chain (v3.6→v3.7→v3.8→v3.9)

## Bug Fixes
- Q13 subquery tri-valued-logic
- Q9 6x perf via hash-join
- ODUK (INSERT...ON DUPLICATE KEY UPDATE) bugfixes (#3370)
- 5 DML helpers extracted from execution_engine.rs
- SET NAMES hang fix
- TPC-H Q21 predicate pushdown
- Q8 + Q21 timeout fix

## Breaking Changes
None (binary-swap upgrade from v3.8.0)

## Known Issues (DRIFT, not blocking)
- C-ARCH-05: execution_engine.rs 1919 > 1800 lines (deferred to v3.9.1)
- SGL-005: 2 storage bypasses (pre-existing)
- No VACUUM (append-only, dead tuples accumulate)
- Q22 has known multi-COUNT divergence vs SQLite (not a bug)
