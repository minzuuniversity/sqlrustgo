# 并行执行器优化分析报告

**Issue:** #3792 — 并行 vs 串行 SOAK 对比测试
**Related:** Issue #3703, Issue #3776, F-36
**Platform:** gaoyuan (Intel Xeon E5-2680 v4, 28 cores, 94 GB RAM)
**Commit:** `6d7ffdfa` (develop/v3.10.0)
**Date:** 2026-07-13

---

## 一、问题背景

### 基准测试结论

在三个数据规模（SF=0.1、SF=1.0、SF=10.0）上，并行执行均未显示出实质性加速：

| 规模 | 行数 | 并行 4x 加速比 | 并行 8x 加速比 |
|------|------|--------------|--------------|
| SF=0.1 | ~10K | 1.01x | 1.01x |
| SF=1.0 | ~100K | 1.00x | 1.00x |
| SF=10.0 | ~1M | 1.01x | — |

**核心结论：** 即使在 1M 行数据、28 核的条件下，并行执行也没有显著加速。瓶颈不在数据规模，而在架构本身。

---

## 二、根因分析

### 2.1 代码层发现的问题

#### 问题 A：`PARALLEL_MIN_ROWS` 在三处定义，值不一致

| 位置 | 值 | 用途 |
|------|-----|------|
| `crates/executor/src/parallel_executor.rs:9` | **100,000** | 执行器分区阈值 |
| `crates/optimizer/src/unified_cost.rs:10` | **500,000** | CBO 代价模型 |
| `crates/storage/src/engine.rs:688` | **500,000** | 存储层分区阈值 |
| `crates/storage/src/file_storage.rs:2464` | **500,000** | FileStorage 分区阈值 |

**影响：** CBO 认为需要 500K 行才值得并行，但执行器在 100K 行就触发并行，导致 optimizer 和 executor 的决策不一致。

#### 问题 B：行级任务调度开销大

当前 `execute_parallel_filter` 对每一行单独调用 Rayon 的 `map()`：

```rust
partitions
    .into_iter()
    .map(|partition: Vec<Record>| partition.into_iter().filter(predicate).collect())
    .collect();
```

Rayon 为每个 partition 创建一个任务，但对于 1M 行数据，每行仍然有迭代器展开的开销。

#### 问题 C：无并行前置开销估算

触发并行后，无论数据规模多小，都会执行完整的分区 + Rayon 调度 + 结果合并流程。没有"预估 I/O 时间 vs 调度开销"的前置判断。

#### 问题 D：无自适应并行度选择

无论数据规模是 100K 还是 10M，都使用相同的并行度（由外部传入），没有根据数据量动态调整。

#### 问题 E：缺少性能埋点

`execute_parallel_filter` 没有输出 `setup_ms`、`partition_ms`、`merge_ms`、`compute_ms`，无法精确诊断瓶颈位置。

---

## 三、v3.10.0 可落地优化项

### 3.1 高优先级（低投入、快速落地）

#### 优化 1 ✅：统一并提高 `PARALLEL_MIN_ROWS` 阈值

**当前问题：** 三处值不一致（100K vs 500K），导致 CBO 和执行器决策矛盾。

**修复方案：** 将 executor 的阈值从 100K 提升至 **2,000,000**（2M 行），与 optimizer 和 storage 保持逻辑一致，同时大幅提高触发门槛。

| 文件 | 修改 |
|------|------|
| `crates/executor/src/parallel_executor.rs:9` | `100_000` → `2_000_000` |
| `crates/optimizer/src/unified_cost.rs:10` | `500_000` → `2_000_000`（同步调整） |

**预期收益：** 消除 SF=1.0（100K 行）场景下并行开销 > 收益的性能倒退。

---

#### 优化 2 ✅：增加并行触发前置判断

**当前问题：** 无条件触发并行执行，即使预估收益为负。

**修复方案：** 在 `execute_parallel_filter` 入口增加开销估算：

```rust
pub fn execute_parallel_filter(
    &self,
    rows: Vec<Record>,
    predicate: &dyn Fn(&Record) -> bool,
    parallelism: usize,
) -> SqlResult<Vec<Record>> {
    let setup_overhead_ns = rows.len() * 50; // 估算：每行 50ns 分区开销
    let estimated_io_time_ns = rows.len() * 200; // 估算：每行 200ns I/O 时间
    let parallel_benefit_ns = estimated_io_time_ns * (parallelism - 1) / parallelism;

    // 仅在预估收益 > 2x 开销时触发并行
    if parallel_benefit_ns <= 2 * setup_overhead_ns {
        // 回退到串行
        let filtered: Vec<Record> = rows.into_iter().filter(predicate).collect();
        return Ok(filtered);
    }
    // ... 原有并行路径
}
```

