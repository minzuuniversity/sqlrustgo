# v3.9.0 RC1 Release Notes

> **Version**: v3.9.0-rc1
> **Date**: 2026-06-05
> **Tag**: `v3.9.0-rc1` @ `29e2475f` (Gitea: pending push)
> **Branch**: `develop/v3.9.0`
> **Stage**: **RC1** (post-beta, pre-ga)
> **Type**: **Production Readiness Release** (engineering hardening, no new SQL)

---

## 1. Stage Summary

v3.9.0 has reached **RC1 (Release Candidate 1)** stage by:

1. Completing all 16 v3.9.0 sub-tasks (P0/P1/P2/P3, 16/16 closed)
2. Activating 10 G1-G10 gate framework (G1-G10 orchestrator + 10 underlying scripts)
3. Cutting `v3.9.0-beta` tag (W11) with 10/10 soak tests PASS
4. Implementing W11 G11/G12/G16 + Performance Baseline (parallel session, +2,200 lines)

| Metric | Status |
|--------|--------|
| Sub-tasks completed | **16/16** (100%) |
| G1-G10 gates | **10/10 PASS** (G10 non-blocking warn) |
| G11 QPS (5 workloads × 4 threads) | infrastructure ready, real bench in RC |
| G12 Sysbench OLTP | infrastructure ready (5 scripts + 30+ tests) |
| G16 Compatibility (v3.8 → v3.9) | **5/7 PASS** (compat tests + harness) |
| Performance Baseline | `docs/releases/v3.9.0/perf/PERFORMANCE_BASELINE.md` |
| TPC-H 22/22 baseline | **PASS** (inherited from v3.8.0-rc1) |
| 72h soak compressed-time | **10/10 tests PASS** |

## 2. Path to RC1

### 2.1 Phase 0 (W0): Setup
- Branch `develop/v3.9.0` forked from `main@v3.8.0`
- `v3.9.0-alpha1` tag cut at `61509ae5`
- 16-issue milestone (id=32) created

