# Phase 3-4：Savepoint MVCC + Storage + View/Procedure/Trigger

> **时间**: 2026-06-27
> **目标**: 完成剩余高级特性集成
> **匹配文档**: `docs/standard/SQL92_FUNCTIONALITY_MATRIX.md` §7.2-7.3

---

## Phase 3: Savepoint MVCC + Storage Gaps

### T1: Savepoint MVCC 物理回滚 (#3172)

**当前状态**:
- `savepoint.rs` undo_log 记录 Insert/Delete/Update 操作
- `rollback_to()` 仅清除 undo_log 条目，不还原物理数据
- **影响**: `ROLLBACK TO SAVEPOINT` 不能真正还原 tuple 状态

**需要的改动**:

| 模块 | 改动 | 风险 |
|------|------|------|
| `crates/transaction/src/savepoint.rs` | 实现 undo 记录的反向遍历和物理应用 | 低 |
| `crates/storage/src/engine.rs` | 提供 `StorageEngine::undo(key, old_value)` 或类似 API | 中 |
| `crates/executor/src/execution/engine.rs` | 在 rollback 路径中调用 SavepointManager undo | 中 |

**undo 应用逻辑** (已在 `savepoint.rs` 中预留):
```rust
for record in self.undo_log[sp.undo_log_index..].iter().rev() {
    match record {
        UndoRecord::Insert { key } => storage.delete(key),
        UndoRecord::Delete { key, old_value } => storage.insert(key, old_value),
        UndoRecord::Update { key, old_value } => storage.update(key, old_value),
    }
}
```

### T2: 实现 drop_column / modify_column

**当前状态**:
- `StorageEngine::drop_column()` 返回 `Err("not supported")`
- `StorageEngine::modify_column()` 返回 `Err("not supported")`

**需要的改动**:

| 引擎 | 改动 |
|------|------|
| `MemoryStorage` | `drop_column`: 从每行中移除列值 |
| `MemoryStorage` | `modify_column`: 更新 ColumnDefinition |
| `FileStorage` | `drop_column`: 读写 JSON 文件 |
| `FileStorage` | `modify_column`: 更新 schema |

---

## Phase 4: View + Procedure + Trigger

### T1: CTE 物化 + View 物化执行

**当前状态**:
- Parser: CTE/View 能解析 ✅
- Executor: `WithSelect` 已分发，提取 SELECT 执行 ✅
- Executor: CTE 名称引用 (FROM cte) 需要物化 ❌
- Executor: View 在 `views: HashMap<>` 中存储 SQL ❌（不执行）

**需要的改动**:
1. CTE 物化: 执行每个 CTE 子查询 → 结果存为临时表 → 执行主查询
2. View: 存储 view SQL，使用时展开为子查询

### T2: Procedure 执行框架

**当前状态**:
- Parser: `CREATE PROCEDURE` / `CALL` 能解析 ✅
- `execute_call()` 存在但需要 `Catalog` 支持
- `execute_create_procedure()` 存在但需要 `Catalog`

**需要的改动**:
1. 确保 Catalog 中存储过程生命周期完整
2. 实现 `CALL` 语句的参数传递和执行

### T3: Trigger 评估和执行

**当前状态**:
- Parser: `CREATE TRIGGER` 能解析 ✅
- `execute_create_trigger()` 存在且写入 Storage ✅
- Trigger 执行: `TriggerExecutor` 存在 (`trigger.rs`)
- Trigger 触发: 在 `execute_insert/update/delete` 中有调用 ✅

**需要的改动**:
1. 验证 Trigger 在 DML 路径中正确触发
2. 补充 Trigger 端到端测试

---

## 建议优先级

```
T1 (Savepoint) [P0] → T2 (drop_column) [P1] → T3 (Trigger test) [P1]
                                                 ↓
                                        T4 (CTE materialize) [P2]
                                                 ↓
                                        T5 (Procedure) [P3]
                                                 ↓
                                        T6 (View full) [P3]
```
