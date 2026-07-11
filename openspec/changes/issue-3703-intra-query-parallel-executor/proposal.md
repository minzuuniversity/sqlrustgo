# Proposal — Issue #3703: Intra-Query Parallel Executor (Scan/Join/Agg)

> **Issue**: [#3703](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3703) — `[FEAT] Multi-threaded query executor (parallel scan/join/agg)`
> **Owner**: @openclaw (assigned 2026-07-11)
> **Roadmap Item**: v3.10.0 M-5 (INT-2 ParallelExecutor 生产路径, 未解决)
> **Date**: 2026-07-11
> **Schema**: spec-driven
> **Depends on**: nothing (independent of WAL/storage work)

---

## Why

v3.9.0 SOAK 测试 (3x sysbench, 20 client threads on M4) 确认 **server CPU 120% ≈ 1.2 core 饱和** — 单核 VolcanoExecutor 是瓶颈。connection-level 并行 (ServerThreadPool via `--server-threads N`) 已处理多客户端并发, 但**单查询吞吐受限于顺序执行**。

**关键洞察 (issue #3703 comment 69559)**: 120% CPU 不是 bug — 是 1 core 满载 + 少量系统开销 (WAL fsync, network IRQ)。增加客户端线程只会增加 context switching, 不提升吞吐。

**核心目标不是峰值 QPS, 而是降低大查询的 P99 尾延迟**:
- 现状: 1 connection = 1 core, full scan 10s
- 目标: 1 query = N cores, full scan 0.7s (N=16)

---

## What Changes

### 1. 新增能力 (New Capabilities)

- **`intra-query-parallel-scan`**: 在 `LocalExecutor::execute_select()` 中接入 `ParallelVolcanoExecutor::partition_rows()`, 按 `--executor-parallelism=N` 切分扫描行, N 个 worker 并行 Filter+Project+Agg, 主线程 merge
- **`intra-query-parallel-hash-join`**: hash join build side 按 partition_id 分到 N 个线程, 每个 thread 构建 partial hash table; probe side 同样 fan-out
- **`intra-query-parallel-group-by`**: GROUP BY partial aggregates per partition → final merge on main thread
- **`cli-executor-parallelism-flag`**: 新增 `--executor-parallelism=N` CLI flag (默认 1, 保留单线程行为零回归风险)

### 2. 修改能力 (Modified Capabilities)

无 spec-level 行为变化. 现有 `parallel_executor.rs` (134 行 trait shell) 和 `parallel_vector_executor.rs` (757 行 rayon 实现) 已存在, 但生产路径零调用者 — 修复属于 wiring 集成 (impl detail).

### 3. 依赖

- `rayon` 1.x (已在 `parallel_vector_executor.rs` 使用, 无新依赖)
- 现有 `ParallelVolcanoExecutor` trait + `partition_scan` partition 算法
- 现有 `hash_join::multi_way_hash_chain` (issue #3249/#3334 已 ship)

### 4. 不在范围 (Non-Goals)

- ❌ **不重写 Volcano 模型**: 完整 push-based / morsel-driven 重写需 3-4 个月. Pipeline approach 保留 Volcano 核心
- ❌ **不改 ServerThreadPool**: connection-level 并行 (P2-1) 单独 change, 互补不替代
- ❌ **不引入新执行引擎**: ParallelVolcanoExecutor 已存在, 本 change 是 wiring + partition 逻辑
- ❌ **不在 v3.10.0 RC 之前启用**: 默认 `--executor-parallelism=1`, opt-in only

---

## Capabilities

### New Capabilities

- `intra-query-parallel-scan`: 单查询扫描阶段的 partition-based 并行, 覆盖 Scan→Filter→Project→Agg 流水线
- `intra-query-parallel-hash-join`: hash join build/probe 两侧的 partition 并行
- `intra-query-parallel-group-by`: GROUP BY partial aggregate + final merge
- `cli-executor-parallelism-flag`: 新增 `--executor-parallelism=N` CLI flag, 默认 1

### Modified Capabilities

无 — 所有现有 capability spec (binary-prepared-statement-roundtrip, multi-join-3-table-resolution 等) 行为不变, 仅在 `--executor-parallelism>1` 时启用并行.

---

## Acceptance Criteria

| 标准 | 验证方法 |
| --- | --- |
| `--executor-parallelism=1` (默认) 行为零变化 | 25 lib tests + 22 TPC-H 22/22 + 全 integration suite PASS |
| `--executor-parallelism=4` 正确性 100% 一致 | cell-level 比对 N=1 vs N=4, 22/22 TPC-H 一致 |
| 大查询 (SF=0.1) P99 延迟显著下降 | bench 对比 Q1 SF=0.1 N=1 vs N=8 (目标 ≥ 2x speedup) |
| 不引入 regression (正确性/并发 RW) | integration RW test 50 iter, diff vs baseline = 0 |
| CLI flag 通过 clap 集成 | `cargo run --bin sqlrustgo-mysql-server -- --help` 显示新 flag |
| feature gated (避免影响 default build) | `--features parallel-executor` 才能启用 N>1, 默认编译 0 字节增加 |

---

## Risk Assessment

| 风险 | 严重度 | 缓解 |
| --- | --- | --- |
| 触核心执行路径 | HIGH | default N=1, opt-in; 完整 TPC-H + integration 回归 |
| Rayon work-stealing 死锁 | MED | feature-gated, 单测覆盖 |
| StorageEngine 多线程安全 | MED | issue #3680 parking_lot RwLock 已 fix (G13-OLTP-2) |
| 性能 regression 风险 | MED | benchmark gate before/after, 任何 N>1 性能下降 > 5% 立即 revert |
| WAL/事务并行化冲突 | LOW | 本 change 不涉及 WAL 路径 |

---

## Phased Rollout

### Phase 1: Parallel Scan (本 change 重点, ~1 周)
- 接入 `ParallelVolcanoExecutor::partition_scan` 到 `execute_select`
- CLI flag + feature gate
- 22/22 TPC-H 一致性 + bench

### Phase 2: Parallel Hash Join (v3.10.0 Phase 2, ~1-2 周)
- build side partition across threads
- probe side fan-out matching

### Phase 3: Parallel GROUP BY (v3.10.0 Phase 3, ~1 周)
- partial aggregates per partition → final merge

**本 OpenSpec change 覆盖 Phase 1 only**; Phase 2/3 各自独立 change proposal.

---

## Release Target

**v3.10.0 Phase 0/1** (per `V310_DEVELOPMENT_PLAN.md` §4) — M-5 债务闭环.

**NOT v3.10.0 GA gate** — `--executor-parallelism>1` 是 opt-in, 不影响 GA 门禁.

---

## Refs

- Issue #3703 + comment 69559 (SOAK 分析 + Pipeline Executor 架构建议)
- `V310_DEVELOPMENT_PLAN.md` §2.2 M-5 (INT-2 ParallelExecutor 生产路径)
- `V310_ISSUES_PLAN.md` (未列, 需补一个 sub-issue)
- Existing: `crates/executor/src/parallel_executor.rs` (134 行 trait shell)
- Existing: `crates/executor/src/parallel_vector_executor.rs` (757 行 rayon 实现)
- Existing: PR #3199 (INT-2 G2 gate merged in v3.9.0)
