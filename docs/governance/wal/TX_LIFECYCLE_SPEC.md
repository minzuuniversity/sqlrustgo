# Transaction Lifecycle Specification（事务生命周期规范）

> Hermes A: Contract Guardian
> 职责：锁死 WAL/TX contract，不参与实现，只做规范与验证。

---

## 1. 事务状态机

### 1.1 状态定义

```
                    ┌──────────────────────────────────────────┐
                    │                                          │
                    ▼                                          │
┌─────────┐    BEGIN    ┌─────────┐   DML    ┌───────────┐   COMMIT   ┌───────────┐
│  IDLE   │ ─────────▶  │ ACTIVE  │ ───────▶ │ MODIFYING │ ─────────▶ │ COMMITTED │
└─────────┘             └─────────┘          └───────────┘            └───────────┘
     ▲                      │                    │                       │
     │                      │                    │                       │
     │                      │ ROLLBACK            │ ROLLBACK              │
     │                      └─────────────────────┘                       │
     │                                                                   │
     └───────────────────────────────────────────────────────────────────┘
                                    ABORTED
```

### 1.2 状态说明

| 状态 | 含义 | 允许的操作 |
|------|------|-----------|
| `IDLE` | 无活跃事务 | BEGIN |
| `ACTIVE` | 事务已开始（已调用 BEGIN） | DML 查询 |
| `MODIFYING` | 事务有修改（已执行 INSERT/UPDATE/DELETE）| COMMIT / ROLLBACK |
| `COMMITTED` | 事务已提交（数据持久化）| 事务结束 |
| `ABORTED` | 事务已回滚 | 事务结束 |

---

## 2. 状态转换规则

### 2.1 合法转换

| 当前状态 | 操作 | 下一状态 | 副作用 |
|----------|------|----------|--------|
| IDLE | BEGIN | ACTIVE | 创建 tx 上下文，分配 tx_id |
| ACTIVE | SELECT | ACTIVE | 读取数据，快照隔离 |
| ACTIVE | DML | MODIFYING | 修改内存数据，标记 dirty |
| MODIFYING | COMMIT | COMMITTED | 写 WAL Commit，刷盘 |
| MODIFYING | ROLLBACK | ABORTED | 清除修改，恢复快照 |
| ACTIVE | ROLLBACK | ABORTED | 清除 tx 上下文 |

### 2.2 非法转换（必须 panic）

| 当前状态 | 操作 | 期望行为 |
|----------|------|----------|
| IDLE | DML | ❌ panic: "no active transaction" |
| ACTIVE | DML without tx | ❌ panic: "DML requires active transaction" |
| COMMITTED | DML | ❌ panic: "transaction already committed" |
| COMMITTED | COMMIT | ❌ panic: "transaction already committed" |
| ABORTED | DML | ❌ panic: "transaction already aborted" |
| ABORTED | COMMIT | ❌ panic: "transaction already aborted" |
| ANY | COMMIT after ROLLBACK | ❌ panic: "cannot commit after rollback" |

---

## 3. require_tx Invariant（强制事务检查）

### 3.1 定义

```rust
/// require_tx — DML 操作的前置条件检查
///
/// 所有 INSERT / UPDATE / DELETE 执行路径必须满足：
///   1. 存在活跃事务（tx.is_some()）
///   2. 事务状态为 ACTIVE 或 MODIFYING
///   3. 事务未 commit 或 abort
///
/// 违反上述任一条件 → 必须 panic
///
fn require_tx(ctx: &QueryContext) {
    assert!(ctx.tx.is_some(), "DML requires active transaction");
    let tx = ctx.tx.as_ref().unwrap();
    assert!(
        tx.state == TxState::Active || tx.state == TxState::Modifying,
        "Transaction not in valid state for DML"
    );
}
```

### 3.2 违反 require_tx 的场景

| 场景 | 代码路径 | 预期结果 |
|------|----------|----------|
| 无事务执行 INSERT | `execute("INSERT ...") without BEGIN` | panic |
| 无事务执行 UPDATE | `execute("UPDATE ...") without BEGIN` | panic |
| 无事务执行 DELETE | `execute("DELETE ...") without BEGIN` | panic |
| COMMIT 后执行 DML | `COMMIT → execute("INSERT ...") ` | panic |
| ROLLBACK 后执行 DML | `ROLLBACK → execute("INSERT ...")` | panic |
| 只读事务执行 DML | `BEGIN READ ONLY → INSERT` | panic 或返回错误 |

---

## 4. WAL Entry 生命周期

### 4.1 DML 与 WAL 的对应关系

| DML 操作 | WAL Entry 序列 | 何时写入 WAL |
|----------|---------------|--------------|
| INSERT | Begin → Insert → Commit | Insert 前写 WAL |
| UPDATE | Begin → Update → Commit | Update 前写 WAL |
| DELETE | Begin → Delete → Commit | Delete 前写 WAL |
| Bulk INSERT | Begin → (Insert)×N → Commit | 每条 Insert 前写 WAL |

### 4.2 WAL Entry 写入时序（铁律）

```
T1: BEGIN
T2: INSERT INTO t VALUES (...)   ← WAL entry Insert 必须在数据写入前
T3: UPDATE t SET ... WHERE ... ← WAL entry Update 必须在数据写入前
T4: COMMIT                     ← WAL entry Commit 必须在内存数据刷盘前
```

**任何违反上述时序的操作 → panic**

### 4.3 WAL LSN 不变量

