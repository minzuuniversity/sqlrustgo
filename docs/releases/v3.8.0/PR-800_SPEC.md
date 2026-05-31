# PR-800 SPEC — COM_QUERY AST Routing

> **PR Number**: PR-800  
> **PR Title**: COM_QUERY AST Routing  
> **Version**: v3.8.0 Phase 0 Architecture Freeze  
> **Branch**: `develop/v3.8.0` (SHA: `745f24f1`)  
> **Auditor**: Hermes Agent  
> **Created**: 2026-05-30  
> **Status**: DRAFT — For Review  

---

## 1. 概述

### 1.1 PR 目标

PR-800 是 v3.8.0 Architecture Unification 的第一步，也是整个 PR DAG 的入口点。

**目标**: 将 mysql-server 的 COM_QUERY 处理从 `eng.execute(raw_sql)` 路径，改为 `Parser → AST → Planner → Executor` 标准路径。

### 1.2 当前架构（Before）

```
mysql-server
  └── COM_QUERY handler
        └── eng.execute(raw_sql)        ← 要删除这个
              └── 直接解析 raw_sql
              └── 绕过 Parser
              └── 绕过 AST
              └── 绕过 Planner
```

**问题**: raw_sql 直接执行，跳过所有标准中间层：
- Parser 不被使用
- AST 不被生成
- Planner 不被调用
- PhysicalPlan 不被使用
- LocalExecutor 不被使用

### 1.3 目标架构（After）

```
mysql-server
  └── COM_QUERY handler
        ├── Parser::parse(sql)          ← 新增
        │     └── AST (Statement enum)
        ├── Planner::plan(ast)          ← 新增
        │     └── PhysicalPlan
        ├── LocalExecutor::execute(plan) ← 新增
        │     └── StorageEngine
        └── Result
```

---

## 2. 要删除的内容

### 2.1 删除 `eng.execute(raw_sql)` 路径

```rust
// 文件: src/execution_engine.rs
// 删除: eng.execute() 中对 raw_sql 的直接解析

// Before (要删除):
pub fn execute(&mut self, sql: &str) -> SqlResult<ExecutorResult> {
    // 直接解析 raw_sql，绕过 Parser
    if sql.trim().to_uppercase().starts_with("SELECT") {
        // 直接执行，没有 AST
    }
}
```

### 2.2 删除的直接调用点

| 文件 | 行 | 内容 | 说明 |
|------|-----|------|------|
| `src/execution_engine.rs` | 337 | `pub fn execute(&mut self, sql: &str)` | 入口函数 |
| `src/execution_engine.rs` | 421+ | `fn execute_select()` | 直接执行逻辑 |
| `src/execution_engine.rs` | 800+ | `fn execute_insert()` | 直接执行逻辑 |

### 2.3 删除验证

```bash
# 验证 eng.execute(raw_sql) 不存在
grep -rn "execute.*raw_sql\|raw_sql.*execute" src/
# 期望: 0 matches

# 验证 eng.execute(sql: &str) 仅用于向后兼容（如果有）
grep -rn "fn execute.*sql.*&str" src/
# 期望: 0 matches 或仅用于测试桩
```

---

## 3. 要保留的内容

### 3.1 Parser 模块

```rust
// 保留: sqlrustgo-parser crate
pub mod parser {
    pub fn parse(sql: &str) -> Result<Statement, ParseError>;
    pub fn parse_statements(sql: &str) -> Result<Vec<Statement>, ParseError>;
}
```

**保留理由**: Parser 已有完整的 SQL 解析能力（lexer + parser），仅需被 COM_QUERY 入口调用。

### 3.2 AST 类型

```rust
// 保留: sqlrustgo-types crate 中的 AST 类型
pub enum Statement {
    Select(SelectStatement),
    Insert(InsertStatement),
    Update(UpdateStatement),
    Delete(DeleteStatement),
    CreateTable(CreateTableStatement),
    // ... 其他变体
}
```

**保留理由**: AST 是 Parser 和 Planner 之间的标准接口。

### 3.3 Planner 模块

```rust
// 保留: sqlrustgo-planner crate
pub mod planner {
    pub fn plan(stmt: Statement) -> Result<PhysicalPlan, PlannerError>;
    pub fn plan_batch(stmts: Vec<Statement>) -> Result<Vec<PhysicalPlan>, PlannerError>;
}
```

**保留理由**: Planner 将 AST 转换为 PhysicalPlan，是查询优化的基础。

### 3.4 LocalExecutor 模块

```rust
// 保留: sqlrustgo-executor crate 中的 LocalExecutor
pub struct LocalExecutor {
    // ...
}

impl LocalExecutor {
    pub fn execute(&mut self, plan: &PhysicalPlan) -> Result<ExecutorResult, ExecutorError>;
    pub fn execute_batch(&mut self, plans: &[PhysicalPlan]) -> Result<Vec<ExecutorResult>, ExecutorError>;
}
```

**保留理由**: LocalExecutor 是标准执行器，所有 PhysicalPlan 都通过它执行。

---

## 4. 要新增的内容

### 4.1 COM_QUERY Handler 路由

