---
id: coverage-recovery-v340
title: Coverage Recovery v3.4.0 - Trust Convergence
version: 1.0
status: draft
created: 2026-05-21
author: openclaw
---

# Coverage Recovery v3.4.0 规范

## 概要

根据 Trust Convergence 原则（禁止架构扩散），v3.4.0 只做 5 类事情：
- Bug 修复
- 性能恢复
- 覆盖率补全
- Recovery/稳定性验证
- GMP 深度验证

本 spec 聚焦 **覆盖率补全**，目标将关键路径覆盖率提升到门禁要求。

## 当前状态

| 指标 | 当前值 | 目标值 | 差距 |
|------|--------|--------|------|
| TOTAL regions | 68.45% | ≥70% | -1.55pp |
| TOTAL functions | 71.72% | - | - |

**关键发现（P0 Bug）：** UPDATE 语句在 planner 层被转换为空 SeqScanExec，实际不执行任何更新操作。这是核心功能 bug，不是架构扩展。

## 关键模块覆盖率目标

| 模块 | 当前覆盖 | 目标覆盖 | 优先级 |
|------|----------|----------|--------|
| executor regions | ~68.5% | ≥80% | P0 |
| WAL | 未测 | ≥90% | P1 |
| audit chain | 未测 | ≥95% | P1 |
| recovery | 未测 | ≥90% | P1 |
| MVCC visibility | 未测 | ≥90% | P1 |
| signature verify | 未测 | ≥95% | P1 |

## 覆盖范围

### 1. Executor 模块（当前 ~68.5%）

#### 1.1 UPDATE 执行路径（P0 Bug）

**问题：** planner.rs 中 LogicalPlan::Update 被转换为 SeqScanExec（空操作）

```rust
// planner.rs:358-361
LogicalPlan::Update { .. } => {
    Ok(Box::new(SeqScanExec::new(String::new(), Schema::empty())))
}
```

**修复策略：** 这是 Trust Convergence 阶段的 bug 修复，不是新架构。修复需要在 planner 层实现真正的 UpdateExec。

**测试用例（修复后验证）：**
- `test_update_simple_set` - 简单 UPDATE SET
- `test_update_with_predicate` - 带 WHERE 条件的 UPDATE
- `test_update_multiple_columns` - 多列更新
- `test_update_with_expression` - UPDATE SET col = col + 1
- `test_update_nonexistent_row` - 无匹配行的 UPDATE
- `test_update_violates_constraint` - 违反约束的 UPDATE（如果支持约束）

#### 1.2 DELETE 执行路径（已部分覆盖）

**现有测试：** delete_execution_test.rs（277 行）

**未覆盖路径：**
- `storage.delete()` 返回错误
- `storage.delete_by_predicate()` 返回错误
- 复杂谓词删除

**新增测试：**
- `test_delete_with_complex_predicate` - 复杂 WHERE 条件
- `test_delete_storage_error_handling` - 存储层错误处理

#### 1.3 MERGE 执行路径

**现有测试：** 基本 MERGE 测试

**未覆盖路径：**
- `evaluate_merge_condition()` 复杂条件
- MATCHED clause 更新失败
- NOT MATCHED clause 插入失败

**新增测试：**
- `test_merge_condition_evaluation`
- `test_merge_update_failure`
- `test_merge_insert_failure`

#### 1.4 错误路径覆盖

**现有测试：** executor_error_path_test.rs（1001 行）

**未覆盖的错误类型：**
- storage 层返回错误
- 类型转换错误
- 谓词求值错误

### 2. Crash Recovery 验证

**现有测试：** crash-sim/crash_sim_test.rs

**需要验证的关键点：**

| 点 | 验证内容 |
|----|----------|
| WAL append | kill 后 WAL 记录完整 |
| page split | kill 后 B+树一致 |
| checkpoint | kill 后 checkpoint 有效 |
| commit | kill 后已提交事务不丢失 |
| audit append | kill 后审计链不断 |
| signature persist | kill 后签名验证仍有效 |

### 3. WAL 覆盖率

**目标：** ≥90%

**需要覆盖：**
- WAL header 读写
- WAL record 类型（INSERT/UPDATE/DELETE/COMMIT/ABORT）
- WAL segment 管理
- checkpoint 流程
- WAL replay 恢复

### 4. Audit Chain 覆盖率

**目标：** ≥95%

**需要覆盖：**
- audit record 写入
- hash chain 连续性
- signature 验证
- timestamp ordering
- orphan record 检测

## 实施计划

### Phase 1: 修复 UPDATE Bug（P0）

1. 在 planner 层实现真正的 UpdateExec
2. 在 executor 层实现 execute_update()
3. 添加 UPDATE 单元测试
4. 验证 UPDATE QPS 正常

### Phase 2: DELETE/MERGE 错误路径

1. 分析 delete_execution_test.rs 覆盖缺口
2. 新增错误路径测试
3. 新增 MERGE 边界测试

### Phase 3: Crash Recovery 测试

1. 运行 crash-sim 测试
2. 分析 WAL覆盖率
3. 新增 WAL 边界测试

### Phase 4: Coverage 验证

1. 运行改进的 coverage 脚本
2. 验证达到目标覆盖率
3. 更新 gate 检查

## 资源控制要求

coverage 测试脚本必须：
- 控制并行任务数（`CARGO_BUILD_JOBS` 或 `--jobs`）
- 控制内存消耗（避免 OOM）
- 监控 CPU 负载（避免系统挂起）
- 支持分 crate 独立运行
- 支持增量 coverage（只测变更模块）

## 验收标准

1. executor regions ≥80%
2. WAL regions ≥90%
3. 所有 P0 测试通过
4. crash-sim 测试通过
5. 无新增警告