```
LSN(T2) < LSN(数据页 T2)  // Insert: WAL 在前
LSN(T3) < LSN(数据页 T3)  // Update: WAL 在前
LSN(T4) < LSN(数据页 T4)  // Commit: WAL Commit 在前，刷盘在后
```

---

## 5. Failure Mode Table（故障模式表）

### 5.1 未提交事务（Uncommitted Transactions）

| 时间点 | 场景 | Recovery 行为 | 要求 |
|--------|------|---------------|------|
| T1 | BEGIN 后 crash | 回滚 | 数据不存在 ✅ |
| T2 | BEGIN → INSERT 后 crash | 回滚 | 数据不存在 ✅ |
| T3 | BEGIN → INSERT → COMMIT 前 crash | 回滚 | 数据不存在 ✅ |
| T4 | BEGIN → INSERT → COMMIT（crash before flush）| 回滚 | 数据不存在 ✅ |

### 5.2 未回滚事务（未完全回滚）

| 时间点 | 场景 | Recovery 行为 | 要求 |
|--------|------|---------------|------|
| T1 | BEGIN → ROLLBACK 后 crash | 正常 | 无数据 ✅ |
| T2 | ROLLBACK 执行中 crash | 完成回滚 | 无数据 ✅ |

### 5.3 Partial Write（部分写入）

| 时间点 | 场景 | Recovery 行为 | 要求 |
|--------|------|---------------|------|
| T1 | INSERT 8KB 数据，写入 4KB 时 crash | 回滚 | 数据不存在 ✅ |
| T2 | UPDATE 部分页写入时 crash | 回滚 | 保持原数据 ✅ |
| T3 | DELETE 部分写入时 crash | 回滚 | 保持原数据 ✅ |
| T4 | COMMIT 刷盘时 crash（部分页）| recovery 重放 WAL Commit | 重新刷盘完成 ✅ |

### 5.4 WAL Replay 故障

| 场景 | 预期行为 | 要求 |
|------|----------|------|
| Commit replay 两次 | 第二次忽略 | ✅ |
| Insert replay 两次（duplicate）| 第二次忽略 | ✅ |
| Update replay 两次 | 幂等 | ✅ |
| Delete replay 两次 | 第二次忽略 | ✅ |
| Rollback replay 两次 | 第二次忽略 | ✅ |
| 无对应 Begin 的 Commit | panic | ✅ |

---

## 6. Transaction 边界与 VtuGuard

### 6.1 VtuGuard 状态跟踪

```rust
pub struct VtuGuard {
    tx_id: Option<u64>,        // 当前事务 ID
    state: TxState,            // 当前状态
    wal_entry_written: bool,   // WAL entry 已写标志
    dirty: bool,               // 有未提交的修改
}

impl VtuGuard {
    // DML 前必须调用
    pub fn on_dml_start(&mut self) {
        assert!(self.tx_id.is_some(), "No active transaction");
        self.wal_entry_written = false;
    }
    
    // WAL 写入后调用
    pub fn on_wal_written(&mut self) {
        self.wal_entry_written = true;
    }
    
    // 数据页写入后验证 WAL 在前
    pub fn on_data_page_written(&mut self) {
        assert!(self.wal_entry_written, "WAL must precede data page");
    }
    
    // COMMIT 前验证
    pub fn on_commit(&mut self) {
        assert!(self.dirty || self.state == TxState::Active,
            "Nothing to commit");
    }
}
```

### 6.2 VtuGuard 调用点

| 调用点 | 检查项 |
|--------|--------|
| `ExecutionEngine::execute_dml()` | `require_tx()` + `on_dml_start()` |
| `Storage::write_page()` | `on_data_page_written()` |
| `WalManager::append()` | 记录 LSN |
| `ExecutionEngine::commit()` | `on_commit()` + WAL Commit 写入 |

---

## 7. 测试验证矩阵

| Test ID | 状态转换 | 操作 | 期望结果 | 当前状态 |
|--------|----------|------|----------|----------|
| TX-001 | IDLE → DML | INSERT | panic | ❌ 缺失 |
| TX-002 | ACTIVE → MODIFYING | INSERT | 进入 MODIFYING | ❌ 缺失 |
| TX-003 | MODIFYING → COMMITTED | COMMIT | 进入 COMMITTED | ✅ 存在 |
| TX-004 | MODIFYING → ABORTED | ROLLBACK | 进入 ABORTED | ✅ 存在 |
| TX-005 | COMMITTED → DML | INSERT | panic | ❌ 缺失 |
| TX-006 | ABORTED → DML | INSERT | panic | ❌ 缺失 |
| TX-007 | IDLE → ROLLBACK | ROLLBACK | panic | ✅ 存在 |
| TX-008 | BEGIN → COMMIT | COMMIT | 进入 COMMITTED | ✅ 存在 |
| TX-009 | BEGIN → ROLLBACK | ROLLBACK | 进入 ABORTED | ✅ 存在 |
| WAL-001 | WAL before data | Insert | WAL 在前 | ❌ 缺失 |
| WAL-002 | data before WAL | Insert | panic | ❌ 缺失 |
| WAL-003 | commit before WAL | Commit | panic | ❌ 缺失 |
| REPLAY-001 | Commit twice | Replay | 第二次忽略 | ❌ 缺失 |
| REPLAY-002 | Insert twice | Replay | duplicate 忽略 | ❌ 缺失 |
| RECOVERY-001 | crash before commit | Recovery | 回滚 | ❌ 缺失 |

---

**文档状态**: DRAFT
**作者**: Hermes A (Contract Guardian)
**日期**: 2026-05-31
**版本**: v0.1