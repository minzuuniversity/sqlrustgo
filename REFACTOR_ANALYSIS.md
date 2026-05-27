# SQLRustGo 源码重构分析报告
> **日期**: 2026-05-28
> **版本**: v3.5.0 (develop/v3.5.0, commit 39a2643e)
> **目标**: 提升 RC 覆盖率至 87%+，同时改善长期可维护性和性能

---

## 一、源码全景分析

### 1.1 项目规模

| 指标 | 数值 |
|------|------|
| 总crates数量 | 59 |
| Rust源文件 | 806 个 |
| 总代码行数 | ~296,812 LOC |
| L1核心crate | 8 个 |
| L1总代码量 | ~88,000 LOC |

### 1.2 各crate测试密度（测试LOC / 源码LOC）

| Crate | 源码LOC | 测试LOC | 测试比 | 覆盖率 | 优先级 |
|-------|---------|---------|--------|--------|--------|
| parser | 18,145 | 17,520 | **96.6%** | 78.18% | 🔴 瓶颈 |
| sync | 976 | 286 | 29.3% | — | 🟡 |
| server | 9,454 | 2,402 | 25.4% | — | 🟡 |
| optimizer | 8,123 | 1,969 | 24.2% | 83.67% | 🟡 |
| transaction | 8,834 | 1,345 | 15.2% | 87.79% | ✅ |
| mysql-server | 3,821 | 567 | 14.8% | 33.57% | ⚠️ 非L1 |
| wal-verification | 1,025 | 206 | 20.1% | — | 🟡 |
| **executor** | **31,224** | **11,608** | **37.2%** | **83.00%** | 🔴 最大风险 |
| **storage** | **33,026** | **127** | **0.4%** | **81.75%** | 🔴 致命缺口 |

### 1.3 关键发现

```
【发现1】storage 测试缺口极其严重：33,026 LOC 的代码只有 127 LOC 测试（0.4%）
         这是整个 L1 体系的致命弱点，远比 parser 更需要紧急处理。

【发现2】parser 已有 17,520 LOC 测试（比源码还多），但覆盖率只有 78.18%
         原因：tests_preprocessed/ 下有 8 个 auto-generated 覆盖测试文件
         这些文件可能覆盖了大量边界情况但仍然有遗漏

【发现3】executor 单文件 31,224 LOC，是最大的代码单体
         包含 8 个子模块（stored_proc, trigger_eval, test_framework 等）
         虽有一定测试但覆盖率仅 83%，stored_proc/execution.rs 仅 65.77%

【发现4】planner 8,933 LOC，零独立测试文件（0%外部测试）
         依赖 optimizer 的集成测试覆盖

【发现5】types 1,817 LOC，零外部测试文件
         依赖其他crate的集成测试
```

---

## 二、模块耦合度分析

### 2.1 依赖关系（基于Cargo.toml）

```
mysql-server
  └── gmp-api → gmp-retrieval → gmp → llm
  └── executor → optimizer → planner → parser → types
  └── storage
  └── catalog
  └── transaction

executor
  ├── storage (trait StorageEngine)
  ├── catalog (TableCache)
  ├── optimizer (PlanHandle)
  ├── transaction (TransactionCtx)
  └── types

optimizer
  ├── parser (ParsedQuery)
  ├── types
  └── storage (Statistics)

storage (33,026 LOC — 最大单体)
  ├── types
  └── 内部子模块：mvcc, buffer_pool, file_engine, sstable, wal

planner
  └── parser → types

catalog
  ├── storage
  └── types
```

### 2.2 循环依赖风险

```
⚠️ 潜在循环：optimizer ↔ storage
  optimizer 需要 storage 的统计信息 (storage::Statistics)
  storage 的查询优化需要 optimizer 的 cost model

✅ 当前状态：通过 trait 抽象解耦，无硬循环
```

---

## 三、重构方案

### 方案A：渐进式优化（推荐，3-6个月）

**核心理念**：不改变模块边界，通过**增加测试密度**和**提取可测试单元**提升覆盖率，同时为未来彻底重构埋下伏笔。

#### A1. 紧急：修复 storage 测试缺口（1个月）

```
目标：将 storage 测试比从 0.4% 提升到 15%+

行动：
  1. 为 storage 的核心 trait (StorageEngine, TableHandle, Transaction) 编写单元测试
  2. 提取 MVCC、BufferPool、SSTable 为独立内部模块（不改边界，只改组织）
  3. 为每个内部模块创建 tests/ 目录

预期提升：storage 覆盖率 81.75% → 87%+（+5.25%对总平均贡献）
```

#### A2. 高优先级：parser 覆盖深化（2周）

```
目标：parser 覆盖率 78.18% → 85%+

已有优势：96.6% 测试比，说明测试基础设施完善
瓶颈：parse_create_trigger, parse_create_event, parse_sign_record

行动：
  1. 分析 llvm-cov 未覆盖的 3805 个 region（重点在 parser.rs）
  2. 在 tests/ 下新增 dedicated tests for DDL statements
  3. 复用已有的 coverage_increase_v*.rs auto-gen 模式

预期提升：parser 6.82% gap → 0（L1平均 +1.7%）
```

