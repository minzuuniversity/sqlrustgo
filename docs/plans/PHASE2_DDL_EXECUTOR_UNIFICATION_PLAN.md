# Phase 2：DDL Executor 统一 + LocalExecutorDml 清理

> **时间**: 2026-06-27
> **目标**: 关闭 Parser-Executor Gap，统一 DDL/DML 执行路径
> **匹配文档**: `docs/standard/SQL92_FUNCTIONALITY_MATRIX.md` §7.1

---

## 任务分解

### T1: 补全 DDL Executor 方法

**当前缺失的 dispatch** (ExecutionEngine::execute 未分发):

| 语句 | 当前状态 | 修复方式 |
|------|---------|---------|
| `DROP INDEX` | ❌ catch-all | 添加 dispatch + execute_drop_index |
| `CREATE VIEW` | ❌ catch-all | 添加 dispatch + execute_create_view |
| `DROP VIEW` | ❌ catch-all | 添加 dispatch + execute_drop_view |
| `MERGE` | ❌ catch-all | 添加 dispatch + execute_merge |

### T2: 统一 DML/DDL 执行路径

**当前两条路径**:
- `ExecutionEngine::execute()` - 主路径，通过 SQL → AST → dispatch
- `LocalExecutorDml` - 占位符路径，只支持 MERGE

**目标**: DDL 和 DML 都走 `ExecutionEngine::execute()` 主路径

### T3: 删除 LocalExecutorDml 占位符

**当前**:
- `LocalExecutorDml` 使用 `NoopExecutionEngine`（所有方法返回错误）
- 只支持 MERGE 操作（通过 MergeExecutor）

**目标**:
- 将 MERGE 功能迁移到 `ExecutionEngine::execute()` 中
- 删除 `LocalExecutorDml` 和 `NoopExecutionEngine`

---

## 执行顺序

```
T1: 补全 DDL dispatch → T2: 统一路径 → T3: 清理占位符
```

## 风险

| 风险 | 影响 | 缓解 |
|------|------|------|
| MERGE 通过 MergeExecutor 执行，与直接 SQL 路径行为可能不同 | 功能退化 | 保持 MergeExecutor 不动，execute() 中转发到 MergeExecutor |
| 删除 LocalExecutorDml 可能影响现有调用者 | 编译错误 | 检查所有引用点 |
