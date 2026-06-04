# SQLRustGo 执行语义标准 (Execution Semantics Standards)

> **Version**: v1.0 (v3.8.0-rc1)
> **Date**: 2026-06-04
> **Author**: Hermes Agent
> **Issue**: #2975 (SEM-1)
> **Audience**: 所有 SQLRustGo contributor
> **Status**: **Draft, 待 PR-3034 review**

---

## 0. 背景与目的

SQLRustGo 当前存在 **5+ Executor 入口**:

```
executor::Executor (volcano-style, v1)
executor::MockExecutor (testing)
local_executor::LocalExecutor (主力 96K)
local_executor_dml::LocalExecutorDml
sql_executor::SqlExecutor (v2)
merge::MergeExecutor (新)
execution::engine::ExecutionEngine (旧)
execution::facade::ExecutionFacade
execution::transactional_facade::TransactionalFacade
execution::wal_transactional_facade::WalTransactionalFacade
```

**问题**:
- DML 之前能 bypass TransactionManager (PR-3019 修复)
- 各 facade 接口不统一
- 错误处理/结果集格式差异

**SEM-1 目标**: 统一执行模型, 定义**所有 SQL 执行路径的语义约定**.

**注意**: 本文档**不**删除旧 facade, 而是定义**统一 facade** 作为唯一推荐入口. 旧 facade 标记 `#[deprecated]`, 引导到新 facade.

---

## 1. 核心原则 (5 原则)

### 1.1 单一入口原则 (Single Entry Point)
- **所有 SQL 执行**必须通过 `ExecutionFacade::execute()`
- 旧 facade 标记 `#[deprecated]`

### 1.2 事务自动包裹原则 (Auto-Transaction Wrapping)
- **DML** (`INSERT`/`UPDATE`/`DELETE`/`MERGE`): 自动 begin + commit/rollback
- **DQL** (`SELECT`): 不需要显式事务
- **DDL** (`CREATE`/`DROP`/`ALTER`): 隐式 commit 之前事务

### 1.3 错误处理一致原则 (Consistent Error Handling)
- **所有 execute** 返回 `Result<T, SqlError>` (用 `SqlError`, 不引入新错误类型)

### 1.4 结果集一致原则 (Consistent Result Format)
- **DQL** → `ExecutorResult` (含 `rows: Vec<Vec<Value>>` + `affected_rows: usize`)
- **DML** → `ExecutorResult` (`affected_rows` 含触发器副作用)
- **DDL** → `ExecutorResult::empty()`

### 1.5 资源清理原则 (Resource Cleanup)
- 任何错误自动清理资源 (临时表, 锁, 游标)

---

## 2. 统一抽象 (已存在)

### 2.1 `Executor` trait (`crates/executor/src/executor.rs`)

```rust
pub trait Executor: Send + Sync {
    fn execute(&self, plan: &dyn PhysicalPlan) -> SqlResult<ExecutorResult>;
    fn name(&self) -> &str;
    fn is_ready(&self) -> bool;
}
```

**所有具体 executor 都应实现 `Executor` trait**:
- `LocalExecutor` (主力)
- `MergeExecutor` (新)
- `MockExecutor` (testing)

### 2.2 `ExecutorResult` (统一结果集)

```rust
pub struct ExecutorResult {
    pub rows: Vec<Vec<Value>>,
    pub affected_rows: usize,
}

impl ExecutorResult {
    pub fn new(rows: Vec<Vec<Value>>, affected_rows: usize) -> Self;
    pub fn empty() -> Self;
}
```

### 2.3 `SqlError` (统一错误, 已存在)

**15+ variant**:
- `ParseError(String)` - SQL 解析错误
- `ExecutionError(String)` - 执行错误
- `TypeMismatch(String)` - 类型不匹配
- `DivisionByZero` - 除零
- `NullValueError(String)` - NULL 错误
- `ConstraintViolation(String)` - 约束违反
- `TableNotFound(String)` - 表未找到
- `ColumnNotFound(String)` - 列未找到
- `DuplicateKey(String)` - 主键重复
- `IoError(String)` - I/O 错误
- `ProtocolError(String)` - 协议错误
- `TimeoutError(String)` - 超时
- `OverflowError(String)` - 数值溢出
- `Authentication(String)` - 认证

---

## 3. 各类 SQL 语句语义

### 3.1 DQL (SELECT)
1. 不需要显式事务
2. 内部 read-only transaction
3. 失败自动 rollback
4. 返回 `ExecutorResult { rows, affected_rows: 0 }`

