# v4.0.0 → v4.1.0 Review Queue

> **Date**: 2026-09-23
> **Status**: OPEN — 4 commits pending manual PR review
> **Decision source**: User opt-in (not auto-resolvable by sync tooling)

## 1. Background

During the 5-remote `develop/v4.1.0` sync convergence on 2026-09-20 →
2026-09-23, the sync tooling brought 3 v4.0.0 doc-only commits into the
v4.1.0 graph (via empty-tree merge commits: `fca71cb525`,
`a65497e276`, `7e30fbc0cb`). Four additional v4.0.0 commits
**could not be auto-merged** because they have real code conflicts with
the v4.1.0 zombie-fix / workers.push / dml-storage-regressions commits
already landed on `develop/v4.1.0`.

These four commits are all **gitcode-only** — they exist on
`gitcode/develop/v4.0.0` but **not on any other remote's `develop/v4.0.0`**.
The other four remotes (`gitea250`, `gitea252`, `gitee`, `github`) have
`develop/v4.0.0` tips that predate these four commits.

```
gitcode    develop/v4.0.0 = 615afff025   ← ahead, contains the 4 commits
gitea250   develop/v4.0.0 = 1d2588d937
gitea252   develop/v4.0.0 = f350eb13a5
gitee      develop/v4.0.0 = 38566af0f8
github     develop/v4.0.0 = 6d504d1b3c
```

## 2. Pending commits (in gitcode-only v4.0.0 history)

| SHA | Author | Date | Title |
|---|---|---|---|
| `34ed5b68c8` | hermes-z6g4 | 2026-09-17 03:32 +0800 | fix(v4.0.0 alpha gate): executor BINARY collation, admin Windows compat, remove tpch_hash_test |
| `61b198f469` | hermes-z6g4 | 2026-09-17 04:43 +0800 | fix(storage): checkpoint JSON escapes Windows paths; recovery tolerates unknown prefix as NULL |
| `615afff025` | hermes-z6g4 | 2026-09-17 15:22 +0800 | fix: cross-platform /proc and filename compatibility for Windows |
| `ec1278f72c` | hermes-z6g4 | 2026-09-17 03:45 +0800 | merge gitee/develop/v4.0.0 into local develop/v4.0.0 |

## 3. Conflict summary

When auto-merging `gitcode/develop/v4.0.0` into `7e30fbc0cb` (the
current v4.1.0 tip), three files have real code conflicts:

### 3.1 `crates/executor/src/expr/mod.rs`
- **HEAD (v4.1.0)**: BINARY collation by default — `eq_cross` does
  byte-for-byte `Text == Text` comparison.
- **`gitcode/develop/v4.0.0` (`34ed5b68c8`)**: PAD SPACE semantics for
  `CHAR(n)` columns — trim trailing whitespace on both sides before
  compare.
- **Conflict nature**: SQL semantic divergence. Resolving this changes
  observable SQL behavior (e.g. `'U1' = 'U1        '`).

### 3.2 `crates/mysql-server/src/lib.rs`
- **HEAD (v4.1.0)**: `list_threads()` uses Linux `/proc/self/task` with
  runtime hint + `max(1, n)` fallback.
- **`gitcode/develop/v4.0.0` (`34ed5b68c8`, `615afff025`)**: uses
  `cfg(windows)` block for the Windows fallback, plus `connection_tracker`
  drops a `Default` impl.
- **Conflict nature**: thread counting differs on non-Linux platforms.

### 3.3 `crates/storage/src/recovery_engine.rs`
- **HEAD (v4.1.0)**: includes `log::debug!` for unknown value prefix
  substitution during partial record recovery.
- **`gitcode/develop/v4.0.0` (`61b198f469`)**: same recovery logic but
  drops the `log::debug!` call.
- **Conflict nature**: log verbosity difference only; logic equivalent.

## 4. `ec1278f72c` — merge commit dependency

`ec1278f72c` is a merge commit on `gitcode/develop/v4.0.0` whose first
parent is `34ed5b68c8`. Therefore it cannot be cherry-picked
independently — it inherits the conflicts above. Resolution:

- Once `34ed5b68c8` is merged into `develop/v4.1.0`, `ec1278f72c`
  becomes reachable automatically (follow-up merge of `gitcode/develop/v4.0.0`
  into `develop/v4.1.0`).

## 5. Resolution procedure

For each of the three conflict files, the resolver needs to decide:

1. Keep v4.1.0's zombie-fix semantics as authoritative
   (chosen during the 2026-09-22 sync).
2. Or accept gitcode/v4.0.0's version and adapt the surrounding code.

Recommended workflow:

```bash
# In a fresh worktree
git worktree add /tmp/wt-review 7e30fbc0cb
cd /tmp/wt-review
git cherry-pick 34ed5b68c8    # resolve 3 conflicts
git cherry-pick 61b198f469    # resolve 1 conflict (log verbosity)
git cherry-pick 615afff025    # resolve 1 conflict (thread counting)
# ec1278f72c follows as a merge commit once 34ed5b68c8 lands
```

After manual resolution, the resulting branch becomes the next
`develop/v4.1.0` tip. Push via the same 5-remote sync procedure used
previously (force-push to unprotected remotes + SSH `git update-ref`
on protected Gitea remotes).

## 6. References

- `fca71cb525` / `a65497e276` / `7e30fbc0cb` — empty merges that brought
  doc-only v4.0.0 commits into v4.1.0 graph
- `d6dd4fab28` — v4.1.0 zombie-fix core merge
- `67b624cbd2` — v4.1.0 zombie-fix core cherry-pick (the commit that
  causes the PAD SPACE / BINARY conflict with `34ed5b68c8`)
- `b4d46e6e18` — workers.push fix
- `f3595e7361` — dml-storage regressions port
- `docs/releases/v4.0.0/SYNC_AUDIT_v4.1.0_2026-09-20.md` — 5-remote
  sync audit (predecessor document)
