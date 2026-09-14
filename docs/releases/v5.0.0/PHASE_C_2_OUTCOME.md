# Phase C.2 outcome: deferred (UnsafeCell redesign required)

**Date**: 2026-09-14
**Author**: sqlrustgo core team
**Status**: Phase C.2 deferred. Phase C as a whole is closed without
delivering the 7-8× TPS target from `PHASE_C_SERVER_LOCK_REMOVAL.md`.

## Summary

Two attempts to land Phase C.2 (replace `Arc<RwLock<S>>` with
`Arc<S>` at the `ExecutionEngine` layer) were made on
2026-09-14. Both attempts were reverted. This document explains
why and what the correct path forward looks like.

## Phase C goal recap (from PHASE_C_SERVER_LOCK_REMOVAL.md)

The Phase C design document predicted:

| Workload | Current (v4.0.0 + Phase B) | Target (v5.0.0 Phase C) |
|----------|---------------------------|--------------------------|
| oltp_read_only 4t/10k | 215 TPS | 1500–1700 TPS (≈ 7–8×) |

with the rationale:

> `oltp_read_only` runs **12 statements × 8 threads = 96
> server-side `engine.read()` lock acquires per second**. Each
> acquire costs ~30–50 ns (parking_lot atomic CAS). [...] Removing
> the outer read lock makes the 96 acquires vanish

This rationale is **wrong**. Phase C.1 implementation
(`8c382d5e51` + `2a3246808f` + `760a78e61e` + `4357e2a2c6`,
already merged on `feat/v5.0.0-phase-c`) made the inner storage
locking correct but the cost analysis is off by 2 orders of
magnitude:

* At 2056 OPS (4-thread pure read-only measured against `e14f519b2e`
  baseline), 60–600 ns of lock acquire per query is **0.012% – 0.12%**
  of CPU.
* The dominant cost is `save_table → serde_json::to_string_pretty`
  over the whole table (O(rows) per INSERT).

So C.1+C.2 combined cannot deliver 7-8× TPS as the design
document claimed. The remaining TPS gap must come from a
different vector (binary storage, vectorised execution, etc.).

## What C.1 delivered (and is kept)

* `write_lock: parking_lot::Mutex<()>` field on `FileStorage`
  (`crates/storage/src/file_storage.rs`).
* 17 inherent `&mut self` methods rewritten as `&self` (with
  `Self::with_write_lock(self.as_mut_self(), |s| { ... })`).
* 12 trait impl method bodies wrapped in `with_write_lock`.
* 5 race-condition tests under `crates/storage/tests/phase_c_1_race.rs`
  catching deadlock / lost-write / torn-row / tx-state-corruption.
* TPS: no significant change (within ±1% of baseline).

C.1 provides a **correct concurrency model** for the existing
`Arc<RwLock<FileStorage>>` server pattern, even though it doesn't
move the TPS needle.

## Why C.2 failed (both attempts)

### The core Rust aliasing problem

`Arc<S>` shared between threads + `&mut self` trait method calls
on `S` is **undefined behaviour** in safe Rust. The borrow
checker refuses it:

```rust
let storage: Arc<S> = ...;
move |rec| {
    let mut storage = storage.write();  // Arc<S> has no write() method
    s.delete(...);                     // requires &mut S
}
```

The original code had `Arc<parking_lot::RwLock<S>>` and the
RwLock's internal atomic CAS + lock guard provided the necessary
`&mut` exclusivity guarantee: at any instant, at most one thread
holds a `RwLockWriteGuard`, so obtaining `&mut S` via the guard's
DerefMut is sound.

C.1's `as_mut_self` unsafe bridge works **only when there is an
outer RwLock** (or equivalent synchronisation) gating the
`&self → &mut self` reborrow. Once the outer RwLock is removed,
the unsafe bridge becomes UB.

### Attempt 1 (no `StorageLock` trait, direct inline)

`Arc<RwLock<S>>` → `Arc<S>`, `storage_read()` returns `&S`,
`storage_write()` returns a `StorageWriteGuard<'_, S>` whose
construction uses `unsafe { &mut *(self.storage as *const S as *mut S) }`
(the same pattern as `FileStorage::as_mut_self`).

Failed because:

1. 99 cargo errors in the first run (mostly unwrap / move
   patterns that the global `read()/write()` rewrite broke).
2. The unsafe bridge is sound only if `Arc<S>` is exclusively
   owned by one thread, which breaks the multi-threaded
   executor pattern that C.2 is supposed to *enable*.

### Attempt 2 (with `StorageLock` sidecar trait + `Box::leak` mutex)

Added `StorageLock::write_lock(&self) -> &parking_lot::Mutex<()>`
to `crates/storage/src/engine.rs`. Each engine impl provides
either a real mutex (`FileStorage`, `BoxStorageEngine`) or a
`leaked_unit_mutex()` (no-op). `ExecutionEngine::storage_write()`
acquires S's lock + uses the unsafe bridge to get `&mut S`.

Got 131 cargo errors, of which the irreducible ones were:

