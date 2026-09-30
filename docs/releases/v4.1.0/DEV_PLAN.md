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

> **状态核实修正**（2026-09-30）：本节原列 3 项为 open。实跑复核后
> **2 项已解决、1 项仍未解决**。依据 `ALIGNMENT_AUDIT_2026-09-30.md` §7。

- ✅ **已解决** — V400-09 168h SOAK FINAL_REPORT 已存在
  (`docs/releases/v4.0.0/V400_09_168H_SOAK_FINAL_REPORT.md`)
- ✅ **已解决** — 3 个继承的 v4.0.0 alpha gate 阻断项全部清除并附实跑证据
  (`docs/releases/v4.1.0/evidence/gate-runs-2026-09-30/`；
  `check_anti_fabrication` 经 CHECK 4 白名单修复后复跑 exit 0)
- ⏳ **待实施** — WP-A（4 条，re-opened 2026-09-30，见 ISSUES_PLAN §4.0）
  + WP-C/D/F/G 共 4 个 WP 组 / 12 条 issue（原写「5/7」，计数不成立）
  自 v4.0.0 迁移至 v4.1.0；连同 WP-B 6 条 + WP-E #4626 + WP-H #4639，
  v4.1.0 backlog 合计 **24** 条，详见 `ISSUES_PLAN.md` §4.7
- ⏸ **待治理决策** — `v4.1.0-alpha1` tag 处置、v4.0.0 阶段裁定、
  本地 `main` 与 `develop/v4.0.0` 分叉处置

## 2. Branch topology

| Branch | Purpose | Status |
|---|---|---|
| develop/v4.1.0 | Active dev trunk | open |
| develop/v4.0.0 | Frozen post-GA reference | PUSH_PROTECTED, read-only |
| release/v4.0.0 | Pins ga/v4.0.0 tag | PUSH_PROTECTED |
| main | Aggregated release line | **DIVERGED** from develop/v4.0.0 (verified 2026-09-30: `main=f8a149b474`, `rev-list --count main..develop/v4.0.0 = 16905`, and `main` is NOT an ancestor of develop/v4.1.0). Supersedes the earlier "16902-commit gap" wording — the branches have forked, not merely fallen behind. Disposition pending governance decision; see `ALIGNMENT_AUDIT_2026-09-30.md` §6. |

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