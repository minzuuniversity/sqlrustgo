# Tag Protection Policy — v4.0.0 GA

> **Date**: 2026-09-20
> **Owner**: openclaw
> **Applies to**: `refs/tags/ga/v4.0.0`, `refs/tags/rc/v4.0.0`, `refs/tags/beta/v4.0.0`
> **Refs**: docs/governance/BRANCH_GOVERNANCE.md, docs/releases/v4.0.0/FORCE_PUSH_AUDIT_2026-09-19.md

## 1. Problem

Gitea's REST API does **not** expose tag-protection rules. The
`/api/v1/repos/{owner}/{repo}/branch_protections/` endpoint only
matches **branches**, not refs/tags/*.

This is a gap that allowed 5 force-pushes to `ga/v4.0.0` within 30 minutes
on 2026-09-19 (see FORCE_PUSH_AUDIT_2026-09-19.md). While no content
was lost, the workflow violated tag immutability contracts.

## 2. Mitigation Strategy

### 2.1 Branch Pinning (Implemented 2026-09-20)

Create a **protected branch** that mirrors the GA tag content, so any
code change requires a PR:

```bash
git checkout -b release/v4.0.0 <ga/v4.0.0 tag SHA>
git push gitea250 release/v4.0.0
git push gitea252 release/v4.0.0
git push gitee   release/v4.0.0
```

Then apply `develop/v4.0.0`-equivalent protection:

| Rule | Value | Effect |
|------|-------|--------|
| `enable_push` | `false` | Direct push rejected |
| `enable_force_push` | `false` | Force push rejected |
| `block_admin_merge_override` | `true` | Admins cannot bypass |
| `required_approvals` | `1` | 1 reviewer required |
| `block_on_rejected_reviews` | `true` | Block merge on rejection |
| `block_on_outdated_branch` | `true` | Branch must be up-to-date |

**Effect**: Any change to v4.0.0 must go through a PR. Tag is now
implicitly frozen because the branch is frozen.

### 2.2 Tag Verification Cron (Future Work)

Add a daily CI job that asserts:
```bash
EXPECTED=$(git rev-parse release/v4.0.0)
ACTUAL=$(git rev-parse ga/v4.0.0^{commit})
[ "$EXPECTED" = "$ACTUAL" ] || alert "tag/branch drift detected"
```

### 2.3 Tag Force-Push Detection (Future Work)

Wrap `git tag --force` in a script that:
- Refuses unless `--force-ok` flag is set
- Requires a justification comment
- Logs to audit trail

## 3. Active Tag State (2026-09-20)

| Tag | Commit | Stable? |
|-----|--------|---------|
| `beta/v4.0.0` | `917fd83e3f` | ✅ (no force-push since 2026-09-19) |
| `rc/v4.0.0` | `c9bea8dcc2` | ✅ |
| `ga/v4.0.0` | `2ce8f28b41` (5 force-pushes) | ⚠️ frozen via release/v4.0.0 branch |

## 4. v4.0.1 Process Improvements

1. **Adopt a "single canonical tag" rule**: After GA cut, no force-push
   to `ga/v*.*` tags. Any post-GA fix is a new `ga/v*.*-<patch>` tag.
2. **Add `git push --tag` confirmation script** to detect accidental
   force-pushes
3. **CI assertion** that `ga/v*.*^{commit}` matches `release/v*.*`
4. **Gitea server plugin** (if available) to add tag protection rules
5. **Pre-release** to use `release/v*.*` branch for accumulation, then
   tag at GA cut (only one tag operation)

## 5. References

- docs/governance/BRANCH_GOVERNANCE.md
- docs/governance/BRANCH_PROTECTION_v4.0.0.md
- docs/releases/v4.0.0/FORCE_PUSH_AUDIT_2026-09-19.md
- docs/releases/v4.0.0/SYNC_AUDIT_2026-09-19.md