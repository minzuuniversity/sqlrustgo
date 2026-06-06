# v3.9.0 GA Remediation Program — Milestone Tracker

**Date**: 2026-06-07 (initial setup)
**Branch**: develop/v3.9.0 @ 9aabbb3f (pre-R1) → @ 63579eb2 (R1, local only — Gitea unreachable)
**Status**: Program structure established. Phase 1 P1 (R1 crash monkey) implemented locally, awaiting Z6G4/Gitea recovery for push.

---

## 1. Program Structure

Gitea organization (5 milestones + 7 labels):

| Milestone | ID | Phase | Color | Issues |
|-----------|----|-------|-------|--------|
| **GA-P0-TPC-H** | 33 | Phase 1 | red | T1-T5 (5 sub-issues #3258-#3262) + 6 TPC-H carry-overs (#2948, #3230, #3231, #3227, #3248, #3256) |
| **GA-P0-Stability** | 34 | Phase 2 | red | S1-S4 (4 sub-issues #3263-#3266) + 4 soak/perf carry-overs (#3224, #3225, #3228, #3229) |
| **GA-P1-Recovery** | 35 | Phase 3 | yellow | R1-R3 (3 sub-issues #3267-#3269) |
| **GA-P1-INT** | 36 | Phase 4 | yellow | INT-2 + INT-3 (#3270, #3271) + #3108, #3146 carry-overs |
| **GA-P2-Governance** | 37 | Phase 5 | green | (P2 sub-issues to be created as needed) |

Labels: `ga-remediation` (master), `ga-p0-tpch`, `ga-p0-stability`, `ga-p1-recovery`, `ga-p1-int`, `ga-p2-governance`, `blocked-ga`.

Total open issues: 27 (13 new sub-issues + 14 carry-overs).

## 2. Sub-Issue Index

| ID | Phase | Owner-class | Title | Z440-feasible |
|----|-------|-------------|-------|---------------|
| #3258 | P0/T1 | blocked-user | Establish TPC-H baseline (DuckDB + PG) | No (needs DuckDB/PG) |
| #3259 | P0/T2 | blocked-user | Rebuild SF=0.01 dataset (dbgen) | No (needs TPC-H toolchain) |
| #3260 | P0/T3 | ai-claim | Implement `cargo test tpch_verify` | **Yes** (post-T1+T2) |
| #3261 | P0/T4 | blocked-user | Fix Q4/Q8/Q9/Q15 bugs | No (TPC-H user) |
| #3262 | P0/T5 | ai-claim | TPC-H CI gate (block PR on 22/22) | **Yes** (post-T3) |
| #3263 | P0/S1 | blocked-user | Recover Z6G4 SSH | No (needs console access) |
| #3264 | P0/S2 | blocked-user | 24h soak | No (needs Z6G4) |
| #3265 | P0/S3 | blocked-user | 72h soak | No (needs Z6G4) |
| #3266 | P0/S4 | blocked-user | 168h soak (GA gate) | No (needs Z6G4) |
| #3267 | P1/R1 | ai-claim | Crash monkey | **DONE locally** (commit 63579eb2, 4/4 PASS, 100k in 0.41s) |
| #3268 | P1/R2 | ai-claim | Recovery fuzzer | **Yes** (next candidate) |
| #3269 | P1/R3 | ai-claim | Expand recovery 9→50+ scenarios | **Yes** (medium) |
| #3270 | P1/INT-2 | blocked-user | Cross-version upgrade chain | No (multi-week) |
| #3271 | P1/INT-3 | blocked-user | Mixed-scenario integration | No (multi-week) |

## 3. Status (as of 2026-06-07)

### Phase 1 (TPC-H, 3-5 days)
- **T1, T2, T4**: blocked on user (DuckDB/PG, dbgen, TPC-H bugs)
- **T3 (verifier)**: not yet started — wait for T1+T2
- **T5 (CI gate)**: not yet started — wait for T3
- **6 TPC-H carry-overs**: re-classified, not closed

### Phase 2 (Stability, 1 week)
- **S1-S4**: all blocked on Z6G4 recovery (#3263 highest priority)
- 4 soak/perf carry-overs re-classified, not closed

### Phase 3 (Recovery hardening, 3 days)
- **R1 (crash monkey)**: ✅ IMPLEMENTED LOCALLY. commit 63579eb2, 4/4 tests PASS, 100k iteration sweep in 0.41s. Push pending (Gitea unreachable).
- **R2 (recovery fuzzer)**: not yet started
- **R3 (50+ scenarios)**: not yet started

### Phase 4 (INT, 2-3 weeks)
- **INT-2 + INT-3**: blocked on user (cross-version binaries + multi-week work)
- 2 carry-overs (#3108, #3146) re-classified

### Phase 5 (Governance, 1 week)
- No sub-issues created yet. Candidates: A1 ignore→0, A2 clippy 98→0, A3 coverage

## 4. Blocked Issues (carry-overs from previous attempts)

| Issue | Title | Reason | Resolution |
|-------|-------|--------|-----------|
| #3234 | UPDATE replay known bug + arch debt | PR-841/842 branch not pushed | re-labeled `blocked-ga`; subsumed by #3261 (T4) |
| #3235 | Storage visibility (execute_update empty) | Same — PR-841/842 | re-labeled `blocked-ga` |
| #3236 | Refactor UPDATE to storage.update() 唯一入口 | Same — PR-841/842 | re-labeled `blocked-ga` |

These are the "ghost" issues from the v3.8.x era. Will be re-evaluated after PR-841/842 is pushed (or after the TPC-H milestone closes).

## 5. Time Line (per GA Remediation Plan)

| Window | Phase | Goal | Z440 contribution | User contribution |
|--------|-------|------|-------------------|-------------------|
| 2026-06 | P0 TPC-H + P1 Recovery | T1-T5 close, R1-R3 done | R1 ✅, R2, R3, T3, T5 | T1, T2, T4 |
| 2026-07 | P0 Stability | S1-S4 done (24h/72h/168h) | (nothing — needs Z6G4) | S1 (Z6G4 fix), S2-S4 (run soaks) |
| 2026-07 late | P1 INT | INT-2 + INT-3 done | (nothing — multi-week user work) | INT-2, INT-3 |
| 2026-08 | RC2 + GA | G1-G5 all PASS | (final PRs / governance cleanup) | (RC2 sign-off, GA release) |

## 6. Z440-Actionable Backlog (priority order)

1. **R2** (#3268) — Recovery fuzzer: random WAL with adversarial patterns. Z440-only, 1-2 days. *Recommended next.*
2. **R3** (#3269) — Expand recovery 9→50+ scenarios. Builds on R1+R2. 3 days.
3. **T3** (#3260) — `cargo test tpch_verify` (after T1+T2 land). 1-2 days.
4. **T5** (#3262) — TPC-H CI gate (after T3). 2-3 hours.
5. **P2 A1** — tx_wal ignore 6→0 cleanup. 1 day.
6. **P2 A2** — clippy 98 test errors → 0. 1-2 days.
7. **P2 A3** — coverage statistics. 2-3 hours.

## 7. Recommendations for Next Session

When Z6G4 / Gitea is back online:
1. Push `63579eb2` (R1 crash monkey) → PR#? → merge
2. Start R2 (recovery fuzzer) immediately while waiting on Z6G4
3. Push program structure to Gitea milestones + labels (already done, but verify the 14 new sub-issues are visible in the milestone view)

If Z6G4 stays offline:
- Continue Z440 work: R2, R3, T3 (if user provides T1/T2), P2 A1-A3
- Document a "Z6G4 offline runbook" so the next session has a path to recovery