### 3.2 DML (INSERT/UPDATE/DELETE/MERGE)
1. 自动 `begin_transaction()` (无 active tx)
2. 执行 DML
3. 触发 trigger
4. 写 WAL
5. 更新 MVCC
6. 自动 commit (autocommit) 或保持 open
7. 返回 `ExecutorResult { rows: vec![], affected_rows }`

### 3.3 DDL (CREATE/DROP/ALTER)
1. 隐式 commit 之前事务
2. 自动 begin + DDL + commit
3. 更新 catalog
4. 返回 `ExecutorResult::empty()`

### 3.4 TCL (BEGIN/COMMIT/ROLLBACK)
- `BEGIN` → `empty()`
- `COMMIT` → `empty()` (或 `affected_rows=0`)
- `ROLLBACK` → `empty()`

---

## 4. 关键不变量 (Invariants)

### 4.1 事务不变量
- 单个 execute **不能** 跨多 transaction
- 不允许嵌套 begin
- commit/rollback 后 tx_id 立即失效

### 4.2 一致性不变量
- 同 SQL 多次执行结果集一致 (read-only)
- DML `affected_rows` 含触发器副作用
- WAL 写入**必须先于** commit 返回

### 4.3 错误不变量
- 任何错误自动清理资源
- 错误不能留下 partial 状态
- 错误不能留下 orphan transaction

### 4.4 性能不变量
- 简单 SELECT P95 < 1ms
- 简单 INSERT P95 < 5ms
- 复杂 JOIN P95 < 100ms

---

## 5. 迁移路径

### 5.1 Stage 1 (rc1 启动, 5h, 本 PR) ✅
- ✅ 写 SEMANTICS.md
- ✅ 创建 `sem1_semantics_test.rs` 基础测试
- ⏳ 实际代码: 0 修改 (复用现有抽象)

### 5.2 Stage 2 (rc1 中期, 10h)
- 内部委托: `LocalExecutor::execute` → `ExecutionFacade::execute`
- 旧 facade 加 `#[deprecated]`
- 写 `sem1_deprecation_test.rs`

### 5.3 Stage 3 (rc1 末期, 5h)
- 所有 callers 切换
- 全量回归测试

---

## 6. 测试要求

### 6.1 单元测试 (`sem1_semantics_test.rs`)

- ✅ `sem1_1_executor_trait_unified` - 5+ facade 遵循 trait
- ✅ `sem1_2_executor_result_format` - 结果集统一
- ✅ `sem1_3_error_type_unified` - SqlError 统一
- ✅ `sem1_4_dml_auto_transaction` - 自动事务 (PR-3019 已实现)
- ✅ `sem1_5_consistency_invariants` - 不变量

### 6.2 回归测试
- 全量 `cargo test` 通过
- Corpus 通过率不下降

---

## 7. 拒绝的反模式

### 7.1 ❌ 多个执行入口
```rust
// 错误
let r1 = executor.execute(stmt);
let r2 = local_executor.execute(stmt);
```

### 7.2 ❌ 错误类型不一致
```rust
// 错误
let r: Result<Row, SqlError> = ...;
let r: Result<Row, ExecutorError> = ...;
```

### 7.3 ❌ 结果集格式不一致
```rust
// 错误
let r: ExecutorResult = ...;  // rows + affected_rows
let r: ExecutionResult = ...; // affected_rows + payload
```

### 7.4 ❌ 手动事务管理
```rust
// 错误
let tx = facade.begin_transaction();
let r = facade.execute_dml(stmt, tx);
facade.commit_transaction(tx);
```

---

## 8. 与其他文档的关系

- **ARCH-2** (#2974): 统一 DML 入口
- **INT-4** (#2973): VtuGuard 强制 DML 经过 TM
- **INT-1** (#2966, PR-3019): DML 真实走 TM (已修)
- **V380_ROADMAP.md**: rc1 阶段 7 项之一

---

## 9. ADR

### ADR-001: 不引入新错误类型
- **背景**: 已有 `SqlError` 含 15+ variant
- **决策**: 复用 `SqlError`, 不引入 `ExecutionError`
- **理由**: 避免错误类型碎片化

### ADR-002: 不重写现有 facade
- **背景**: 5+ facade 并存
- **决策**: 定义 SEMANTICS, 复用现有 `Executor` trait
- **理由**: 现有抽象已统一, 不需要新增

---

**v3.8.0-rc1 启动: SEM-1 (Issue #2975)**
**Stage 1 (5h): 本 SEMANTICS.md + 5 单元测试**
**Stage 2-3 (15h): 实际代码迁移**
**总 20h, 与 Issue #2975 估算一致**
