# SQLRustGo 执行架构 (Execution Architecture)

> **Version**: v1.0 (v3.8.0-rc1)
> **Date**: 2026-06-04
> **Author**: Hermes Agent
> **Issue**: #2974 (ARCH-2)
> **Audience**: 所有 SQLRustGo contributor
> **Status**: **Draft, 待 PR-3037 review**

---

## 0. 背景与目的

SQLRustGo 当前存在 **2 套 ExecutionEngine**:

```
src/execution_engine.rs         (root crate, 1661 行, 50 methods, 主力)
crates/executor/src/execution/  (crates/executor, 4 methods, abstract trait)
```

**问题**:
- `src::ExecutionEngine` 有完整 DML 实现 (`execute_insert/update/delete`)
- `crates::ExecutionEngine` trait 只有 `execute` (SQL string) + `begin/commit/rollback`
- `MergeExecutor` (crates/executor) 拿不到 root 的 DML API
- 解决方案: `MergeExecutor` 走 SQL 字符串重新解析 (低效, 易错)

**ARCH-2 目标**: 统一 DML 入口, 让 `crates::ExecutionEngine` trait 也能直接调用 DML.

**本次 PR (Stage 1)**:
- ✅ 公开 `src::ExecutionEngine::execute_insert/update/delete` (1 行 × 3)
- ✅ 写本 ARCHITECTURE.md
- ✅ 加 DML API 集成测试
- ⏳ 后续 (Stage 2-3): 扩展 `crates::ExecutionEngine` trait + 统一 facade

---

## 1. 当前架构 (现状)

### 1.1 root: `src/execution_engine.rs::ExecutionEngine<MemoryStorage>`

**位置**: `/src/execution_engine.rs` (1661 行)

**类型参数**: `ExecutionEngine<S: Storage>` (默认 `MemoryStorage`)

**DML 公开方法** (本次 PR 公开):
- `pub fn execute_insert(&mut self, insert: &InsertStatement) -> SqlResult<ExecutorResult>`
- `pub fn execute_update(&mut self, update: &UpdateStatement) -> SqlResult<ExecutorResult>`
- `pub fn execute_delete(&mut self, delete: &DeleteStatement) -> SqlResult<ExecutorResult>`

**特性**:
- 完整 DML 实现 (含 trigger, MVCC, WAL, constraint check)
- 内部自动事务管理 (INT-1 修复, PR-3019)
- 受 `tx_status` 状态机保护
- **外部代码可调用**

### 1.2 crates: `crates/executor::ExecutionEngine` (trait)

**位置**: `/crates/executor/src/execution/engine.rs` (393 字节)

**方法** (4 个):
- `fn execute(&mut self, ctx: &mut QueryContext) -> Result<ExecutionResult, SqlError>`
- `fn begin(&mut self) -> Result<u64, SqlError>`
- `fn commit(&mut self, txn: u64) -> Result<(), SqlError>`
- `fn rollback(&mut self, txn: u64) -> Result<(), SqlError>`

**特性**:
- **抽象 trait**, 无 DML 直接方法
- 接受 SQL 字符串 (重新 parse)
- `LocalExecutor` 是主实现
- **外部代码只能走 `execute()`**

### 1.3 `MergeExecutor` (crates/executor/src/merge.rs)

**位置**: `/crates/executor/src/merge.rs` (23K)

**当前实现** (line 95-128):
```rust
// VTU path: execute UPDATE through ExecutionEngine
let update_sql = self.build_update_sql(...);
let mut ctx = QueryContext::new(update_sql);
self.engine.lock().unwrap().execute(&mut ctx)?;  // 重新 parse SQL

// VTU path: execute INSERT through ExecutionEngine
let insert_sql = self.build_insert_sql(...);
let mut ctx = QueryContext::new(insert_sql);
self.engine.lock().unwrap().execute(&mut ctx)?;  // 重新 parse SQL
```

