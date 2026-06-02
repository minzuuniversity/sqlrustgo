# PR-800F SPEC — TransactionalFacade 实现层

> **PR Number**: PR-800F (F-06 子项)
> **PR Title**: TransactionalFacade 实现层 (WalTransactionalFacade)
> **Version**: v3.8.0 Phase 0 Architecture Freeze
> **Branch**: `test/v380-test-coverage-a1-a4` (基于 develop/v3.8.0)
> **Auditor**: Hermes Agent
> **Created**: 2026-06-02
> **Status**: ACTIVE — 既有实现 + 缺失测试

---

## 1. 概述

### 1.1 目标

PR-800 Foundation 定义了 `TransactionalFacade` trait 作为执行引擎的事务抽象层。**当前状态**：
- ✅ Trait 已定义（`crates/executor/src/execution/transactional_facade.rs`）
- ✅ `WalTransactionalFacade` 已实现（`wal_transactional_facade.rs`，145 行）
- ✅ `WriteOp` enum 已定义（`write_op.rs`）
- ✅ `DriftGate` 强制层已实现
- ❌ **缺失独立测试** —— 既无 `transactional_facade_test.rs` 也无 `wal_transactional_facade_test.rs`

### 1.2 功能范围

**已实现**（本 PR 验证）：
- `begin()` — 开启事务，分配 tx_id，写 WAL log_begin
- `commit()` — 提交事务，写 WAL log_commit，返回 commit timestamp
- `rollback()` — 回滚事务
- `is_in_transaction()` — 查询事务状态
- `current_tx_id()` — 获取当前 tx_id
- `execute_write(WriteOp)` — 执行 INSERT/UPDATE/DELETE
- `execute_read(sql)` — 委托 storage 执行查询
- `validate_operation(WriteOp, TransactionContext)` — DriftGate 校验

**未实现**（不在本 PR 范围）：
- `execute_read` 完整 SQL 解析（当前为 storage 直传）
- 多语句事务（multi-statement transaction）的 ACID 全套
- 跨存储后端的统一抽象（当前仅 WalStorage 包装）

---

## 2. 接口签名

```rust
// crates/executor/src/execution/transactional_facade.rs
pub trait TransactionalFacade: Send + Sync {
    fn begin(&self) -> SqlResult<u64>;
    fn commit(&self) -> SqlResult<Option<u64>>;
    fn rollback(&self) -> SqlResult<()>;
    fn is_in_transaction(&self) -> bool;
    fn current_tx_id(&self) -> Option<u64>;
    fn execute_write(&self, ctx: &TransactionContext, op: WriteOp) -> SqlResult<ExecutionResult>;
    fn execute_read(&self, sql: &str) -> SqlResult<ExecutionResult>;
    fn validate_operation(&self, op: &WriteOp, ctx: &TransactionContext) -> Result<(), DriftViolation>;
}
```

---

## 3. WalTransactionalFacade 行为规约

| 方法 | 前置条件 | 后置条件 | 错误情形 |
|------|----------|----------|----------|
| `begin()` | 无 | tx_manager.begin() 返回新 tx_id；storage.log_begin(tx_id) 写入 WAL | storage.begin_transaction() 失败 |
| `commit()` | is_in_transaction() == true | DriftGate.validate_pre_commit 通过；storage.commit_transaction；storage.log_commit | DriftGate 拒绝 → ExecutionError |
| `rollback()` | is_in_transaction() == true | storage.rollback_transaction；tx_manager.rollback | 任一步失败 |
| `execute_write(op)` | 事务内（ctx 有效） | DriftGate 校验；storage.insert/update/delete；storage.log_mutation | DriftGate 拒绝 / storage 失败 |

---

## 4. 测试覆盖目标

### 4.1 单元测试 (L1) — 10+ 用例

| ID | 场景 | 期望 |
|----|------|------|
| UTF-01 | 初始状态 `is_in_transaction() == false` | true |
| UTF-02 | `begin()` 后 `is_in_transaction() == true` 且 tx_id 非 0 | true |
| UTF-03 | `current_tx_id()` begin 后返回 Some(n) | Some(1) |
| UTF-04 | 重复 `begin()` 应递增 tx_id | 1, 2, 3... |
| UTF-05 | `commit()` 后 `is_in_transaction() == false` | true |
| UTF-06 | `rollback()` 后 `is_in_transaction() == false` | true |
| UTF-07 | 未 begin 直接 commit → error | ExecutionError |
| UTF-08 | 未 begin 直接 rollback → 不 panic | Ok(()) |
| UTF-09 | `execute_write` 在事务外 → 行为可接受（不强制 error） | 测一次执行不 panic |
| UTF-10 | `validate_operation` Insert 合法 op → Ok | Ok(()) |

### 4.2 集成测试 (L2) — 8+ 用例

| ID | 场景 | 期望 |
|----|------|------|
| WTF-01 | begin → INSERT → commit → 数据持久化 | rows.len == 1 |
| WTF-02 | begin → INSERT → rollback → 数据未持久化 | rows.len == 0 |
| WTF-03 | begin → INSERT 1 → INSERT 2 → commit → 2 行 | rows.len == 2 |
| WTF-04 | 多个 begin/commit 循环 → tx_id 递增 | tx_id1 < tx_id2 |
| WTF-05 | UPDATE in transaction → commit → row updated | column value changed |
| WTF-06 | DELETE in transaction → commit → row gone | rows.len == n-1 |
| WTF-07 | DriftGate 拒绝冲突 op → 不会写 storage | execute_write returns Err |
| WTF-08 | 并发 begin/commit (4 threads × 10 iters) → 最终一致 | 40 transactions OK |

### 4.3 边界/异常 (L3) — 5+ 用例

| ID | 场景 | 期望 |
|----|------|------|
| WTF-E1 | 空字符串表名 INSERT → storage 错误 | Err (Storage 错误，非 panic) |
| WTF-E2 | UPDATE 不存在的 table → storage 错误 | Err |
| WTF-E3 | DELETE 空表 → Ok(0 rows) | rows_affected == 0 |
| WTF-E4 | 1000 次 INSERT 在一个事务中 → commit 成功 | rows.len == 1000 |
| WTF-E5 | 嵌套 commit (commit without begin) → 错误 | Err |

---

## 5. 不在本 PR 范围

- 与 mysql-server COM_QUERY 集成（PR-810 范围）
- 与 ExecutionEngine::execute 集成（PR-810 范围）
- 完整 MVCC 集成（PR-890 范围）
- 跨存储后端实现（保留 future work）

---

## 6. 验收参考

- `PR-800F_TEST_PLAN.md` — 测试计划
- `PR-800F_TEST_DESIGN.md` — 测试设计
- `PR-800F_TEST_REVIEW.md` — 测试审核
- `PR-800F_ACCEPTANCE.md` — 测试验收

---

**最后更新**: 2026-06-02  
**更新者**: Hermes Agent
