# PR-870: VTU Merge — MergeExecutor 接入

> **版本**: v1.0
> **日期**: 2026-06-01
> **PR 系列**: PR-870
> **状态**: STUB - 未完成

---

## 1. 功能概述

### 1.1 目标

PR-870 旨在将 MergeExecutor 接入 LocalExecutor，实现完整的 MERGE 语句执行能力。

### 1.2 当前状态

**PR-870 是 STUB 实现**：代码只返回错误消息，`execute_merge()` 从未被实际调用。

```rust
// local_executor.rs 约 line 1501
if sql_upper.starts_with("MERGE") {
    return Err(SqlError::ExecutionError(
        "MERGE via ExecutionEngine: wired but needs parser support. ".to_string()
    ));
}
```

---

## 2. 已知缺口 (CRITICAL)

### 2.1 阻止 MERGE 执行的缺口

| ID | 缺口 | 优先级 | 说明 |
|----|------|--------|------|
| G1 | Parser 不支持 MERGE 语法 | P0 | crates/parser/src/ 中无 MERGE 匹配 |
| G2 | `execute_merge()` 从未被调用 | P0 | 代码只返回错误 |
| G3 | LocalExecutor 缺少 `Arc<Mutex<dyn ExecutionEngine>>` | P1 | MergeExecutor::new() 需要此参数 |
| G4 | `WHEN MATCHED THEN DELETE` 未实现 | P2 | 只支持 UPDATE/INSERT |

### 2.2 完整实现需要

1. **Parser**: 添加 MERGE SQL 语法支持
   ```rust
   // 需要在 parser/src/ 添加 MERGE 规则
   ```

2. **LocalExecutor**: 暴露 `Arc<Mutex<dyn ExecutionEngine>>`
   ```rust
   // 当前 LocalExecutor 只有 storage_arc
   // 需要添加 execution_engine_arc
   ```

3. **MergeExecutor**: 实现完整的 WHEN MATCHED/WHEN NOT MATCHED 逻辑
   ```rust
   // 当前只支持 UPDATE/INSERT
   // 需要添加 DELETE 分支
   ```

---

## 3. 接口定义

### 3.1 目标接口

```rust
// LocalExecutor 需要添加
trait LocalExecutor {
    fn execute_merge(&self, ctx: &QueryContext, stmt: MergeStatement) -> Result<ExecutionResult, SqlError>;
}
```

### 3.2 MergeExecutor 需要的参数

```rust
impl MergeExecutor {
    pub fn new(
        storage: Arc<RwLock<dyn StorageEngine>>,
        execution_engine: Arc<Mutex<dyn ExecutionEngine>>,
    ) -> Self {
        // ...
    }
}
```

---

## 4. 数据结构

### 4.1 MergeStatement

```rust
struct MergeStatement {
    target_table: TableName,
    source_table: TableNameOrSubquery,
    on_condition: Expr,
    matched: Vec<MatchedAction>,    // WHEN MATCHED THEN ...
    not_matched: Vec<NotMatchedAction>, // WHEN NOT MATCHED THEN ...
}

enum MatchedAction {
    Update { set: Vec<SetClause> },
    Delete,
}

enum NotMatchedAction {
    Insert { columns: Vec<Identifier>, values: Vec<Expr> },
}
```

### 4.2 当前支持的语法

```sql
MERGE INTO target_table USING source_table
ON target.key = source.key
WHEN MATCHED THEN
    UPDATE SET col1 = source.val1
WHEN NOT MATCHED THEN
    INSERT (col1, col2) VALUES (source.val1, source.val2)
```

---

## 5. 测试设计

### 5.1 单元测试 (当前)

```rust
// crates/executor/tests/merge_vtu_test.rs
// 只测试 MergeStatement 构造，不测试实际执行
```

### 5.2 需要的 E2E 测试

| 测试 | 描述 | 状态 |
|------|-------|------|
| test_merge_basic | 基本的 MERGE INTO ... WHEN MATCHED | ❌ 缺失 |
| test_merge_insert | WHEN NOT MATCHED INSERT | ❌ 缺失 |
| test_merge_update | WHEN MATCHED UPDATE | ❌ 缺失 |
| test_merge_delete | WHEN MATCHED DELETE | ❌ 缺失 |
| test_merge_multi | 多行 MERGE | ❌ 缺失 |

### 5.3 测试用例

```rust
#[test]
fn test_merge_basic() {
    // 1. 创建 target 表
    // 2. 创建 source 表
    // 3. 执行 MERGE
    // 4. 验证 target 表数据正确
}
```

---

## 6. 验收标准

### 6.1 功能验收

| 验收点 | 标准 |
|--------|------|
| Parser 支持 MERGE | 可以解析 MERGE SQL |
| execute_merge 被调用 | 不再返回错误 |
| MERGE 执行正确 | UPDATE/INSERT/DELETE 按预期执行 |

### 6.2 测试验收

| 验收点 | 标准 |
|--------|------|
| E2E 测试存在 | test_merge_* 系列测试存在 |
| 测试通过 | cargo test --test merge_vtu_test |

---

## 7. 实现计划

### Phase 1: Parser 支持
- [ ] 添加 MERGE 语法到 parser/src/grammar.y
- [ ] 添加 MERGE AST 节点
- [ ] 添加解析测试

### Phase 2: LocalExecutor 改造
- [ ] 添加 `Arc<Mutex<dyn ExecutionEngine>>` 到 LocalExecutor
- [ ] 实现 execute_merge 方法
- [ ] 连接 MergeExecutor

### Phase 3: MergeExecutor 完善
- [ ] 实现完整的 matched/not_matched 逻辑
- [ ] 添加 WHEN MATCHED DELETE 支持
- [ ] 添加 E2E 测试

---

## 8. 风险

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| Parser 修改复杂 | 高 | 参考现有 INSERT/UPDATE 语法 |
| 并发 MERGE | 中 | 添加适当的锁机制 |
| 性能 | 中 | 批量操作优化 |

---

*本文档为 PR-870 功能设计报告 v1.0 - STUB*
