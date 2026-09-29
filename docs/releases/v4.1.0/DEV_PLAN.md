# v4.1.0 — DEV_PLAN

> **Date**: 2026-09-29
> **Status**: DRAFT (per `docs/releases/v4.1.0/STAGE.yaml`)
> **Inherits**: `docs/releases/v4.0.0/DEV_PLAN.md` (canonical for shared dev workflow)

## 1. Scope

v4.1.0 development workflow is identical to v4.0.0. v4.1.0-specific
work is limited to three areas:

### 1.1 Real bugfix carry-forward (largely DONE on develop/v4.1.0 HEAD)

- V400-05/06/07 cross-model transaction + AuditChain ALCOA+
- zombie-fix core (bulk-insert + DLM repair)
- workers.push wrapper restore
- DML/storage regression test fixes

These have all landed on develop/v4.1.0 by 2026-09-21. See
`docs/releases/v4.1.0/CHANGELOG.md` for the full commit list.

### 1.2 5-remote sync tooling (DONE on develop/v4.1.0 HEAD)

- scripts/sync/5remotes_sync.sh — push to 5 remotes with Gitea
  protected-branch bypass via SSH container update-ref
- scripts/sync/5remotes_drift_check.sh — read-only TSV output,
  cron-friendly
- scripts/sync/README.md — operator documentation

### 1.3 Pending work (open_issues per STAGE.yaml)

- 5/7 WP-C..G legacy issues deferred from v4.0.0 → migrate to v4.1.0 scope
- V400-09 168h SOAK FINAL_REPORT — kickoff 2026-09-19, no FINAL as of 2026-09-29
- 3 v4.0.0 alpha gate FAILs (arch_invariants / anti_fabrication / anti_ignore_gate) — must resolve before v4.1.0 ALPHA

## 2. Branch topology

| Branch | Purpose | Status |
|---|---|---|
| develop/v4.1.0 | Active dev trunk | open |
| develop/v4.0.0 | Frozen post-GA reference | PUSH_PROTECTED, read-only |
| release/v4.0.0 | Pins ga/v4.0.0 tag | PUSH_PROTECTED |
| main | Aggregated release line | contains v3.12.0 GA + v4.0.0 gate docs (no v4.0.0 implementation code; 16902-commit gap with develop/v4.0.0) |

## 3. PR flow

Per `docs/governance/BRANCH_PROTECTION_v4.0.0.md`:

1. Branch from develop/v4.1.0 as `feature/v4.1.0-*` or `fix/v4.1.0-*`
2. Open PR to develop/v4.1.0
3. After 2 approvals + green CI: merge
4. After merge: `scripts/sync/5remotes_sync.sh develop/v4.1.0` to push to 5 remotes
5. Track drift via `scripts/sync/5remotes_drift_check.sh` (cron)

## 4. References

- `docs/releases/v4.0.0/DEV_PLAN.md` — full development workflow
- `docs/governance/BRANCH_PROTECTION_v4.0.0.md` — branch protection rules
- `docs/governance/STAGE_CONFIG.yaml` — stage framework SSOT
- `scripts/sync/README.md` — 5-remote sync tooling