#### A3. 中优先级：executor 模块化（1个月）

```
目标：executor 覆盖率 83.00% → 87%+

行动：
  1. 将 stored_proc/execution.rs 的 3195 行分解为：
     - stored_proc/execution/jump_table.rs（控制流）
     - stored_proc/execution/memory.rs（内存管理）
     - stored_proc/execution/native_calls.rs（内置函数调用）
  2. 为每个子模块编写独立测试
  3. 通过 `#[cfg(test)]` 模块在每个子文件内建立单元测试

预期提升：executor +4%（L1平均 +0.8%）
```

#### A4. 持续：planner/types 测试补全（2周）

```
planner（0%外部测试）→ 目标 20%+ 测试比
types（0%外部测试）→ 目标 10%+ 测试比
```

#### 渐进式总收益预估

| Crate | 当前覆盖率 | 目标覆盖率 | 提升 | 对L1平均贡献 |
|-------|-----------|-----------|------|-------------|
| storage | 81.75% | 87.00% | +5.25% | **+1.75%** |
| parser | 78.18% | 85.00% | +6.82% | **+1.70%** |
| executor | 83.00% | 87.00% | +4.00% | **+0.80%** |
| planner | ~85% | 88.00% | +3.00% | **+0.30%** |
| optimizer | 83.67% | 86.00% | +2.33% | **+0.47%** |
| **L1平均** | **84.99%** | **88.50%** | **+3.51%** | |

**结论：渐进式可将 L1 平均从 84.99% 提升至 88.5%+，稳超 85% 门槛**

#### 风险评估（渐进式）

| 风险 | 概率 | 影响 | 缓解 |
|------|------|------|------|
| 改测试破坏现有CI | 低 | 高 | 每步PR独立验证，revert成本低 |
| storage内部模块化失败 | 中 | 低 | 仅重组不改变API |
| parser auto-gen测试质量 | 低 | 低 | 已有96.6%基础设施 |

---

### 方案B：彻底重构（6-18个月）

**核心理念**：按性能优化的架构原则，重新划分模块边界，实现真正的关注点分离。

#### B1. storage 彻底拆分（6个月）

```
当前问题：33,026 LOC 单一crate，0.4%测试

重构方案：
  storage/
  ├── storage-core/         (核心trait和接口, ~3000 LOC)
  │   ├── StorageEngine trait
  │   ├── TableHandle trait
  │   └── Transaction trait
  ├── storage-mvcc/         (多版本并发控制, ~8000 LOC)
  ├── storage-buffer/       (BufferPool, ~5000 LOC)
  ├── storage-sstable/      (SSTable + Compaction, ~8000 LOC)
  ├── storage-wal/          (Write-Ahead Log, ~3000 LOC)
  └── storage-engine/        (文件系统层, ~6000 LOC)

收益：
  1. 每个子crate可独立测试（测试密度自然提升）
  2. 编译时间大幅缩短（增量编译）
  3. 性能：消除 storage 层内跨模块调用开销
  4. 覆盖率：子crate独立测试 → 总覆盖率 85%+

风险：
  - 极高：storage 是核心依赖，重构期间阻断所有功能开发
  - 需要完整的 API 兼容性测试套件
  - 数据库迁移/兼容层工作量大
```

#### B2. executor 管道化重构（4个月）

```
当前问题：31,224 LOC 单体，stored_proc/execution.rs 65.77%

重构方案：
  executor/
  ├── executor-core/        (基础executor trait, ~2000 LOC)
  ├── executor-physical/    (PhysicalPlan → Pipeline IR, ~10000 LOC)
  ├── executor-vectorized/  (向量化执行引擎, ~8000 LOC)
  ├── executor-spark/       (Spark-like任务调度, ~5000 LOC)
  └── executor-debug/       (EXPLAIN/TRACE工具, ~3000 LOC)

关键重构点：
  1. 将 stored_proc 从顺序执行改为 jump-table 派发表
  2. 提取 CTE/Trigger 为独立执行 Pass
  3. Pipeline IR 中间表示支持并行优化
```

#### B3. parser 编译器式重构（3个月）

```
当前问题：parser.rs 9000+ 行，手写解析器难以维护

重构方案：
  parser/
  ├── lexer/               (词法分析器, ~2000 LOC) ✅ 已独立
  ├── parser-lalr/         (用 lalrpop 或 peg 替代手写解析)
  │   └── grammar.pest     (PEG 语法定义)
  └── parser-ast/          (AST定义, ~3000 LOC)

收益：
  1. 解析器正确性由 grammar 定义保证
  2. 新语法规则添加成本大幅降低
  3. 自动覆盖 grammar 所有分支（解析器覆盖率 ≈ grammar 覆盖率）
