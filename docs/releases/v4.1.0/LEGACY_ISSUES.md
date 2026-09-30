# v4.1.0 — LEGACY_ISSUES

> **Date**: 2026-09-29
> **Status**: DRAFT (per `docs/releases/v4.1.0/STAGE.yaml`)
> **Inherits**: `docs/releases/v4.0.0/LEGACY_ISSUES.md` (canonical legacy catalog for v4.x)

## 1. Purpose

v4.1.0 inherits v4.0.0's legacy issue catalog. This document records
any v4.1.0-specific legacy items introduced since the v4.0.0 GA gate.

## 2. Inherited legacy WP items (status from v4.0.0 CLAIM_DOWNGRADE_MANIFEST §2)

| WP | Issues | v4.0.0 decision | v4.1.0 action |
|---|---|---|---|
| WP-A | parser legacy #4708 #4696 #4710 #4720 | DONE | None (inherited DONE) |
| WP-B | types #4721 #4674 #4716 #4676 #4675 #4670 | partial | Inherited; v4.1.0 carry forward |
| WP-C | DDL/integrity #4682 #4652 #4672 #4669 #4709 #4703 | deferred | **MIGRATE to v4.1.0 scope** |
| WP-D | joins #4668 #4656 #4649 #4636 | deferred | **MIGRATE to v4.1.0 scope** |
| WP-E | transaction #4847 #4626 | partial via V400-05 | Carry forward |
| WP-F | schema #4848 | deferred | **MIGRATE to v4.1.0 scope** |
| WP-G | type/comparison #4846 | deferred | **MIGRATE to v4.1.0 scope** |
| WP-H | v3.13/defer #4717 #4707 #4701 #4692 #4699 #4688 #4671 #4639 | DONE (7/8) + #4639 to v4.1 | Own #4639 in v4.1.0 |

## 3. v4.1.0-specific legacy items

### 3.1 STAGE.yaml SSOT contradiction (inherited from v4.0.0)

v4.0.0 `docs/releases/v4.0.0/STAGE.yaml` declares `current_stage: "DRAFT"` (2026-09-08 entry, never advanced), but `docs/releases/v4.0.0/GA_GATE_REPORT.md` (2026-09-20) declares "v4.0.0 GA CONDITIONAL PASS". These are contradictory:

- Per `docs/governance/STAGE_CONFIG.yaml`, STAGE.yaml is the per-version SSOT
- v4.0.0's self-claimed GA promotion did not run the STAGE_CONFIG gate flow that would advance DRAFT → ALPHA → BETA → RC → GA
- v4.0.0's `main` branch (876a289251) is `develop/v4.1.0 HEAD`'s parent commit and contains only v3.12.0 GA promotion docs + v4.0.0 GA gate docs — no v4.0.0 implementation code

Decision: Either
- (a) Backfill STAGE_CONFIG gate flow on v4.0.0 (run alpha / beta / rc / ga gates in order, update current_stage to "GA"), OR
- (b) Roll back v4.0.0 self-claim (revert main to v3.12.0 GA, treat v4.0.0 as continuing DRAFT)

This is a governance decision; deferred to user / openclaw.

### 3.2 v4.0.0 main 分支状态：已分叉（非单纯落后）

> **核实修正**（2026-09-30）：本节原写「落后 16902 commits」。
> 实测结果如下，依据 `ALIGNMENT_AUDIT_2026-09-30.md` F-06 与
> `evidence/gate-runs-2026-09-30/git-state-snapshot.txt`（sha256:05ea86ecd6002e14）：

```
main           = f8a149b474
develop/v4.0.0 = ac3fa16afb
develop/v4.1.0 = e2355c0680
rev-list --count main..develop/v4.0.0 = 16905
main is ancestor of develop/v4.1.0     = NO
```

即：本地 `main` 落后 `develop/v4.0.0` **16905** 个提交（非 16902），
且 `main` **不是** `develop/v4.1.0` 的祖先分支 —— 两条线已分叉，
不是单纯的 fast-forward 落后关系。

`ISSUES_PLAN.md` §1.2 原记「16902-commit gap closed on main / DONE 2026-09-28」
与本节矛盾，已按实测改为 PARTIAL（`release/v4.0.0` 确已在 github 创建）。

Effect on v4.1.0: minimal — v4.1.0 development branch is the active trunk.
But for `main` to be a meaningful "latest release" pointer, the divergence
(16905 commits + fork) must be explicitly resolved or recorded.

### 3.3 5-remote sync protocol依赖 on SSH access

`scripts/sync/5remotes_sync.sh` requires SSH access to `z440@192.168.0.250` and `liying@192.168.0.252` for the protected-branch update-ref path. Without these, gitea250/gitea252 protected branches cannot be advanced via the sync tooling.

Fallback path: manual Gitea web admin force-reset, or wait for unprotected PR-based flow.

## 4. References

- `docs/releases/v4.0.0/LEGACY_ISSUES.md` — full legacy catalog
- `docs/releases/v4.0.0/WP_LEGACY_TRIAGE.md` — WP-A..H triage
- `docs/releases/v4.0.0/CLAIM_DOWNGRADE_MANIFEST.md` — GA caveat items
- `docs/governance/STAGE_CONFIG.yaml` — stage framework SSOT
- `scripts/sync/README.md` — sync tooling constraints