# PR Merge Blocker Analysis — 2026-09-17

> **Task**: Merge feat/v4.0.0-wal-group-commit → develop/v4.0.0 (per user request 2026-09-17)
> **Result**: 🔴 **BLOCKED — manual code conflict resolution required**
> **Status**: Branch restored to pre-rebase state (21dad7697f). 5 remotes unchanged.

## What was attempted

1. **Pre-rebase backup**: `backup/v400-wal-group-commit-pre-rebase-20260917` tag created at `21dad7697f`
2. **Rebase onto develop/v4.0.0** (`gitea250/develop/v4.0.0` = `04c381b8c3`) with `git -c rebase.backend=merge rebase -Xours`:
   - 11 commits to replay
   - 3 PHASE_B_MVCC_GC commits dropped (content already upstream — develop/v4.0.0 has the GC content from PR #3755 #3771)
   - 8 commits replayed successfully
3. **Post-rebase build**: ❌ FAIL with `error[E0609]: no field scan_skip_cache on type &MvccStorage<S>`

## Root cause of build failure

The MVCC code conflict is **semantic, not textual**:

| Aspect | develop/v4.0.0 (post-PR #3755 + #3771) | feat/v4.0.0-wal-group-commit (rebased onto it) |
|---|---|---|
| `MvccStorage<S>` struct | 5 fields: `inner, mvcc, write_count` + write-throttle `maybe_gc()` | Adds `scan_skip_cache` field for fast-path optimization |
| GC strategy | `maybe_gc()` throttled via `write_count % GC_INTERVAL == 0` (PR #3755) | `scan_skip_cache` optimization added in `1f434dea9e` (Round 4 #3) |
| Background GC | Bound to per-write throttle | Bound to scan skip-cache fast path |

The `scan_skip_cache` field (introduced in our `1f434dea9e perf: auto-build PK B+Tree index`) **does not exist on develop/v4.0.0's struct** because develop/v4.0.0 has its own `fix/v400-mvcc-gc` branch that refactored MVCC storage differently (PR #3755 + #3771).

A naive rebase keeps our code referring to the field but the struct definition is the upstream version — leading to compile error at line 246 (`crates/storage/src/mvcc_storage.rs:246`).

## What is needed to proceed

The merge **cannot be completed safely via git rebase alone**. It requires:

1. **Pick the right GC strategy** (decide which one wins):
   - Option A: Keep develop/v4.0.0's write-throttle `maybe_gc()` strategy, port our `scan_skip_cache` to use existing fields
   - Option B: Keep our `scan_skip_cache` strategy, port upstream's `maybe_gc()` throttle into it
   - Option C: Both — non-conflicting unification

2. **Re-test**: After resolving, must re-run storage tests (currently 749 passed, 1 pre-existing fail) + 20-min SOAK to confirm B+Tree overwrite bug not regressed

3. **Update PR**: Re-author commit messages to reflect actual conflicts

4. **Update STAGE.yaml**: Add a note about merge plan + conflict resolution in `worktree_local_additions`

## Why I did not force through

Per user profile rules:
- **"❌ 禁止强制覆盖生产数据"** — forcing through with broken code would break develop/v4.0.0 for all downstream developers
- **"❌ 禁止在未备份时执行"** — branch is backed up (`backup/v400-wal-group-commit-pre-rebase-20260917`)
- **"they verify claims themselves and will call out unverified assertions"** — making a broken build silently would mislead

## Current state (preserved)

- `feat/v4.0.0-wal-group-commit` = `21dad7697f` (original pre-rebase tip) — restored after failed rebase
- `backup/v400-wal-group-commit-pre-rebase-20260917` = `21dad7697f` (safety net)
- `backup/v400-wal-group-commit-rebased-20260917` = `4c0a859c38` (broken-rebase artifact, kept for diffing)
- All 5 remotes: in sync at `21dad7697f`
- Working tree: clean, builds OK

## Recommended next steps for user

**Option 1 (smallest scope)**: Just merge the **DOC commits only** (last 2: `8a78a2ed7e V400_04_20MIN_SOAK` + `4c0a859c38 doc rectification`) — these don't touch MVCC storage code and won't conflict. This gets the governance docs and the SOAK bug disclosure into develop/v4.0.0 without risking the runtime.

**Option 2 (manual conflict resolution)**: Engage with PR #3755 / #3771 history, resolve the MVCC storage struct conflict, then rebase + retest.

**Option 3 (cherry-pick strategy)**: Instead of rebase, cherry-pick individual commits that don't conflict, drop the conflicting ones, and document the skipped commits in STAGE.yaml.

**Option 4 (block merge)**: Per the B+Tree overwrite bug disclosure in `V400_04_20MIN_SOAK.md`, recommend NOT merging until bug is fixed. Use this worktree as a sandbox for the next iteration.

I can proceed with any of these if the user provides direction.

---

*Analysis date: 2026-09-17*
*Author: openclaw + Claude*
*Related: V400_04_20MIN_SOAK.md §6 (B+Tree overwrite bug), STAGE.yaml#worktree_local_additions*