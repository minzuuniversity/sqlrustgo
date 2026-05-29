# Architecture Violations Report

> v3.7.0 Core Integrity Release - 架构违规基线
> 生成时间: 2026-05-30
> 状态: **基线冻结** - 不允许新增违规

## 基线状态

| 严重级别 | 当前数量 | Policy |
|---------|---------|--------|
| CRITICAL | 7 | **禁止新增** |
| HIGH | 3 | **禁止新增** |
| MEDIUM | 0 | - |
| **TOTAL** | **10** | **必须逐步减少** |

## 基线冻结政策

```yaml
violation_baseline:
  date: "2026-05-30"
  CRITICAL: 7
  HIGH: 3
  TOTAL: 10

policy:
  no_new_critical: true    # 禁止新增 CRITICAL
  no_new_high: true       # 禁止新增 HIGH
  reduction_target: 2      # 每周减少 2 个
  final_target: 0         # v3.7.0 GA 前全部修复
```

## 违规详情

### CRITICAL 违规 (7)

| ID | Rule | File | Line | Description |
|----|------|------|------|-------------|
| AV-001 | F1-DML_WITHOUT_TXN | `executor/src/trigger.rs` | 427,505,507,529 | storage.insert/delete 直接调用 |
| AV-002 | F1-DML_WITHOUT_TXN | `executor/src/harness.rs` | 274,315,375 | storage.insert 直接调用 |
| AV-003 | F1-DML_WITHOUT_TXN | `executor/src/merge.rs` | 88,107 | storage.update/insert 直接调用 |
| AV-004 | F1-DML_WITHOUT_TXN | `executor/src/parallel_vector_executor.rs` | 689,706,724,743 | storage.insert 直接调用 |
| AV-005 | F1-DML_WITHOUT_TXN | `executor/src/parallel_executor.rs` | 1140,1145,1253,1258,1578,1583,1661,1666,1733,1738 | memory_storage.insert 直接调用 |
| AV-006 | F1-DML_WITHOUT_TXN | `executor/src/local_executor.rs` | 1054 | storage.delete 直接调用 |
| AV-007 | F1-DML_WITHOUT_TXN | `executor/src/vector_executor.rs` | 193,220,242,293 | storage.insert 直接调用 |

### HIGH 违规 (3)

| ID | Rule | File | Line | Description |
|----|------|------|------|-------------|
| AV-008 | F2-ISOLATED_IN_MAINLINE | `executor/src/lib.rs` | - | vec_simd 模块被主路径引用 |
| AV-009 | F2-ISOLATED_IN_MAINLINE | `executor/src/local_executor_dml.rs` | - | isolated PLACEHOLDER module |
| AV-010 | R1-MISSING_WAL | `executor/src/executor.rs` | - | VolcanoExecutor 缺少 WAL 集成 |

## 违规文件详细分析

### executor/src/vector_executor.rs

**问题**: 向量执行器中的 DML 操作直接调用 storage
**影响范围**: 向量查询处理
**建议**: TASK-R1-2: 实现 LocalExecutorDml，将 DML 路由到 TransactionManager

### executor/src/harness.rs

**问题**: 测试 harness 中的 DML 直接调用 storage
**影响范围**: 测试框架
**建议**: 使用 TransactionalExecutor 或 WalTransactionalExecutor

### executor/src/merge.rs

**问题**: Merge 执行器中的 DML 直接调用 storage
**建议**: TASK-R1-2: 统一事务路径

### executor/src/parallel_vector_executor.rs

**问题**: 并行向量执行器中的 DML 直接调用 storage
**状态**: **ISOLATED** - 无主路径调用
**建议**: TASK-R2-2: 删除或合并到主执行器

### executor/src/trigger.rs

**问题**: 触发器执行中的 DML 直接调用 storage
**影响范围**: 触发器功能
**建议**: TASK-R1-2: 触发器 DML 必须经过事务

### executor/src/local_executor.rs

**问题**: 本地执行器中的 DML 直接调用 storage
**状态**: 73KB 主执行器
**建议**: TASK-R1-2: 必须实现事务包装

### executor/src/parallel_executor.rs

**问题**: 并行执行器中的 DML 直接调用 storage
**状态**: **ISOLATED** - 63KB 但无主路径调用
**建议**: TASK-R2-2: 删除或合并

### executor/src/lib.rs

**问题**: vec_simd 模块被主路径导出
**状态**: **EXPERIMENTAL** - 但被 production 引用
**建议**: 移除导出或标记为 experimental-only

### executor/src/local_executor_dml.rs

**问题**: PLACEHOLDER - 仅占位符
**状态**: **ISOLATED** - 未实现
**建议**: TASK-R1-2: 必须实现

### executor/src/executor.rs

**问题**: 主 executor 缺少 WAL 集成
**状态**: VolcanoExecutor 主执行器
**建议**: TASK-R1-3: WAL First Policy

## 治理策略

### Rule 1: 不允许新增 violation

任何 PR:
- 禁止引入新的 CRITICAL/HIGH 违规
- 如果发现新违规，必须同时修复

### Rule 2: 旧 violation 逐步减少

每次提交:
- 必须减少至少 1 个违规
- 或有明确的迁移计划

### Rule 3: 违规数量冻结

| 时间 | CRITICAL | HIGH | 目标 |
|------|----------|------|------|
| 基线 (2026-05-30) | 7 | 3 | - |
| +1 周 | 7 | 3 | 禁止新增 |
| +2 周 | 6 | 3 | -1 |
| +4 周 | 5 | 2 | -2 |
| +8 周 | 3 | 1 | -3 |
| v3.7.0 GA | 0 | 0 | 全部修复 |

## 修复优先级

### P0 (立即修复)

| ID | File | 行动 |
|----|------|------|
| AV-001 ~ AV-007 | DML without txn | TASK-R1-2: 实现事务包装 |
| AV-010 | executor.rs | TASK-R1-3: WAL First Policy |

### P1 (本周修复)

| ID | File | 行动 |
|----|------|------|
| AV-008 | vec_simd | 从 production lib.rs 移除 |
| AV-009 | local_executor_dml | 实现或删除 |

### P2 (迁移期间修复)

| ID | File | 行动 |
|----|------|------|
| AV-001 ~ AV-007 | parallel_* executors | TASK-R2-2: 删除/合并 |

## 验证命令

```bash
# 检查违规
bash scripts/gate/check_mainline.sh

# 检查无新增违规
cargo xtask architecture-check --mode=all

# 检查 isolated 模块
cargo xtask dead-modules
```

## 相关文档

- `EXECUTION_PATH.md` - 执行路径定义
- `TRANSACTION_BOUNDARY.md` - 事务边界
- `ARCHITECTURE_RULES.yaml` - 架构规则
- `MAINLINE_COMPONENTS.md` - 主路径组件
- `ISOLATED_MODULES.md` - 孤岛模块

## Issue 追踪

- #2602: R1 - Transaction/WAL 主路径重构
- #2603: R2 - 执行引擎统一
- #2606: R5 - Gate 重构