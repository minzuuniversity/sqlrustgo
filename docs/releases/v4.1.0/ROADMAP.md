# v4.1.0 — ROADMAP

> **Date**: 2026-09-29
> **Status**: DRAFT (per `docs/releases/v4.1.0/STAGE.yaml`)
> **Inherits**: `docs/releases/v4.0.0/ROADMAP.md` (canonical for v4.x series milestones)

## 1. v4.1.0 milestones

| Milestone | Target | Status | Evidence |
|---|---|---|---|
| M-1: PHASE_0 docs scaffolded (this commit) | 2026-09-29 | DONE | docs/releases/v4.1.0/{STAGE.yaml, VERSION_PLAN.md, DEV_PLAN.md, ROADMAP.md, TEST_PLAN.md, ISSUES_PLAN.md, LEGACY_ISSUES.md, README.md, CHANGELOG.md} |
| M-2: Bugfix carry-forward merged | 2026-09-21 | DONE | commit d6dd4fab28 + ancestors on develop/v4.1.0 |
| M-3: Review queue closed | 2026-09-26 | DONE | commit 0bbb044da3 |
| M-4: 5-remote sync tooling | 2026-09-28 | DONE | scripts/sync/* + 3 sync rounds (4 → 5 endpoints → drift=0) |
| M-5: v4.0.0 main + release branches synced to 5 ends | 2026-09-28 | DONE | main=a8dba8d31e, release/v4.0.0=07178c9d66, develop/v4.0.0=f350eb13a5 (all 5 ends) |
| M-6: 3 v4.0.0 alpha gate FAILs resolved (inherited) | TBD | NOT STARTED | open_issue per STAGE.yaml |
| M-7: V400-09 168h SOAK FINAL | TBD | NOT STARTED | kickoff 2026-09-19, no FINAL as of 2026-09-29 |
| M-8: WP-C..G deferred items migrated to v4.1.0 scope | TBD | NOT STARTED | per docs/releases/v4.0.0/WP_LEGACY_TRIAGE.md |
| M-9: v4.1.0-alpha1 tag cut | TBD | blocked by M-6/M-7 | per STAGE.yaml exit_criteria |
| M-10: v4.1.0-beta1 / rc1 / ga | TBD | blocked by M-9 | per STAGE.yaml exit_criteria |

## 2. Cross-references

- v4.0.0 ROADMAP: `docs/releases/v4.0.0/ROADMAP.md` (covers V400-00..V400-10 + WP-A..H)
- v4.1.0 STAGE.yaml: `docs/releases/v4.1.0/STAGE.yaml`
- 5-remote sync status: `bash scripts/sync/5remotes_drift_check.sh`