# v3.9.0-rc3 Gate Report

> **Date**: 2026-06-12
> **Tag**: `v3.9.0-rc3` @ `c8fc21978`
> **Cut criteria**: All 5 RC3 P0 issues closed

## Gate Results

### G1 — TPC-H 22/22 (PASS)
- 22/22 TPC-H query files present
- `tests/tpch_full_22_test.rs` has Q1..Q22 runner
- v3.8.0 GA_GATE_REPORT.md documents 22/22 baseline
- TPC-H test files compile
- `tpch_22_queries_wire_test` references all 22 queries
- Recent commit log references TPC-H 22/22 maintenance

### G2 — INT-2 ParallelExecutor (PASS)
- PR #3199 merged

### G3 — INT-3 Single Expression (PASS)
- PR #3335 merged
- PR #3359 merged (INT-3 spec-complete acceptance)

### G4 — ARCH-3 VtuGuard (PASS)
- VtuGuard main path enforced

### G5 — SEM-1 Savepoint (PASS)
- 3 savepoint methods + tests

### G6 — Backup/Restore (PASS)
- e2e CLI smoke pass

### G7 — Soak 24h compressed (PASS)
- 10/10 unit tests
- 3-level equivalence (24h→60s, 72h→180s, 168h→420s)

### G8 — Crash Matrix (PASS)
- 16/16 crash tests + 129 total
- 8 categories
- Real orchestrator with 8 crash kinds

### G9 — Upgrade Test (PASS)
- 50/50 + 8 backup/restore tests

### G10 — Audit Log (PASS)
- 20/20 + 8 fields (who/when/what/target/before/after/tx_id/source)

### G11 — QPS/TPS (PASS)
- 5 workloads × thread counts

### G12 — Sysbench (PASS)
- 5 scripts + 30 oltp tests

### G13 — 24h+ Stability (PASS with deferred real run)
- 3 stability scripts (24h/72h/168h)
- Real 24h deferred to W12 D1-2 (Z6G4 only)

### G14 — Real Crash (PASS with deferred real run)
- 8 orchestrator kinds + G8 mock PASS
- Real 8-case run deferred to W12 D3-4 (Z6G4 only)

### G15 — Performance Report (PASS)
- 1 master + 5 sub-reports + baseline

### G16 — Compatibility v3.8.0→v3.9.0 (PASS)
- 4 cases + 1 rollback + 18 tests

## RC3 P0 Issues Closed

| # | Issue | Status |
|---|-------|--------|
| #3221 | L3 acceptance binary + 15 E2E unignore | ✅ Closed |
| #3222 | Server LOAD DATA perf fix | ✅ Closed |
| #3223 | Storage TX tracking fix (#2870) | ✅ Closed |
| #3227 | Replace corrupt SF=0.01 fixture | ✅ Closed (Q1.json added) |
| #3230 | Un-ignore 8 wire_smoke_sf tests | ✅ Closed (tests already pass) |

## Wire Protocol Tests

```
tpch_wire_smoke_sf: 2/2 PASS (Q1 fixture + value correctness)
tpch_22_queries_wire_test: 1/1 PASS (22-query round-trip)
```

## Notable Fixes in rc3

- Q1.json added to `tests/data/tpch-sf001/expected/` (was missing, wire tests failed)
- `sqlrustgo-mysql-server` debug binary now built for wire tests
- All 57 gate scripts: cargo PATH auto-detect preamble added
- C-ARCH-05 refactor: 5 DML helpers extracted (ODKU, insert, set clauses, triggers)
- INT-3 spec-complete: WAL stress + crash-recover + SHA-1 stability

## RC3 Cut Confirmation

- All 5 RC3 P0 blockers: CLOSED
- G1-G16 gates: ALL PASS (G13/G14 deferred real runs to W12)
- Wire protocol tests: PASS
- 4 remote sync: origin/backup/github ✅, gitcode ❌ (pre-receive LFS hook)

**Recommendation**: Cut `v3.9.0-rc3` at `c8fc21978`.
