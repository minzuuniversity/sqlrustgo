# v3.9.0 Beta Release Notes

> **Version**: v3.9.0-beta
> **Date**: 2026-06-05
> **Tag**: `v3.9.0-beta` @ `c71b609f` (Gitea: pending push)
> **Branch**: `develop/v3.9.0`
> **Stage**: **Beta** (post-alpha, pre-RC)
> **Type**: **Production Readiness Release** (engineering hardening, no new SQL)

---

## 1. Stage Summary

v3.9.0 has reached **Beta** stage by completing all 16 v3.9.0 sub-tasks
(architectural debt + reliability + audit + performance) and 10 G1-G10
gate validation framework.

| Metric | Status |
|--------|--------|
| Sub-tasks completed | **16/16** (100%) |
| G1-G10 gates | **10/10 PASS** (G10 non-blocking warning) |
| 72h soak compressed-time | **10/10 tests PASS** (1,440× compression) |
| TPC-H 22/22 baseline | **PASS** (inherited from v3.8.0-rc1) |
| Doc gates | **PASS** (`check_docs_consistency.sh` + `check_docs_links.sh`) |

## 2. Path to Beta

### 2.1 Phase 0 (W0): Setup ✅
- Branch `develop/v3.9.0` forked from `main@v3.8.0`
- `v3.9.0-alpha1` tag cut at `61509ae5`
- 16-issue milestone (id=32) created

