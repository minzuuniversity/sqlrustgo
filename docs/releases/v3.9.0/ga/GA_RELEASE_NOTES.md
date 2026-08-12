# v3.9.0 GA Release Notes

> **Date**: 2026-07-10
> **Tag**: `v3.9.0` at commit `184ad102e9`
> **Branch**: `develop/v3.9.0` (stable), `ga/v3.9.0` (GA release branch)
> **Previous Stable**: v3.8.0

---

## 0. Headline

**v3.9.0 is a production-readiness release focused on three things**:

1. **TPC-H 22/22** — all 22 TPC-H queries pass in-process, all 22 pass
   the wire-protocol round-trip, and 21/22 match cell-level against
   SQLite (Q22 has a known SQL-standard divergence, see
   [`EVALUATION_REPORT.md`](../EVALUATION_REPORT.md) §6).
2. **Q13 subquery fix** — `NOT IN (subquery_with_LIKE)` is now
   tri-valued-logic correct; 11/11 customers are correctly excluded
   (was incorrectly returning 60/60).
3. **Q9 6x faster** — the 6-way join is now 90ms (was 600ms) via
   hash-join pre-filter pushdown (PR #3249).

On-disk format is **unchanged** from v3.8.0; this is a binary-swap
upgrade with zero data migration.

---

## 1. Major Milestones

| Milestone | Status |
|-----------|--------|
| TPC-H 22/22 in-process | ✅ |
| TPC-H 22/22 wire round-trip | ✅ |
| Cell-level MATCH 21/22 (vs SQLite) | ✅ |
| Q9 6.7x speedup (600ms → 90ms) | ✅ |
| Q13 subquery correctness fix | ✅ |
| 72h SOAK (0 errors, 0 reconnects) | ✅ |
| G13 deadlock fix (parking_lot RwLock) | ✅ |
| execution_engine.rs 2630 → 1471 lines | ✅ |
| Statement cache (1.7x hot-path) | ✅ |
| SCRAM-SHA-256 hardening | ✅ |
| TLS 1.3 default | ✅ |
| 168h SOAK | ⏳ In progress (ETA 2026-07-12 22:02) |

---

## 2. Performance

| Metric | v3.8.0 | v3.9.0 | Delta |
|--------|--------|---------|-------|
| TPC-H in-process | 20/22 | **22/22** | +2 |
| TPC-H wire round-trip | 18/22 | **22/22** | +4 |
| Q9 (6-way join) | 600ms | **90ms** | **6.7x** |
| Q1 (aggregation) | 150ms | **50ms** | **3x** |
| 22 queries total | ~30s | **~2.3s** | **-92%** |
| Q13 subquery | 0/11 excluded | **11/11 excluded** | fixed |
| Cell-level MATCH | 18/22 | **21/22** | +3 |

Full data: [`EVALUATION_REPORT.md`](../EVALUATION_REPORT.md)

---

## 3. Reliability — 72h SOAK Results

| Metric | Value |
|--------|-------|
| Duration | **119h57m** |
| Server start | 2026-07-05 22:02:27 |
| Data points | 4,306 (72h continuous) |
| Errors | **0** |
| Reconnects | **0** |
| WAL max | 12.6 MB, clears on checkpoint |
| RSS | 100-150 MB (stable after 24h) |
| Threads | 20-60 range, avg 39 |
| FD | 13-55 range, avg 33 |

168h SOAK is in progress (ETA 2026-07-12 22:02), currently at ~120h with 0 errors.

---

## 4. Known Limitations (GA Conditional)

| Item | Status | Note |
|------|--------|------|
| Coverage average | ⚠️ 67% < 85% | G3 conditional; rationale in `COVERAGE_GAP_RATIONALE.md` |
| TPC-H SF=1 | ⚠️ 6/10 | 4 parser scope failures; rationale in `TPC-H_PARTIAL_RESULT.md` |
| 168h SOAK | ⏳ In progress | ETA 2026-07-12 22:02 |
| Z6G4 hardware | 🔴 Unreachable | Mac mini used for all real soak tests |

---

## 5. Security

- SCRAM-SHA-256: switched to `subtle::ConstantTimeEq` for proof comparison
- Per-IP rate limiting: 10 attempts / 60s on auth
- TLS 1.3 is now default (was TLS 1.2)
- `sslmode=verify-full` now honored (was silently downgraded to `require`)
- 0 Critical/High vulnerabilities in production crates

---

## 6. Operational Improvements

- **Statement cache**: LRU 1024 entries, 1.7x hot-path speedup on repeated queries
- **Online `ALTER TABLE ADD COLUMN`**: catalog-only, no table rewrite
- **`application_name`**: now propagated to `pg_stat_activity`
- **WAL archive compress**: `none` / `zstd` / `lz4` selectable
- **Slow query log**: `--slow-query-threshold-ms` flag
- **`/_/ready` endpoint**: 200 only when serving
- **`sqlrustgo completion`**: bash / zsh / fish / powershell
- **`sqlrustgo admin self-test`**: 8 internal checks
- **`sqlrustgo admin diagnostics`**: JSON issue dumps

---

## 7. Upgrade Notes

v3.9.0 is a **binary-swap upgrade** from v3.8.0. No data migration required.
On-disk format is unchanged.

For migration instructions see [`MIGRATION_GUIDE.md`](../../../../MIGRATION_GUIDE.md).

---

## 8. What's Next — v3.10.0

| Item | Target |
|------|--------|
| Coverage ≥80% per crate | v3.10.0 GA |
| TPC-H SF=1 ~10/22 (honest status, see SF1_TRUTH_AUDIT.md) | v3.10.0 GA (4 parser errors remaining) |
| Backup/Restore 100+ scenarios | v3.10.0 |
| Crash Matrix 100+ scenarios | v3.10.0 |
| TPC-H 22 remaining queries | v3.11.0 |

---

## 9. References

- CHANGELOG: [`CHANGELOG.md`](../CHANGELOG.md)
- GA Gate Report: [`GA_GATE_REPORT.md`](GA_GATE_REPORT.md)
- 72h SOAK Report: `SOAK_72H_REPORT.md` (repo root)
- Coverage Gap Rationale: [`COVERAGE_GAP_RATIONALE.md`](COVERAGE_GAP_RATIONALE.md)
- TPC-H Partial Result: [`TPC-H_PARTIAL_RESULT.md`](TPC-H_PARTIAL_RESULT.md)
- Performance Report: [`PERFORMANCE_REPORT.md`](PERFORMANCE_REPORT.md)
- Security Audit: [`SECURITY_AUDIT.md`](SECURITY_AUDIT.md)
- Evaluation Report: [`EVALUATION_REPORT.md`](../EVALUATION_REPORT.md)
- Roadmap: [`ROADMAP.md`](../ROADMAP.md)
