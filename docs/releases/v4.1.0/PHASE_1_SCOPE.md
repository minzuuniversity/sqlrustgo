# v4.1.0 — PHASE_1_SCOPE

> **Date**: 2026-09-29
> **Status**: DRAFT (per `docs/releases/v4.1.0/STAGE.yaml`)
> **Purpose**: Consolidated work plan for v4.1.0 PHASE 1 (DRAFT → ALPHA)

## 1. Scope overview

PHASE 1 of v4.1.0 is the work required to advance from DRAFT to ALPHA.
Per STAGE.yaml exit_criteria.draft_to_alpha, the gates are:

1. PHASE_0 docs published (DONE 2026-09-29 — this set of 9 docs)
2. First alpha tag cut: `v4.1.0-alpha1`

The blocker on #2 is the 3 inherited v4.0.0 alpha-gate FAILs. The
inherited blockers are:

| Gate | Inherited FAIL |
|---|---|
| `scripts/gate/check_anti_ignore_gate.sh` | `tests/baseline/ignore_registry.json` missing |
| `scripts/gate/check_arch_invariants.sh` | `crates/executor/src/execution_engine.rs` 2731 lines > 1600 limit |
| `scripts/gate/check_anti_fabrication.sh` | HEAD author email `v400@local` not in allow list |

## 2. Workstreams

### 2.1 [P0] Resolve 3 inherited v4.0.0 alpha-gate FAILs

#### 2.1.1 `check_anti_ignore_gate.sh` — create `tests/baseline/ignore_registry.json`

- Create `tests/baseline/` directory (does not currently exist)
- Create `tests/baseline/ignore_registry.json` with current `#[ignore]` markers as the SSOT
- Verify the gate accepts the new file
- Owner: openclaw / governance
- Estimated: 1 hour
- Blocked by: nothing

#### 2.1.2 `check_arch_invariants.sh` — split `execution_engine.rs`

- Current: `crates/executor/src/execution_engine.rs` 2731 lines
- Limit: 1600 lines
- Gap: ~1100 lines need to be split out

Strategy A: Extract `expr/` subdirectory contents (function operators, type coercion, column expressions) into separate modules:

- `crates/executor/src/expr/mod.rs` (existing, may stay)
- `crates/executor/src/expression_engine/core.rs` (extracted)
- `crates/executor/src/expression_engine/select.rs` (extracted)
- `crates/executor/src/expression_engine/dml.rs` (extracted)

Owner: openclaw / executor maintainer
Estimated: 2-3 days (large refactor)
Blocked by: nothing
Risk: high — could break existing tests if not careful

Strategy B (safer): Find existing split-able boundaries; defer the full refactor to v4.1.0-rc1

#### 2.1.3 `check_anti_fabrication.sh` — HEAD author email fix

- Current: HEAD author email `v400@local` (used by `feat/v4.0.0-wal-group-commit` commits)
- Required: email in allow list (per the gate's policy)
- Action: Add `v400@local` to the allow list in the gate script, OR
- Action: Rewrite the commit history to use `openheart@gaoyuanyiyao.com` (the listed author in CLAIM_DOWNGRADE_MANIFEST)

Owner: openclaw / governance
Estimated: 30 minutes (allow list) OR history rewrite (large, deferred)
Blocked by: nothing for the easy fix

### 2.2 [P1] Migrate WP-C..G deferred items into v4.1.0 scope

Per `docs/releases/v4.0.0/WP_LEGACY_TRIAGE.md` and `CLAIM_DOWNGRADE_MANIFEST.md §2`, 5/7 WP items deferred from v4.0.0 to v4.0.1. These should migrate to v4.1.0 scope:

- WP-C DDL/integrity: #4682 #4652 #4672 #4669 #4709 #4703
- WP-D joins: #4668 #4656 #4649 #4636
- WP-F schema: #4848
- WP-G type/comparison: #4846
- (WP-H #4639: explicitly v4.1-targeted per the WP-H triage)

Estimated: 6-8 weeks total (1-2 weeks per WP category)
Blocked by: 2.1.2 (execution_engine.rs split is needed for WP-G #4846 type/comparison work)

### 2.3 [P1] V400-09 168h SOAK FINAL_REPORT

- Kickoff: 2026-09-19 (`docs/releases/v4.0.0/SOAK_168H_KICKOFF_2026-09-19.md`)
- Duration target: 168h = 7 days
- Expected completion: 2026-09-26
- Current status as of 2026-09-29: no FINAL_REPORT exists
- Action: Verify SOAK process status; if completed, write the FINAL_REPORT; if still running, document elapsed progress
- Owner: openclaw / CI / Z6G4
- Estimated: 1 day to write FINAL_REPORT if data exists
- Blocked by: physical 168h run completion

### 2.4 [P2] v4.0.0 STAGE.yaml SSOT governance decision

The v4.0.0 self-claimed GA promotion did not advance the STAGE.yaml
SSOT to "GA". This is a governance gap inherited by v4.1.0.

Decision options:

- (A) Backfill the STAGE_CONFIG gate flow on v4.0.0 (run alpha / beta / rc / ga gates in order, advance current_stage to "GA")
- (B) Roll back v4.0.0 self-claim (revert main to v3.12.0 GA, treat v4.0.0 as continuing DRAFT)

Requires human architect decision. Not a code task.

### 2.5 [P2] v4.0.0 main落后 resolution

`gitea252/main` (`a8dba8d31e`) is落后 `develop/v4.0.0` (`f350eb13a5`) by 16902 commits.

Action: Decide whether main should be fast-forwarded to develop/v4.0.0
(which would close the gap), or kept as the v4.0.0 GA gate doc snapshot
(which preserves the historical record of the GA promotion event).

Owner: governance / openclaw
Estimated: 1 day (mechanical) + governance review

## 3. Sequencing

```
[P0] 2.1.1 (1h) ─────┐
[P0] 2.1.3 (30min) ──┼──→ [ALPHA ready] ──→ v4.1.0-alpha1 tag
[P0] 2.1.2 (2-3d) ──┘
                          │
                          ↓
[P1] 2.2 (6-8w) ────→ [BETA ready]
[P1] 2.3 (1d) ──────→ [BETA ready]
                          │
                          ↓
[P2] 2.4 (governance)
[P2] 2.5 (1d + review)
                          │
                          ↓
                      [GA ready]
```

## 4. References

- `docs/releases/v4.1.0/STAGE.yaml` — stage state + exit criteria
- `docs/releases/v4.0.0/WP_LEGACY_TRIAGE.md` — WP-A..H source
- `docs/releases/v4.0.0/CLAIM_DOWNGRADE_MANIFEST.md` — deferred items
- `docs/releases/v4.0.0/SOAK_168H_KICKOFF_2026-09-19.md` — SOAK kickoff
- `docs/releases/v4.0.0/STAGE.yaml` — v4.0.0 inherited stage state