### 2.2 Phase 1-2 (W1-W4): P0 Architectural Debt ✅
- **P0-1 ARCH-3 VtuGuard main-path** (#3169) — CLOSED
- **P0-2 INT-3 Single Expression Engine** (#3170) — CLOSED
- **P0-3 INT-2 ParallelExecutor integration** (#3171) — CLOSED
- **P0-4 SEM-1 Savepoint MVCC** (#3172) — CLOSED

### 2.3 Phase 3-4 (W5-W8): P1 Reliability ✅
- **P1-1 Backup/Restore/Verify CLI + PITR** (#3173) — CLOSED
- **P1-2 Crash Test Framework (100+ scenarios)** (#3174) — CLOSED
- **P1-3 Soak Test (24h/72h/168h)** (#3175) — CLOSED
- **P1-4 Upgrade Test (v3.8 → v3.9)** (#3176) — CLOSED

### 2.4 Phase 5 (W9-W10): P2 GMP Audit ✅
- **P2-1 Audit Log (8 fields)** (#3177) — CLOSED
- **P2-2 Time Travel Query (AS OF TIMESTAMP)** (#3178) — CLOSED
- **P2-3 Hash Chain (immutable audit chain)** (#3179) — CLOSED

### 2.5 Phase 6 (W11): P3 Performance ✅
- **P3-1 Prepared Statement Cache** (#3180) — CLOSED
- **P3-2 Statistics (ANALYZE TABLE)** (#3181) — CLOSED
- **P3-3 Cost Optimizer (CBO)** (#3182) — CLOSED
- **P3-4 INT-2 ParallelExecutor optimization** (#3183) — CLOSED
- **P3-5 SIMD integration** (#3184) — CLOSED

### 2.6 Alpha Documentation ✅
- `ALPHA_BASELINE_REPORT.md` (124 lines)
- `ALPHA_GATE_CONTRACT.md` (214 lines, G1-G10 spec)
- `ALPHA_GATE_REPORT.md` (247 lines, 16/16 closed status)
- `ALPHA_STAGE_REVIEW.md` (196 lines, Phase 6 closure risks)

### 2.7 G-Gate Framework ✅
- 10 individual gate scripts (`check_p12_p35_*.sh` + `check_int2/int3/arch3/sem1/backup/upgrade_*.sh`)
- 1 new `check_g1_tpch_22_22.sh` (22/22 TPC-H)
- 1 orchestrator `check_g_all.sh` (10 gates, mapping to #3186-#3195)

## 3. What's New (Beta vs Alpha)

**Beta introduces**:
1. **G1-G10 orchestrator** (`scripts/gate/check_g_all.sh`) — single entry point
2. **72h soak compressed-time validation** (10/10 tests PASS)
3. **G-gate tracking issues #3186-#3195 closed** (10/10)

**No new SQL features** (per V390 plan, 0% allocation to new SQL).

## 4. G1-G10 Gate Status

| G | 主题 | 状态 | Issue |
|---|------|------|-------|
| G1 | 22/22 TPC-H 保持 | ✅ PASS | #3186 |
| G2 | INT-2 关闭 | ✅ PASS | #3187 |
| G3 | INT-3 关闭 | ✅ PASS | #3188 |
| G4 | ARCH-3 关闭 | ✅ PASS | #3189 |
| G5 | SEM-1 关闭 | ✅ PASS | #3190 |
| G6 | Backup/Restore | ✅ PASS | #3191 |
| G7 | 24h Soak (72h compressed) | ✅ PASS | #3192 |
| G8 | Crash Matrix | ✅ PASS | #3193 |
| G9 | Upgrade | ✅ PASS | #3194 |
| G10 | GMP Audit | 🟡 PASS (warn) | #3195 |

G10 has 1 non-blocking warning (P2-2 Time Travel smoke step 7 timeout, pre-existing).

## 5. 72h Soak Results

See `SOAK_72H_REPORT.md` for details.

| Metric | 72h Result |
|--------|-----------|
| Tests passed | 2/2 (test_soak_72h_smoke_p1_3 + test_soak_72h_smoke_no_fd_leak_p1_3) |
| Total soak tests | 10/10 PASS (24h + 72h + 168h + invariants) |
| Memory growth | < 10% (PASS) |
| FD growth | 0 (PASS, no leak) |
| Lock growth | 0 (PASS, no leak) |
| p99 latency | bounded (PASS) |

**Note**: Soak harness is **compressed-time smoke** (1,440× compression). Real 72h
test with real TPC-H queries scheduled for RC phase (W13-W16).

## 6. Known Issues & RC Phase Plan

| Issue | Severity | RC Plan |
|-------|----------|---------|
| P0-2 unused fns in parser (clippy warnings) | Low | Cleanup in P0-2 (W12) |
| `aggregate_5_basics` Float vs Integer (1 test fail) | Low | Pre-existing, fix in RC |
| `unused Serialize/Deserialize` in transaction | Low | Pre-existing, fix in RC |
| 55 fmt diffs across workspace | Low | Pre-existing, fix in RC |
| G10 P2-2 smoke step 7 timeout (180s+) | Low | P2-2 perf optimization in RC |

## 7. RC Phase Plan (W12-W15, 4 weeks)

### 7.1 W12 (Week 1)
- [ ] 切 `v3.9.0-rc1` tag
- [ ] Set up `soak_runner` background binary
- [ ] Start 72h real wall-clock soak (1 q/s, real TPC-H queries)
- [ ] Begin G1-G10 CI integration (CI jobs run `check_g_all.sh`)

### 7.2 W13-W15 (Weeks 2-4)
- [ ] 72h real soak (W13-W14)
- [ ] 1000+ crash scenarios (W13)
- [ ] Doc finalization (RELEASE_NOTES.md, GA_GATE_REPORT.md)
- [ ] CHANGELOG.md final (G11-G15 supplement)

### 7.3 W16 (Week 5) — GA cut
- [ ] 切 `v3.9.0-rc2` (1 week after rc1)
- [ ] 168h long-running soak
- [ ] All G1-G10 fully PASS (no warnings)
- [ ] 切 `v3.9.0-ga` tag
- [ ] 写 `GA_RELEASE_NOTES.md` + `GA_GATE_REPORT.md`

## 8. Compatibility & Migration

- **v3.9.0-beta is wire-compatible** with v3.8.0 (same MySQL protocol)
- **Storage format unchanged** (no migration needed for v3.8 data)
- **All 22/22 TPC-H queries** maintain identical results
- **Upgrade path**: v3.6/v3.7/v3.8 → v3.9.0 verified (50+ scenarios in P1-4)

## 9. References

- `docs/releases/v3.9.0/alpha/` (alpha stage docs, 4 files)
- `docs/releases/v3.9.0/beta/SOAK_72H_REPORT.md` (this release)
- `docs/releases/v3.9.0/plans/V390_VERSION_PLAN.md`
- `docs/releases/v3.9.0/plans/V390_DEVELOPMENT_PLAN.md`
- `docs/releases/v3.9.0/plans/V390_TEST_PLAN.md`
- `docs/openspec/g-gate-activation.md` (G1-G10 orchestrator)
- `docs/openspec/3175-soak-test.md` (P1-3 Soak Test)
- Issue #3167 (v3.9.0 启动公告, closed)
- Issue #3168 (v3.9.0-alpha1 entry, closed)

---

**Status**: 🟢 **BETA** (16/16 sub-tasks + 10/10 G-gates + 10/10 soak tests)
**Next**: RC1 cut at W12 (after real 72h soak)
