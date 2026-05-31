# WAL Contract（契约守护者输出）

> Hermes A: Contract Guardian
> 职责：锁死 WAL/TX contract，不参与实现，只做规范与验证。

---

## 1. WAL Entry Invariants

### 1.1 WAL Before Commit Rule（核心铁律）

```
铁律 #WAL-001: 任何 DML 操作（INSERT/UPDATE/DELETE）
必须在数据页修改前先写入 WAL。
顺序绝对禁止颠倒。
```

**验证方式**：
- WAL entry LSN 必须小于等于数据页 LSN
- 如果数据页 LSN < WAL LSN → 违反契约 → 必须 panic

### 1.2 WAL Entry Types

| Entry Type | tx_id | table_id | key | data | 何时写入 |
|------------|-------|----------|-----|------|----------|
| `Begin` | ✅ | ❌ | ❌ | ❌ | `require_tx()` 调用时 |
| `Insert` | ✅ | ✅ | ✅ | ✅ | 行插入前 |
| `Update` | ✅ | ✅ | ✅ | ✅ | 行更新前 |
| `Delete` | ✅ | ✅ | ✅ | ❌ | 行删除前 |
| `Commit` | ✅ | ❌ | ❌ | ❌ | commit 调用时 |
| `Rollback` | ✅ | ❌ | ❌ | ❌ | rollback 调用时 |
| `Checkpoint` | ❌ | ❌ | ❌ | ❌ | checkpoint 时 |

### 1.3 WAL Entry Ordering Constraints

```
1. 每笔 tx 第一个 entry 必须是 Begin
2. Commit/Rollback 必须是该 tx 的最后一个 entry
3. 中间可以有多条 Insert/Update/Delete
4. 不同 tx 的 entry 交错但严格保序（LSN 单调递增）
```

**违反此顺序 → panic**

### 1.4 Idempotency Under Replay

WAL replay 必须幂等：

| Scenario | Replay 行为 | 要求 |
|----------|-------------|------|
| Commit replay 两次 | 第二次忽略 | ✅ |
| Insert replay 两次 | 第二条忽略（duplicate key）| ✅ |
| Update replay 两次 | 幂等 | ✅ |
| Delete replay 两次 | 第二次忽略 | ✅ |
| Rollback replay 两次 | 第二次忽略 | ✅ |

**实现要求**：每个 entry 携带唯一标识（tx_id + LSN），replay 时检测重复。

---

## 2. DML Single Entry Point Contract

### 2.1 ExecutionEngine.execute() 是唯一入口

```
铁律 #DML-001: 所有 DML（INSERT/UPDATE/DELETE）必须经过
ExecutionEngine.execute() → 禁止绕过此路径直接写 storage。
```

### 2.2 require_tx Invariant

```rust
// 所有 DML 执行路径必须满足：
fn execute_dml(&mut self, ctx: &mut QueryContext) -> Result<ExecutionResult, SqlError> {
    // 前置条件：ctx.tx 必须为 Some
    assert!(ctx.tx.is_some(), "DML without active transaction");
    // 或：panic!("DML requires active transaction");
    
    // ... 执行 DML ...
}
```

**违反 `require_tx` 的场景（必须 panic）**：

| 场景 | 要求 |
|------|------|
| DML without BEGIN | ❌ 必须 panic |
| DML after COMMIT | ❌ 必须 panic |
| DML after ROLLBACK | ❌ 必须 panic |
| DML inside read-only tx | ❌ 必须 panic 或 return error |

---

## 3. WAL 契约验证测试缺口（MISSING TESTS）

> 以下测试当前不存在于 codebase，必须在 PR-800 中补齐。

### 3.1 WAL Order Violation Tests

```rust
// TEST WAL-001: 数据页修改早于 WAL 写入 → 必须 panic
#[test]
fn test_data_page_before_wal_panics() {
    // 构造场景：直接写数据页，跳过 WAL
    // 期望：panic 或 WAL assertion failure
}

// TEST WAL-002: commit 前未写 WAL entry → 必须 panic
#[test]
fn test_commit_without_wal_entry_panics() {
    // BEGIN → INSERT（跳过 WAL）→ COMMIT
    // 期望：panic
}

// TEST WAL-003: entry 顺序违反 → 必须 panic
#[test]
fn test_wal_entry_out_of_order_panics() {
    // BEGIN → COMMIT → INSERT（顺序错误）
    // 期望：panic
}
```

### 3.2 Transaction Lifecycle Tests

```rust
// TEST TX-001: DML without BEGIN → 必须 panic
#[test]
fn test_insert_without_tx_panics() {
    // 没有 BEGIN，直接执行 INSERT
    // 期望：panic "DML requires active transaction"
}

// TEST TX-002: DML after COMMIT → 必须 panic
#[test]
fn test_insert_after_commit_panics() {
    // BEGIN → INSERT → COMMIT → INSERT
    // 期望：panic
}

// TEST TX-003: DML after ROLLBACK → 必须 panic
#[test]
fn test_insert_after_rollback_panics() {
    // BEGIN → INSERT → ROLLBACK → INSERT
    // 期望：panic
}

// TEST TX-004: crash before commit → 必须 rollback
#[test]
fn test_crash_before_commit_rolls_back() {
    // BEGIN → INSERT（LSN=100）→ crash → restart
    // 期望：replay 后数据不存在（已 rollback）
}
```

