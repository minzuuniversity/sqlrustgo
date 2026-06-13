# openspec/3185 - P3-6 Exchange Operator (Broadcast / Repartition / Gather)

> **Issue**: #3185 (NEW, derived from #3183 §1.2 延后项)
> **作者**: Hermes Agent
> **日期**: 2026-06-14
> **Phase**: 6 (W11-12) — v3.10+
> **工作量**: 8h 本 PR (per #3183 评估) + 6h spill 协同 v3.10.1 + 4h backpressure v3.10.1 = 18h total
> **状态**: 草案 (kickoff for v3.10)
> **前置依赖**: #3183 (P3-4 ParallelExecutor framework, 1974 lines ✅)
> **协同**: #3184 (P3-5 SIMD)
> **变更记录**:
>   - 2026-06-14: kickoff 草案, T0 skeleton commit 0df6f450 (this PR)

## 一、问题分析

### 1.1 背景 — 为什么 v3.9.0 跑了多线程却没"加速"

v3.9.0 的 INT-2 物质测试 (G2: 13/13 PASS) 证明 **parallel_vector_executor 框架可工作**,
但 30-min TPC-H 跑 (2026-06-14 Z440 实测) 显示:

| 现象 | 数据 | 解释 |
|------|------|------|
| CPU 持续 13% | 多 thread 活跃 | framework 在跑 |
| Q9 (6-table join) 7.6s | 重 query 未被加速 | 缺 exchange, 多 partition 结果无法合流 |
| RSS 峰值 3351MB | 一度飙高后回落 | 多 partition 各持中间结果, 但合流靠单 thread merge |
| Q1-Q8 / Q10-Q18 30-60ms | 简单 query 也没并行加速 | 多数 TPC-H query 不触发 parallel path (row 数太少) |

**根因**: framework 完成 partition 阶段后, **partition 间数据合流 (shuffle / broadcast / gather) 没有 operator**, 退化到单 thread merge → 并行无收益。

### 1.2 Exchange Operator 的 3 种语义 (按分布式 DB 文献)

| 模式 | 何时触发 | 数据流向 | SQL 例子 |
|------|----------|----------|----------|
| **Gather** (1→1) | 多个 partition 各产 partial 结果, 单点合流 | All → One | `COUNT(*)` (sum 局部 counts) |
| **Broadcast** (1→N) | 小表 × 大表 JOIN | One → All | Hash join 小 build side |
| **Repartition** (N→N) | JOIN 两边都分区, 按 join key 重分区 | All → All | Hash join 两边都大 |

**Repartition 内部 strategy (CF2 clarifications)**:
- 默认 Hash, 复用 `crates/distributed/src/partition.rs::PartitionStrategy::Hash`
- Range/Key/List 通过 `ExchangeSpec::partition_strategy: PartitionStrategy` 暴露 (v3.10.1)
- v3.10 本 PR 只用 Hash; v3.10.1 加 CBO 选择 (uniform → Range 偏斜检测, skew → Hash)
- 不重复实现 partition 函数, 严格走 `partition.rs` 已有 4 种 strategy (per #3183 §1.1)

参考: Apache Spark Exchange, Apache Flink DataExchange, DuckDB exchange operator。

### 1.3 现状审计 (2026-06-14)

| 现有组件 | 行数 | 状态 | 缺什么 |
|----------|------|------|--------|
| `parallel_vector_executor.rs` | 757 | ✅ 完整 scan/agg/filter parallel | **无 post-partition exchange** |
| `partition.rs` (Hash/Range/Key/List) | 817 | ✅ 4 种 strategy + pruner | Hash pruner 不感知 join key 倾斜 |
| `worker_pool.rs` | 266 | ✅ pool + queue + tuning | 无 backpressure |
| `parallel_executor.rs` | 134 | ✅ 高层 wrapper | 调 scan 后直接 return, 缺 merge 步 |
| `vectorization.rs` (SIMD) | 1552 | ✅ SIMD scan + agg | exchange vector gather 未矢量化 |

**总缺口**:
- ~250 lines 缺 (Exchange operator trait + 3 impls + 7 unit tests + 3 integration)
- ~150 lines 缺 (broadcast cost model)
- ~100 lines 缺 (backpressure)
- ~100 lines 缺 (exchange-vector SIMD gather)

### 1.4 与 #3183 / #3184 的协同

```
[Partition 阶段]   [Exchange 阶段]   [Post-merge 阶段]
parallel_scan →  ExchangeOp  →  sequential merge
                ↑ (本 spec 3185)
[vector SIMD scan] (v3.9.0)   [vector SIMD merge]  (v3.9.0 vectorization.rs)
                ↑ (#3184 SIMD)
```

- **#3184 SIMD**: 加速 partition 内部 scan + agg (单 thread 内)
- **#3185 Exchange**: 跨 partition 重新分布数据 (本 spec)
- **#3183 主体**: 已经实现 partition 阶段, 本 spec 补 exchange 阶段

## 二、实施方案

### 2.1 范围限定 (按治理 §2.1 最小修改 + 复用现有 framework)

**v3.10 本 PR 范围 (8h, per #3183 评估)**:

1. **新文件**: `crates/executor/src/exchange.rs` (250 lines, **T0 skeleton 已 commit 0df6f450 in this PR**)
   - `ExchangeOp` trait
   - `GatherExchange` impl (1→1, default for COUNT/SUM partial)
   - `BroadcastExchange` impl (1→N, for hash join build)
   - `RepartitionExchange` impl (N→N, for hash join both, **inner Hash via PartitionStrategy**)
2. **新文件**: `tests/exchange_test.rs` (200 lines, 12 tests)
3. **新文件**: `tests/exchange_harness.rs` (shared helper, 5 self-tests)
4. **新文件**: `docs/openspec/3185-exchange-operator.md` (本文件, **T0 已 commit c9280566 in this PR**)

**v3.10.1 延后 (本 PR 不动, 各自 spec)**:
- Spill-to-disk 配合 exchange (大结果集临时落盘) — 6h, 单独 spec
- Backpressure (exchange queue depth 限制) — 4h, 单独 spec
- Exchange SIMD gather (跨 partition vector merge) — 3h, 跟 #3184 spec 协调
- CBO exchange cost (CBO 决策) — 5h, v3.11
- `ExchangeSpec::partition_strategy` 显式参数化 (Range/Key/List) — 3h, v3.10.1

### 2.2 ExchangeOp 接口设计

```rust
// crates/executor/src/exchange.rs (T0 commit 0df6f450 — 已落位, v3.10 完善)
pub enum ExchangeMode {
    Gather,        // 1→1, sum/merge partial results
    Broadcast,     // 1→N, replicate small side
    Repartition,   // N→N, hash on join key (默认 Hash, 复用 partition.rs)
}

pub struct ExchangeSpec {
    pub mode: ExchangeMode,
    pub num_partitions: usize,
    pub key: Option<Vec<ColumnRef>>,  // for Repartition
    // v3.10.1:
    // pub partition_strategy: PartitionStrategy,  // CF2 explicit
    // pub spill_threshold_bytes: Option<usize>,   // spill-to-disk
    // pub backpressure_queue_depth: Option<usize>, // backpressure
}

pub trait ExchangeOperator: Send + Sync {
    fn execute(&self, inputs: Vec<Vec<Record>>) -> Result<Vec<Vec<Record>>>;
    fn estimate_cost(&self, input_rows: usize) -> Cost;
}

pub struct GatherExchange;
impl ExchangeOperator for GatherExchange { /* sum/merge partials by AggKind */ }

pub struct BroadcastExchange { /* small side cached, 16MB cap */ }
impl ExchangeOperator for BroadcastExchange { /* replicate */ }

pub struct RepartitionExchange {
    key: Vec<ColumnRef>,
    num_partitions: usize,
    // v3.10: strategy: PartitionStrategy,  // CF2 复用 partition.rs
}
impl ExchangeOperator for RepartitionExchange {
    fn execute(&self, inputs) -> Result<...> {
        // v3.10 实现:
        //   for each row in inputs:
        //     let key_hash = hash(row[key]);
        //     let target_partition = key_hash % num_partitions;
        //     outputs[target_partition].push(row);
        //   return outputs;
    }
}
```

### 2.3 12 Tests (3 类别)

| 类别 | Count | 示例 |
|------|-------|------|
| **Gather** (4) | partial COUNT, partial SUM, empty input, multi-partition | |
| **Broadcast** (4) | small side replicate, large input, key collision, ordering preserved | |
| **Repartition** (4) | hash balance, key preservation, partition count, single-row passthrough | |
| **TOTAL** | **12** | |

### 2.4 G10 Gate (8 checks, 扩展 #3183 现有 7 项)

1. `crates/executor/src/exchange.rs` exists
2. `tests/exchange_test.rs` exists + 12 tests registered
3. `tests/exchange_harness.rs` exists + 5 self-tests
4. 3 类别全覆盖 (Gather / Broadcast / Repartition)
5. cargo check pass (workspace)
6. ≥12 tests pass
7. crates/executor (parallel_vector) + crates/distributed (partition, worker_pool) 仍编译
8. TPC-H 22/22 (G1 维持) — 关键不回归

## 三、风险评估

| 风险 | 影响 | 缓解 |
|------|------|------|
| Exchange 改变并行数据流 | TPC-H 22/22 失败 | default 关 exchange, 仅 opt-in |
| Broadcast 小表无 cost limit | OOM | size > 16MB 拒绝, 走 Repartition |
| Repartition 哈希冲突 | 数据错位 | 严格 hash, test 用确定性 seed |
| 性能 regression | Q1-Q22 变慢 | 仅触发 multi-partition 时启用 |

## 四、验收标准 (G10 门禁, v3.10 阶段)

```
✅ exchange_test: ≥12 tests PASS
✅ 3 类别全覆盖 (Gather / Broadcast / Repartition)
✅ G10 gate: 8/8 PASS
✅ TPC-H 22/22 (G1 维持, regression 0)
✅ v3.9.0 INT-2 物质测试 13/13 仍 PASS
✅ Performance: multi-partition query (e.g. Q9 6-table) ≥ 1.5x 加速 vs sequential
```

## 五、Subsumed Issues

- #3183 §1.2 延后项: exchange operator (Broadcast/Repartition)
- 关联: #3184 (P3-5 SIMD, 加速 exchange gather)
- 协同: spill-to-disk 后续工作 (v3.10.1)

## 六、回滚计划

如 exchange_test 编译失败 或 TPC-H 22/22 regress:
1. 删除 `crates/executor/src/exchange.rs` (新文件, 无 backward compat 风险)
2. 不动 `parallel_vector_executor.rs` (本 spec 不改)
3. G10 gate 新增 check 标 DEFER
4. v3.9.0-rc8+ 维持 exchange-opt-in flag (默认关)

## 七、依赖

**上游**:
- #3183 (P3-4 ParallelExecutor framework, 1974 lines) ✅
- #3171 (P0-3 INT-2 主路径集成) ✅

**下游**:
- #3184 (P3-5 SIMD, exchange gather 矢量化, v3.10.1)
- spill-to-disk 协同 (v3.10.1)
- CBO exchange cost (v3.11)

**不冲突**:
- v3.9.0 GA 当前阻塞 (24h/72h/168h soak) — 本 spec 纯 v3.10 工作, 不影响 v3.9.0 cut

## 八、参考资料

- Issue #3183 §1.2 延后项
- #3183 spec: `docs/openspec/3183-parallel-executor.md` (主路径 + 物质测试)
- #3184 spec: `docs/openspec/3184-simd-integration.md` (SIMD 协同)
- V390_DEVELOPMENT_PLAN.md §P3-6 (新位置)
- V390_TEST_PLAN.md §G10
- crates/executor/src/parallel_vector_executor.rs (757 lines)
- crates/executor/src/parallel_executor.rs (134 lines)
- crates/distributed/src/partition.rs (817 lines)
- crates/distributed/src/worker_pool.rs (266 lines)
- crates/executor/src/vectorization.rs (1552 lines, SIMD scan + agg)
- 外部参考: Apache Spark Exchange, Apache Flink DataExchange, DuckDB exchange operator
- 30min TPC-H 实测 (2026-06-14 Z440, test_results/tpch_30min_20260614_002900/)

## 九、Changelog & 评审记录

### 2026-06-14 — kickoff (T0 commit 0df6f450 + spec c9280566)

**评审 clarifications (resolved)**:
- **CF1 (工作量拆分)**: 3183 spec 估 8h, 3185 原草案写 14h。**resolved**: 8h 是 v3.10 本 PR 范围, 6h spill + 4h backpressure 推 v3.10.1。Total 18h。
- **CF2 (Repartition inner strategy)**: 原草案只说 "hash on key", 未显式复用 `partition.rs`。**resolved**: v3.10 默认 Hash 复用 `crates/distributed/src/partition.rs::PartitionStrategy::Hash`; v3.10.1 加 `partition_strategy: PartitionStrategy` 字段让 CBO 选 (Range/Key/List)。

### T0 落地 (skeleton only, 2h, 415 lines)
- `crates/executor/src/exchange.rs` 完整结构 (ExchangeOp trait + 3 mode + 3 stub impls + 5 unit tests)
- 不动 v3.9.0 行为 (不注册 mod, 仅本文件独立可编译)
- 5 unit tests 覆盖: 3-mode coverage, spec validate (Gather/Repartition), gather concat, repartition round-robin
- 文档更新: 顶部 + §1.2 CF2 + §2.1 8h vs 18h 拆分

### 落地任务清单 (v3.10 启动按此开 PR, 8h 本 PR)

- [ ] T0: `ExchangeOp` trait + skeleton (DONE — 0df6f450)
- [ ] T1: 实现 `GatherExchange` (2h, dispatch by AggKind)
- [ ] T2: 实现 `BroadcastExchange` (2h, enforce 16MB cap)
- [ ] T3: 实现 `RepartitionExchange` (2h, 复用 partition.rs Hash)
- [ ] T4: `tests/exchange_harness.rs` (1h)
- [ ] T5: `tests/exchange_test.rs` 12 tests (1h)
- [ ] T6: G10 gate 扩展 8 项 (1h, 注册到 scripts/gate/check_p34_parallel_executor.sh 现有 G10)
- [ ] T7: TPC-H 22/22 regression 验证 (1h)
- [ ] T8: 跑 Q9 (6-table) 性能对比, 验证 ≥ 1.5x 加速 (1h)

**v3.10.1 followups (单独 spec)**:
- Spill-to-disk (6h)
- Backpressure (4h)
- Exchange SIMD gather (3h, 跟 #3184 协调)
- `ExchangeSpec::partition_strategy` 显式参数化 (3h, CF2)
- CBO exchange cost (5h, v3.11)

**NOT in v3.9.0 GA 路径** — v3.9.0 rc8/rc9 维持 opt-in flag (默认关), v3.10 默认开
