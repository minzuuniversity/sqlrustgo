# SQLRustGo 遗留问题总清单

> **文档版本**: v3.8.0
> **更新日期**: 2026-05-30
> **维护者**: Hermes Agent
> **用途**: v3.7.0 GA Freeze 遗留问题归档 + v3.8.0 开发计划输入

---

## 1. 概述

本文档记录 SQLRustGo 所有**未关闭的遗留问题**，按版本和优先级分类。

**分类原则**：
- **v3.7.x**: 当前稳定版本的 P1 修复，可在 GA 后独立处理
- **INT-1~INT-4**: 历史架构债务，跨越 v1.2.0~v3.7.0，需在 v3.8.0 解决
- **SYSTEMIC**: 系统级技术债务，影响多个版本

---

## 2. v3.7.0 GA 遗留问题（v3.7.x 范围）

> v3.7.0 GA 已冻结，以下问题在 v3.7.x stabilization 中处理

### 2.1 P1 Issues（高优先级）

| Issue | 标题 | 说明 | 影响 | 修复方案 | 状态 |
|-------|------|------|------|----------|------|
| #2583 | SHOW TABLES 未实现 | `SHOW TABLES` 返回语法错误 | 用户无法列出表 | 实现 catalog show handler | OPEN |
| #2584 | 空密码认证 edge case | root 空密码无法登录 | 认证流程不完整 | `auth_response.is_empty()` 处理逻辑修复 | OPEN |
| #2585 | VTU 未接入主执行路径 | ParallelVolcanoExecutor 存在但未使用 | 性能优化无效 | v3.8.0 PR-870 | Planned |
| #2586 | execution_engine.rs 膨胀 | 4658→6829 行（v3.4→v3.7 增长 47%） | 可维护性差 | v3.8.0 PR-900 | Planned |

### 2.2 P2 Issues（中优先级）

| Issue | 标题 | 说明 | 状态 |
|-------|------|------|------|
| #2587 | 覆盖率测量不统一 | Z6G4 81.97% vs Z440 32.59% | OPEN |
| #2596 | 覆盖率 Z6G4 vs Z440 差异 | 测量方法/工具不一致 | OPEN |
| #2597 | execution_engine.rs 行数增长 | 架构膨胀可维护性差 | Planned → v3.8.0 |

### 2.3 P3 Issues（低优先级）

| Issue | 标题 | 说明 | 状态 |
|-------|------|------|------|
| #2600 | Coverage 工具统一 | cargo-llvm-cov vs 其他工具 | OPEN |
| #2607 | Benchmark 基线记录 | TPC-H SF=1 基线未文档化 | OPEN |
| #2608 | Error code 标准化 | MySQL error code 一致性 | OPEN |

---

## 3. 历史架构债务（INT-1~INT-4）

> 跨越多个版本（v1.2.0~v3.7.0），必须在 v3.8.0 解决

### 3.1 INT-1: DML 不经过 WAL（最严重）

| 字段 | 值 |
|------|-----|
| Issue | #2588 |
| 首次引入 | v1.2.0 |
| 跨越版本 | v1.2.0~v3.7.0（7 个版本） |
| 影响范围 | 所有 DML（INSERT/UPDATE/DELETE） |
| 根因 | DML 直接写入 storage，跳过 TransactionManager + WAL |
| 后果 | 无 crash recovery，非预期终止导致数据丢失 |
| 涉及文件 | `execution_engine.rs`, `mysql-server/src/lib.rs` |
| v3.8.0 PR | PR-830, PR-840 |
| 依赖 | PR-800, PR-810, PR-820 |
| 风险 | 🔴 CRITICAL（核心 ACID 变更） |

**架构问题描述**：

```
当前路径（v3.7.0）：
INSERT INTO t VALUES(...)
  ↓
eng.execute_insert()  ← 直接调用 storage.insert()
  ↓
无 WAL 记录

修复后路径（v3.8.0）：
INSERT INTO t VALUES(...)
  ↓
parse() → AST
  ↓
TransactionManager.intercept_dml()
  ↓
WriteBuffer staging + WAL append
  ↓
COMMIT → flush WAL + apply buffer
```

---

### 3.2 INT-2: ParallelVolcanoExecutor 未接入

| 字段 | 值 |
|------|-----|
| Issue | #2589 |
| 首次引入 | v2.6.0 |
| 跨越版本 | v2.6.0~v3.7.0（5 个版本） |
| 影响范围 | Query execution / SIMD optimization |
| 根因 | VTU 代码存在于 LocalExecutor，但 mysql-server 不经过 |
| 后果 | SIMD 加速/向量化执行对 mysql-server 无效 |
| 涉及文件 | `local_executor.rs`, `volcano_executor.rs`, `predicate.rs` |
| v3.8.0 PR | PR-870, PR-880 |
| 依赖 | PR-850, PR-860 |
| 风险 | 🟡 中高（性能路径变更） |

**架构问题描述**：

```
当前路径（v3.7.0）：
mysql-server → ExecutionEngine → MemoryStorage  ← 无 VTU
bench-cli → LocalExecutor (有 VTU)               ← VTU 仅此处

修复后路径（v3.8.0）：
mysql-server → Planner → ParallelVolcanoExecutor ← 统一 VTU
```

---

### 3.3 INT-3: expr crate 孤岛

