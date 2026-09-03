# V312-RC-GA Path B — Execution Plan

> **provenance:** generated_by=claude-macmini, generated_at=2026-09-04T01:30:00+08:00,
> branch=develop/v3.12.0, HEAD=c67d4fddc072d94b2080c940d0af46ab8a8d0686,
> source_repo=openclaw/sqlrustgo, policy=Anti-Fabrication-Policy-v1.0
>
> **scope:** Path B (full 10–14 day plan) chosen by user on 2026-09-04.
> Path B = Path A (5–7 day GA) + extra hardening and full ChatGPT-pattern coverage.
>
> **supersedes:** nothing (first comprehensive path-B plan for v3.12.0 GA)
> **depends on:** docs/releases/v3.12.0/RC_GA_TRIAGE_AND_GATE_PLAN_2026-09-03.md

## 0. Why Path B

Per Round-24 review and 2026-09-03 B-track hardening update, the minimum GA cut
requires at minimum **Path A** (5–7 days). The user selected **Path B**
(10–14 days) which adds:

1. **Path A baseline**: 6 B-track blocker fixes + 6 RC-B gate implementations
   + 9 claim-caveat release-note downgrades + V312-57 stage2 merge
2. **Path B additions**:
   - Path B-1: PR-batch fix v3.13/defer P1 items (#4699 CTE, #4698 math cluster)
   - Path B-2: Per-issue evidence sections per ChatGPT Pattern 2/3 (Anti-Pattern
     10 conditions + per-issue evidence doc)
   - Path B-3: Reviewer 2 (hermes-z6g4 / Codex) actual allocation + sign-off
   - Path B-4: Final GA GATE_REPORT.md regeneration in `--full` mode with
     12/12 sign-off criteria PASS

## 1. Current State Snapshot (HEAD `c67d4fddc0`)

| Bucket | Total | open | closed |
|---|---|---|---|
| GA-blocker | 15 | 7 | 8 |
| GA-claim-caveat | 17 | 9 | 8 |
| v3.13/defer | 18 | 16 | 2 |
| **SUM** | **50** | **32** | **18** |

### Already applied Gitea labels (Phase 1.1 ✅)

- 7 GA-blocker → `GA-blocker` + `v3.13-followup`
- 9 GA-claim-caveat → `GA-claim-caveat`
- 16 v3.13/defer → `v3.13-followup`

### Already landed commits (Phase 1.2 + 1.4 ✅)

- commit `c5e4a4d2bd`: 6 RC-B gate skeletons + CLAIM_DOWNGRADE_MANIFEST.md
  (7 files added, 452 insertions)

## 2. Phase breakdown

### Phase 1 — Non-code infrastructure (≈ 1 day)

Status: 3 of 6 sub-phases complete as of `c5e4a4d2bd`.

| ID | Sub-phase | Status | Output |
|----|-----------|--------|--------|
| 1.1 | Gitea label governance | ✅ done (Phase 1.1) | 32 issues labeled |
| 1.2 | RC-B gate scripts skeleton | ✅ done (Phase 1.2) | 6 honest-stub scripts |
| 1.3 | Per-issue evidence section | ⏳ pending (Phase 1.3) | 16 evidence docs |
| 1.4 | GA-claim-caveat scope downgrade | ✅ done (Phase 1.4) | CLAIM_DOWNGRADE_MANIFEST.md |
| 1.5 | V312-57 stage2 branch merge | ⏳ pending (Phase 1.5) | feat/v312-57-stage2 → develop |
| 1.6 | Path B overall plan | ✅ THIS DOC (Phase 1.6) | this document |

### Phase 2 — Code fixes (≈ 5–7 days, depends on Path B scope additions)

| PR | Issue | Work-Package | Est. | Owner |
|----|-------|-------------|------|-------|
| PR-A1 | #4708 中文标识符/中文注释/quoted id | WP-A | 1–2 days | (TBD) |
| PR-A2 | #4682 sqlite_master + sqlite_sequence | WP-C | 1 day (PR #4739 continuation) | (TBD) |
| PR-A3 | #4668 NATURAL JOIN multi-column USING | WP-D | 1–2 days | (TBD) |
| PR-A4 | #4703 ON DUPLICATE KEY + VALUES() | WP-A | 1 day | (TBD) |
| PR-A5 | #4674 CHAR_LENGTH semantics | WP-B | 0.5 day | (TBD) |
| PR-A6 | #4652 CREATE PROCEDURE / FUNCTION no-op | WP-C | 0.5 day | (TBD) |
| PR-A7 | #4626 SELECT FOR UPDATE + ROLLBACK | WP-C | 1 day | (TBD) |

**Each PR must satisfy Round-24 strict standards**:
- regression test RED → fix GREEN
- per-issue evidence doc (Phase 1.3)
- per-PR provenance (commit/branch/source_agent/source_run)
- Gitea label `GA-blocker` carried through PR
- merge commit SHA referenced in GA GATE_REPORT

### Phase 3 — v3.13/defer P1 hardening (Path B addition, ≈ 2 days)

| PR | Issue | Note |
|----|-------|------|
| PR-B1 | #4699 WITH RECURSIVE execution | Requires recursive CTE design — likely spill |
| PR-B2 | #4698 math function cluster | MOD/POWER/LOG/EXP/SQRT/GREATEST/LEAST |
| PR-B3 | #4719 sqlite_stat1 + ANALYZE | System-table sibling to #4682 |

Each PR is best-effort; if blocked at architecture level, must follow "DEFERRED-with-explicit-boundary" path per Round-24:

```markdown
DEFERRED-with-explicit-boundary
- Owner: openclaw (or designee)
- Expiry: 2027-06-30
- Closing boundary: requires <architectural prerequisite> available
- Supersedes prior `SUBSTANTIALLY_COMPLETE` or `ACCEPTED-WITH-BINDING-MANIFEST`
```

### Phase 4 — Reviewer 2 + GA sign-off (Path B addition, ≈ 1 day)

| Step | Action |
|------|--------|
| 4.1 | Reviewer 2 allocation: hermes-z6g4 (designee) per Round-24 §4.2 |
| 4.2 | Sign-off criteria table (12 items, expanded from prior 7) |
| 4.3 | Reviewer 2 produces independent sign-off doc at `evidence/v312-rc-ga/REVIEWER-2-SIGNOFF.md` |
| 4.4 | GA GATE_REPORT.md regenerated `--full` mode referencing all PR merge commits |

### Phase 5 — Final GA promotion (≈ 1 day, depends on all phases)

| Step | Action |
|------|--------|
| 5.1 | All Phase 2 PRs merged → develop/v3.12.0 |
| 5.2 | Phase 4.4 sign-off present |
| 5.3 | 6 RC-B gates exit 0 (with full fixtures from Phase 1.3) |
| 5.4 | README.md + RELEASE_NOTES.md carry claim downgrade from §1.4 |
| 5.5 | GA GATE_REPORT.md final version with `--full` mode |
| 5.6 | Tag `v3.12.0` after gate PASS |

## 3. Critical Path & Parallelization

```
Phase 1.1 (done) ── Phase 1.2 (done) ─┬─ Phase 1.3
                                      │
                                      ├─ Phase 1.4 (done) ──────┐
                                      │                          │
                                      ├─ Phase 1.5 (merge) ─────┤
                                      │                          │
                                      └─ Phase 1.6 (this) ──────┤
                                                                 │
                                                                 ▼
                                              ┌──────────────────┴──────────────────┐
                                              ▼                                     ▼
                                          Phase 2 PR-A1..A7                       Phase 3 PR-B1..B3
                                          (code fixes)                            (v3.13 P1 hardening)
                                              │                                     │
                                              └──────────────┬──────────────────────┘
                                                             ▼
                                                       Phase 4 (Reviewer 2)
                                                             │
                                                             ▼
                                                       Phase 5 (GA promotion)
```

**Parallelization opportunities**:
- PR-A1, PR-A4, PR-A5 are independent (parser/function areas)
- PR-A2, PR-A3 are independent (storage vs planner)
- PR-A7 can be sequenced after A2/A3 (transaction correctness depends on storage state)
- Phase 3 PRs can run in parallel with Phase 2 if separate work-tree branches

**Critical path (longest sequential)**:
PR-A1 (#4708 中文) → PR-A2 (#4682 sqlite) → PR-A3 (#4668 NATURAL JOIN) → Phase 4 → Phase 5
≈ 1.5 + 1 + 1.5 + 1 + 1 = ~6 days minimum

## 4. Work-Package Definitions (from RC-GA §7)

| WP | Scope | Issues | Owner suggestion |
|----|-------|--------|------------------|
| A | Parser real-script compatibility | #4708, #4696, #4710, #4703 | parser working group |
| B | Type/function correctness | #4674, #4676, #4675, #4716, #4698 | function/runtime group |
| C | DDL/DML no-op and integrity | #4709, #4652, #4672, #4682, #4626 | executor/storage group |
| D | Join/subquery correctness | #4649, #4668, #4656, #4636 | planner group |
| E | Gate hardening | RC-B1..B7 + per-issue evidence | test infra |
| F | Claim downgrade/defer | all claim-caveat + v3.13/defer | docs |

## 5. Anti-Pattern Conditions (Path B enforcement, per Round-24)

Each PR in Phase 2/3 MUST fail closed if ANY of these are true:

1. ❌ No RED regression test before FIX commit
2. ❌ No per-issue evidence doc published
3. ❌ No Anti-Pattern 10 conditions listed in PR body
4. ❌ No reviewer-2 sign-off reference
5. ❌ No SQLite oracle diff (where oracle applicable)
6. ❌ Closes GA-blocker without merging PR
7. ❌ Forced "ACCEPTED-WITH-BINDING-MANIFEST" / "SUBSTANTIALLY_COMPLETE" / "DEFERRED-without-tracking"
8. ❌ Bypasses expiry date
9. ❌ Skips `bash -n` syntax check on new gate scripts
10. ❌ Skips `cargo test` for executor changes

## 6. Risk register

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|-----------|
| B-track fixtures too narrow | Medium | High | Use SQLite as oracle; require ≥80% pass per gate |
| PR-A1 中文标识符 has lexer-level impact | High | Medium | Isolate to identifier lexing, not comments |
| PR-A3 NATURAL JOIN requires planner rewrite | High | High | Consider downgrading to claim-boundary instead |
| Reviewer 2 not available | Medium | High | Allocate from minimas/hermes-z6g4 pool; have Codex fallback |
| GA tag cut before all gates PASS | Low | Critical | Phase 5 step 5.6 gated on 5.1–5.5 |
| CLAIM_DOWNGRADE_MANIFEST out of sync with new closings | Medium | Medium | Update after each Phase 2 PR merge |
| Existing FA docs reference closed issues without supersede note | Medium | Low | Per-Phase-1.3 evidence doc carries supersedes link |

## 7. Checkpoint Decision Points

User approval is required at:

- **C1 (after Phase 1.3)**: confirm 16 evidence doc format + reviewer 2 candidate list
- **C2 (after Phase 2 first 3 PRs)**: confirm WP-A approach (PR-A1 中文 is highest risk)
- **C3 (after Phase 3 PR-B1)**: confirm v3.13 P1 feasibility vs deferral
- **C4 (after Phase 4.3)**: confirm Reviewer 2 sign-off before Phase 5

## 8. Phase Status Board (Path B)

| Phase | Status | Last update | Next trigger |
|-------|--------|-------------|--------------|
| Phase 1.1 | ✅ done | 2026-09-04 | — |
| Phase 1.2 | ✅ done | 2026-09-04 (c5e4a4d2bd) | — |
| Phase 1.3 | ⏳ pending | — | user choice |
| Phase 1.4 | ✅ done | 2026-09-04 (c5e4a4d2bd) | — |
| Phase 1.5 | ⏳ pending | — | needs gate evidence matching BR |
| Phase 1.6 | ✅ done | 2026-09-04 (this doc) | — |
| Phase 2 PR-A1..A7 | ⏳ pending | — | needs WP-A start |
| Phase 3 PR-B1..B3 | ⏳ pending | — | needs WP-C/D finish |
| Phase 4 | ⏳ pending | — | needs Reviewer 2 alloc |
| Phase 5 | ⏳ pending | — | needs Phase 4 done |

## 9. Reference / Provenance chain

| Doc | Generated when | Status |
|-----|----------------|--------|
| `RC_GA_TRIAGE_AND_GATE_PLAN_2026-09-03.md` | 2026-09-03 13:45 CST (by codex-cli) | AS-IS |
| `V313-ROUND24-EVIDENCE-MANIFEST.md` | 2026-08-15 05:00 UTC (by openclaw-minimax) | AS-IS — needs provenance refresh |
| `CLAIM_DOWNGRADE_MANIFEST.md` | 2026-09-04 01:30 CST | ✅ landed in c5e4a4d2bd |
| This doc | 2026-09-04 01:30 CST | ⏳ awaiting commit |

---

*Path B is the user's selected 10–14 day plan. Path A (5–7 day minimum) is contained as
Phases 1–2–5; Path B adds Phases 3 (v3.13 P1) and 4 (Reviewer 2 + sign-off).*
