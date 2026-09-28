# v4.0.0 → v4.1.0 Review Queue

> **Date opened**: 2026-09-23
> **Date closed**: 2026-09-26
> **Status**: **CLOSED — RESOLVED**
> **Resolution merge commit**: `9c6767a512d9540127d5529807246e3dcea9ab15` on `develop/v4.1.0` (5-remote consistent)
> **Source branch consumed**: `sync/v410-v400-review-queue` (hermes-z6g4 + claude-macmini, 2026-09-23)
> **Branch deleted**: gitea252 `sync/v410-v400-review-queue` cleaned up 2026-09-26

## 1. Background

During the 5-remote `develop/v4.1.0` sync convergence on 2026-09-20 →
2026-09-23, the sync tooling brought 3 v4.0.0 doc-only commits into the
v4.1.0 graph (via empty-tree merge commits: `fca71cb525`,
`a65497e276`, `7e30fbc0cb`). Four additional v4.0.0 commits
**could not be auto-merged** because they have real code conflicts with
the v4.1.0 zombie-fix / workers.push / dml-storage-regressions commits
already landed on `develop/v4.1.0`.

These four commits were all **gitcode-only** — they existed on
`gitcode/develop/v4.0.0` but **not on any other remote's `develop/v4.0.0`**.
The other four remotes (`gitea250`, `gitea252`, `gitee`, `github`) had
`develop/v4.0.0` tips that predate these four commits.

## 2. Original pending commits (gitcode-only v4.0.0 history)

| SHA | Author | Date | Title |
|---|---|---|---|
| `34ed5b68c8` | hermes-z6g4 | 2026-09-17 03:32 +0800 | fix(v4.0.0 alpha gate): executor BINARY collation, admin Windows compat, remove tpch_hash_test |
| `61b198f469` | hermes-z6g4 | 2026-09-17 04:43 +0800 | fix(storage): checkpoint JSON escapes Windows paths; recovery tolerates unknown prefix as NULL |
| `615afff025` | hermes-z6g4 | 2026-09-17 15:22 +0800 | fix: cross-platform /proc and filename compatibility for Windows |
| `ec1278f72c` | hermes-z6g4 | 2026-09-17 03:45 +0800 | merge gitee/develop/v4.0.0 into local develop/v4.0.0 |

## 3. Resolution path

Between 2026-09-23 03:01–03:02 (UTC+8), `hermes-z6g4` (with
`claude-macmini` co-authored-by) cherry-picked the three substantive
commits onto a new branch `sync/v410-v400-review-queue` on gitea252.
The branch was a strict descendant of `develop/v4.1.0` HEAD at that
time (`2bd69b223f`).

### 3.1 Cherry-picked commits

| cherry-picked SHA | source SHA | title |
|---|---|---|
| `6603820f1e` | `34ed5b68c8` | fix(v4.0.0 alpha gate): executor BINARY collation, admin Windows compat, remove tpch_hash_test |
| `daad2c687d` | `61b198f469` | fix(storage): checkpoint JSON escapes Windows paths; recovery tolerates unknown prefix as NULL |
| `82772919e2` | `615afff025` | fix: cross-platform /proc and filename compatibility for Windows |

### 3.2 Conflict resolution choices (mirrored v4.1.0 zombie-fix semantics)

The cherry-picks resolved the three documented conflicts in favor of the
v4.1.0 zombie-fix semantics:

- **BINARY collation** (`crates/executor/src/expr/mod.rs`): cherry-pick
  adopted BINARY byte-for-byte comparison (v4.1.0 choice), rejecting
  the v4.0.0 PAD SPACE behavior. SQL semantics: `'F' = 'F '` is **false**
  (matches MySQL/PostgreSQL/SQLite `=` semantics for non-CHAR text).
- **recovery_engine.rs log verbosity**: cherry-pick omitted the
  `log::debug!` call (gitcode/v4.0.0 choice), accepting the loss of the
  v4.1.0 debug logging on unknown value prefix substitution. The
  recovery logic itself (push `Value::Null`, advance by 2-byte prefix)
  is identical between v4.1.0 and the cherry-pick.