| 字段 | 值 |
|------|-----|
| Issue | #2590 |
| 首次引入 | v3.0.0 |
| 跨越版本 | v3.0.0~v3.7.0（4 个版本） |
| 影响范围 | Expression evaluation |
| 根因 | expr crate 未与 ExecutionEngine 集成 |
| 后果 | Expression evaluation 逻辑分散，无法统一优化 |
| 涉及文件 | `expr/`, `execution_engine.rs`（expression 部分） |
| v3.8.0 PR | PR-860 |
| 依赖 | PR-850 |
| 风险 | 🟡 中（收敛型变更） |

---

### 3.4 INT-4: mysql-server 双执行路径

| 字段 | 值 |
|------|-----|
| Issue | #2591 |
| 首次引入 | v2.6.0 |
| 跨越版本 | v2.6.0~v3.7.0（5 个版本） |
| 影响范围 | mysql-server, bench-cli, LocalExecutor |
| 根因 | mysql-server 和 bench-cli 各自有独立执行路径 |
| 后果 | 行为不一致，维护两套执行逻辑 |
| 涉及文件 | `mysql-server/src/lib.rs`, `local_executor.rs`, `execution_engine.rs` |
| v3.8.0 PR | PR-850 |
| 依赖 | PR-840 |
| 风险 | 🔴 高（架构统一核心） |

**架构问题描述**：

```
当前双路径（v3.7.0）：
Path A: mysql-server → ExecutionEngine → MemoryStorage
Path B: bench-cli → LocalExecutor → StorageEngine

修复后单路径（v3.8.0）：
ALL: mysql-server → Planner → LocalExecutor → StorageEngine
```

---

## 4. 系统级技术债务（SYSTEMIC）

### 4.1 Architecture Debt

| Issue | 标题 | 说明 | 首次引入 | 状态 |
|-------|------|------|----------|------|
| #2597 | execution_engine.rs 膨胀 | 6829 行，架构职责不清 | v1.0 | Planned → v3.8.0 PR-900 |
| #2599 | Double execution path | mysql-server vs bench-cli | v2.6 | Planned → v3.8.0 |
| #2603 | R2: 执行引擎统一 | INT-4 | v3.0 | Planned → v3.8.0 PR-850 |
| #2604 | R3: expr crate | INT-3 | v3.0 | Planned → v3.8.0 PR-860 |
| #2605 | R4: mysql-server | INT-4 | v2.6 | Planned → v3.8.0 PR-850 |

### 4.2 Testing Debt

| Issue | 标题 | 说明 | 状态 |
|-------|------|------|------|
| #2596 | 覆盖率测量差异 | Z6G4 vs Z440 测量结果差 49pp | OPEN |
| #2600 | Coverage 工具统一 | cargo-llvm-cov 标准化 | Planned → v3.8.0 PR-900 |

### 4.3 Documentation Debt

| Issue | 标题 | 说明 | 状态 |
|-------|------|------|------|
| #2610 | 历史版本断链 | v3.4.0 等旧版本文档死链 61 个 | 历史遗留，不修复 |
| #2611 | 活跃文档死链 | 当前版本死链 23 个 | 已修复 |

---

## 5. Issue → PR → 文件 映射表（v3.8.0）

| Issue | PR | 文件/模块 | 描述 |
|-------|-----|-----------|------|
| #2588 | PR-830 | `crates/wal/`, `crates/transaction/` | WAL + WriteBuffer 接入 |
| #2588 | PR-840 | `execution_engine.rs` | DML → TransactionManager 拦截 |
| #2589 | PR-870 | `volcano_executor.rs` | ParallelVolcanoExecutor 接入 |
| #2589 | PR-880 | `predicate.rs`, `mutation.rs` | VTU pipeline 统一 |
| #2590 | PR-860 | `expr/`, `planner/` | expr crate 收敛 |
| #2591 | PR-850 | `mysql-server/src/lib.rs`, `local_executor.rs` | mysql-server 统一执行路径 |
| #2596 | PR-900 | `scripts/coverage/` | 覆盖率测量统一 |
| #2597 | PR-900 | `execution_engine.rs` | 拆分 + 降至 <1500 行 |

---

## 6. 优先级矩阵

| 优先级 | v3.7.x | v3.8.0 |
|--------|--------|--------|
| P0 | — | INT-1: WAL 集成 |
| P1 | SHOW TABLES, 空密码 auth | INT-2: VTU, INT-4: 双路径统一 |
| P2 | 覆盖率差异 | INT-3: expr 收敛, execution_engine 拆分 |
| P3 | Error code 标准化 | Coverage 工具统一 |

---

## 7. 关闭条件

### v3.7.0 GA 已关闭

| Issue | 关闭条件 |
|-------|----------|
| P0-1: Transaction state session 化 | ✅ 已修复，commit 01db4fdf |
| P0-2: SKIP_AUTH bypass | ✅ 已修复，commit 2607d788 |

### v3.8.0 目标关闭

| Issue | 关闭条件 |
|-------|----------|
| INT-1 (#2588) | WAL replay + DML interception 测试通过 |
| INT-2 (#2589) | ParallelVolcanoExecutor 在 mysql-server 中激活 |
| INT-3 (#2590) | expr crate 与 planner 集成测试通过 |
| INT-4 (#2591) | `grep storage.insert` 仅在 storage layer |
| #2597 | execution_engine.rs < 1500 行 |
| #2596 | Z6G4 vs Z440 覆盖率 delta < 10pp |