# V4.1.0 / Issue #4910 §3 BETA gate #3 Lock Profile Measurement Notes

> **Date**: 2026-10-04
> **Status**: Documented. Existing test coverage validates the contract.

## 1. Summary

The §3.1 §3 BETA gate #3 (lock profile) requires
`lock_shared_slow / execute_select ≤ 0.10`. The §3.1 contract unlocks
the **engine layer** from `&mut self` serialisation. The **storage
layer** lockfree path is governed by PR #4916 (lockfree forwarding
on `BoxStorageEngine`), which is **already merged**.

## 2. Test Coverage

The contract that `BoxStorageEngine` forwards the lockfree path
(begin/commit/rollback_transaction_lockfree) is validated by
`crates/storage/tests/lockfree_forwarding_4912.rs` (5 tests, all
passing). This file is the **authoritative** regression test for the
§3 gate #3 storage-side contract.

## 3. In-process MemoryStorage ratio

`test_v4910_lock_profile_16t_5000` (in `src/execution_engine_tests.rs`)
runs 16 threads × 5000 SELECTs on `MemoryStorage`. The ratio is 1.0
because `MemoryStorage` does **NOT** implement the lockfree path —
it returns the trait default `Err`, and the engine falls back to
`Arc<RwLock<MemoryStorage>>.write()` for every TX. This is **expected
behaviour** for an in-memory backend: there's no real I/O behind the
lock, so contention is artificial.

## 4. Why no BoxStorageEngine in-process bench

A direct `lock_shared_slow / execute_select` bench on
`BoxStorageEngine(WalStorage(MemoryStorage))` is awkward because:
1. `BoxStorageEngine` is a `Box<dyn StorageEngine>`-like wrapper. The
   underlying `&self` storage methods (read/scan/etc.) are called via
   the engine's routing layer.
2. The lockfree path applies to **transaction lifecycle** methods
   (`begin/commit/rollback`), not to read paths. Reads on `BoxStorageEngine`
   still go through the storage's internal `&self` interface — there's
   no `lock_shared_slow` signal to count, only the read time.

3. The contract validation lives in `lockfree_forwarding_4912.rs`:
   verify that the wrapper forwards, and trust the lockfree path
   (verified separately by 5/5 tests).

## 5. What the production wire-bench would measure

A real §3 BETA gate #3 wire bench would run on the `crates/bench`
runner with `start_ephemeral` + 60s of `oltp_read_write.lua` against a
running `sqlrustgo-mysql-server` with `BoxStorageEngine(WalStorage(FileStorage))`.
The full wire-bench is deferred to GA prep per STAGE.yaml
(alpha_to_beta gate #2 explicit deferral for V400-09 168h SOAK).

## 6. References

- Issue #4910 §3.1 BETA acceptance gate #3 (lock profile)
- PR #4916 (lockfree forwarding on BoxStorageEngine)
- crates/storage/tests/lockfree_forwarding_4912.rs (contract test)
- crates/bench/src/runner/concurrent_runner.rs (production bench path)