**问题**:
- 每次 MERGE 操作重新 parse SQL (低效)
- `build_update_sql` 构造 SQL 字符串有 escape 风险
- 当 PK 不是第 0 列时, `build_update_sql` 的 WHERE filter 错位 (潜在 bug, 之前 review 发现)
- **不通过 root `ExecutionEngine` 的 DML API** (因为在 crates 没法访问)

---

## 2. 统一 DML 入口 (ARCH-2 目标)

### 2.1 Stage 1 (本 PR, 5h) ✅
- ✅ 公开 `src::ExecutionEngine::execute_insert/update/delete` (3 行修改)
- ✅ 写 ARCHITECTURE.md (本文档)
- ✅ 写 DML API 集成测试 (`tests/arch2_dml_api_test.rs`)
- ✅ 验证 root DML API 可被外部调用

### 2.2 Stage 2 (后续 PR, 5h)
- ⏳ 扩展 `crates::ExecutionEngine` trait:
  ```rust
  pub trait ExecutionEngine {
      // 已有 4 个
      fn execute(&mut self, ctx: &mut QueryContext) -> Result<ExecutionResult, SqlError>;
      fn begin(&mut self) -> Result<u64, SqlError>;
      fn commit(&mut self, txn: u64) -> Result<(), SqlError>;
      fn rollback(&mut self, txn: u64) -> Result<(), SqlError>;

      // 新增 3 个
      fn execute_insert(&mut self, insert: &InsertStatement) -> Result<ExecutionResult, SqlError>;
      fn execute_update(&mut self, update: &UpdateStatement) -> Result<ExecutionResult, SqlError>;
      fn execute_delete(&mut self, delete: &DeleteStatement) -> Result<ExecutionResult, SqlError>;
  }
  ```
- ⏳ `LocalExecutor` impl 这 3 个新方法 (内部委托给 root `ExecutionEngine`)
- ⏳ `NoopExecutionEngine` impl 空版本 (返回 error)

### 2.3 Stage 3 (后续 PR, 5h)
- ⏳ `MergeExecutor` 改用新 trait API
- ⏳ 删除 `build_insert_sql` / `build_update_sql` (不再需要)
- ⏳ 修 `build_update_sql` 的 PK filter 错位 bug

### 2.4 Stage 4 (后续 PR, 后续版本)
- 旧 SQL re-parse path 加 `#[deprecated]`
- 全部 callers 切换

---

## 3. 关键设计决策 (ADR)

### ADR-001: DML API 公开 (本 PR Stage 1)
- **决策**: 改 `fn` → `pub fn` (3 行)
- **理由**: 外部代码 (MergeExecutor 等) 需要 DML 直接访问
- **风险**: 公开 API 增加维护成本
- **缓解**: 详细文档 + 单元测试 + 集成测试

### ADR-002: Stage 2 加 3 个 trait 方法, 不重命名
- **决策**: 在 `ExecutionEngine` trait 加 3 个新方法, 不替换 `execute`
- **理由**: 兼容现有 callers, 渐进迁移
- **风险**: trait method 增多
- **缓解**: 详细 ADR 文档

### ADR-003: LocalExecutor 内部委托, 不重写 DML
- **决策**: `LocalExecutor::execute_insert` 内部调用 root `ExecutionEngine::execute_insert`
- **理由**: 避免重复 DML 实现
- **风险**: 多一层抽象
- **缓解**: 文档清晰, 性能开销小

### ADR-004: MergeExecutor Stage 3 改用 DML API
- **决策**: 替换 SQL re-parse 为直接 DML API
- **理由**: 性能 + 正确性 (修 PK filter 错位 bug)
- **风险**: 改 MergeExecutor 测试覆盖可能不足
- **缓解**: 保留 MERGE E2E tests + 加 Stage 3 单元测试

---

## 4. 不变量 (Invariants)

