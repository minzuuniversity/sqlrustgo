# 5-Remote Sync Tools

Tools for the SQLRustGo 5-remote topology:

| Remote   | URL                                               | Push policy |
|----------|---------------------------------------------------|-------------|
| gitcode  | gitcode.com/BreavHeart/sqlrustgo                  | unprotected (force-push ok) |
| gitea250 | http://192.168.0.250:3000/openclaw/sqlrustgo      | protected (force-push blocked) |
| gitea252 | http://192.168.0.252:3000/openclaw/sqlrustgo      | protected (force-push blocked) |
| gitee    | gitee.com/yinglichina/sqlrustgo                   | unprotected (force-push ok) |
| github   | github.com/yinglichina8848/sqlrustgo-public       | unprotected (force-push ok) |

The two protected Gitea remotes enforce `enable_push: false`,
`enable_force_push: false`, and `block_admin_merge_override: true`. To
advance a protected branch we SSH into the container that hosts the
repo and run `git update-ref` directly, after first pushing the commit
to a temporary ref so the commit objects land in the container's
object pool.

## Scripts

### `5remotes_sync.sh` — bring all 5 remotes to a given tip

```bash
scripts/sync/5remotes_sync.sh main                # sync main from gitea252
scripts/sync/5remotes_sync.sh develop/v4.1.0 gitea252  # sync develop/v4.1.0 from gitea252
scripts/sync/5remotes_sync.sh --all              # sync the 3 core branches: develop/v4.1.0, main, release/v4.0.0
```

Steps:
1. Resolve source SHA from the source remote (default `gitea252`)
2. Fetch all remotes
3. Push to gitcode / gitee / github (`git push --force-with-lease`)
4. For gitea250 / gitea252: push to a temporary ref, then SSH into the
   container and `git update-ref`
5. Verify all 10 pairwise `rev-list --count` return 0/0

Exit codes:
- 0: converged
- 1: drift detected (printed, nothing pushed)
- 2: protected-branch update failed
- 3: network/SSH error
- 4: argument error

### `5remotes_drift_check.sh` — read-only health check

```bash
scripts/sync/5remotes_drift_check.sh                                    # default branches
scripts/sync/5remotes_drift_check.sh --branches develop/v4.1.0,main     # subset
scripts/sync/5remotes_drift_check.sh --alert-threshold 5                # custom threshold
```

Output is tab-separated for log scraping:
```
branch               pair_a      pair_b      ahead    behind
develop/v4.1.0       gitcode     gitea250    0        0
develop/v4.1.0       gitcode     gitea252    0        0
...
```

Exit codes:
- 0: all branches within threshold
- 1: drift exceeds threshold
- 2: network error

### `verify_merge_ref.sh` — assert a merge actually landed on its base ref

```bash
scripts/sync/verify_merge_ref.sh --pr 5080       # verify an existing merged PR
scripts/sync/verify_merge_ref.sh --selftest      # construct a merge, assert, clean up
```

Gitea can return HTTP 200 + `merged=True` from the merge API while
`refs/heads/<base>` never moves (issue #5076, observed with PR #5075 on
gitea252, 2026-10-07). Anything trusting `merged=True` alone gets a false
positive: the merge looks successful but no branch contains the commit.

This script closes that gap client-side (issue #5076 AC4). After a merge
it asserts `base ref == merge_commit_sha` (reported as `PASS-EXACT`) or,
if the tip moved on, that the merge commit is reachable from the base tip
(`PASS-LANDED`); unreachable is `FAIL-DRIFT` with the correction path.
`--selftest` builds a throwaway `selftest/*` branch pair, merges via the
API, asserts the AC4 property, and deletes everything on exit — a live
regression run that leaves only a closed test PR behind.

AC1/AC3 (server must move the ref / must not report success for a failed
merge) and AC2 (`PATCH /git/refs/heads/*`, currently HTTP 405) are Gitea
server-side behavior and cannot be fixed from this repo; the client-side
correction path remains `5remotes_sync.sh` Step 3 (temp ref → docker
`update-ref` → **`git fetch` the local ref** → sync). Skipping the fetch
makes the syncer re-push the stale local SHA and rolls the fix back.

Exit codes:
- 0: base ref contains merge_commit_sha (EXACT = strict AC4)
- 1: assertion failed (drift / not merged / selftest failure)
- 2: network or API error
- 4: argument error

Credentials: `GITEA_URL` / `GITEA_USER` / `GITEA_PASS` / `GITEA_REPO`
environment overrides (defaults match the sibling scripts).

## When to use

- After a PR merge on the "publishing" remote (gitea252) — run
  `5remotes_sync.sh --all` to push to the other 4 remotes.
- As a periodic cron health check — run `5remotes_drift_check.sh` and
  alert on non-zero exit.

## Known limitations

- The SSH update-ref path requires the operator to have `ssh` access
  to `z440@192.168.0.250` and `liying@192.168.0.252`. Without
  that, the gitea remotes cannot be advanced.
- The container IDs (`fd56a3da85f0` for gitea250,
  `fff98c53f6f5` for gitea252) are hard-coded. If the Gitea
  containers get recreated with different IDs, edit the
  `GITEA_HOSTS` array at the top of `5remotes_sync.sh`.
- `develop/v4.0.0` is intentionally NOT in the `--all` branch list
  because gitcode retains 5 gitcode-only commits on that branch
  (BINARY collation, Windows compat, etc.) that should not be lost.
  Sync it explicitly with `scripts/sync/5remotes_sync.sh develop/v4.0.0`.

## References

- `docs/governance/BRANCH_PROTECTION_v4.0.0.md` — protected branch policy
- `docs/releases/v4.0.0/SYNC_AUDIT_v4.1.0_2026-09-20.md` — original audit
- `docs/releases/v4.1.0/V400_TO_V410_REVIEW_QUEUE.md` — review queue closure