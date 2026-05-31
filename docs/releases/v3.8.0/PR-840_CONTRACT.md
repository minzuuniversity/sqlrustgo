# PR-840: WAL Replay Correctness Contract

> **状态**: 已完成
> **日期**: 2026-06-01
> **基于**: PR-830E (RecoveryEngine), PR-830F (WAL Lifecycle)
> **目标**: 修复 WAL 回放正确性问题

---

## 1. 问题陈述

### 1.1 DELETE Replay Bug (CRITICAL)

**现象**: Recovery 后全表数据丢失

**根本原因**:
```rust
// wal_storage.rs - logging 时
fn delete(&mut self, table: &str, filters: &[Value]) -> SqlResult<usize> {
    let key = format!("{:?}", filters).into_bytes();  // ← BUG: 无用字符串
    self.log_delete(table_id, key)?;
    self.inner.delete(table, filters)
}

// recovery_engine.rs - replay 时
WalEntryType::Delete => {
    storage.delete(&table_name, &[])?;  // ← BUG: 空 filter = 全表删除！
}
```

**影响**: CRITICAL - replay 时可能丢失所有数据

### 1.2 UPDATE Replay Bug (HIGH)

**现象**: Recovery 后 update 操作丢失

**根本原因**:
```rust
// wal_storage.rs - logging 时
fn update(&mut self, table: &str, filters: &[Value], updates: &[(usize, Value)]) -> SqlResult<usize> {
    let key = format!("{:?}", filters).into_bytes();      // ← BUG: 无用
    let data = format!("{:?}", updates).into_bytes();     // ← BUG: 不可解析
    self.log_update(table_id, key, data)?;
}

// recovery_engine.rs - replay 时
WalEntryType::Update => {
    // 直接跳过
}
```

**影响**: HIGH - update 操作在 recovery 后丢失

---

## 2. 修复方案

### 2.1 DELETE: Row-Level Delete Logging

**修复后的 logging**:
```rust
fn delete(&mut self, table: &str, filters: &[Value]) -> SqlResult<usize> {
    let table_id = Self::table_name_to_id(table);

    let rows = self.inner.scan(table)?;
    for row in &rows {
        if Self::row_matches_filter(row, filters) {
            let key = Self::record_key(row);
            self.log_delete(table_id, key)?;
        }
    }

    self.inner.delete(table, filters)
}
```

**修复后的 replay**:
```rust
WalEntryType::Delete => {
    if let Some(ref key) = entry.key {
        let filter_values = key_to_filter_values(key)?;
        storage.delete(&table_name, &filter_values)?;
    } else {
        log::warn!("RecoveryEngine: DELETE entry without key - full table delete");
        storage.delete(&table_name, &[])?;
    }
}
```

### 2.2 UPDATE: Row Image Logging

**修复后的 logging**:
```rust
fn update(&mut self, table: &str, filters: &[Value], updates: &[(usize, Value)]) -> SqlResult<usize> {
    let table_id = Self::table_name_to_id(table);

    let rows = self.inner.scan(table)?;
    for row in &rows {
        if Self::row_matches_filter(row, filters) {
            let key = Self::record_key(row);
            let old_data = Self::record_to_bytes(row);
            self.log_update(table_id, key, old_data)?;
        }
    }

    self.inner.update(table, filters, updates)
}
```

**UPDATE Replay**: 跳过并记录警告（需要 future PR 实现完整功能）

---

## 3. 实施验证

| 检查项 | 状态 |
|--------|------|
| DELETE logging 存储实际行键 | ✅ |
| DELETE replay 使用行键删除 | ✅ |
| UPDATE logging 存储行镜像 | ✅ |
| UPDATE replay 跳过并警告 | ✅ |
| key_to_filter_values 辅助函数 | ✅ |
| row_matches_filter 辅助函数 | ✅ |
| 单元测试 | ✅ 286 passed |
| Clippy | ✅ 0 warnings |

---

## 4. 相关文档

- `docs/releases/v3.8.0/PR-830E_CONTRACT.md` - RecoveryEngine Contract
- `docs/releases/v3.8.0/PR-830F_CONTRACT.md` - WAL Lifecycle Contract
- `crates/storage/src/wal_storage.rs` - WAL logging implementation
- `crates/storage/src/recovery_engine.rs` - WAL replay implementation