### 4.1 DML 不变量
- DML 必须**只**通过 `execute_insert/update/delete` API
- 禁止直接 `storage.insert/update/delete` (bypass TM)
- 禁止 SQL re-parse 后用 `execute()` 走 DML (低效)

### 4.2 错误处理
- DML API 返回 `Result<ExecutorResult, SqlError>`
- 错误时自动 rollback (PR-3019 INT-1 修复)

### 4.3 性能不变量
- 简单 INSERT P95 < 5ms
- 简单 UPDATE P95 < 5ms
- 简单 DELETE P95 < 5ms

### 4.4 API 稳定性
- `pub fn execute_insert/update/delete` 签名**不能**变 (Stage 2 之后)
- 任何变更需要走 ADR + PR review

---

## 5. 测试要求

### 5.1 Stage 1 测试 (`tests/arch2_dml_api_test.rs`)

```rust
// 验证 3 个 DML API 可被外部调用
#[test]
fn arch2_execute_insert_public() {
    let engine = ExecutionEngine::with_memory();
    let insert = InsertStatement { table_name: "t".into(), values: vec![...] };
    let result = engine.execute_insert(&insert);
    assert!(result.is_ok());
}
```

### 5.2 回归测试
- 全量 `cargo test` 通过
- D9 9 维门禁 8/8 ALL PASS
- INT-1 6/6 PASS (无 regression)

### 5.3 Stage 2-3 测试 (后续)
- `LocalExecutor::execute_insert` 转发测试
- `MergeExecutor` 用 DML API E2E 测试
- MERGE 性能基准 (比 SQL re-parse 快 30%+)

---

## 6. 反模式 (Anti-Patterns)

### 6.1 ❌ 禁止 SQL 字符串构造 DML
```rust
// 错误
let sql = format!("INSERT INTO {} VALUES ({})", table, value);
ctx.execute(&sql)?;
```

### 6.2 ❌ 禁止直接 storage 访问 (bypass TM)
```rust
// 错误
storage.insert(table, row);  // bypass TM
```

### 6.3 ❌ 禁止用 execute() 走 DML
```rust
// 错误 (低效)
let update_sql = build_update_sql(...);
self.engine.execute(&mut QueryContext::new(update_sql))?;

// 正确
self.engine.execute_update(&update_stmt)?;
```

---

## 7. 与其他 issue 关系

| Issue | 关系 |
|-------|------|
| #2966 INT-1 (PR-3019) | 修 autocommit 强制 TM, 本 PR 复用 |
| #2973 INT-4 | 修 explicit TX 也走 TM, 本 PR 复用 |
| #2975 SEM-1 (PR-3035) | 统一执行语义, 本 PR 进一步实施 |
| #2977 TPCH-01 | Stage 1 完成有助 TPCH 22/22 |
| V380_ROADMAP | rc1 阶段 ARCH-2 必做项 |

---

## 8. Stage 进度追踪

| Stage | 内容 | 状态 | 工作量 |
|-------|------|------|--------|
| **1** | 公开 DML API + 文档 + 测试 | ✅ DONE (本 PR) | 5h |
| 2 | 扩展 `crates::ExecutionEngine` trait | ⏳ 待 PR-3038 | 5h |
| 3 | `MergeExecutor` 改用 DML API | ⏳ 待 PR-3039 | 5h |
| 4 | 旧 API deprecate | ⏳ v3.9.0+ | 后续 |

**总 15h, 与 Issue #2974 估算一致** (本次 PR Stage 1 = 5h, 后续 PR Stage 2-3 = 10h)

---

**v3.8.0-rc1 启动: ARCH-2 merge.rs 统一 DML 入口 (Issue #2974)**
**Stage 1: 公开 DML API + ARCHITECTURE.md + 集成测试**
**Stage 2-3: 后续 PR 完成 trait 扩展 + MergeExecutor 迁移**
**总 15h, 与 Issue #2974 估算一致**