```rust
// mysql-server/src/lib.rs 或 execution_engine.rs

async fn handle_com_query(sql: &str) -> Result<CommandResponse, ServerError> {
    // Step 1: Parse
    let ast = parser::parse(sql)
        .map_err(|e| ServerError::ParseError(e.to_string()))?;

    // Step 2: Plan
    let plan = planner::plan(ast)
        .map_err(|e| ServerError::PlanError(e.to_string()))?;

    // Step 3: Execute via LocalExecutor
    let result = local_executor.execute(&plan)
        .map_err(|e| ServerError::ExecutionError(e.to_string()))?;

    // Step 4: Convert to MySQL response
    Ok(convert_to_response(result))
}
```

### 4.2 ExecutionEngine 角色转变

```rust
// ExecutionEngine 从"执行器"变为"路由协调器"
pub struct ExecutionEngine {
    parser: Arc<Parser>,
    planner: Arc<Planner>,
    executor: Arc<LocalExecutor>,
    txn_manager: Arc<TransactionManager>,
}

impl ExecutionEngine {
    // 新的单一入口
    pub async fn execute_statement(&mut self, sql: &str) -> SqlResult<ExecutorResult> {
        let ast = self.parser.parse(sql)?;
        let plan = self.planner.plan(ast)?;
        self.executor.execute(&plan)
    }
}
```

---

## 5. 数据流变化

### 5.1 Before (v3.7.0)

```
COM_QUERY(raw_sql)
    │
    └→ eng.execute(raw_sql)      ← 直接执行
          │
          ├→ 没有 Parser 调用
          ├→ 没有 AST 生成
          ├→ 没有 Planner 调用
          ├→ 直接调用 storage
          │
          └→ Result
```

### 5.2 After (PR-800)

```
COM_QUERY(raw_sql)
    │
    └→ ExecutionEngine::execute_statement(sql)
          │
          ├→ Parser::parse(sql)      ← 新增
          │     └→ AST
          │
          ├→ Planner::plan(ast)      ← 新增
          │     └→ PhysicalPlan
          │
          ├→ LocalExecutor::execute(plan)  ← 新增
          │     └→ StorageEngine
          │
          └→ Result
```

---

## 6. 依赖关系

### 6.1 依赖的 PR

- PR-800: **本 PR**（无前置依赖）

### 6.2 被依赖的 PR

- PR-810: ExecutionEngine → Router（依赖 PR-800）
- PR-820: TransactionManager Session Binding（依赖 PR-810）
- PR-830: WAL + WriteBuffer（依赖 PR-820）
- PR-840: DML Transaction Interception（依赖 PR-830）
- PR-850: mysql-server → LocalExecutor（依赖 PR-840）
- PR-860: Planner Layer Consolidation（依赖 PR-850）
- PR-870: ParallelVolcanoExecutor（依赖 PR-860）
- PR-880: VTU Predicate/Mutation Pipeline（依赖 PR-870）
- PR-890: Snapshot + MVCC + Rollback（依赖 PR-840）
- PR-900: ExecutionEngine 拆分（依赖 PR-890）

### 6.3 依赖的 Crate

| Crate | 版本 | 说明 |
|-------|------|------|
| sqlrustgo-parser | any | SQL 解析 |
| sqlrustgo-planner | any | AST → PhysicalPlan |
| sqlrustgo-executor | any | LocalExecutor |
| sqlrustgo-storage | any | StorageEngine |
| sqlrustgo-transaction | any | TransactionManager（未来） |

---

## 7. 风险和缓解

### 7.1 风险 1:向后兼容破坏

**风险**: 现有代码可能直接调用 `eng.execute(sql)`，PR-800 会破坏这些调用。

**缓解**:
1. PR-800 后，`eng.execute(sql)` 改为调用 `execute_statement(sql)`（内部路由到 Parser → Planner → Executor）
2. 不删除 public API，只改变内部实现
3. 全面回归测试（mysql-server E2E tests）

### 7.2 风险 2:性能回归

**风险**: 新增的 Parser + Planner 调用可能引入延迟。

**缓解**:
1. Parser + Planner 开销应在 μs 级别，不影响整体延迟
2. 如果延迟可测量，在 PR-800 后建立 QPS 基线
3. 后续 PR-870 (ParallelVolcanoExecutor) 会补偿这部分开销

### 7.3 风险 3:测试覆盖缺失

**风险**: 旧路径 `eng.execute(raw_sql)` 可能没有对应的 Parser 测试。

**缓解**:
1. PR-800 后，所有 SQL 都经过 Parser
2. 现有的 parser_coverage_tests 应该覆盖大部分 SQL
3. 如果有 SQL 类型未被覆盖，添加到 parser_coverage_tests

---

## 8. SSOT 引用

- `docs/releases/v3.8.0/ARCHITECTURE.md` — v3.8.0 架构文档
- `docs/releases/v3.8.0/DEVELOPMENT_PLAN.md` — PR DAG
- `docs/releases/v3.7.0/ARCHITECTURE_VIOLATIONS.md` — AV-001~AV-010
- `src/execution_engine.rs` (行 337) — 当前 `execute()` 入口