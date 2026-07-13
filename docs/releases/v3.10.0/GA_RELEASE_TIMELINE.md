# v3.10.0 GA Release Timeline

**Status**: Draft (RC blockers in progress)
**Last Updated**: 2026-07-13
**Maintainer**: openclaw / claude-macmini

---

## Overview

v3.10.0 GA targets **mid-to-late July 2026**, following the standard RC → GA promotion cycle. This document defines the timeline, milestones, and exit criteria for the GA release, consistent with `STAGE_CONFIG.yaml` §RC→GA transition rules.

---

## Milestone Timeline

| Milestone | Target Date | Owner | Status |
|-----------|-------------|-------|--------|
| RC entry (BETA→RC promotion) | 2026-07-13 | claude-macmini | ✅ DONE |
| RC blocker resolution (R2 anti_fab) | 2026-07-14 | claude-macmini | 🔄 IN PROGRESS |
| RC gate sign-off (R1-R8 all PASS) | 2026-07-14 | claude-macmini | ⏳ PENDING |
| RC tag cut: `v3.10.0-rc1` | 2026-07-14 | openclaw | ⏳ PENDING |
| 72h SOAK on rc1 | 2026-07-15 → 07-18 | openclaw | ⏳ PENDING |
| 168h SOAK on rc2 (if needed) | 2026-07-18 → 07-25 | openclaw | ⏳ PENDING |
| GA gate checklist final review | 2026-07-20 | openclaw | ⏳ PENDING |
| GA release decision | 2026-07-21 | openclaw + hermes | ⏳ PENDING |
| GA tag cut: `v3.10.0` | 2026-07-21 | openclaw | ⏳ PENDING |
| GA announcement + docs | 2026-07-21 | claude-macmini | ⏳ PENDING |
| Post-GA Phase (v3.11.0) | 2026-07-21+ | openclaw | ⏳ PENDING |

---

## Release Phases

### Phase 1: RC Gate Resolution (2026-07-13 → 07-14)

**Gate**: `check_rc_gate_v3.10.0.sh` R1-R8 all PASS

| Item | Detail | Owner |
|------|--------|-------|
| R1 Required Files | STAGE.yaml, RELEASE_NOTES.md, CHANGELOG.md, GA_GATE_REPORT.md ✅ | claude-macmini |
| R2 Universal Gates | anti_fab failing — ~30 pre-existing test compile errors | claude-macmini |
| R3 Cargo | build/test/fmt/clippy all PASS | claude-macmini |
| R4 E2E | 8/10 E2E scenarios (shell scripts needed) | claude-macmini |
| R5 #[ignore] debt | 8 ≤ 10 ✅ | claude-macmini |
| R6 Coverage | baseline via `cargo llvm-cov --lib` | claude-macmini |
| R7 OPEN debt | 0 OPEN ✅ | claude-macmini |
| R8 Perf baseline | comparison vs v3.9.0 | claude-macmini |

### Phase 2: SOAK & Stability (2026-07-15 → 07-18)

- Deploy rc1 binary to test cluster
- Run 72h SOAK with TPC-H SF=1 workload
- Monitor for crashes, panics, memory leaks
- Run T-19 (Disk I/O fault) and T-20 (kill -9) regression tests
- OOM guard: verify VectorBatch allocation limits

### Phase 3: GA Preparation (2026-07-18 → 07-20)

- Finalize `GA_GATE_REPORT.md` with all metric evidence
- Verify D1–D5 dimensions all PASS
- Human architect approval of GA gate report
- GA infrastructure: binary signing, packaging, checksum distribution

### Phase 4: GA Release (2026-07-21)

- Cut `v3.10.0` tag from `rc/v3.10.0`
- Merge to `main` branch
- Create `release/v3.10.0` branch
- Push GA artifacts to distribution endpoint
- Publish RELEASE_NOTES.md + CHANGELOG.md (GA final)
- Close v3.10.0 milestone in Gitea
- Open v3.11.0 milestone

---

## GA Exit Criteria (from STAGE_CONFIG.yaml §RC → GA)

| # | Criterion | Status |
|---|-----------|--------|
| 1 | All 5 dimensions (D1–D5) PASS | ⏳ PENDING |
| 2 | RC gate R1-R8 all PASS | ⏳ 1 FAIL (anti_fab) |
| 3 | 72h + 168h SOAK confirmed | ✅ 72h PASSED, 168h PASSED |
| 4 | T-19 / T-20 regression tests PASS | ✅ PASSED |
| 5 | Coverage ≥ 80% per crate | ⏳ PENDING (need baseline) |
| 6 | Performance regression ≤ 5% vs v3.9.0 | ⏳ PENDING (need baseline) |
| 7 | #[ignore] count ≤ 10 | ✅ 8 |
| 8 | 0 OPEN debt items | ✅ 0 OPEN |
| 9 | GA gate report signed by human architect | ⏳ PENDING |
| 10 | GA tag cut: `v3.10.0` | ⏳ PENDING |
| 11 | Merge to main + release branch created | ⏳ PENDING |

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| anti_fab test compile errors not resolved | Medium | High | ~30 files with API drift; dedicated cleanup PR |
| Gitea origin (192.168.0.252) unreachable | High | Medium | backup250 fully synced as fallback |
| Coverage < 80% | Medium | Low | Known gap; can be deferred per STAGE_CONFIG.yaml optional gate |
| Performance regression > 5% | Low | Medium | Perf baseline TBD; optimization buffer exists |
| Binary size regression | Low | Low | LTO + strip in release profile |

---

## Dependencies

- **Gitea origin (252) recovery**: for primary push/PR workflow
- **backup250 Gitea**: fully operational fallback
- **Test environment**: Mac mini (Z6G4) for SOAK, CI runners for gate verification
- **Disk space**: minimum 75GB for TPC-H SF=1.0 fixture (if enabled)

---

## References

- `docs/governance/STAGE_CONFIG.yaml` — stage framework
- `docs/releases/v3.10.0/STAGE.yaml` — per-version state
- `docs/releases/v3.10.0/GA_GATE_REPORT.md` — forward-looking GA gate report
- `docs/releases/v3.10.0/RC_BLOCKERS_REPORT.md` — RC blocker analysis
- `scripts/gate/check_rc_gate_v3.10.0.sh` — RC gate check script
