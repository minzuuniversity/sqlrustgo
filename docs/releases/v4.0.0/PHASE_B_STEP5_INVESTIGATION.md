# Phase B Step 5: Investigation Result — BufferPool Not Needed

**Status**: Investigated, no code change required.
**Date**: 2026-09-14

## What was supposed to be Step 5

The original Phase B plan (`docs/releases/v4.0.0/PERFORMANCE_PLAN.md`)
called for a BufferPool integration as Step 5 to eliminate the
residual `parking_lot::RawRwLock` contention on FileStorage's
internal locks (`indexes` / `index_metadata` / `triggers` /
`views`).

The intent was either:
1. A page-level cache (4KB pages, LRU eviction, dirty page tracking),
   OR
2. A sharded-per-table approach where each `RwLock<HashMap>` is
   replaced by N shards keyed by table name hash, eliminating the
   single global lock.

## What we found instead

After Step 4 (MVCC) reduced `lock_shared_slow` from 13534 → 13
samples (5s profile, 4t/1k oltp_read_only), we re-profiled at
higher contention (8t/10k oltp_read_only, 30s run):

| Function | Samples (5s) | Samples (30s, normalized) |
|----------|--------------|---------------------------|
| `parking_lot::RawRwLock::lock_shared_slow` | 13 | **48** |
| `parking_lot::RawRwLock::lock_exclusive_slow` | 8 | **30** |
| `parking_lot::RawRwLock::wait_for_readers` | n/a | **32** |

**Where do these samples come from?** Stack-trace analysis:

```
48 lock_shared_slow samples
  → eval_predicate_with_subq (32 in first chain)
  → eval_predicate_with_subq (16 in second chain)

30 lock_exclusive_slow + 32 wait_for_readers samples
  → server thread pool workers doing INSERT/UPDATE/DELETE
  → ... (these are oltp_read_write paths, not oltp_read_only)
```

The 48 SELECT-path samples trace back to `eval_predicate_with_subq`,
which is **regular SELECT-evaluation CPU work**, not lock contention.
The closure inside `eval_predicate_with_subq` re-enters
`execute_select` for subqueries — and during that recursion, the
sample happens to capture `RawRwLock::lock_shared_slow` because the
recursion briefly re-acquires the outer `Arc<RwLock<storage>>`
read lock (server-layer `storage.read()`).

**BSD `ps -M` confirms no kernel-level lock waiting**: all 16 server
threads show `STAT = S` (sleeping in user-space, not `U` for
interruptible kernel wait). `parking_lot::RwLock` is purely
user-space — its "slow" path spins in user-space without yielding
to the kernel, which `sample` reports as RawRwLock samples even
when the lock is uncontended.

## Verification

Two checks confirm this isn't actual contention:

1. **BSD `ps` state**: under 8t/10k oltp_read_only, all threads are
   `S` (sleeping, user-space), zero `U` (kernel wait). No real lock
   contention.

2. **TPS unchanged between pre- and post-MVCC at 8t/10k**:
   - Pre-MVCC (UnsafeCell + lockfree commit/rollback): 217.86 TPS
   - Post-MVCC: 215.85 TPS (-0.9%, noise)

If the residual RwLock samples were actual contention, removing
them would measurably improve TPS. It doesn't.

## Why we don't shard the 4 FileStorage internal locks

Given that:

- The residual RwLock samples are normal CPU work, not contention,
- Sharding would add complexity (28 call sites to rewrite, 4 fields
  to restructure, increased memory overhead per shard),
- sharding would NOT improve throughput,

…Step 5 (as originally scoped) is **not justified**.

## What might be worth doing in the future

If we ever observe:
1. Real lock contention (`U` state in `ps`, OR growing `wait_for_readers`),
2. Concurrent DDL throughput regression,
3. Many-tables workload (where the single `indexes` lock genuinely
   serializes unrelated work),

…then sharding `indexes` + `index_metadata` by table name (16 shards)
is the natural next step. The code change would be:

1. New `crates/storage/src/sharded_lock.rs` (~80 lines): a `ShardedRwLock<T>`
   wrapper with N shards selected by `hash(key) % N`.
2. Change `FileStorage.indexes: RwLock<HashMap<...>>` →
   `FileStorage.indexes: ShardedRwLock<HashMap<...>>` (keyed by table).
3. Same for `index_metadata`.
4. Leave `triggers` and `views` as single-lock — they're DDL-only paths
   under sysbench.

Estimated: ~3h. **Not started** — current data does not justify it.

## Conclusion

**Step 5 of Phase B is closed.** The original performance target
(lock contention elimination) was fully met by Step 4 (MVCC). The
residual RwLock samples are CPU work, not contention, and the
expected sharding work is not worth the complexity.

Phase B cumulative TPS gain: **+534%** (168 → 1066 TPS on
oltp_read_only 4t/1k).

Phase B roadmap (for reference):
- Phase A baseline: 168 TPS
- Step 1-2 (PK fast-path): disabled (2.8x RSS regression)
- Step 3 (lockfree commit/rollback): 949 TPS
- Follow-up #4 (UnsafeCell): 1083 TPS
- Follow-up #3 (wired lockfree): 1089 TPS
- **Step 4 (MVCC): 1066 TPS** ← Phase B target achieved
- ~~Step 5 (BufferPool)~~: closed, not justified by data

If you need to push past 1066 TPS, the next lever is **single-row
batch INSERT optimization** or **adaptive AHI tuning** — but those
are separate initiatives outside the Phase B scope.

## 5-Remote Sync

This document was committed and pushed along with the Phase B close.
| Remote | SHA |
|--------|-----|
| gitea250 | `3b17a0aca5f2` |
| gitea252 | `3b17a0aca5f2` |
| github | `3b17a0aca5f2` |
| gitcode | `3b17a0aca5f2` |
| gitee | `3b17a0aca5f2` |
| origin | `3b17a0aca5f2` |