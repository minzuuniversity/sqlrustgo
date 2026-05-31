# ARCHITECTURE_DECISIONS.md — v3.8.0

> **版本**: v3.8.0  
> **类型**: Architecture Decision Records（架构决策记录）  
> **用途**: 记录 v3.8.0 架构重构的关键决策，供后续版本追溯  
> **维护者**: Hermes Agent  
> **创建日期**: 2026-05-30  
> **状态**: Active  

---

## 概述

本文档记录 v3.8.0 Architecture Unification 的关键架构决策（AD-001 ~ AD-005）。每个决策包含：

- **Status**: 决策状态
- **Context**: 问题背景
- **Decision**: 最终决策
- **Alternatives**: 考虑的替代方案
- **Consequences**: 决策后果

**ADR 一旦 accepted 不允许修改历史，只允许 supersede。**

---

## AD-001: ExecutionEngine 拆分

### Status

**Accepted** — v3.8.0 Phase 0

### Context

v3.7.0 中 `execution_engine.rs` 已膨胀至 6829 行（v3.4 的 4658 行增长 47%）。这个文件包含：
- SQL 解析逻辑
- 查询优化逻辑
- 执行逻辑
- 存储调用
- 事务管理
- 测试 harness

单一文件承担太多职责，导致：
- 可维护性差（6000+ 行文件难以阅读）
- 测试困难（难以 mock 依赖）
- 并行开发冲突（多人需要同时修改此文件）
- 架构债务累积（AV-001~AV-007 都是在此文件中）

### Decision

**将 ExecutionEngine 拆分为多个专门模块**：

```
ExecutionEngine (Router/Coordinator)
  ├── Parser (sqlrustgo-parser)
  ├── Planner (sqlrustgo-planner)
  ├── LocalExecutor (sqlrustgo-executor)
  ├── TransactionManager (sqlrustgo-transaction)
  └── StorageEngine (sqlrustgo-storage)
```

**PR-800 阶段目标**: 入口重构，删除 `eng.execute(raw_sql)`，改为调用 Parser → Planner → LocalExecutor。

**PR-900 最终目标**: ExecutionEngine 降至 <1500 行。

### Alternatives Considered

| Alternative | 描述 | 拒绝原因 |
|-------------|------|----------|
| 保留单体 | 不拆分，继续在 execution_engine.rs 添加代码 | 可维护性不可接受 |
| 完全重写 | 从头重写执行引擎 | 风险太高，6 个月延迟 |
| 渐进拆分 | 按 PR DAG 逐步迁移功能 | ✅ 采用，PR-800~PR-900 |

### Consequences

**Positive**:
- ExecutionEngine 职责单一（路由协调）
- 每个模块可独立测试
- 并行开发可行（PR-810 由一人负责，其他人负责其他 PR）
- 架构债务可追踪（AV-001~AV-010 可逐步修复）

**Negative**:
- 拆分过程需要多 PR（PR-800~PR-900）
- 期间可能有临时架构妥协
- 测试需要重新适配新路径

---

## AD-002: Single-path Execution

### Status

**Accepted** — v3.8.0 Phase 0

### Context

v3.7.0 存在双执行路径：

**路径 A（Legacy）**:
```
eng.execute(raw_sql) → 直接调用 storage
```

**路径 B（Target）**:
```
Parser → AST → Planner → PhysicalPlan → LocalExecutor → StorageEngine
```

AV-001~AV-007 的根因就是路径 A：多个执行器直接调用 `storage.insert/delete/update`，绕过事务管理层。

### Decision

**统一为单一执行路径**：

```
mysql-server
  └── COM_QUERY
        ├── Parser::parse(sql)      ← 必须经过
        ├── Planner::plan(ast)     ← 必须经过
        ├── LocalExecutor::execute(plan) ← 必须经过
        └── StorageEngine          ← 仅通过 Executor 访问
```

**删除**: `eng.execute(raw_sql)` 直接执行路径
**保留**: Parser/Planner/LocalExecutor/StorageEngine 标准模块

### Alternatives Considered

| Alternative | 描述 | 拒绝原因 |
|-------------|------|----------|
| 保留双路径 | 路径 A 用于简单查询，路径 B 用于复杂查询 | 维护两套路径成本高 |
| 路径 A 优先 | 简单 SQL 用路径 A，复杂 SQL 用路径 B | Parser/Planner 无法优化简单 SQL |
| 路径 B 唯一 | 所有 SQL 必须经过路径 B | ✅ 采用，PR-800 删除路径 A |

### Consequences

**Positive**:
- 所有 SQL 享受 Parser 解析的好处（AST 生成）
- 所有 SQL 享受 Planner 优化的好处（PhysicalPlan）
- 架构违规（AV-001~AV-007）可统一修复
- 单路径简化测试策略

**Negative**:
- 简单 SELECT 的延迟可能略有增加（Parser 开销）
- 现有调用 `eng.execute(raw_sql)` 的代码需要迁移

---

## AD-003: WAL First

### Status

**Accepted** — v3.8.0 Phase 1

### Context

INT-1（v3.7.0 GA 遗留）: DML 操作直接写入 storage，跳过 WAL。

当前（v3.7.0）:
```
INSERT INTO t VALUES(...)
  ↓
eng.execute_insert()  ← 直接调用 storage.insert()
  ↓
无 WAL 记录
  ↓
crash → 数据丢失
```

