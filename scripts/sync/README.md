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
scripts/sync/5remotes_sync.sh --all              # sync the 4 core branches: develop/v4.1.0, main, release/v4.0.0, base/v4.1.0-frozen
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

#### Environment

| Variable | Required | Default | Meaning |
|---|---|---|---|
| `GITEA_USER` | yes, for the protected-branch path | — | Gitea user for HTTP auth |
| `GITEA_TOKEN` | yes, for the protected-branch path | — | Gitea token or password for HTTP auth |
| `GITEA_HOSTS` | no | `192.168.0.250:liying 192.168.0.252:liying` | space-separated `host:ssh_user` list to sync |
| `GITEA_PUSH_URL` | no | derived from host + credentials | where the temporary ref is pushed |
| `GIT_SSH_COMMAND` | no | set by the script | caller wins; otherwise the script sets it from its own SSH options |

```bash
export GITEA_USER=openclaw
export GITEA_TOKEN="$(cat ~/.ssh/openclaw-gitea.PAT)"
scripts/sync/5remotes_sync.sh develop/v4.1.0
```

`GITEA_USER` / `GITEA_TOKEN` used to be hardcoded in the script. Those values
were **live** and are tracked as a credential leak in [#5381](https://192.168.0.252:3000/openclaw/sqlrustgo/issues/5381).
There is deliberately no silent fallback to a baked-in credential: the script
reports which variable to set and skips that host.

#### Credential rotation (the remaining #5381 action)

Removing the literal from the working tree does **not** revoke it: the value
lives in the git history of every branch, in every clone, and in every CI log
that captured a `git push` URL. Revocation has to happen **at the server**.

1. **Mint a new token.** In Gitea: *Settings → Applications → Generate Token*,
   with the scopes the sync path needs (`write:repository`, `write:user`).
   Save it outside the repo — e.g. `~/.ssh/openclaw-gitea.PAT`, mode `0600`.
2. **Install it for every consumer.** The scripts read the credential from
   `GITEA_TOKEN` first, then from the local git credential store
   (`gitea_credential_resolve` in `gitea_credential_common.sh`):

   ```bash
   export GITEA_USER=openclaw
   export GITEA_TOKEN="$(cat ~/.ssh/openclaw-gitea.PAT)"
   # and, so that non-interactive runs (cron, CI) find it too:
   printf 'protocol=https\nhost=192.168.0.252:3000\nusername=%s\npassword=%s\n\n' \
       "$GITEA_USER" "$GITEA_TOKEN" | git credential approve
   ```

3. **Revoke the old token.** In Gitea: *Settings → Applications* → delete the
   previous token. Until this step the old value still authenticates, and
   every clone of this repository still holds it.
4. **Verify the old value is dead** (expect `HTTP=401`):

   ```bash
   curl -s -o /dev/null -w "HTTP=%{http_code}\n" \
     -u "openclaw:$OLD_TOKEN" "http://192.168.0.252:3000/api/v1/user"
   ```

5. **Purge the history** (optional, destructive — owner decision). The value
   stays reachable in the git history of the affected branches regardless of
   step 3; `git filter-repo` / BFG plus a force-push removes it from the
   history. Do this only with the owner's go-ahead, and coordinate it with
   anyone holding a clone.

Steps 1–3 are the load-bearing ones: they are what actually revokes the
credential. Steps 4 and 5 are the evidence that it happened and the cleanup
of the copies.

#### SSH is independent of the caller's ssh-agent

`SSH_OPTS` in the script is the single source of SSH options, and every
`ssh` call plus git's own SSH channel go through it. The load-bearing
option is `IdentityAgent=none`:

> When `SSH_AUTH_SOCK` points at a dead agent — macOS `launchd` leaves the
> socket file behind after the agent exits — OpenSSH blocks on the agent
> round-trip and produces **no output at all**. `BatchMode=yes` only
> suppresses interactive password prompts and `IdentitiesOnly=yes` only
> limits which identities are offered; neither stops the agent round-trip.

Measured on the same machine, same network, same key:

| Command | Result |
|---|---|
| `ssh -T git@github.com` | hangs, no output |
| `ssh -o IdentityAgent=none -T git@github.com` | authenticates |
| `ssh -o IdentitiesOnly=yes -o IdentityAgent=none -T git@{github,gitee,gitcode}.com` | 3/3 authenticate |

Before this was fixed, the script hung for 900s in `discover_gitea_target`
and three public remotes were wrongly reported as "unreachable" — see
[#5380](https://192.168.0.252:3000/openclaw/sqlrustgo/issues/5380).

### `test_5remotes_sync_ssh.sh` — regression test for the SSH channel

```bash
bash scripts/sync/test_5remotes_sync_ssh.sh
```

Fully offline. The five remotes are local bare repositories, and `ssh` /
`curl` are stubs. The `ssh` stub is a **behavioural double**: it blocks
exactly like real `ssh` does when `SSH_AUTH_SOCK` points at a dead agent
and the arguments lack `IdentityAgent=none`. The script under test
therefore hangs — and the test fails — if the option is ever dropped.

```bash
bash scripts/sync/mutate_5remotes_sync_ssh.sh   # mutation check for the test itself
```

Three mutations (drop `IdentityAgent=none`, restore the whole-filesystem
`find /`, drop the `GIT_SSH_COMMAND` export) are all required to be
CAUGHT.

### `test_5remotes_sync_strict.sh` — regression test for **failure propagation**

```bash
bash scripts/sync/test_5remotes_sync_strict.sh
```

Fully offline (local bare repos as the five remotes; `ssh` / `curl` / `git`
are stubs). It asserts that **every** failure is reported and blocks the
run with a **non-zero** exit code — the anti-fabrication rule of this
repository ("a failure must block, it must not pass silently"):

| Case | What is injected | Expected |
|---|---|---|
| T1 | the SSH `update-ref` fails | exit ≠ 0, and the output names the reason |
| T2 | one remote does not answer | exit ≠ 0 (**not** reported as "consistent") |
| T3 | no usable credential | exit ≠ 0, and the output says the credential is missing |
| T4 | — | the credential (plain **and** base64) appears in **no** argv, and not in `.git/config` |
| T5 | the remote does **not** have the branch | the branch is **created** and advanced (exit 0) — "missing" is not "unknown" |

T5 is the case that separates "the remote does not have this branch" from
"I cannot tell" (#5520 §4.A.1). Against the pre-fix script the same test
reports `PASS=7 FAIL=4` — the credential is in `curl`'s argv, the missing
branch cannot be created, and the run still prints
"✓ all 10 pairs consistent".

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

**The `ahead` / `behind` columns always mean "measured against the server".**
They are computed between the tips reported by `git ls-remote`, never between
local remote-tracking refs. A remote that cannot be measured yields a **word**
in those columns instead of a number:

```
develop/v4.1.0       gitcode     gitee       UNREACHABLE   UNREACHABLE
develop/v4.1.0       gitcode     gitee       STALE-DATA    STALE-DATA
develop/v4.1.0       gitcode     gitee       NO-BRANCH     NO-BRANCH
```

Per-remote state (printed in the `--- remote state ---` block):

| state | meaning |
|---|---|
| `OK` | the server answered, the objects were fetched, the tip is known |
| `UNREACHABLE` | the server did not answer (`ls-remote` failed) ⇒ the tip is unknown |
| `STALE-DATA` | the server answered but the object view could not be confirmed (fetch failed) |

Why this matters (issue #5791, board row `DRIFT-STALE-1`): the previous
implementation read `git rev-parse $remote/$branch` — the **local** tracking
ref — after a `git fetch --all … | tail -3` whose exit code is *tail's*, so a
failed fetch was invisible and the report printed that remote's numbers from a
stale ref. Measured consequence: gitcode had been pushed to the 252 tip
(proved by `ls-remote`) while the report said `gitcode gitea252 0 357` — the
opposite of the truth. The sibling `5remotes_sync.sh` had already learned this
("ask the server, never the cache", Step 4); this script now follows it.

Exit codes:
- 0: all branches within threshold AND `main` identical across remotes
- 1: drift exceeds threshold, OR `main` diverged, OR a remote does not carry a
  branch the others have
- 2: at least one remote could not be measured (`UNREACHABLE` / `STALE-DATA`) —
  "cannot tell" is not "in sync", so the run refuses to conclude

### `test_5remotes_drift_check.sh` — regression test for the drift checker's **measurement trustworthiness**

```bash
bash scripts/sync/test_5remotes_drift_check.sh
```

Fully offline: the five "remotes" are local bare repos and a stub `git`
injects fetch failures. Five arms, 15 assertions — a remote that cannot be
measured must yield a **word**, never a number, and the run must not exit 0:

| arm | construction | expectation |
|---|---|---|
| T1 | everything aligned | rc=0, every row is `0 0` |
| T2 | one remote pointed at an unreachable path | that row says `UNREACHABLE`, rc≠0 |
| T3 | the server answers but the fetch fails | `STALE-DATA`, no numbers on that remote's rows |
| T4 | remote already aligned, local ref stale, fetch fails | no reverse drift number, rc≠0 |
| T5 | fetch succeeds, local ref stale | numbers computed from the **server** tip (`0 0`) |

Reverse mutation: the same fixture against the pre-fix script fails 8
assertions (T4 prints `gitcode gitea252 0 1` — the opposite of the truth — and
mis-reports `main` as diverged).

Archived evidence: `audit/v4.1.0/runs/20261116-drift-stale-001/`
(`evidence_hash=sha256:b42ba26dbf4fb968d701daee90419c34797ddf587df904ed32a03902a836521b`).

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

### `claude_mirror_intake.sh` — 收口「远端只读分析」分支（镜像 → 252 → 全远端）

```bash
scripts/sync/claude_mirror_intake.sh --list                       # 镜像上有、252 上还没有的候选分支
scripts/sync/claude_mirror_intake.sh --branch <name> --dry-run    # 只探测，不写任何远端
scripts/sync/claude_mirror_intake.sh --branch <name> --land-only  # 只落到 252（不开 PR）
scripts/sync/claude_mirror_intake.sh --branch <name> --merge --sync   # 落地 → PR → 合并 → 回灌 → 校验
scripts/sync/claude_mirror_intake.sh --selftest                   # 离线自检（本地 bare 仓库，不触网）
```

闭环的形状：远端 agent 够不到 252，做完只读分析后把结果发布到 GitHub
镜像的一个文档分支；本脚本把该分支**读**下来落到 252，在 252 上开 PR，
合并后由 `5remotes_sync.sh` 回灌全部远端，最后由 `5remotes_drift_check.sh`
证明零漂移。评审与合并始终发生在真源侧，不在镜像侧。

刻意的边界：

- **不 force-push 任何远端。** 252 是受保护分支；镜像上 force 推送会抹掉别的
  远端独有的提交 —— 2026-11-26 就发生过：github 仓 `develop/v4.0.0` 上的 3 个
  dependabot 提交被 `5remotes_sync.sh` 的 `--force-with-lease` 覆盖掉（事后靠
  `recovered/develop-v4.0.0-pre-sync-20261126` 找回）。目标分支在 252 上已存在
  且 tip 不同时，本脚本**拒绝**并退出，由人工先对齐两个 tip。
- **不在镜像上开 PR、不改镜像内容**（镜像只读，`AGENTS.md` §3.2 / §8 #11）。
- **凭据不进 argv、不进 URL**：git 侧走 `git -c include.path=<0600 文件>`，
  curl 侧走 `curl -K <0600 文件>`，与 `5remotes_sync.sh` 同一份实现
  （`gitea_credential_common.sh`）。
- **合并后回读校验**：base ref 必须等于 `merge_commit_sha`（`PASS-EXACT`），
  或至少包含它（`PASS-LANDED`），否则 `FAIL-DRIFT` 并以非零码退出
  —— 复用 `verify_merge_ref.sh` 的 AC4 判据。

`--selftest` 是离线的：用两个本地 bare 仓库断言 ①`--dry-run` 不写目标远端、
②`--land-only` 落地的 tip 与镜像一致、③目标端 tip 分叉时**拒绝**且不覆盖、
④分支不存在时失败、⑤非法分支名退出码为 4。

Exit codes:
- 0: 成功（每个被要求的阶段都完成了）
- 1: 某一步失败（分支不存在 / 分叉 / 合并未落地 / 漂移未清零）
- 2: network / API / credential error
- 4: argument error

Credentials: `GITEA_URL` / `GITEA_REPO` environment overrides,
`GITEA_USER` / `GITEA_TOKEN`（也可由本机 git credential store 提供；
取不到时**明确失败**，不回落到写死的值 —— #5381）。

## When to use

- After a PR merge on the "publishing" remote (gitea252) — run
  `5remotes_sync.sh --all` to push to the other 4 remotes.
- When an agent that cannot reach gitea252 has published an analysis
  branch on the GitHub mirror — run `claude_mirror_intake.sh --list`,
  then `--branch <name> --merge --sync` to land it on 252 and bring all
  remotes back to one tip.
- As a periodic cron health check — run `5remotes_drift_check.sh` and
  alert on non-zero exit.

## Known limitations

- The SSH update-ref path requires the operator to have `ssh` access
  to `z440@192.168.0.250` and `liying@192.168.0.252`. Without
  that, the gitea remotes cannot be advanced.
- The container IDs and repository paths are **discovered on every run** —
  they are not hardcoded. `discover_gitea_target()` finds the container
  whose docker port mapping is `0.0.0.0:3000->3000/tcp` and then probes
  the layouts Gitea actually uses (`/git/*/`, `/data/git/repositories/*/`,
  `/data/git/gitea-repositories/*/`). A whole-filesystem `find /` used to
  be used for this and was measured taking more than 9 minutes on 250.
  A container restart therefore needs no edit; only a *new* layout would.
- `develop/v4.0.0` is intentionally NOT in the `--all` branch list
  because gitcode retains 5 gitcode-only commits on that branch
  (BINARY collation, Windows compat, etc.) that should not be lost.
  Sync it explicitly with `scripts/sync/5remotes_sync.sh develop/v4.0.0`.
- `base/v4.1.0-frozen` **is** in the `--all` branch list (added 2026-11-26).
  It is the frozen baseline ref the v4.1.0 readiness work is bound to
  (`BASELINE_FREEZE.json` → `source_ref`); it used to exist only on
  gitea252/gitee, so the github mirror never had it. It is an ancestor of
  `develop/v4.1.0`, so syncing it is a fast-forward everywhere.

## References

- `docs/governance/BRANCH_PROTECTION_v4.0.0.md` — protected branch policy
- `docs/releases/v4.0.0/SYNC_AUDIT_v4.1.0_2026-09-20.md` — original audit
- `docs/releases/v4.1.0/V400_TO_V410_REVIEW_QUEUE.md` — review queue closure