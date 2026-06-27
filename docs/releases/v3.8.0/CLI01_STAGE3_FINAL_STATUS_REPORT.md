# v3.8.0-rc1 CLI-01 Stage 3 + Stage 2 v2 Final Report

**Date**: 2026-06-04
**Author**: openclaw
**Status**: ⚠️ **Gitea Object Corruption - admin intervention required**

## 1. Summary

This session produced **two** local commits for v3.8.0-rc1 release
closure:

| Commit | Branch | Description | Status |
|--------|--------|-------------|--------|
| `624fccdc1` | `fix/v380-rc1-corpus-tpch-stage2-v2` | Stage 2 v2: Corpus 85.8% → 90.3% + TPCH-01 Q1 PASS | ⚠️ Blocked |
| `651200d9` | `fix/v380-rc1-cli3-file-storage` | Stage 3: REPL cross-session persistence (`--init-sql`, `--save-on-exit`) | ⚠️ Blocked |

Both are **local-only**. Gitea remote has **Gitea server-side object
corruption** (empty object file for commit 624fccdc1c97) that causes
all pushes to fail with "missing necessary objects" / "bad object
refs/heads/fix/v380-rc1-corpus-tpch-stage2-v2" — this is a server-side
issue, not a client-side issue.

## 2. What Was Completed (Local)

### 2.1 Stage 2 v2 — Corpus 85.8% → 90.3% (commit 624fccdc1)

- **SETUP blocks** for 3 SQL files (+23 cases):
  - `self_join.sql` 2/15 → 15/15
  - `outer_join.sql` 13/19 → 18/19
  - `join_combinations.sql` 11/19 → 16/19
- **Parser fixes** (+9 cases):
  - `Token::If` as scalar function (2 places)
  - `POSITION(x IN y)` special form (uses `parse_primary_expression`
    to avoid `parse_comparison_expression`'s IN-list greedy match)
- **TPCH-01 Q1 verified PASS** at SF=0.1 (88.6s release build, in-process
  test_tpch_sf01_gate). 22/22 deferred to rc2 (40h work).
- 79/79 regression PASS, 0 failures

### 2.2 Stage 3 — CLI-01 Cross-Session Persistence (commit 651200d9)

- **`repl --init-sql <file>`** — replay SQL file at REPL startup
  (CREATE TABLE + INSERT statements). End-to-end cross-session test
  verified: session 1 dump → session 2 replay + own INSERT → SELECT 3 rows.
- **`repl --save-on-exit <file>`** — wired hook (writes header-only
  placeholder; full catalog dump needs `MemoryExecutionEngine::list_tables()`
  which is Stage 4 work).
- **`.source <file>`** refactored to use the same replay path. Previously
  `.source` used `exec_one` (new engine per statement) which silently
  dropped state between statements — a real correctness bug.
- 86/86 regression PASS, 0 failures (7 new tests in
  `tests/cli03_persistence_test.rs`)

## 3. Gitea Push Status (Blocker)

### 3.1 Symptom

```
$ git push origin fix/v380-rc1-cli3-file-storage
remote: fatal: bad object refs/heads/fix/v380-rc1-corpus-tpch-stage2-v2
 ! [remote rejected]   ... (missing necessary objects)
```

### 3.2 Root cause (server-side)

Gitea's pre-receive hook validates **all** refs, not just the one being
pushed. The `fix/v380-rc1-corpus-tpch-stage2-v2` ref points to commit
`624fccdc1c97eb8e3d6f88f6cb3b4df45549cdb2` whose object file on the
Gitea server is **empty (0 bytes)**:

```
remote: error: object file /data/git/repositories/openclaw/sqlrustgo.git/objects/62/4fccdc1c97eb8e3d6f88f6cb3b4df45549cdb2 is empty
```

This was caused by a previous push during the Gitea Z6G4 outage: Gitea
wrote the ref into its ref database but the underlying object file was
truncated/never fully flushed. The Gitea database knows the ref exists
(ref check returns 404 because the Gitea API lookup walks the ref
log), but git itself can't dereference the SHA.

### 3.3 Why this is server-side, not client-side

- The local repo has the full commit 624fccdc1 (`git cat-file -p` works locally).
- The remote API lists 18 other `fix/v380-*` branches successfully.
- The remote API can read all other commits (e.g. `dd7d7fb0`).
- Only `624fccdc1` is corrupted on the server.

### 3.4 What the Gitea admin needs to do

```bash
# On the Gitea server (Z6G4):
ssh gitea-macmini
cd /data/git/repositories/openclaw/sqlrustgo.git
# Remove the empty object file:
rm -f objects/62/4fccdc1c97eb8e3d6f88f6cb3b4df45549cdb2
# Remove the dangling ref:
git update-ref -d refs/heads/fix/v380-rc1-corpus-tpch-stage2-v2
# Run gc:
git gc --prune=now --aggressive
# Restart gitea if needed:
systemctl restart gitea
```

After this, both `624fccdc1` and `651200d9` should be push-able as
fresh branches.

## 4. Workaround Attempted

- ✗ Renaming the branch (`fix/v380-rc1-cli3-file-storage-v2`) — Gitea
  hook still checks all refs
- ✗ `git push --force` — same hook check
- ✗ Pushing individual commits by SHA — same hook check
- ✗ Gitea API `POST /git/refs` — returns success but verify shows
  "not found" (Gitea API writes are also broken / inconsistent)
- ✗ Gitea API `DELETE` of the corrupt ref — returns success but
  push still fails (delete didn't take effect)

## 5. Deliverables (Local, ready when Gitea admin fixes fs)

### Branch 1: `fix/v380-rc1-corpus-tpch-stage2-v2`

```
commit 624fccdc1c97eb8e3d6f88f6cb3b4df45549cdb2
Author: openclaw
v3.8.0-rc1 Stage 2 v2: Corpus 85.8% → 90.3% (post develop sync)
```

Files: 5 files, +197 / -3

### Branch 2: `fix/v380-rc1-cli3-file-storage`

```
commit 651200d979af89d634d455260f6aead4934d82b3
Author: openclaw
v3.8.0-rc1 CLI-01 Stage 3: REPL cross-session persistence via --init-sql
```

Files: 3 files, +500 / -16

## 6. Next Steps (when Gitea is fixed)

1. Push the two local branches.
2. Create PRs and merge in this order:
   - PR-A: `fix/v380-rc1-corpus-tpch-stage2-v2` (Stage 2 v2 corpus)
   - PR-B: `fix/v380-rc1-cli3-file-storage` (Stage 3 persistence)
3. PR-3058 (corpus v1) is still open, mergeable=false. Close it as
   "superseded by PR-A" or leave open.
4. Continue with next stage of v3.8.0-rc1 release-closure work.

## 7. Lessons Learned

1. **Gitea pre-receive hooks are not isolated**: A corrupt object on
   one branch blocks all other branches' pushes. Future writes should
   use lightweight tags or stash differently to avoid putting all
   eggs in one corrupted basket.
2. **Gitea "Please try again later" 限速** can be > 5 minutes when
   the server is degraded. Don't rely on rapid retry.
3. **Gitea API silent failures are a real footgun**: `POST /git/refs`
   returns 200 but doesn't actually create the ref. Always verify
   after writing.
4. **CLI REPL patterns** (cross-session persistence via SQL replay)
   is a clean architecture: avoid the generic `ExecutionEngine<S>`
   plumbing and instead use the SQL surface as the serialization
   format. The replay path is the same as the interactive path,
   so there's no divergence to maintain.