- **mysql-server thread counting**: cherry-pick adopted the
  `cfg(windows)` block using `available_parallelism()` as RSS/thread
  count proxy, plus the `connection_tracker` `Default` impl removal.
- **tpch_hash_test**: deleted (referenced `tpch_hashes_v380.json`
  fixture that v4.0.0 had already removed).

`ec1278f72c` was not separately brought in. Its parent chain includes
`34ed5b68c8` which is now reachable through `6603820f1e`; `ec1278f72c`
itself remains unreachable from `develop/v4.1.0` (it is a gitcode-only
merge on `develop/v4.0.0`, not pulled forward).

## 4. Final tree changes (vs `2bd69b223f`)

```
.gitignore                               |  4 ++
Cargo.toml                               |  3 -
crates/admin/src/storage_commands.rs     |  2 +
crates/mysql-server/src/lib.rs           | 30 ++++++++--
crates/storage/src/checkpoint.rs         | 22 ++------
crates/storage/src/recovery_engine.rs    |  2 +
crates/tools/src/upgrade.rs              |  3 +-
tests/integration/tpch/tpch_hash_test.rs | 94 --------------------------------
8 files changed, 40 insertions(+), 120 deletions(-)
```

## 5. Merge into develop/v4.1.0

On 2026-09-26, the `sync/v410-v400-review-queue` tip
(`82772919e283a8164c0b343a0e4dd5b91bec138f`) was merged into
`develop/v4.1.0` via an empty-conflict merge:

```
9c6767a512d9540127d5529807246e3dcea9ab15
  parents=2bd69b223fad7fceee5d1880691f8d65d254c987 82772919e283a8164c0b343a0e4dd5b91bec138f
```

Merge tree sha: `64ef53fd22fbde6ccf6e114128727bbe731789a4` (matches
`sync/v410-v400-review-queue` tip tree sha — no conflicting changes
because the cherry-picks had already pre-resolved all conflicts).

## 6. 5-remote sync of `9c6767a512d9`

| Remote | push mechanism | result |
|---|---|---|
| `gitcode`  | `git push --force-with-lease` | ✓ |
| `gitee`    | `git push --force-with-lease` | ✓ |
| `github`   | `git push --force-with-lease` | ✓ |
| `gitea250` | SSH `docker exec` → `git update-ref` | ✓ |
| `gitea252` | SSH `docker exec` → `git update-ref` | ✓ |

All 10 pairwise `rev-list --left-right --count` between the five
remotes returned `0 0`.

## 7. Outstanding items

- `34ed5b68c8`, `61b198f469`, `615afff025`, `ec1278f72c` themselves
  remain **unreachable** from `develop/v4.1.0`. They are preserved on
  `gitcode/develop/v4.0.0` for audit trail. Their semantic content is
  in `develop/v4.1.0` via `6603820f1e` / `daad2c687d` / `82772919e2`
  (cherry-pick equivalents).

- `gitea252` retains another stale branch `sync/v410-zombie-graph-port`
  (a 2026-09-21 graph-module zombie-fix branch, ahead of v4.1.0 by 2
  commits). Not in scope of this resolution.

## 8. References

- `fca71cb525` / `a65497e276` / `7e30fbc0cb` — empty merges that brought
  doc-only v4.0.0 commits into v4.1.0 graph (2026-09-22 sync)
- `d6dd4fab28` — v4.1.0 zombie-fix core merge
- `67b624cbd2` — v4.1.0 zombie-fix core cherry-pick (the commit that
  causes the PAD SPACE / BINARY conflict with `34ed5b68c8`)
- `b4d46e6e18` — workers.push fix
- `f3595e7361` — dml-storage regressions port
- `sync/v410-v400-review-queue` — hermes-z6g4's resolution branch
  (now deleted from gitea252)
- `9c6767a512d9` — final merge commit landing all cherry-picks on
  `develop/v4.1.0` (5-remote consistent)
- `docs/releases/v4.0.0/SYNC_AUDIT_v4.1.0_2026-09-20.md` — 5-remote
  sync audit (predecessor document)