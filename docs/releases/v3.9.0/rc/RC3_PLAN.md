# v3.9.0 Adjusted Release Plan — RC3/RC4/GA

> **Date**: 2026-06-05
> **Status**: 🔴 **Adjusted due to test authenticity findings**
> **Source**: `docs/audit/status/2026-06-06-test-authenticity-analysis-v390.md`

---

## 1. 关键结论

v3.9.0-rc2 (`76efe391`) was cut based on **form-only gate validation**:
- G1 TPC-H gate: 0/6 steps actually run TPC-H (checks file existence, compilation, commit log)
- Soak tests: simulated CPU loop (synthetic latencies, not real queries)
- 77 `#[ignore]` tests, 43 TBD perf placeholders, 0/22 wire TPC-H

**Real production-equivalent coverage: ~35%** (not 90%+ implied by previous reports).

**v3.9.0-rc2 tag is NOT reverted** (per user direction). It is documented as a **form-only validation milestone**, not a production-ready candidate.

---

## 2. Adjusted Release Plan (rc2 → rc3 → rc4 → ga)

### 2.1 rc2 (current @ `76efe391`) ✅

- Form-only validation milestone
- 10/10 G1-G10 form PASS, 1 non-blocking warn
- 10/10 simulated soak PASS
- 6 perf reports committed (with 43 TBD placeholders)
- **Not production-ready** — see audit report

### 2.2 rc3 (P0 cut, after server perf + L3 + TX/WAL fixes)

**Goal**: All P0 governance blockers closed (no `#[ignore]` for critical correctness, no TX/WAL gaps).

**Required closed issues** (5):
- [ ] #3221 L3 acceptance + unignore 15 e2e_canonical
- [ ] #3222 Server `LOAD DATA` perf fix
- [ ] #3223 Storage tx tracking + unignore 9 TX/WAL
- [ ] #3227 Replace corrupt SF=0.01 fixture
- [ ] #3230 Unignore 8 wire_smoke_sf (value-correctness)

**Cut criteria**:
- [ ] G1 gate re-engineered to actually run TPC-H 22/22 (with `TPCH_FORCE=1`)
- [ ] G8 Crash Matrix + G16 Compatibility (real, not simulated)
- [ ] All P0 `#[ignore]` tests either closed (un-ignored + passing) or explicitly deferred
- [ ] Doc gates PASS

**Estimated**: 2-3 weeks of focused work on server perf + L3 implementation.

### 2.3 rc4 (P1 cut, after Z6G4 real runs)

**Goal**: All Z6G4 real-run critical-path items closed (real soak, perf baseline, TPC-H baseline).

**Required closed issues** (4 + 1 partial):
- [ ] #3224 Z6G4 perf measurement + fill PERFORMANCE_BASELINE.md
- [ ] #3225 Real 24h/72h wall-clock soak
- [ ] #3228 Unignore 10 long_run_stability
- [ ] #3229 Real 168h wall-clock soak (7 days)
- [ ] #3231 Capture TPC-H 22/22 SHA-256 baseline

**Cut criteria**:
- [ ] 24h real soak PASS (memory < 10%, FD = 0, lock = 0)
- [ ] 72h real soak PASS
- [ ] 168h real soak kick-off (continued into GA)
- [ ] PERFORMANCE_BASELINE.md 43 TBD → 0 TBD
- [ ] TPC-H 22/22 SHA-256 captured on Z6G4 canonical SF=0.01
- [ ] All G11/G12/G13/G14/G15 (perf + crash) actually run, not just infra

**Estimated**: 3-4 weeks on Z6G4 hardware (depends on availability).

### 2.4 ga (final, after all 13 closed + 168h real soak)

**Goal**: Production-ready release.