### 3.3 WAL Replay Idempotency Tests

```rust
// TEST REPLAY-001: Commit replay 两次 → 第二次忽略
#[test]
fn test_commit_replay_twice_ignored() {
    // replay Commit(TX=1, LSN=50) 两次
    // 期望：第二次不报错，不重复提交
}

// TEST REPLAY-002: Insert replay 两次 → duplicate 忽略
#[test]
fn test_insert_replay_twice_duplicate_ignored() {
    // replay Insert(TX=1, key=K1, LSN=101) 两次
    // 期望：第二次不报错，主键冲突忽略
}

// TEST REPLAY-003: Update replay 两次 → 结果一致
#[test]
fn test_update_replay_twice_idempotent() {
    // replay Update(TX=1, key=K1, data=D2, LSN=102) 两次
    // 期望：最终数据 = D2
}
```

### 3.4 Partial Write Recovery Tests

```rust
// TEST RECOVERY-001: Prepare 后 crash → rollback
#[test]
fn test_prepare_then_crash_rolls_back() {
    // 分布式事务：Prepare(TX=1) → crash
    // 期望：recovery 扫描到 Prepare 无 Commit → 回滚
}

// TEST RECOVERY-002: Begin 后 crash → rollback
#[test]
fn test_begin_then_crash_rolls_back() {
    // BEGIN(TX=1) → INSERT → crash（无 Prepare）
    // 期望：replay 后数据不存在
}

// TEST RECOVERY-003: partial page write → recovery 正确
#[test]
fn test_partial_page_write_recovery() {
    // 写入 8KB 数据，crash 在第 4KB
    // 期望：replay 后 page 状态一致
}
```

---

## 4. MISSING TESTS 汇总

| ID | Category | 描述 | 期望结果 |
|----|----------|------|----------|
| WAL-001 | WAL Order | 数据页修改早于 WAL 写入 | panic |
| WAL-002 | WAL Order | commit 前未写 WAL entry | panic |
| WAL-003 | WAL Order | WAL entry 顺序违反 | panic |
| TX-001 | Lifecycle | DML without BEGIN | panic |
| TX-002 | Lifecycle | DML after COMMIT | panic |
| TX-003 | Lifecycle | DML after ROLLBACK | panic |
| TX-004 | Lifecycle | crash before commit | rollback |
| REPLAY-001 | Replay | Commit replay 两次 | 第二次忽略 |
| REPLAY-002 | Replay | Insert replay 两次 | duplicate 忽略 |
| REPLAY-003 | Replay | Update replay 两次 | 结果一致 |
| RECOVERY-001 | Recovery | Prepare 后 crash | rollback |
| RECOVERY-002 | Recovery | Begin 后 crash | rollback |
| RECOVERY-003 | Recovery | partial page write | 状态一致 |

**总计：13 个缺失测试**

---

## 5. 契约违规检测机制

### 5.1 VtuGuard（硬约束）

```rust
pub struct VtuGuard {
    tx_active: bool,
    wal_written: bool,
}

impl VtuGuard {
    pub fn assert_wal_before_data(&self) {
        assert!(self.wal_written, "WAL entry must precede data page modification");
    }
    
    pub fn assert_require_tx(&self) {
        assert!(self.tx_active, "DML requires active transaction");
    }
}
```

### 5.2 WAL LSN Ordering Check

```rust
pub fn validate_page_lsn(&self, page_lsn: u64, wal_lsn: u64) {
    if page_lsn < wal_lsn {
        // 数据页 LSN 小于 WAL LSN = 先写数据后写 WAL = 违规
        panic!("WAL CONTRACT VIOLATION: data page LSN {} < WAL LSN {}", page_lsn, wal_lsn);
    }
}
```

---

## 6. 当前代码中的契约缺口（Evidence）

### 6.1 ExecutionEngine.execute() 入口检查缺失

**文件**: `crates/executor/src/execution/engine.rs`

当前代码**未检查** `ctx.tx.is_some()`：

```
grep 搜索结果：无 require_tx 检查
```

→ 这是 #DML-001 违反，需要修复。

### 6.2 WAL Entry Type 完整性检查缺失

**文件**: `crates/storage/src/wal.rs`

当前 `log_update` / `log_delete` 实现**不完整**：

```
grep 搜索结果：log_delete 存在但未被充分测试
```

→ replay 幂等性未验证。

---

## 7. 验证方法

### 7.1 静态检查（编译期）

```rust
// 在 DML 执行路径上强制注入 VtuGuard
// 确保每个 execute_dml 调用前有 require_tx 检查
```

### 7.2 动态检查（运行时）

```bash
# 运行 contract validation tests
cargo test --package sqlrustgo-executor contract::
cargo test --package sqlrustgo-storage wal_contract::
```

### 7.3 覆盖率要求

| 测试集 | 最低覆盖率 |
|--------|-----------|
| WAL Contract Tests | 95% |
| TX Lifecycle Tests | 90% |
| Replay Idempotency Tests | 90% |
| Recovery Tests | 85% |

---

**文档状态**: DRAFT
**作者**: Hermes A (Contract Guardian)
**日期**: 2026-05-31
**版本**: v0.1