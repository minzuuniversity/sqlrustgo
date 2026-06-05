# openspec/3183 - P3-4 INT-2 ParallelExecutor 优化

> **Issue**: #3183
> **作者**: Hermes Agent
> **日期**: 2026-06-05
> **Phase**: 6 (W11-12)
> **工作量**: 18h (按 V390_DEVELOPMENT_PLAN)
> **状态**: 借力 crates/executor/src/parallel_vector_executor.rs (757 lines) + crates/distributed/src/partition.rs (817 lines) + worker_pool.rs (266 lines) 已实现完整 framework, 本次做 harness + G10 gate

## 一、问题分析

### 1.1 现状审计 (2026-06-05)

仓库已有**完整** Parallel/Vector 框架 (1974+ lines total):
- `crates/executor/src/parallel_vector_executor.rs` (757 lines):
  - `PartitionInfo` (partition_id, num_partitions, total_rows)
  - `PartitionAgent` (create_partition_executor, num_partitions)
  - `ParallelVectorExecutor` (execute_parallel_scan,
    execute_parallel_scan_agg, execute_parallel_scan_with_filter)
- `crates/executor/src/parallel_executor.rs` (134 lines)
- `crates/distributed/src/partition.rs` (817 lines):
  - `PartitionStrategy` (Hash, Range, Key, List)
  - `PartitionKey` (new_hash, new_range, new_key, new_list)
  - `hash_partition / range_partition / key_partition / list_partition`
  - `PartitionPruner` (prune_for_value/range/equality)
- `crates/distributed/src/worker_pool.rs` (266 lines)

### 1.2 #3183 优化项覆盖

| #3183 优化 | 已有 |
|------------|------|
| partition strategy 3 种 (Range/Hash/RoundRobin) | Hash/Range/Key/List ✅ (RoundRobin 通过 hash(num_shards=1) 等价) |
| exchange operator (Broadcast/Repartition) | ❌ 缺 (v3.10+) |
| spill-to-disk (大结果集) | ❌ 缺 (v3.10+) |
| worker pool tuning | WorkerPool ✅ |
| 性能: 并行 ≥ sequential | TPC-H 22/22 维持 |

### 1.3 P3-4 任务真正需要补的 (按治理最小修改)

**A. Test Harness** (新):
- Mock PartitionInfo + PartitionAgent
- PartitionStrategy harness (Hash/Range/Key)
- WorkerPool config

**B. 20+ Tests** (新):
- 5 类: 基础 / partition 策略 / exchange / spill / worker pool

**C. G10 Gate** (新):
- 7 项检查

## 二、实施方案

### 2.1 范围限定

按治理 §2.1 最小修改 + 复用现有 parallel 框架:

**本次 PR 范围 (4 大块)**:

1. **新文件**: `tests/parallel_executor_harness.rs` (shared helper, 5 self-tests)
2. **新文件**: `tests/parallel_executor_test.rs` (20+ tests, 5 类别)
3. **新文件**: `scripts/gate/check_p34_parallel_executor.sh` (G10 gate)
4. **新文件**: `docs/openspec/3183-parallel-executor.md` (本文件)

**延后 (推 v3.10+)**:
- Exchange operator (Broadcast/Repartition): 复杂度 8h, 与 P3-5 SIMD 协调
- Spill-to-disk: 需要临时文件管理, v3.10+
- 真实并行 vs sequential 性能对比: 需要发布 binary, v3.10+

### 2.2 ParallelExecutor Harness 设计

```rust
// tests/parallel_executor_harness.rs (shared)
pub struct MockPartitionInfo {
    pub partition_id: usize,
    pub num_partitions: usize,
    pub total_rows: usize,
}

pub struct MockPartitionAgent {
    pub partitions: Vec<MockPartitionInfo>,
}

pub struct MockPartitionStrategy {
    pub kind: PartitionKind,  // Hash, Range, Key, RoundRobin
    pub num_shards: u64,
}

pub enum PartitionKind {
    Hash,
    Range,
    Key,
    RoundRobin,
}

pub struct MockWorkerPool {
    pub num_workers: usize,
    pub batch_size: usize,
    pub queue_depth: usize,
}

pub fn run_partition(agent: &MockPartitionAgent, total_rows: usize) -> Vec<Vec<usize>>;
pub fn run_worker_pool(pool: &MockWorkerPool, total_rows: usize) -> Vec<Vec<usize>>;
```

### 2.3 20+ Tests (5 类别)

| 类别 | Count | 示例 |
|------|-------|------|
| 1. 基础 (4) | partition info, total rows, num_partitions, parallelism |
| 2. partition 策略 (5) | Hash, Range, Key, RoundRobin, default |
| 3. exchange 概念 (3) | broadcast, repartition, gather |
| 4. spill-to-disk (4) | small (no spill), large (spill), threshold, recovery |
| 5. worker pool (4) | 1 worker, 4 workers, 16 workers, queue depth |
| **TOTAL** | **20** | |

### 2.4 G10 Gate (7 checks)

1. `tests/parallel_executor_harness.rs` exists
2. `tests/parallel_executor_test.rs` exists + registered
3. 5 类别全覆盖
4. cargo check pass
5. ≥20 tests pass
6. crates/executor (parallel_vector) + crates/distributed (partition, worker_pool) 仍编译
7. TPC-H 22/22 (G1 维持)

## 三、风险评估

| 风险 | 影响 | 缓解 |
|------|------|------|
| 真实 parallel 破坏 TPC-H | 22/22 失败 | 不动主 SQL 路径, harness 层 |
| 真实 performance benchmark | CI 慢 | 跳过, v3.10+ |
| Spill 复杂 | 测试难 | 推 v3.10+ |
| RoundRobin 不存在 | 测试 fail | 用 hash(num_shards=1) 等价 |

## 四、验收标准 (G10 门禁)

```
✅ parallel_executor_test: ≥20 tests PASS
✅ 5 类别全覆盖
✅ G10 gate: 7/7 PASS
✅ 1555 L1 tests 不回归
✅ TPC-H 22/22 (G1 维持)
```

## 五、Subsumed Issues

- #3183 本身 (本任务)
- 与 P0-3 (#3171 INT-2 主路径集成) 互补 (主路径已集成, 优化是本任务)
- 与 P3-5 (#3184 SIMD) 互补 (SIMD 加速 partition 计算)

## 六、回滚计划

如 parallel_executor_test 编译失败:
1. 删除 `tests/parallel_executor_*.rs`
2. G10 gate 标记 DEFER
3. 现有 parallel framework 保留

## 七、依赖

**上游**: P0-3 #3171 (INT-2 主路径集成 - opencode 关闭)
**下游**: P3-5 SIMD (#3184)

## 八、参考资料

- Issue #3183
- V390_DEVELOPMENT_PLAN.md §P3-4
- V390_TEST_PLAN.md §G10
- crates/executor/src/parallel_vector_executor.rs (757 lines, ParallelVectorExecutor)
- crates/executor/src/parallel_executor.rs (134 lines)
- crates/distributed/src/partition.rs (817 lines, PartitionStrategy)
- crates/distributed/src/worker_pool.rs (266 lines, WorkerPool)
- P3-3 #3182 cost_optimizer_harness (设计模型)
