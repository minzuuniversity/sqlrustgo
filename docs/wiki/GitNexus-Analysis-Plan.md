# GitNexus 代码深度分析计划

> **目的**: 通过 GitNexus 知识图谱进行代码级深度分析，支持重构决策
> **日期**: 2026-05-11

---

## 一、GitNexus 能力概述

### 1.1 可用工具

| 工具 | 用途 | 分析场景 |
|------|------|----------|
| `gitnexus_query` | 搜索执行流 | 查找函数调用链 |
| `gitnexus_context` | 符号360度视图 | 理解符号的调用方/被调用方 |
| `gitnexus_impact` | 影响分析 | 评估变更影响范围 |
| `gitnexus_detect_changes` | 变更检测 | 变更前的风险评估 |
| `gitnexus_cypher` | 图查询 | 复杂依赖分析 |
| `gitnexus_route_map` | API 路由映射 | 理解前后端交互 |

---

## 二、重构前分析计划

### 2.1 CBO CostModel 激活分析

| 分析项 | GitNexus 操作 | 预期结果 |
|--------|--------------|---------|
| CostModel 依赖 | `impact(target="cost.rs")` | 受影响的模块列表 |
| Planner 调用链 | `context(symbol="optimize")` | 优化器入口的调用方 |
| Stats 集成点 | `query("CostModel.*Stats")` | 统计信息集成位置 |

### 2.2 MERGE 语句实现分析

| 分析项 | GitNexus 操作 | 预期结果 |
|--------|--------------|---------|
| Executor 调用链 | `context(symbol="execute")` | 执行器层次结构 |
| DML 执行器 | `query("INSERT.*Executor")` | 已有的 DML 执行器 |
| Planner 模式 | `query("UPDATE.*Planner")` | UPDATE 规划模式 |

### 2.3 窗口函数扩展分析

| 分析项 | GitNexus 操作 | 预期结果 |
|--------|--------------|---------|
| WindowAgg 实现 | `context(symbol="WindowAgg")` | 窗口聚合器结构 |
| 现有窗口函数 | `query("ROW_NUMBER.*Executor")` | 已实现的窗口函数 |
| Frame 处理 | `context(symbol="WindowFrame")` | Frame 处理逻辑 |

### 2.4 Join 算法分析

| 分析项 | GitNexus 操作 | 预期结果 |
|--------|--------------|---------|
| HashJoin 实现 | `context(symbol="HashJoinExecutor")` | Hash Join 结构 |
| Join 策略选择 | `query("JoinStrategy")` | Join 策略枚举 |
| Sort 实现 | `query("Sort.*Executor")` | 可复用的排序 |

---

## 三、执行流追踪计划

### 3.1 SELECT 执行流追踪

```cypher
MATCH (p:Parser)-[:CALLS]->(pl:Planner)-[:CALLS]->(e:Executor)-[:CALLS]->(s:Storage)
WHERE p.name CONTAINS 'select'
RETURN p.name, pl.name, e.name, s.name
```

### 3.2 INSERT 执行流追踪

```cypher
MATCH (e:Executor)-[:WRITES]->(w:WAL)-[:SYNC]->(s:Storage)
WHERE e.name CONTAINS 'insert'
RETURN e.name, w.name, s.name
```

### 3.3 事务提交流追踪

```cypher
MATCH (tx:Transaction)-[:WRITES_WAL]->(wal:WAL)-[:FLUSHES]->(storage:Storage)
RETURN tx.name, wal.name, storage.name
```

---

## 四、影响分析计划

### 4.1 变更影响矩阵

| 变更 | 影响模块 | 风险 | 需要更新的测试 |
|------|----------|------|----------------|
| CostModel 激活 | planner, optimizer | 中 | optimizer tests |
| MERGE 实现 | parser, planner, executor | 中 | merge tests |
| 窗口函数扩展 | executor | 低 | window function tests |
| Join 算法添加 | executor, optimizer | 中 | join tests |

### 4.2 影响分析查询

```bash
# 1. CostModel 变更影响
impact(target="crates/optimizer/src/cost.rs", direction="downstream", maxDepth=3)

# 2. Executor 变更影响
impact(target="crates/executor/src/merge_executor.rs", direction="downstream", maxDepth=2)

# 3. Planner 变更影响
impact(target="crates/planner/src/planner.rs", direction="downstream", maxDepth=3)
```

---

## 五、分析执行清单

### 5.1 重构前分析

| 阶段 | 分析项 | GitNexus 操作 | 产出 |
|------|--------|--------------|------|
| 1 | CostModel 依赖 | `impact()` | 影响报告 |
| 2 | Planner 调用链 | `context()` | 集成点文档 |
| 3 | Executor 架构 | `query()` | 可复用代码 |
| 4 | Join 算法实现 | `cypher()` | 算法模板 |

### 5.2 重构中分析

| 阶段 | 分析项 | GitNexus 操作 | 产出 |
|------|--------|--------------|------|
| 1 | 变更检测 | `detect_changes()` | 风险评估 |
| 2 | 影响验证 | `impact()` | 回归风险 |
| 3 | 调用链验证 | `context()` | 接口兼容 |

### 5.3 重构后分析

| 阶段 | 分析项 | GitNexus 操作 | 产出 |
|------|--------|--------------|------|
| 1 | 测试覆盖 | `query()` | 覆盖报告 |
| 2 | 性能影响 | `cypher()` | 执行流分析 |
| 3 | 质量验证 | `context()` | 代码质量 |

---

## 六、GitNexus 使用建议

### 6.1 索引更新

```bash
# 在分析前更新索引
npx gitnexus analyze

# 检查索引状态
npx gitnexus status
```

### 6.2 分析优先级

| 优先级 | 场景 | 推荐工具 |
|--------|------|----------|
| P0 | 影响分析 | `impact()` |
| P1 | 上下文理解 | `context()` |
| P2 | 执行流追踪 | `cypher()` |
| P3 | 代码质量 | `query()` |

---

## 七、附录

### 7.1 GitNexus 图谱结构

```
节点类型:
- File, Folder
- Function, Class, Struct, Enum
- Method, Property
- Process, Community
- Route, Tool

边类型:
- CONTAINS, DEFINES, CALLS
- IMPORTS, EXTENDS, IMPLEMENTS
- HAS_METHOD, HAS_PROPERTY
- ACCESSES, STEP_IN_PROCESS
```

---

*文档版本: v1.0*
*创建日期: 2026-05-11*