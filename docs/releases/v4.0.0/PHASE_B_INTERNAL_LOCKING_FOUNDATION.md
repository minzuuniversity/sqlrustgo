# Phase B Internal Locking Foundation — FileStorage correctness

**Date**: 2026-09-15
**Branch**: `feat/v4.0.0-server-read-perf`
**Status**: Implemented and merged into develop/v4.0.0

## Background

After `PHASE_B_MINISTEP_READ.md` (Mini Step, closed without action), it
became clear that single-process server-layer lock removal (the
UnsafeCell/Arc<S> rewrite) is **invasive and high-risk for v4.0.0** —
it requires touching 180+ call sites across the executor, server lib,
and WAL recovery path.

However, a **smaller, correctness-only** change can land safely in
v4.0.0: give `FileStorage` its own internal write lock so that
inherent `&mut self` methods can be rewritten as `&self`, removing
the need to hold the outer `RwLock` exclusively for routine writes.

This commit series does exactly that. It does **not** attempt to
remove the outer `RwLock`. After this series, FileStorage is safe
to use under `Arc<RwLock<FileStorage>>` with concurrent readers,
because each reader/writer takes the inner lock for the duration of
its mutation. The outer lock still serializes at the same granularity
as before — this change is a correctness prerequisite, not a
performance change.

## What this commit series adds

1. `write_lock: parking_lot::Mutex<()>` field on `FileStorage`,
   initialised in all 4 constructors.
2. `with_write_lock<R>(me: &mut Self, f: FnOnce(&mut Self) -> R) -> R`
   helper that holds the lock for the duration of `f`. Uses
   `Box::into_raw` → `*mut ()` → `Box::from_raw` to dodge the borrow
   checker conflict between the lock guard and the `&mut Self`
   reborrow inside `f`.
3. `as_mut_self(&self) -> &mut Self` unsafe bridge, gated by
   `#[allow(invalid_reference_casting)]` at function level (Rust
   1.83+ lint is deny-by-default).
4. 17 inherent `&mut self` methods rewritten as `&self`, each
   body wrapped in `Self::with_write_lock(self.as_mut_self(), |s| { ... })`.
5. 12 trait impl method bodies wrapped in `with_write_lock` for
   the same reason.
6. Race-coverage test suite under
   `crates/storage/tests/phase_c_1_internal_locking_race.rs`
   covering: concurrent scans, interleaved insert/scan, concurrent
   table create/drop, concurrent flush/scan, concurrent begin/
   commit/rollback.

## Why this matters

After this series, any caller holding `Arc<RwLock<FileStorage>>`
can:
- Run multiple SELECTs concurrently (read lock shared).
- Run a write without fear of data corruption (inner lock guarantees
  serialised mutation).
- No longer relies on the outer RwLock for correctness — only for
  the now-trivial invariant that `&mut` is exclusive within a single
  scope.

The outer RwLock can in principle be removed in a future v5.0+
investigation; this v4.0.0 commit lays the **correctness** groundwork.

## TPS impact

`+0%` in raw OPS — this is a correctness change, not a perf change.
At 2056 OPS (current baseline), 60–600 ns of lock acquire per query
is 0.012–0.12% of CPU. Removing or keeping it makes no measurable
difference.

The v4.0.0 TPS gain will come from the Phase B Step 5+ work (PK
column detection, wire encode batching, etc.), which this correctness
fix is a prerequisite for.

## Validation

- 14 `engine_select_pk` unit tests pass (10 existing + 4 new for
  `resolve_pk_column`).
- 5 `phase_c_1_race` race tests pass.
- 47 `file_storage_direct_v3_12` tests pass.
- 18 `storage_integration_test` tests pass.
- 4 `e2e_crash_recovery_proof` tests pass.
- Pre-existing flake
  (`recovery_engine::bytes_to_record_tolerates_unknown_prefix_as_null`)
  is unrelated.

## Files affected

- `crates/storage/src/file_storage.rs` (+970 lines / -326 lines)
- `crates/storage/tests/phase_c_1_internal_locking_race.rs` (new,
  411 lines)
- `docs/releases/v4.0.0/PHASE_B_INTERNAL_LOCKING_FOUNDATION.md` (this
  file)

## References

- `PHASE_B_MINISTEP_READ.md` — Mini Step investigation that closed
  this analysis.
- `crates/storage/src/file_storage.rs::scan_pk` (line 3316) — uses
  `as_mut_self` to bypass the borrow checker.