* `storage.write()` in `rollback_with_undo` closures: the
  closure captures `Arc<S>` (not `Arc<RwLock<S>>`), and the
  `Arc<S>` cannot be mutably borrowed to call trait methods.
* `storage_ref()` public helper returning
  `&Arc<parking_lot::RwLock<S>>`: signature was wrong even after
  the field type change.
* `engine_setops.rs` and other caller modules that pass generic
  `S` through trait-bound method calls: the bound `S: StorageLock`
  propagated everywhere `ExecutionEngine::execute_statement` is
  called, breaking the existing test fixtures that construct
  `ExecutionEngine<MemoryStorage>` (MemoryStorage did not need
  a write lock because there was no concurrent contention on the
  single-threaded test engine).

The unsafe bridge could in principle be salvaged if every call
site of `storage_write()` is rewritten to use a closure that
captures `Arc<S>` and calls S's trait methods via some other
mechanism. But that mechanism does not exist in safe Rust — the
unsafe bridge is the only way, and it requires *some* outer
synchronisation to be sound.

## The correct path forward (UnsafeCell redesign)

The only way to make `Arc<S>` + `&mut self` methods safe is to
move the synchronisation **inside `S`** rather than at the
`Arc` layer. Specifically:

1. Replace `tables: HashMap<String, TableData>` (and the four
   other protected fields) with
   `tables: UnsafeCell<HashMap<String, TableData>>`.
2. Implement `Sync` for `FileStorage` manually, with the
   invariant documented: concurrent `&` access is safe; any
   `&mut` access must go through `UnsafeCell::get()` plus the
   internal `parking_lot::Mutex<()>` (C.1's write_lock) for
   serialisation.
3. Inherent `&self` methods that need `&mut` go through
   `unsafe { &mut *self.tables.get() }` **without** the
   `as_mut_self` reborrow dance (because `UnsafeCell::get()`
   is the lint-approved path).
4. Trait impl `&mut self` methods stay as-is (they own the
   `&mut`).
5. `ExecutionEngine.storage: Arc<S>` becomes sound: the Arc
   itself is `Sync` (because S's `Sync` impl says so), and the
   internal `UnsafeCell` provides interior mutability.

Estimated scope: **1–2 weeks** of focused work (C.1.1.bis
inherent-block work was the template). Touches:
* `crates/storage/src/file_storage.rs` (replace 5 fields with
  `UnsafeCell` + update all 45 access sites).
* `crates/storage/src/engine.rs` (add `unsafe impl Sync` for
  `FileStorage` with documented invariant).
* `src/execution_engine.rs` (storage field type, helper
  signatures, ~40 caller updates — same as Attempt 1 + 2
  above).
* `crates/storage/src/binary_storage.rs` (`BoxStorageEngine`
  also needs UnsafeCell if it wraps a `FileStorage` directly;
  but the `Dyn` indirection probably means it can stay as-is
  if the wrapped engine provides its own `Sync`).

Risk: medium. The `unsafe impl Sync` is the highest-risk part;
the wrong invariant statement would produce silent UB that
might not crash for months.

## What this means for the Phase C roadmap

Phase C as described in `PHASE_C_SERVER_LOCK_REMOVAL.md`
**cannot deliver its 7-8× TPS target** because:

1. The cost model (§"Why is oltp_read_only so slow today?") was
   wrong by ~2 orders of magnitude.
2. The actual hot path is `save_table → serde_json::to_string_pretty`,
   which C.1/C.2 does not touch.
3. Even with UnsafeCell redesign (above), the TPS gain is
   expected to be < 5% (the lock acquire cost that UnsafeCell
   redesign eliminates).

The 7-8× TPS target would require either:
* Binary storage format (no JSON serialisation per INSERT) — see
  `crates/storage/src/binary_storage_v2.rs`.
* Vectorised execution — see `PHASE_B_STEP5_BENCH.md` and the
  executor parallelism work.
* A different cost model entirely (e.g. columnar storage with
  Arrow / Parquet).

## Closing actions

1. ✅ Phase C.1 stays merged (5 commits, all tests passing).
2. ✅ Phase C.2 attempts reverted; working tree is clean.
3. ⏸️ Phase C.2 deferred until the UnsafeCell redesign
   described above is staffed (1–2 weeks).
4. ⏸️ Phase C.3 and C.4 are blocked on C.2 (they assume the
   outer RwLock is gone).
5. 📋 The 7-8× TPS target should be re-evaluated against the
   binary storage vector (which is the actual hot path).

## References

* `PHASE_C_SERVER_LOCK_REMOVAL.md` — the original plan.
* `PHASE_C_1_INTERNAL_LOCKING.md` — the C.1 design (kept).
* `crates/storage/src/file_storage.rs:76-83` — the `write_lock`
  field added in C.1.
* `src/execution_engine.rs:582+` — the `Arc<RwLock<S>>` field
  that would need to become `Arc<S>` in C.2.
* TPS measurement commands (see commit messages of
  `2a3246808f` and `4357e2a2c6` for the script; rerun on
  any new C.2 attempt to confirm < 5% gain).