期望（v3.8.0）:
```
INSERT INTO t VALUES(...)
  ↓
eng.execute_insert()
  ↓
TransactionManager::begin()
  ↓
WriteBuffer::stage(dml)
  ↓
WAL::append(dml)
  ↓
COMMIT → StorageEngine::apply() + WAL::flush()
  ↓
crash → WAL replay → 数据恢复
```

### Decision

**WAL 集成是 PR-830/840 的核心目标，优先于其他事务功能**：

```
PR-820: TransactionManager Session Binding
PR-830: WAL + WriteBuffer 接入
PR-840: DML Transaction Interception
```

**WAL 优先于 MVCC**: 先确保 DML 有 WAL 记录（crash recovery），再实现 MVCC（并发读）。

### Alternatives Considered

| Alternative | 描述 | 拒绝原因 |
|-------------|------|----------|
| MVCC First | 先实现 MVCC，再实现 WAL | WAL 是 MVCC 的基础，不能跳过 |
| 不实现 WAL | 依赖 Storage 层的 crash recovery | INT-1 跨越 7 版本，必须解决 |
| WAL 延迟到 v3.9.0 | v3.8.0 先完成执行路径统一 | WAL 是 ACID 的基础，v3.8.0 必须完成 |

### Consequences

**Positive**:
- DML 操作有 WAL 记录，crash recovery 可用
- TransactionManager 可拦截 DML
- 为 MVCC 奠定基础

**Negative**:
- WriteBuffer 引入额外内存开销
- WAL 写入可能影响写性能（但可优化）

---

## AD-004: Planner Consolidation

### Status

**Accepted** — v3.8.0 Phase 2

### Context

v3.7.0 Planner 存在以下问题：

1. **双物理计划路径**: `plan_select()` 和 `create_physical_plan()` 并存
2. **PhysicalPlan 双出口**: `planner.rs` 和 `physical_planner.rs` 生成不同的 PhysicalPlan
3. **LocalExecutor 孤岛**: `local_executor.rs` 存在但未被主路径调用

这些问题导致：
- INT-3: expr crate 是孤岛
- INT-4: mysql-server 双路径
- AV-008~AV-009: isolated modules

### Decision

**Planner Layer Consolidation（PR-850/860）**：

```
PR-850: mysql-server → LocalExecutor 统一
PR-860: Planner Layer Consolidation
```

**核心原则**: PhysicalPlan 只通过 `local_executor.rs` 执行，不存在其他执行路径。

### Alternatives Considered

| Alternative | 描述 | 拒绝原因 |
|-------------|------|----------|
| 保留双 planner | `planner.rs` 用于简单 SQL，`physical_planner.rs` 用于复杂 SQL | 维护两套 planner 成本高 |
| 删除 LocalExecutor | 用 ParallelVolcanoExecutor 替代 | PR-870 才实现 ParallelVolcanoExecutor |
| 不统一 mysql-server | mysql-server 继续用旧路径 | INT-4 跨越多个版本，必须解决 |

### Consequences

**Positive**:
- PhysicalPlan 统一来源
- LocalExecutor 被主路径使用
- INT-3/INT-4 可逐步修复

**Negative**:
- Planner 重构可能引入测试失败
- mysql-server 需要适配新路径

---

## AD-005: VTU Mainline

### Status

**Accepted** — v3.8.0 Phase 3

### Context

INT-2（v3.7.0 GA 遗留）: `ParallelVolcanoExecutor` 存在但从未被主路径调用。

当前（v3.7.0）:
- `ParallelVolcanoExecutor`: 1700 行，44 tests，**0 主路径调用**
- `ParallelVectorExecutor`: 750 行，37 tests，**0 主路径调用**
- `RayonTaskScheduler`: 200 行，15 tests，**0 主路径调用**

单元测试通过，但组件从未在真实执行中被调用——这是集成缺失（Integration Gap）。

### Decision

**VTU（Validated Transaction Unit）作为主执行路径，非 fallback 路径**：

```
PR-870: ParallelVolcanoExecutor 接入
PR-880: VTU Predicate/Mutation Pipeline
```

**核心原则**: VTU path = 唯一路径，无 fallback（PR-900 清理阶段删除 legacy fallback）。

### Alternatives Considered

| Alternative | 描述 | 拒绝原因 |
|-------------|------|----------|
| VTU as fallback | VTU 是备选，主要用 LocalExecutor | VTU 优势无法发挥 |
| 不实现 VTU | 保持现状 | INT-2 跨越 7 版本，性能未优化 |
| VTU only | 删除 LocalExecutor | PR-870/880 尚未实现，过早删除 |

### Consequences

**Positive**:
- SIMD batch execution 可用
- 并行执行提升 QPS
- VTU path 被真实使用（不再是孤岛）

**Negative**:
- VTU 实现复杂度高（PR-870/880）
- 测试需要覆盖 VTU 路径
- 性能回归测试必须通过

---

## 附录：AD 状态说明

| Status | 说明 |
|--------|------|
| **Proposed** | 正在讨论中 |
| **Accepted** | 已接受，将在当前版本实现 |
| **Deprecated** | 已被新的 AD 替代 |
| **Superseded by AD-XXXX** | 被指定 AD 替代 |

---

## Metadata

- **Author**: Hermes Agent
- **Date**: 2026-05-30
- **Related ADRs**: ADR-001~ADR-005 (Governance ADR System)
- **Related PRs**: PR-800~PR-900 (v3.8.0 PR DAG)
- **Related Issues**: INT-1~INT-4 (Legacy Issues)