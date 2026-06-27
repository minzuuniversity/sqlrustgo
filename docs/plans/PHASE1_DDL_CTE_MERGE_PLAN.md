# Phase 1：DDL 端到端 + MERGE Subquery + CTE 验证

> **时间**: 2026-06-27
> **目标**: 验证所有已声称功能的端到端覆盖率
> **匹配文档**: `docs/standard/SQL92_FUNCTIONALITY_MATRIX.md` §7.4

---

## 任务分解

### T1: DDL 端到端测试 (CREATE/DROP/INDEX)

**当前状态**:
- Parser: CREATE/DROP TABLE/INDEX 能解析 ✅
- Executor: 无独立 DDL 方法，通过 Storage 间接执行 ⚠️
- Storage: create_table/drop_table/create_index ✅

**缺口**:
- 无 DDL 端到端测试覆盖

**验收标准**:
1. Parser 能解析 DDL 语句
2. Executor 能分发 DDL 到 Storage
3. DDL 执行后数据实际写入/删除

### T2: MERGE Subquery 支持

**当前状态**:
- Parser: `MERGE INTO t USING (SELECT ...) AS s ...` 能解析 ✅
- Executor: `merge.rs` 的 `MergeExecutor` 不支持 Subquery 源 ⚠️

**缺口**: Parser 能解析，Executor 不支持

**验收标准**:
1. Subquery 源 MERGE 能执行
2. 不能破坏已有 MERGE (table source) 的测试

### T3: CTE 端到端验证

**当前状态**:
- Parser: WITH SELECT 和 WITH DML 都能解析 ✅
- Executor: WITH 子句在 TPC-H 中使用，需要验证

**缺口**:
- 无独立的 CTE 端到端测试

**验收标准**:
1. `WITH cte AS (SELECT ...) SELECT * FROM cte` 执行正确
2. `WITH cte AS (SELECT ...) INSERT INTO t SELECT * FROM cte` 执行正确
3. TPC-H Q13/Q15 仍 PASS (这些查询使用 CTE)

---

## 执行顺序

```
T1: DDL 端到端测试 → T2: MERGE Subquery → T3: CTE 验证
         ↓                    ↓                  ↓
   测试 DDL 解析      实现 Subquery 源     验证 CTE 执行
   测试 DDL 执行      添加测试用例         补充测试覆盖
```

## 风险

| 风险 | 影响 | 缓解 |
|------|------|------|
| DDL 通过 Storage 直接执行，无 Executor 层封装 | 架构债 | 只加测试，不改架构 |
| MERGE Subquery 涉及 Planner 改动 | 高风险 | 先在 MergeExecutor 层加 Subquery 支持 |
| CTE 在 DML 路径可能未连通 | TPC-H 退化 | 跑 TPC-H 22/22 验证 |