```

#### B4. 新增性能优化层（3个月）

```
新增 unified-query 层（当前已有 unified-query crate）：
  ├── QueryCache (热点查询结果缓存)
  ├── AdaptiveOptimizer (基于执行反馈的自适应优化)
  └── DistributedPlanner (跨节点查询规划)
```

#### 彻底重构总收益预估

| 维度 | 方案A | 方案B |
|------|-------|-------|
| L1覆盖率 | 88.5%+ | 92%+ |
| 编译时间 | 不变 | -40%（增量编译）|
| 执行性能 | 不变 | +15-30% |
| 测试维护性 | 中等 | 优秀 |
| **总工期** | **3-6个月** | **12-18个月** |
| **阻断业务** | 无 | **12-18个月** |
| **风险等级** | 🟡 中低 | 🔴 极高 |

---

## 四、推荐路径：渐进式 + 关键模块局部重构

### 阶段划分

```
Phase 0（紧急，1周）
  ├── 修复 storage 的 0.4% 测试缺口
  ├── 添加 storage/txn_manager.rs 测试
  └── 验证 storage 覆盖率 +5%

Phase 1（高优先级，2周）
  ├── parser 覆盖率深化（78% → 85%）
  ├── executor stored_proc 模块化（不改变边界）
  └── optimizer/rules.rs 覆盖（60% → 75%）

Phase 2（中优先级，1个月）
  ├── storage 子模块提取（MVCC/BufferPool 独立tests/）
  ├── planner 独立测试文件
  └── types 独立测试文件

Phase 3（持续改进，3个月）
  ├── executor Pipeline IR 设计
  ├── TPC-H 性能基线建立
  └── Adaptive Optimizer 原型
```

### 关键决策点

| 问题 | 建议 | 理由 |
|------|------|------|
| storage 重构还是加测试？ | 先加测试，再考虑重构 | storage 是核心依赖，重构风险极高 |
| parser 用 grammar 重写？ | 否，追加测试 | 18,145 LOC 已有96.6%测试比，重写不划算 |
| executor 要不要拆 crate？ | 暂不，只在内部模块化 | 拆crate会引入跨crate引用复杂性 |
| 性能优化何时启动？ | Phase 3 | 覆盖率达标后，建立基线再优化 |

---

## 五、TPC-H 性能分析（SF=1 实测）

```
Q1:  41859ms > 30000ms ❌ FAIL (冷启动首次执行)
Q6:  20987ms > 15000ms ❌ FAIL (冷启动)
Q13: 8503ms + 150000 rows ⚠️  (复杂JOIN)

22条查询全部执行完毕，总耗时 346,008ms
性能瓶颈：
  1. 无查询缓存预热
  2. executor stored_proc 顺序执行（无 pipeline）
  3. storage 无 SIMD 批量读取
```

---

## 六、文件结构优化建议（不重构代码，只重组）

```
优化前：
  crates/
  ├── executor/src/
  │   ├── stored_proc/execution.rs    (3195行 🔴)
  │   ├── stored_proc/expression.rs  (4801行 🟡)
  │   ├── trigger_eval/              (散落4个文件)
  │   └── lib.rs                     (imports混乱)

优化后（仅文件重组，无API变更）：
  crates/executor/src/
  ├── execution/                     (新目录，分组)
  │   ├── mod.rs                     (导出所有execution子模块)
  │   ├── jump_table.rs              (从execution.rs提取)
  │   ├── memory.rs                  (从execution.rs提取)
  │   ├── native_calls.rs            (从execution.rs提取)
  │   └── execution.rs               (精简为协调器)
  ├── stored_proc/
  │   ├── mod.rs
  │   ├── context.rs
  │   ├── cte.rs
  │   ├── cursor.rs
  │   └── expression.rs              (4801行 → 可进一步拆分)
  ├── trigger/
  │   ├── mod.rs
  │   ├── eval/                      (现有 trigger_eval)
  │   └── trigger.rs
  └── lib.rs                         (只保留 re-export)

收益：
  1. 文件导航更清晰（IDE导航效率+30%）
  2. 每个子文件可独立编写 #[cfg(test)]
  3. 为未来独立测试crate打下基础
  4. 零风险：仅移动代码，不改变逻辑
```

---

## 七、总结

| 方案 | 覆盖率目标 | 工期 | 业务影响 | 风险 |
|------|-----------|------|---------|------|
| **A（渐进式）** | **88.5%+** | **3-6月** | **无** | 🟡 中低 |
| B1（storage彻底重构） | 92%+ | 6月 | 全部 | 🔴 极高 |
| B2（executor管道化） | 90%+ | 4月 | 高 | 🔴 高 |
| **B（彻底重构总计）** | **95%+** | **12-18月** | **12-18月阻断** | 🔴 极高 |

**建议**：采用方案A（渐进式）+ 对executor内部文件结构进行重组（仅移动代码），3-6个月内覆盖率从84.99%提升至88.5%+，同时为未来彻底重构保存完整的测试基础设施。
