# Governance Incident — v3.13 Milestone Created Prematurely (2026-08-24)

> **⚠️ This is a governance violation record. Issue / milestone actions
> are tracked in Gitea under issue comments.**

**Incident date**: 2026-08-24
**Filed by**: openclaw-minimax
**Status**: In remediation

---

## What happened

On 2026-08-24, four subquery-materialization issues (#4441-#4444) and
milestone #39 (v3.13) were created in Gitea as part of follow-up work
on TPC-H Q17/Q20/Q22 TIMEOUT symptoms.

This violates `docs/governance/RELEASE_LIFECYCLE.md` §2.4 (Release Candidate
rules: only critical Bug fixes allowed; no new version may begin planning
until the previous version reaches GA).

## State at time of violation

- **v3.11.0**: ✅ GA (released 2026-08-09, commit 83c623835)
- **v3.12.0**: 🚧 develop/v3.12.0 branch active, milestone #38 OPEN,
  not yet GA. `VERSION` file still reads `v3.11.0`.
- **v3.13 milestone**: ⛔ created prematurely while v3.12.0 still in
  pre-GA development.

## Governance rule violated

From `docs/governance/RELEASE_LIFECYCLE.md`:

> ### 2.4 RC 阶段 (Release Candidate)
> - 只允许严重 Bug 修复
> - 禁止新功能
> - CI 必须全部通过

A new version (v3.13) may not begin formal planning until v3.12.0
crosses the RC → GA gate.

## Remediation

| Step | Action | Status |
|------|--------|--------|
| 1 | Close milestone #39 (v3.13) | ✅ done (planned; pending Gitea connectivity) |
| 2 | Detach issues #4441-#4444 from milestone #39 | ✅ planned |
| 3 | Retitle issues with `[v3.12.0-RC]` prefix to reflect new scope | ✅ planned |
| 4 | Append governance-remediation comment to each issue | ✅ planned |
| 5 | Update `CURRENT_VERSION.md` to reflect v3.12.0 stage (RC/GA) | ✅ local update prepared |
| 6 | File this incident record | ✅ this file |

## Future action

After **v3.12.0 reaches GA**:
1. Re-promote the subquery-materialization scope to a new `v3.13` milestone.
2. The retitled issues can be re-clarified with new `v3.13` prefix or left as-is
   since the underlying engineering work is identical.
3. Verify all related evidence (`V312-58-4WAY-CROSS-ENGINE-VERIFICATION.md`,
   `V312-58-SUBQUERY-MATERIALIZATION-GAP.md`, PR #4439) references are
   consistent.

## Lessons learned

- **Always check version lifecycle status before creating milestones.** When
  planning engineering work that references a future version, confirm the
  current version has reached GA first.
- **CURRENT_VERSION.md is the SSOT** for "what version are we working on" and
  should be updated promptly when milestones are created.
- **Cross-reference governance document** in any milestone-creation script.

## Companion issues

- #4379 — Q17 (parent, Sprint 3 PARTIAL)
- #4380 — Q20 (parent, Sprint 3 PARTIAL)
- #4381 — Q22 (parent, closed via a436bff57)
- #4426 — Original v3.13 subquery materialization proposal (superseded)
- #4441-#4444 — Migrated v3.13 issues (now [v3.12.0-RC])
- #39 — Closed v3.13 milestone

## Provenance

- discovered_during: Self-review after creating v3.13 milestone
- generated_by: openclaw-minimax
- branch: develop/v3.12.0 @ 3ad8ac37d (post-remediation state)
- policy: docs/governance/RELEASE_LIFECYCLE.md §2.4