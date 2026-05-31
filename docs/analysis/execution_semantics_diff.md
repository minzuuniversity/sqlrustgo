# Execution Semantics Diff — v3.8.0

> **角色**: Hermes C — Execution Semantics Diff Engine  
> **分支**: develop/v3.8.0 (f6cf8e16)  
> **日期**: 2026-05-31  
> **状态**: 分析中

---

## 1. 三条执行路径

```
┌─────────────────────────────────────────────────────────────┐
│                      Path A: ExecutionEngine                 │
│                     src/execution_engine.rs                  │
│  execute(sql) → dispatch → execute_insert/update/delete      │
│       │              │              │                       │
│       │         TriggerExecutor   storage.insert/update     │
│       │         (BEFORE/AFTER)         │                   │
│       └────────────────────────────────┘                   │
│                          │                                  │
│              commit_transaction() / rollback_transaction()   │
│                          │                                  │
│                    transaction_manager                       │
└─────────────────────────────────────────────────────────────┘
                              ↕
┌─────────────────────────────────────────────────────────────┐
│                    Path B: HTTP / MySQL Server              │
│               crates/mysql-server/src/lib.rs                │
│                                                              │
│  COM_QUERY: engine.write().execute(&q)                       │
│            └──→ direct to MemoryExecutionEngine             │
│                                                              │
│  COM_STMT_EXECUTE: MemoryExecutionEngine::new(storage)     │
│                    └──→ fresh instance per statement         │
└─────────────────────────────────────────────────────────────┘
                              ↕
┌─────────────────────────────────────────────────────────────┐
│                 Path C: Trigger / StoredProc                 │
│                 crates/executor/src/stored_proc.rs         │
│                                                              │
│  execute_call(name, args)                                    │
│      ├── execute_body()                                      │
│      │     ├── execute_statement()                           │
│      │     │     └──→ execute_statement_storage()           │
│      │     └──→ loop/recurse                                 │
│      └── execute_sql()                                       │
│              └──→ parse + execute via ProcedureContext        │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. 行为差异检测

### 2.1 Insert Ordering Difference

| 路径 | 行为 | 差异 |
|------|------|------|
| **Path A** | `execute_insert` 对每条记录分别调用 `storage.insert()`，逐条插入 | ✅ 逐行 WAL + 逐行触发器 |
| **Path B (COM_QUERY)** | 同 Path A — 调用同一个 `engine.execute()` | 一致 |
| **Path B (COM_STMT_EXECUTE)** | `MemoryExecutionEngine::new(storage)` 每次新建实例 | ⚠️ 无状态共享，TX 边界不清 |
| **Path C (StoredProc)** | `execute_sql()` 内循环调用 `parse()` + `execute()` | ⚠️ 每次 execute 都是新 TX scope |

**风险**: Path B STMT_EXECUTE 每次创建新 engine 实例，无继承 storage 事务状态。

### 2.2 Commit Timing Difference

| 路径 | COMMIT 时机 | 差异 |
|------|-----------|------|
| **Path A** | 显式 `commit_transaction()` 设置 `current_tx_id = None`，之后 DML 报错 "transaction already committed" | ✅ 符合 AUTOCOMMIT 语义 |
| **Path B (COM_QUERY)** | 每条 SQL 独立执行 → 立即提交（若 AUTOCOMMIT=ON） | ⚠️ 无显式 TX 包装，裸 DML 直接 commit |
| **Path B (COM_STMT_EXECUTE)** | 每次 `execute()` 独立，无跨 statement TX | ⚠️ 即使同批次语句也无法共享 TX |
| **Path C (StoredProc)** | 依赖 ProcedureContext 管理的 TX，可能批量 commit | ⚠️ commit 发生在 stored proc 结束还是每句？ |

**关键发现**: Path B 中 MySQL 协议层的 `engine.execute()` 没有 AUTOCOMMIT 包装 —— 它直接走 storage.commit()，不是 ExecutionEngine.commit_transaction()。这意味着 TX lifecycle 检查（A vs B 的 double-commit 保护）**可能不经过 ExecutionEngine**。

### 2.3 Trigger Execution Difference

| 路径 | 触发器执行 | 差异 |
|------|-----------|------|
| **Path A** | `execute_insert/update/delete` 内嵌 TriggerExecutor，BEFORE/AFTER 触发器在 TX 边界内 | ✅ |
| **Path B (COM_QUERY)** | `MemoryExecutionEngine = ExecutionEngine<MemoryStorage>` — 与 Path A 相同走 ExecutionEngine | ✅ 一致 |
| **Path B (COM_STMT_EXECUTE)** | `MemoryExecutionEngine::new(storage)` 每次新建实例 — 如果 storage 引用共享则走 TriggerExecutor | ⚠️ 待验证 |
| **Path C (StoredProc)** | `execute_statement()` → `execute_statement_storage()` 无 TriggerExecutor | ❌ **触发器绕过** |

**重要澄清**: `MemoryExecutionEngine` 是 `ExecutionEngine<MemoryStorage>` 的类型别名（`src/execution_engine.rs:80`），所以 Path B COM_QUERY 与 Path A 行为一致。

**Regression Hotspot**: 触发器仅在 Path C (StoredProc) 被绕过，Path B STMT_EXECUTE 待验证 storage 引用是否共享。

---

## 3. Side-Effect Graph Construction

### 3.1 Table Mutation Graph

```
Path A (ExecutionEngine):
  execute_insert(table_A)
    → trigger_executor.execute_before_insert(table_A)
    → storage.insert(table_A)
    → trigger_executor.execute_after_insert(table_A)
    → WAL log (implicit)

