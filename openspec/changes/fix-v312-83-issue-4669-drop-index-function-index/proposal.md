# Proposal — Issue #4669: DROP INDEX / 函数索引 / 部分索引

## 状态: 部分实现

### DROP INDEX ✅ 已实现

`src/execution_engine.rs` 实现了 `execute_drop_index`：
```rust
fn execute_drop_index(&self, idx: &DropIndexStatement) -> SqlResult<ExecutorResult> {
    let storage = self.storage.read();
    let table_name = storage.list_all_indexes()
        .iter()
        .find(|i| i.name.to_lowercase() == idx.name.to_lowercase())
        .map(|i| i.table.clone())
        .unwrap_or_default();
    drop(storage);
    let mut write_storage = self.storage.write();
    write_storage.drop_index(&table_name, &idx.name)?;
    Ok(ExecutorResult::empty())
}
```

Storage 层的 `FileStorage::drop_index` 和 `MemoryStorage::drop_index` 已存在。

### 函数索引 ❌ 未实现

`CREATE INDEX ON t(UPPER(a))` 仍报错 "Column not found"。

### 部分索引 ❌ 未实现

`CREATE INDEX ... WHERE val > 200` 接受但不生效。

## 验证

```bash
cargo test --all-features --test multi_join_test test_drop_index  # passed
```
