# Branch Protection Status — develop/v4.0.0

> **Date**: 2026-09-19
> **Branch**: `develop/v4.0.0`
> **Owner**: openclaw
> **Policy**: docs/governance/BRANCH_GOVERNANCE.md §2.1 + Anti-Fabrication-Policy-v1.0

## 1. Purpose

Enforce the strictest branch protection policy on `develop/v4.0.0`
to prevent direct push, force push, and admin-bypass — **all changes
must go through Pull Request review**. This is the gate that prevents
code changes from being silently lost or overwritten.

## 2. Active Policy

The following protection rules are applied to `develop/v4.0.0` on
**both** `gitea250` (192.168.0.250:3000) and `gitea252` (192.168.0.252:3000):

| Rule | Value | Effect |
|------|-------|--------|
| `enable_push` | `false` | Direct push rejected |
| `enable_force_push` | `false` | Force push rejected |
| `push_whitelist` | (empty) | No users/teams bypass push |
| `force_push_allowlist` | (empty) | No force push exceptions |
| `block_admin_merge_override` | **`true`** | **Admins cannot bypass** |
| `enable_bypass_allowlist` | `false` | No bypass exceptions |
| `required_approvals` | `1` | At least 1 reviewer |
| `block_on_rejected_reviews` | `true` | Rejected reviews block merge |
| `block_on_official_review_requests` | `true` | Official review requests enforced |
| `block_on_outdated_branch` | `true` | Branch must be up-to-date |
| `dismiss_stale_approvals` | `true` | Stale approvals dismissed on push |
| `require_signed_commits` | `false` | Signed commits not yet required (v4.0.1) |

## 3. Verification

To inspect current state:

```bash
curl -u "<user>:<token>" \
  "http://192.168.0.250:3000/api/v1/repos/openclaw/sqlrustgo/branch_protections/develop/v4.0.0"
```

Expected response fields:
- `enable_push: false`
- `enable_force_push: false`
- `block_admin_merge_override: true`
- `required_approvals: 1`

## 4. How to merge a change

```bash
# 1. Create a feature branch
git checkout -b feat/my-change

# 2. Make changes + commit
git commit -m "feat: my change"

# 3. Push feature branch (no protection on feat/*)
git push gitea250 feat/my-change

# 4. Open a PR via Gitea UI (or API)
curl -X POST -u "<user>:<token>" \
  "http://192.168.0.250:3000/api/v1/repos/openclaw/sqlrustgo/pulls" \
  -H "Content-Type: application/json" \
  -d '{
    "title": "feat: my change",
    "head": "feat/my-change",
    "base": "develop/v4.0.0",
    "body": "Description"
  }'

# 5. Wait for 1 approval + CI

# 6. Merge via PR (squash / merge / rebase per repo setting)
curl -X POST -u "<user>:<token>" \
  "http://192.168.0.250:3000/api/v1/repos/openclaw/sqlrustgo/pulls/<id>/merge" \
  -H "Content-Type: application/json" \
  -d '{"do":"merge"}'
```

## 5. Direct-push failure example

```bash
$ git push gitea250 develop/v4.0.0
remote: error: Not allowed to push to protected branch develop/v4.0.0
 ! [remote rejected] develop/v4.0.0 -> develop/v4.0.0 (pre-receive hook declined)
error: failed to push some refs to 'http://192.168.0.250:3000/openclaw/sqlrustgo.git'
```

## 6. Tags policy

Tags (`beta/v4.0.0`, `rc/v4.0.0`, `ga/v4.0.0`) are **not** branch-protected
and can be force-pushed by admins in emergencies. However, this should
be done via PR with `--force` only in exceptional cases (e.g. sealing
a release after a failed CI run).

## 7. Future expansion (v4.0.1)

- `enable_status_check: true` with `status_check_contexts: ["ci/v400-gate"]`
  to require the v400 gate script to pass before merge
- `require_signed_commits: true` for GPG-signed commits
- Extend protection to `main`, `release/v4.0.0`, `ga/v4.0.0` after GA cut

## 8. References

- docs/governance/BRANCH_GOVERNANCE.md (full policy)
- Gitea API: `/api/v1/repos/{owner}/{repo}/branch_protections/{branch}`
- Issue: #V400-PROTECT-1 (this protection policy)
- Date applied: 2026-09-19