**预期收益：** 杜绝小数据集上的无效并行，自动回退串行。

---

#### 优化 3 ✅：增加性能埋点到 EXPLAIN ANALYZE

**当前问题：** 无法精确诊断并行路径各阶段耗时。

**修复方案：** 在 `execute_parallel_filter` 和 `partition_scan` 中增加结构化计时输出：

```rust
#[derive(Debug, Clone)]
pub struct ParallelExecutionStats {
    pub setup_ms: f64,
    pub partition_ms: f64,
    pub filter_ms: f64,
    pub merge_ms: f64,
    pub total_ms: f64,
    pub rows_input: usize,
    pub rows_output: usize,
    pub parallelism: usize,
}
```

修改 `partition_scan` 的 `#[instrument]` 属性以输出各阶段耗时。

**预期收益：** 为后续优化提供精确数据支撑。

---

### 3.2 中优先级（依赖现有框架能力）

#### 优化 4 🔧：降低任务调度粒度（Batch-Parallel）

**当前问题：** 行级迭代器在 Rayon 中调度开销大。

**修复方案：** 将分区数据按 **8,192 行一批** 处理，而非逐行迭代：

```rust
// 替代方案：batch filter using rayon
let batch_size = 8192;
let filtered: Vec<Record> = partitions
    .iter()
    .flat_map(|partition: &Vec<Record>| {
        partition
            .chunks(batch_size)
            .flat_map(|chunk| chunk.iter().filter(|r| predicate(r)).collect::<Vec<_>>())
            .collect::<Vec<_>>()
    })
    .collect();
```

**预期收益：** 降低 Rayon 调度次数，对 100 万行以上场景可能有 5-15% 改善。

---

#### 优化 5 🔧：利用 Rayon 工作窃取配置

**当前问题：** 28 核全量竞争导致内存带宽饱和和 NUMA 跨路延迟。

**修复方案：** 通过环境变量 `RAYON_NUM_THREADS` 或线程池配置，限制并行查询可用线程数：

```rust
// 在并行执行入口设置
std::env::set_var("RAYON_NUM_THREADS", rayon_threads_for_query(rows.len()));
```

动态公式：`threads = min(4, rows / 500_000).max(1)`

**预期收益：** 降低内存带宽争用，减少跨 NUMA 节点延迟。

---

#### 优化 6 🔧：自适应并行度选择

**当前问题：** 无论数据规模，都使用外部传入的并行度。

**修复方案：** 根据输入数据量动态选择并行度：

```rust
fn adaptive_parallelism(rows: usize) -> usize {
    match rows {
        r if r < 500_000 => 1,    // 不值得并行
        r if r < 2_000_000 => 2,  // 小规模：2 线程
        r if r < 5_000_000 => 4,  // 中等规模：4 线程
        _ => 8,                   // 大规模：8 线程（28 核留 20 给系统）
    }
}
```

**预期收益：** 避免小数据集过度并行，大数据集合理利用硬件。

---

## 四、架构问题（超出 v3.10.0 范围）

以下问题需要存储引擎层或执行引擎架构级改造，不在当前版本范围内：

1. **独立 I/O 通道**：多存储设备支持，需要数据分布策略改造
2. **向量化并行执行**：需要重新设计执行引擎执行模型
3. **异步 I/O + 预取**：依赖存储层接口变更

---

## 五、行动项

| # | 优化项 | 负责人 | 状态 | 预计工时 |
|---|--------|--------|------|---------|
| 1 | 统一并提高 PARALLEL_MIN_ROWS 至 2M | — | 待开始 | 0.5h |
| 2 | 增加并行触发前置判断 | — | 待开始 | 1h |
| 3 | 增加性能埋点（EXPLAIN ANALYZE） | — | 待开始 | 1h |
| 4 | Batch-Parallel 任务调度 | — | 待开始 | 2h |
| 5 | Rayon 线程数动态配置 | — | 待开始 | 0.5h |
| 6 | 自适应并行度选择 | — | 待开始 | 1h |

**v3.10.0 验证计划：**
- 用 SF=10.0（1M 行）和 SF=30（3M 行）验证优化组合效果
- 目标：达到 **1.1x-1.2x 加速比**

---

*Generated based on Issue #3792 benchmark results — 2026-07-13*