### 2.2 Phase 1-2 (W1-W4): P0 Architectural Debt
- P0-1 ARCH-3 VtuGuard main-path (#3169)
- P0-2 INT-3 Single Expression Engine (#3170)
- P0-3 INT-2 ParallelExecutor integration (#3171)
- P0-4 SEM-1 Savepoint MVCC (#3172)

### 2.3 Phase 3-4 (W5-W8): P1 Reliability
- P1-1 Backup/Restore/Verify CLI + PITR (#3173)
- P1-2 Crash Test Framework (100+ scenarios) (#3174)
- P1-3 Soak Test (24h/72h/168h) (#3175)
- P1-4 Upgrade Test (v3.8 → v3.9) (#3176)

### 2.4 Phase 5 (W9-W10): P2 GMP Audit
- P2-1 Audit Log (8 fields) (#3177)
- P2-2 Time Travel Query (AS OF TIMESTAMP) (#3178)
- P2-3 Hash Chain (immutable audit chain) (#3179)

### 2.5 Phase 6 (W11): P3 Performance + G11/G12/G16
- P3-1 Prepared Statement Cache (#3180)
- P3-2 Statistics (ANALYZE TABLE) (#3181)
- P3-3 Cost Optimizer (CBO) (#3182)
- P3-4 INT-2 ParallelExecutor optimization (#3183)
- P3-5 SIMD integration (#3184)
- G11 QPS/TPS baseline (`benches/qps_bench.rs` + `check_g11_qps.sh`)
- G12 Sysbench OLTP (5 workloads + 30+ tests)
- G16 Compatibility (4 case scripts + rollback test + 18 tests)
- Performance Baseline (`docs/releases/v3.9.0/perf/PERFORMANCE_BASELINE.md`)

### 2.6 Alpha → Beta → RC1
- **Alpha1** (W0): `v3.9.0-alpha1` tag — entry baseline
- **Beta** (W11): `v3.9.0-beta` tag — 16/16 + 10/10 G + 10/10 soak
- **RC1** (W12, now): `v3.9.0-rc1` tag — + G11/G12/G16 + Perf Baseline

## 3. What's New (RC1 vs Beta)

**RC1 introduces**:
1. **G11 QPS/TPS infrastructure** (5 workloads × 4 thread counts, criterion-based)
2. **G12 Sysbench OLTP infrastructure** (5 workload scripts + 30+ unit tests)
3. **G16 v3.8.0 → v3.9.0 compatibility** (4 case scripts + rollback + 18 tests)
4. **Performance Baseline** (`docs/releases/v3.9.0/perf/PERFORMANCE_BASELINE.md`)
5. **Test plan supplements** (Round 1: G11-G15, Round 2: G16 + baseline)

**No new SQL features** (per V390 plan, 0% allocation to new SQL).

## 4. G1-G16 Gate Status

| Gate | 主题 | 状态 | Issue/Script |
|------|------|------|--------------|
| G1 | 22/22 TPC-H 保持 | ✅ PASS | #3186 |
| G2 | INT-2 关闭 | ✅ PASS | #3187 |
| G3 | INT-3 关闭 | ✅ PASS | #3188 |
| G4 | ARCH-3 关闭 | ✅ PASS | #3189 |
| G5 | SEM-1 关闭 | ✅ PASS | #3190 |
| G6 | Backup/Restore | ✅ PASS | #3191 |
| G7 | 24h Soak | ✅ PASS | #3192 |
| G8 | Crash Matrix | ✅ PASS | #3193 |
| G9 | Upgrade | ✅ PASS | #3194 |
| G10 | GMP Audit | 🟡 PASS (warn) | #3195 |
| G11 | QPS/TPS | 🟡 infrastructure | (`check_g11_qps.sh`) |
| G12 | Sysbench OLTP | 🟡 infrastructure | (`check_g12_sysbench.sh`) |
| G13 | 24h Soak (real) | ⏳ RC 阶段启动 | (planned) |
| G14 | Crash 1000+ | ⏳ RC 阶段启动 | (planned) |
| G15 | 稳定性 | ⏳ RC 阶段启动 | (planned) |
| G16 | Compatibility | 🟡 5/7 PASS | (`check_g16_compatibility.sh`) |

G11-G12 require real benchmark runs (60+ min), scheduled for RC phase.
G13-G15 will be activated when their scripts are written.

## 5. 72h Soak Results (Recap)

| Metric | Result |
|--------|--------|
| Tests passed | 10/10 (24h/72h/168h smoke + invariants) |
| Memory growth | < 10% (PASS) |
| FD growth | 0 (PASS, no leak) |
| Lock growth | 0 (PASS, no leak) |
| p99 latency | bounded (PASS) |

See `docs/releases/v3.9.0/beta/SOAK_72H_REPORT.md` for full details.

## 6. Known Issues & RC Phase Plan

| Issue | Severity | RC Plan |
|-------|----------|---------|
| P0-2 unused fns in parser (clippy) | Low | Cleanup in RC W13 |
| `aggregate_5_basics` Float vs Integer | Low | Pre-existing, fix in RC W13 |
| `unused Serialize/Deserialize` in transaction | Low | Pre-existing, fix in RC W13 |
| 55 fmt diffs | Low | Pre-existing, fix in RC W13 |
| G10 P2-2 smoke step 7 timeout (180s+) | Low | P2-2 perf optimization in RC W13 |
| G11 QPS bench (need real run, 60+ min) | Low | Run in RC W13-W14 |
| G12 Sysbench (need sysbench binary) | Low | Install + run in RC W13 |
| Real 72h soak (real TPC-H queries) | Medium | W13-W14 (4 days wall-clock) |
| Real 168h soak (GA-final) | Medium | W15-W16 (1 week) |

## 7. RC Phase Plan (W12-W16, 4-5 weeks)

### 7.1 W12 (Week 1, current) — RC1 cut
- [x] 切 `v3.9.0-rc1` tag @ develop HEAD `29e2475f`
- [ ] Set up `soak_runner` background binary
- [ ] Start 72h real wall-clock soak (1 q/s, real TPC-H queries)
- [ ] Begin G1-G10 CI integration (CI jobs run `check_g_all.sh`)

### 7.2 W13-W14 (Weeks 2-3)
- [ ] 72h real soak (W13, 4 days wall-clock)
- [ ] 1000+ crash scenarios (W13)
- [ ] Run G11 QPS bench (W13, 60+ min)
- [ ] Install + run G12 Sysbench (W14)
- [ ] Fix pre-existing P0/P1 issues (W13)

### 7.3 W15 (Week 4)
- [ ] Doc finalization (RELEASE_NOTES.md, GA_GATE_REPORT.md)
- [ ] CHANGELOG.md final (G11-G15 supplement)
- [ ] Pre-existing issue cleanup

### 7.4 W16 (Week 5) — GA cut
- [ ] 切 `v3.9.0-rc2` (1 week after rc1)
- [ ] 168h long-running soak
- [ ] All G1-G16 fully PASS (no warnings)
- [ ] 切 `v3.9.0-ga` tag
- [ ] 写 `GA_RELEASE_NOTES.md` + `GA_GATE_REPORT.md`

## 8. Compatibility & Migration

- **v3.9.0-rc1 is wire-compatible** with v3.8.0 (same MySQL protocol)
- **Storage format unchanged** (no migration needed for v3.8 data)
- **All 22/22 TPC-H queries** maintain identical results
- **Upgrade path**: v3.6/v3.7/v3.8 → v3.9.0 verified (50+ scenarios in P1-4, G9 gate)
- **Rollback path**: v3.9.0 → v3.8.0 verified (G16, 1 rollback test)

## 9. References

- `docs/releases/v3.9.0/alpha/` (alpha stage docs, 4 files)
- `docs/releases/v3.9.0/beta/` (beta stage docs, BETA_RELEASE_NOTES + SOAK_72H_REPORT)
- `docs/releases/v3.9.0/perf/PERFORMANCE_BASELINE.md` (W11 perf baseline)
- `docs/releases/v3.9.0/plans/V390_TEST_PLAN.md`
- `docs/releases/v3.9.0/plans/V390_TEST_PLAN_SUPPLEMENT_PERF.md` (G11-G15)
- `docs/releases/v3.9.0/plans/V390_TEST_PLAN_ROUND2_REVIEW.md` (G16 + Baseline)
- `docs/openspec/g-gate-activation.md` (G1-G10 orchestrator)
- `docs/openspec/3175-soak-test.md` (P1-3 Soak Test)
- Issue #3167 (v3.9.0 启动公告, closed)
- Issue #3168 (v3.9.0-alpha1 entry, closed)

---

**Status**: 🟡 **RC1** (16/16 sub-tasks + 10/10 G1-G10 + G11-G16 infrastructure + 10/10 soak)
**Next**: rc2 cut at W16 (after 72h real soak + 168h GA-final soak)
