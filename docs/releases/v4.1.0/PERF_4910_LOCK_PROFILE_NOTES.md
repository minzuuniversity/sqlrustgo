# V4.1.0 / Issue #4910 §3 BETA gate #3 Lock Profile Measurement Notes

> **Date**: 2026-10-04（原文）· **复核**: 2026-11-07（本线，develop）
> **Status**: 部分成立 —— 见 §0，未成立的部分**不得**当作本线判据

## 0. 2026-11-07 复核（并入 develop 时逐条核过）

| 原文断言 | 在 develop 上的实情 | 怎么核的 |
|---|---|---|
| §2：`BoxStorageEngine` 的 lockfree 转发由 `crates/storage/tests/lockfree_forwarding_4912.rs`（5 条）看守 | **成立**：该文件在 develop 上存在 | `git cat-file -e develop:v4.1.0 路径` |
| §2/§1：lockfree 生命周期方法在存储层存在（PR #4916 已并） | **成立**：`rollback_transaction_lockfree` 等在 `crates/storage/src/{engine,file_storage,binary_storage}.rs` 均有实现 | `git grep` |
| §3：`test_v4910_lock_profile_16t_5000`（在 `src/execution_engine_tests.rs`）给出 in-process 的 ratio = 1.0 | **不成立**：这条测试不在 develop 上。全仓 `git grep -n lock_profile` 在 develop 上只命中 `docs/releases/v4.1.0/PERF_4910_PHASE3_PLAN.md:241`，那里写的是「§3.1 §3 lock_profile ratio … **deferred**」 | `git grep -n lock_profile gitea252/develop/v4.1.0` |
| §5：真正的 wire-bench 留到 GA 前置（STAGE.yaml 的显式延后） | 与 develop 的 `PERF_4910_PHASE3_PLAN.md` 一致 | 两份文档同向 |

⇒ **结论**：本文的 §1/§2/§4/§5 可以作为背景与分析保留；**§3 的数字与「已有测试看守」的说法在本线不成立**，
BETA 门禁 #3（锁画像 ratio ≤ 0.10）在 develop 上仍是**未闭合项**（deferred，无测量、无测试）。
把它当已完成 = 用一份来自别的分支线的历史观测冒充本线证据。

## 1. Summary（原文保留）

The §3.1 §3 BETA gate #3 (lock profile) requires
`lock_shared_slow / execute_select ≤ 0.10`. The §3.1 contract unlocks
the **engine layer** from `&mut self` serialisation. The **storage
layer** lockfree path is governed by PR #4916 (lockfree forwarding
on `BoxStorageEngine`), which is **already merged**.

## 2. Test Coverage（原文保留）

The contract that `BoxStorageEngine` forwards the lockfree path
(begin/commit/rollback_transaction_lockfree) is validated by
`crates/storage/tests/lockfree_forwarding_4912.rs` (5 tests, all
passing). This file is the **authoritative** regression test for the
§3 gate #3 storage-side contract.

## 3. In-process MemoryStorage ratio（原文保留，**本线不成立**）

`test_v4910_lock_profile_16t_5000` (in `src/execution_engine_tests.rs`)
runs 16 threads × 5000 SELECTs on `MemoryStorage`. The ratio is 1.0
because `MemoryStorage` does **NOT** implement the lockfree path —
it returns the trait default `Err`, and the engine falls back to
`Arc<RwLock<MemoryStorage>>.write()` for every TX. This is **expected
behaviour** for an in-memory backend: there's no real I/O behind the
lock, so contention is artificial.

⇒ 该测试只存在于 `fix/omp-engine-phase3-arc-mutex-tx` 那条线
（`src/execution_engine_tests.rs:1970`）。它是自包含的墙钟测量
（16 线程 × 5000 次读，阈值 1000 µs 算 slow），不依赖引擎内部计数器 ⇒
要在本线成立，需要把它带进 `src/execution_engine_tests.rs` 并给一个**稳定**的
判定阈值（否则在共享机器上就是新的 flake 源）。这一步留给门禁批，
不在文档承载批里顺手做。

## 4. Why no BoxStorageEngine in-process bench（原文保留）

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

## 5. What the production wire-bench would measure（原文保留）

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
- `docs/releases/v4.1.0/PERF_4910_PHASE3_PLAN.md` §241（本线把它记为 deferred）

## 7. 承载说明（为什么这份文档现在才进 develop）

它此前只存在于 `fix/omp-engine-phase3-arc-mutex-tx` 一条分支上，develop 上没有 ⇒
「合并不能丢正确的代码或文档」这条尺子要求把它承载进来。带进来的同时逐条复核（§0），
因为**不加复核地搬运会把一条历史观测变成假证据**。