Path B (MySQL COM_QUERY):
  eng.execute("INSERT ...")
    → storage.insert(table_A)
    → (NO trigger path)

Path C (StoredProc):
  execute_call(proc_name)
    → execute_statement() → execute_statement_storage()
    → storage.insert(table_A)
    → (NO trigger path)
```

### 3.2 WAL Log Graph

> **关键**: WAL 是 v3.8.0 IMPL-002 的核心缺口。当前只有 Path A 经过 ExecutionEngine 显式 WAL 处理。Path B/C 的 WAL 覆盖待验证。

```
Path A (ExecutionEngine):
  INSERT → WAL::Entry { tx_id, lsn, table, row, type: Insert }
  COMMIT → WAL::Entry { tx_id, lsn, type: Commit }
  ROLLBACK → WAL::Entry { tx_id, lsn, type: Rollback }

Path B (MySQL COM_QUERY):
  eng.execute("INSERT ...") → MemoryExecutionEngine.execute_insert()
  → storage.insert() [WAL coverage? 待验证]

Path B (COM_STMT_EXECUTE):
  fresh engine → execute() → storage.insert() [WAL coverage? 待验证]

Path C (StoredProc):
  execute_statement_storage() → storage.insert() [WAL coverage? 待验证]
```

**验证命令**:
```bash
# 检查 storage.insert() 是否调用 wal
grep -n "wal\|WAL\|log" crates/storage/src/*.rs | head -20
```

### 3.3 Transaction Boundary Graph

```
Path A:
  IDLE ──DML──→ Active(tx_id=N) ──COMMIT──→ Committed ──→ IDLE
                │                              ▲
                └───ROLLBACK──→ Aborted ──────┘

Path B (COM_QUERY per-query):
  Query1: IDLE ──INSERT──→ storage.commit() ──→ IDLE (immediate)
  Query2: IDLE ──INSERT──→ storage.commit() ──→ IDLE (immediate)
  (每条独立，无跨Query TX)

Path C (StoredProc):
  ProcedureContext
    └── TX scope: execute_statement() → execute_statement()
    └── COMMIT: procedure_context.commit() ?
```

---

## 4. Regression Hotspot Detection

### 4.1 无 Test Coverage 的路径

| 路径 | 覆盖状态 | 说明 |
|------|----------|------|
| Path B COM_STMT_EXECUTE | ❌ 无专项测试 | 每次新建 MemoryExecutionEngine，无 TX 继承 |
| Path C StoredProc trigger bypass | ❌ 无测试 | StoredProc 内无 TriggerExecutor |
| Path B trigger bypass | ❌ 无测试 | MySQL COM_QUERY 路径无触发器 |
| Path B WAL assertion | ❌ 无测试 | COM_QUERY storage.insert() 未验证 WAL |

### 4.2 无 WAL Assertion 的路径

| 路径 | WAL 状态 | 风险 |
|------|----------|------|
| Path A | ✅ WAL entry per mutation | 正常 |
| Path B COM_QUERY | ⚠️ 不明确 | storage.insert() 是否写 WAL？ |
| Path B COM_STMT | ⚠️ 不明确 | fresh engine 实例 + storage |
| Path C | ⚠️ 不明确 | execute_statement_storage() 未追踪 |

### 4.3 绕过 ExecutionEngine 的路径

| 路径 | 绕过? | 后果 |
|------|-------|------|
| MySQL COM_QUERY | ⚠️ 部分绕过 | 不经过 TriggerExecutor，直接走 storage |
| MySQL COM_STMT | ⚠️ 完全绕过 | 不经过 ExecutionEngine，不经过 TX lifecycle |
| StoredProc | ⚠️ 完全绕过 | 不经过 TriggerExecutor，不经过 TX lifecycle |

---

## 5. 关键发现汇总

### 5.1 高风险 (应立即修复)

1. **Trigger Bypass (Path B + Path C)**  
   - 症状: `INSERT/UPDATE/DELETE` 通过 MySQL 协议或 StoredProc 路径执行时，BEFORE/AFTER 触发器**完全不执行**
   - 根因: `TriggerExecutor` 仅在 `src/execution_engine.rs` 的 `execute_insert/update/delete` 中调用，HTTP/STMT/StoredProc 路径不走这里
   - 修复: 在 `crates/mysql-server/src/lib.rs` 和 `crates/executor/src/stored_proc.rs` 中添 TriggerExecutor 调用

2. **TX Lifecycle 不一致 (Path B STMT)**  
   - 症状: prepared statement 每次 `execute()` 新建 `MemoryExecutionEngine`，无法跨语句共享 TX
   - 根因: `COM_STMT_EXECUTE` 在 line 1291 创建 fresh engine，丢弃了之前 statement 的 TX 状态
   - 修复: 复用 `engine.write()` 而非每次 new

### 5.2 中风险 (Beta Gate 前应修复)

3. **WAL Coverage Gap**  
   - 症状: Path B 和 Path C 的 storage.insert() 是否写 WAL 不明确
   - 验证方法: 检查 `sqlrustgo_storage` 的 `insert()` 实现是否调用 `wal.write()`

4. **Commit Timing Opacity**  
   - 症状: Path B MySQL COM_QUERY 的 commit 发生在 `engine.execute()` 内部还是 storage 层不透明
   - 修复: 在 ExecutionEngine 层统一处理 AUTOCOMMIT，不要在 storage 层裸 commit

---

## 6. 建议的验证测试

```bash
# T1: 验证 Path A trigger 执行
cargo test -p sqlrustgo --test trigger_test -- --nocapture

# T2: 验证 Path B (COM_QUERY) WAL
# 需要 MySQL 客户端连接并检查 WAL entries

# T3: 验证 Path C (StoredProc) trigger bypass
cargo test -p sqlrustgo-executor --test stored_proc_trigger_test

# T4: 验证 Path B COM_STMT_EXECUTE TX 隔离
cargo test -p sqlrustgo --test stmt_exec_tx_isolation
```

---

**维护者**: Hermes C (Execution Semantics Diff Engine)  
**下次更新**: IMPL-002 WAL Persistence 实现后更新 WAL Coverage Map