**Required closed issues**: ALL 13 critical-path items + 1 extra (#3226 TPC-H Q8/Q9 fix).

**Cut criteria**:
- [ ] All 11 follow-up issues closed (#3221-#3231)
- [ ] 168h real soak completed successfully
- [ ] All G1-G16 gates run real validation (not form-only)
- [ ] Doc gates PASS (real data, not TBD)
- [ ] GA_RELEASE_NOTES.md + GA_GATE_REPORT.md written
- [ ] `release/v3.9.0` branch cut (maintenance)

**Estimated GA date**: 2026-09-23 (per V390 plan) — at risk if Z6G4 hardware not available by W14.

---

## 3. 13 Critical-Path Items (all in milestone v3.9.0)

| # | Item | Issue | Priority | Phase |
|---|------|-------|----------|-------|
| 1 | Server `LOAD DATA` perf | #3222 | P0 | rc3 |
| 2 | Replace corrupt SF=0.01 fixture | #3227 | P0 | rc3 |
| 3 | L3 acceptance + 15 e2e unignore | #3221 | P0 | rc3 |
| 4 | Unignore 10 long_run_stability | #3228 | P1 | rc4 |
| 5-7 | Real 24h/72h/168h soak (3 items) | #3225 (24h+72h), #3229 (168h) | P1 | rc4 + ga |
| 8-9 | QPS bench + fill perf baseline | #3224 | P1 | rc4 |
| 10 | TX/WAL fix #2870 | #3223 | P0 | rc3 |
| 11 | Unignore 8 wire_smoke_sf | #3230 | P0 | rc3 |
| 12 | TPC-H 22/22 SHA-256 capture | #3231 | P1 | rc4 |
| 13 | l3_canonical_binary acceptance | #3221 (combined) | P0 | rc3 |
| Extra | TPC-H Q8/Q9 真 bug fix | #3226 | P2 | (any) |

**Total: 11 follow-up issues covering 13+ items.**

---

## 4. Updated Milestone Status

| Stage | Tag | Status | Date | Blocker? |
|-------|-----|--------|------|----------|
| alpha1 | `v3.9.0-alpha1` | ✅ | 2026-06-05 | - |
| beta | `v3.9.0-beta` | ✅ | 2026-06-05 | - |
| rc1 | `v3.9.0-rc1` | ✅ (form-only) | 2026-06-05 | - |
| rc2 | `v3.9.0-rc2` | ✅ (form-only) | 2026-06-05 | - |
| **rc3** | (planned) | ⏳ | after P0 issues closed | YES |
| **rc4** | (planned) | ⏳ | after P1 issues closed | YES |
| ga | (planned) | ⏳ | after all 11 closed + 168h | YES |

**Hard-blocking gates** (per V390 plan §6):
- G1 (TPC-H 22/22 保持): currently form-only, must be real by rc3
- G2-G5 (P0 架构债): real by rc2 (already done)
- G6-G9 (P1 可靠性): real by rc4 (currently form-only)
- G10 (审计): currently form-only, must be real by rc3

---

## 5. Per-Stage Gate Criteria

### rc3 gates (must PASS to cut)
- [ ] **G1 REAL**: 22/22 TPC-H actually runs (not just file checks)
- [ ] **G2-G5**: All P0 architectural debt REAL (already passed rc2)
- [ ] **G8**: Crash matrix runs real scenarios (not just compile checks)
- [ ] **G16**: Compatibility v3.8 → v3.9 with real canonical data
- [ ] **No `#[ignore]`** in items 1, 2, 3, 10, 11, 13

### rc4 gates (must PASS to cut)
- [ ] **G7/G13 REAL**: 24h+72h real wall-clock soak (not simulated)
- [ ] **G11/G12 REAL**: QPS/Sysbench actual measurements
- [ ] **G14 REAL**: 8 real crash categories (kill -9, OOM, etc.)
- [ ] **G15**: Performance report with real data (no TBD)
- [ ] **TPC-H SHA-256 captured** (item 12)
- [ ] **No `#[ignore]`** in items 4, 5, 6, 7, 8, 9, 12

### ga gates (must PASS to cut)
- [ ] **All 11 follow-up issues closed**
- [ ] **168h real soak completed** (item 7)
- [ ] **PERFORMANCE_BASELINE: 0 TBD** (item 9)
- [ ] **All G1-G16 gates run real validation** (not form-only)
- [ ] **GA_RELEASE_NOTES + GA_GATE_REPORT** complete with real data

---

## 6. Critical Path Diagram

```
W0 (Alpha1) ──────> W11 (rc2 form-only) ──────> rc3 ────> rc4 ────> ga
                              │                  │         │         │
                              │                  │         │    [168h real soak]
                              │                  │         │         │
                              ▼                  ▼         ▼         ▼
                          form-only      server perf  Z6G4    all 11
                          milestone      + L3 + TX    real    issues
                                          (P0)         runs    closed
                                                       (P1)
```

Estimated timeline:
- **rc2 → rc3**: 2-3 weeks (server perf + L3 work)
- **rc3 → rc4**: 3-4 weeks (Z6G4 real runs)
- **rc4 → ga**: 2-3 weeks (168h soak + final cleanup)

**GA target: 2026-09-23** (per V390 plan, 12 weeks from alpha1). Achievable if Z6G4 hardware available by W14.

---

## 7. References

- `docs/audit/status/2026-06-06-test-authenticity-analysis-v390.md` (248 lines, 11KB)
- `docs/audit/status/2026-06-06-v390-test-infrastructure-remediation.md` (parallel session)
- 11 follow-up issues: #3221, #3222, #3223, #3224, #3225, #3226, #3227, #3228, #3229, #3230, #3231
- V390_DEVELOPMENT_PLAN.md (original 16-task plan)
- V390_TEST_PLAN.md (G1-G10 spec)
- ALPHA_GATE_CONTRACT.md (gate framework)

---

**Honest Reframing**: v3.9.0 is a **Production Readiness Release** (engineering hardening). Cutting ga without resolving the 13 critical-path items would violate that promise. The adjusted plan delays ga by 2-6 weeks (depending on Z6G4 availability) to ensure legitimate production-readiness.
