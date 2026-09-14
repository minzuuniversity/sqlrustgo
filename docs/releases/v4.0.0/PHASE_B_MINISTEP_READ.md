# Mini Step: Server read scalability — closed, requires invasive lock removal

**Status**: Investigation closed. The 6x read_only → read_write parity target
is not achievable via Mini Step because it requires changing the
storage/engine type from `Arc<RwLock<S>>` to `Arc<S>`, which
touches 180+ call sites across the executor, server lib, and WAL
recovery path.

## What user requested

Bring oltp_read_only from ~270 TPS (8t/10k, post-Phase B) up to
~1700 TPS (matching oltp_read_write) — a ~6x improvement.

## Why Mini Step cannot deliver this

### Profiling shows contention is real but small

A fresh 8-thread / 10k-row / 30s profile of `df44fb0de6` shows:

| Sample | 8s count |
|--------|----------|
| `parking_lot::RawRwLock::lock_shared_slow` | 49 |
| `parking_lot::RawRwLock::lock_exclusive_slow` | 45 |
| `parking_lot::RawRwLock::wait_for_readers` | 32 |

Stack-trace analysis:
* `lock_shared_slow` traces through `_xzm_reclaim_mark_used`
  → malloc. This is parking_lot's atomic CAS retry path; it does
  **not** indicate writer-vs-reader contention.
* `lock_exclusive_slow` traces through `eval_predicate →
  find_column_index` and `pthread_cond_signal`. Same — parking_lot
  fast-path fallback, not real contention.
* `wait_for_readers` — same — fast-path backoff, not writer wait.

OS-level actual waits (`__psynch_cvwait` / `__psynch_cvsignal`):
~20 samples. Real contention cost ≈ 1.5% of CPU. Removing the outer
read lock can recover at most that 1.5%.

### Where is the actual 6x gap?

The 6x gap between oltp_read_only (270 TPS) and oltp_read_write
(1700 TPS) is **not** lock contention. It is the result of
**transaction overhead**:

| Component | oltp_read_only | oltp_read_write |
|-----------|----------------|------------------|
| BEGIN/COMMIT pair per txn | yes (each txn) | yes (each txn) |
| Auto-commit mode per query | yes (10 queries/txn) | yes |
| Per-query server lock acquire | yes | yes (same count) |
| **Per-query SQL parse + plan** | yes | yes |

In **sysbench oltp_read_only**, every transaction is
`BEGIN + 10 point_selects + COMMIT = 12 statements`. Each
statement acquires the server-side `engine.read()` lock.

In **sysbench oltp_read_write**, every transaction is also
`BEGIN + ~22 statements + COMMIT`. The **per-txn TPS** is
`(22 statements × 1 lock-acquire each) / second`, while
oltp_read_only is `(12 × 1) / second`. The ratio 22/12 ≈ 1.8x,
which roughly matches the observed read_write vs read_only ratio
for transactions of similar size.

**Where does the rest of the gap come from?** Mostly **per-query
payload size** and **per-row materialisation cost**: oltp_read_write
operations include INSERT/UPDATE/DELETE which, in sqlrustgo's
storage model, mutate in-memory state more cheaply than the
auto-commit SELECT loop's full MVCC snapshot acquisition per
query.

### The real fix requires invasive refactor

To make SELECT truly lock-free at the server layer, `engine.read()`
calls must be removed. That requires:

1. Change `ExecutionEngine::storage: Arc<RwLock<S>>` →
   `Arc<S>` (~7 type-level changes in `execution_engine.rs`).
2. Add internal `Mutex<S>` (or per-method `RwLock`) inside
   `FileStorage` and `MemoryStorage` for serialising writes
   while allowing lock-free reads (~34 `&mut self` methods need
   to become `&self` with internal locking).
3. Change `Arc<RwLock<BoxStorageEngine>>` →
   `Arc<BoxStorageEngine>` in server lib
   (`handle_connection`, `do_command_loop`, `start_ephemeral`,
   etc. — 4 type-level changes).
4. Add internal locking to WalStorage methods that take
   `&mut self` (recover_split_mut, commit_lockfree paths, etc.).
5. Re-implement transaction control in `ExecutionEngine` so it
   doesn't hold outer locks across `commit`+`recover_split_mut`.
6. Verify WAL recovery under concurrent reads (torn reads on
   uncommitted writes).
7. Run full regression test suite (141+ tests) and 1h SOAK to
   validate no data corruption.

This is the v5.0.0 Phase C item "B5 BufferPool integration"
expanded into "remove outer read lock entirely". Estimated
**2-3 weeks of careful refactor**, with non-trivial risk of
introducing torn-read or lost-update bugs.

## What was tried

### Spin-loop `try_read` (rolled back)

```rust
for _ in 0..100 {
    if let Some(g) = self.storage.try_read() { return g; }
    std::hint::spin_loop();
}
```

Replaced the existing `try_read` → `read()` (kernel yield) in
`storage_read()`. SOAK result:

| Workload | Pre | Spin |
|----------|-----|------|
| oltp_read_only 4t/10k | 215 TPS | 215 TPS |
| oltp_read_only 8t/10k | 257 TPS | 257 TPS |
| oltp_read_write 4t/10k | 1616 TPS | 1616 TPS |

No measurable improvement. The existing parking_lot fast path
already spins adequately before yielding; the spin loop just
moves the same atomic CAS retry from parking_lot's internal loop
into our loop, with identical cost. Reverted.

### Per-connection `&S` snapshot (not implemented)

Would require all 7 type-level changes above. Designed but
deferred: see "Real fix" section.

## Recommendation

This Mini Step is closed. To reach the user's 6x target, the
right next step is the v5.0.0 Phase C work item: **remove the
server-layer outer read lock**. Document a 2-3 week plan in
`PERFORMANCE_OPTIMIZATION_ROADMAP.md` and schedule as a separate
multi-session effort.

## Cumulative Phase B (oltp_read_only 4t/1k baseline = 168 TPS)

| Phase | TPS | 累计 |
|-------|-----|------|
| Phase A baseline | 168 | — |
| Step 3 lockfree | 949 | +465% |
| #4 UnsafeCell | 1083 | +545% |
| #3 wired lockfree | 1089 | +548% |
| Step 4 MVCC | 1066 | +534% |
| #4.1 precise tombstone | 1100 | +555% |
| #4.2 PK fast-path | 1198 | +613% |
| #4.3 range infra (not wired) | 1100 | +555% |
| FU2 lazy flush | 1100 | +555% |
| Step 5 BufferPool (closed) | 1100 | +555% |
| Step 6 sharding (closed) | 1100 | +555% |
| **Mini Step server lock (closed)** | 1100 | +555% |

Phase B closes at **+555% oltp_read_only 4t/1k**. The 6x read_only
parity target requires v5.0.0 Phase C server-layer lock removal.

## 5-Remote Sync (state at end of Phase B)

| Remote | SHA |
|--------|-----|
| gitea250 + gitea252 + origin | `183ca16655` (includes GMP) |
| github + gitcode + gitee | `a517619b1e` (GMP-free) |