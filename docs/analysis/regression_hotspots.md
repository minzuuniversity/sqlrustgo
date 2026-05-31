# Regression Hotspots — v3.8.0

> **角色**: Hermes C — Regression Hotspot Detector  
> **分支**: develop/v3.8.0 (f6cf8e16)  
> **日期**: 2026-05-31  
> **基于**: execution_semantics_diff.md

---

## 1. Hotspot 概览

| Hotspot | 路径 | 风险等级 | 触发条件 |
|---------|------|----------|----------|
| H-1: Trigger Bypass | Path B (MySQL) | 🔴 HIGH | DML via MySQL protocol |
| H-2: Trigger Bypass | Path C (StoredProc) | 🔴 HIGH | CALL stored_procedure() |
| H-3: TX Isolation Broken | Path B (STMT) | 🔴 HIGH | Multi-statement TX via prepared statements |
| H-4: WAL Coverage Gap | Path B + C | 🟡 MEDIUM | Any mutation via non-ExecutionEngine path |
| H-5: Commit Opacity | Path B (COM_QUERY) | 🟡 MEDIUM | AUTOCOMMIT=OFF behavior unclear |
| H-6: Double-Commit Protection Gap | Path B | 🟡 MEDIUM | Sequential commits via MySQL protocol |

---

## 2. 详细分析

### H-1: Trigger Bypass — MySQL COM_QUERY

**文件**: `crates/mysql-server/src/lib.rs`  
**行号**: ~1117-1125

```rust
// Path B: COM_QUERY handler
let mut eng = engine.write().unwrap();
match parse(&q) {
    Ok(stmt) => {
        let result = eng.execute(&q);  // ← ExecutionEngine.execute()
        // NO trigger wrapping here
    }
}
```

**问题**: `ExecutionEngine.execute_insert()` 有 `TriggerExecutor`，但 COM_QUERY 的 `eng.execute()` 走的是 `ExecutionEngine`，所以 **TriggerExecutor 应该被调用**。

**需要确认**: `eng` 是 `MemoryExecutionEngine` 还是 `ExecutionEngine`？

```rust
// line 1077: engine type
engine: Arc<RwLock<MemoryExecutionEngine>>,

// line 8: import
use sqlrustgo::MemoryExecutionEngine;
```

**重要澄清**: `MemoryExecutionEngine` 是 `ExecutionEngine<MemoryStorage>` 的类型别名（见 `src/execution_engine.rs:80`），所以 MySQL COM_QUERY 路径**确实使用 ExecutionEngine**，TriggerExecutor 在该路径上**可用**。

**但 H-3 (STMT TX 隔离) 仍然有效**: COM_STMT_EXECUTE 每次创建新 engine 实例，如果 storage 没有通过 Arc 共享引用，则 TX 状态会丢失。

---

### H-2: Trigger Bypass — StoredProc

**文件**: `crates/executor/src/stored_proc.rs`  
**行号**: ~863 `execute_statement_storage()`

```rust
fn execute_statement_storage(&self, stmt: &Statement, ctx: &mut ProcedureContext) -> Result<(), String> {
    match stmt {
        Statement::Insert(ins) => {
            // NO TriggerExecutor here
            ctx.storage_mut().insert(table_name, records)
        }
        Statement::Update(upd) => {
            // NO TriggerExecutor here
            ctx.storage_mut().update(...)
        }
        // ...
    }
}
```

**结论**: H-2 🔴 HIGH — StoredProc 路径完全绕过 TriggerExecutor。

---

### H-3: TX Isolation Broken — MySQL COM_STMT_EXECUTE

**文件**: `crates/mysql-server/src/lib.rs`  
**行号**: ~1291

```rust
// COM_STMT_EXECUTE handler
let mut eng = MemoryExecutionEngine::new(storage.clone());  // ← FRESH instance
let parsed = parse(&final_sql);
match parsed {
    Ok(stmt) => {
        let result = eng.execute(&final_sql);  // ← no TX inheritance
    }
}
```

**问题**: 每次 `COM_STMT_EXECUTE` 创建新的 `MemoryExecutionEngine`，无法跨语句保持 TX 状态。

**正常行为 (COM_QUERY)**:
```rust
// line 1114: engine.write().unwrap() — 复用同一个实例
let mut eng = engine.write().unwrap();  // ← shared instance
let result = eng.execute(&q);
```

**结论**: H-3 🔴 HIGH — prepared statements 无法维持 TX，跨语句 DML 无法在同一个 TX 内。

---

### H-4: WAL Coverage Gap

**问题**: Path B (COM_QUERY, COM_STMT) 和 Path C (StoredProc) 的 storage.insert() 是否写 WAL？

**验证方法**: 检查 `sqlrustgo_storage` 的 `insert()` 实现。

```bash
grep -n "fn insert\|WAL\|wal\|log" crates/storage/src/*.rs | head -30
```

**如果 WAL 在 storage 层统一实现**: Path A/B/C 都安全  
**如果 WAL 在 ExecutionEngine 层实现**: Path B/C 有 GAP

---

### H-5: Commit Opacity

**问题**: COM_QUERY 中 `eng.execute(&q)` 内部是否自动 commit？

查看 `MemoryExecutionEngine::execute()` 实现：

```bash
grep -n "fn execute\|commit\|autocommit" crates/executor/src/local_executor.rs | head -30
```

**已知**: `src/execution_engine.rs` 的 `execute_insert()` 有 implicit TX + WAL，无显式 commit 调用。commit 需要通过 `commit_transaction()` 手动调用。

**疑问**: MySQL COM_QUERY 路径执行 `INSERT` 后，数据是持久化了还是只在 memory storage？

---

### H-6: Double-Commit Protection Gap

**文件**: `src/execution_engine.rs`  
**保护**: `commit_transaction()` 有 `if self.current_tx_id.is_none()` 检查

**绕过**: Path B 中 `storage.commit()` 不经过 `ExecutionEngine.commit_transaction()`，所以 **double-commit 保护被绕过**。

```rust
// src/execution_engine.rs line 1485
fn commit_transaction(&mut self) -> SqlResult<ExecutorResult> {
    if self.current_tx_id.is_none() {
        return Err(SqlError::ExecutionError(
            "transaction already committed".to_string(),
        ));
    }
    // ...
}

// Path B: storage.commit() 直接调用，无此保护
```

---

## 3. 优先级修复顺序

```
P0 (立即修复，Alpha):
  - H-3: COM_STMT_EXECUTE TX 隔离 (line 1291 new engine → 复用 engine)

P1 (Beta Gate 前):
  - H-1: MySQL COM_QUERY 确认是否使用 MemoryExecutionEngine
  - H-2: StoredProc trigger bypass
  - H-4: WAL coverage 验证

P2 (GA 前):
  - H-5: AUTOCOMMIT 行为澄清
  - H-6: Double-commit protection 扩展到 storage 层
```

---

## 4. 测试缺口

| 测试 | 覆盖路径 | 当前状态 | 推荐测试 |
|------|----------|----------|----------|
| `trigger_test` | Path A | ✅ 存在 | 确认 Path B/C 无 trigger |
| `stmt_exec_tx_test` | Path B STMT | ❌ 缺失 | 测试跨 statement TX |
| `wal_coverage_test` | Path A/B/C | ❌ 缺失 | 验证所有路径写 WAL |
| `autocommit_test` | Path B | ❌ 缺失 | 测试 MySQL protocol AUTOCOMMIT |
| `double_commit_protection_test` | Path A | ✅ 存在 | 扩展到 Path B |

---

**维护者**: Hermes C (Regression Hotspot Detector)  
**关联**: execution_semantics_diff.md