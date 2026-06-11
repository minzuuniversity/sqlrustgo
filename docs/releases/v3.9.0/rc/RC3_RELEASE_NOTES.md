# v3.9.0-rc3 Release Notes

> **Date**: 2026-06-12
> **Tag**: `v3.9.0-rc3` @ `c8fc21978`

## What's New Since rc2

### Engineering

- **L3 acceptance binary**: WAL stress + crash-recover + SHA-1 stability
- **Storage TX tracking fix**: PREPARE-without-COMMIT now correctly recovers as incomplete
- **Server LOAD DATA perf**: Performance fixes for bulk data loading
- **C-ARCH-05 refactor**: 5 DML helpers extracted from execution_engine.rs
  - `begin_implicit_dml_tx`, `commit_implicit_dml_tx` (TX lifecycle)
  - `build_insert_records` (INSERT VALUES eval)
  - `apply_odku` (ON DUPLICATE KEY UPDATE)
  - `apply_set_clauses` (UPDATE SET eval)
  - `run_before_update_triggers` (BEFORE UPDATE trigger)
- **Gate scripts**: All 57 gate scripts now have cargo PATH auto-detection for CI runners
- **INT-3 spec-complete**: WAL stress + crash-recover + SHA-1 stability tests added

### Testing Infrastructure

- **`Q1.json` added**: `tests/data/tpch-sf001/expected/Q1.json` restored (was missing)
- **Wire smoke test**: 2/2 PASS (`tpch_wire_smoke_sf001_fixture_loads_and_q1_executes` + `tpch_wire_smoke_sf001_q1_value_correctness`)
- **22-query wire test**: 1/1 PASS (`tpch_22_queries_wire_roundtrip`)

### Documentation

- Sprint 8 final: G1-G16 all gates PASS
- Sprint 9: PR conflict resolution (#3344/#3347/#3363)
- RC3 gate report + release notes added

## Known Limitations

- **gitcode remote**: blocked by pre-receive hook enforcing LFS migration (3/4 remotes synced)
- **Real 24h soak**: deferred to W12 D1-2 (Z6G4 only)
- **Real 8-case crash**: deferred to W12 D3-4 (Z6G4 only)
- **168h soak**: deferred to post-RC3 (ongoing)

## All 5 RC3 P0 Blockers Closed

| # | Issue | Resolution |
|---|-------|-----------|
| #3221 | L3 acceptance binary + 15 E2E | ✅ Closed |
| #3222 | Server LOAD DATA perf | ✅ Closed |
| #3223 | Storage TX tracking (#2870) | ✅ Closed |
| #3227 | Replace corrupt SF=0.01 fixture | ✅ Closed (Q1.json added) |
| #3230 | Un-ignore 8 wire_smoke_sf tests | ✅ Closed (tests already pass) |

## Next: RC4

RC4 focuses on real runs (Z6G4 hardware required):
- Real 24h/72h wall-clock soak
- QPS/Sysbench actual measurements
- Performance baseline with 0 TBD
- TPC-H 22/22 SHA-256 capture

See [RC3_PLAN.md](./RC3_PLAN.md) for full critical path.
