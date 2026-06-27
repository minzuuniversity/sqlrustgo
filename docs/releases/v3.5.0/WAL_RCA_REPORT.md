# WAL 回归根因分析报告

> Issue: #1395  
> 日期: 2026-05-26  
> 状态: **根因已定位**

---

## 执行摘要

**根因**: `WalStorage::commit_transaction()` 中的冗余双次 flush

**性能影响**:
- WAL flush latency: ~8ms (目标 < 5ms)
- System Risk: 0.38

---

## 根因分析

### 问题 1: 冗余双次 flush

**文件**: `crates/storage/src/wal_storage.rs:59-62`

```rust
if self.wal_enabled {
    self.wal.log_commit(tx_id)?;  // 内部调用 writer.flush()
    self.wal.sync()?;              // 又打开文件调用 sync_all()
}
```

**调用链问题:**

| 层级 | 操作 | 延迟 |
|------|------|------|
| `writer.flush()` | BufWriter 刷到 OS 缓冲区 | ~0.1-0.5ms |
| `file.sync_all()` | 完整 fsync 到磁盘 | **~5-10ms** |

**问题**: `sync()` 方法每次调用都打开新文件句柄并执行 `sync_all()`，这是主要的延迟来源。

### 问题 2: observability 锁竞争

**文件**: `crates/storage/src/wal.rs:326-331`

```rust
if let Ok(stats) = sqlrustgo_observability::tables::OBSERVABILITY
    .wal_stats
    .write()  // 每次 write 都要获取锁
{
    stats.record_write(bytes.len() as u64, lsn);
}
```

**问题**: 每次 `append()` 都要获取 `wal_stats` 锁，高并发时会造成锁竞争。

---

## 修复建议

### 方案 1: 移除冗余 sync_all()

在 commit 路径中保留 `writer.flush()` 即可，`sync_all()` 可移到 periodic checkpoint。

### 方案 2: 批量统计更新

减少 `wal_stats` 锁竞争，使用 batch 更新。

### 方案 3: GroupCommitWriter

考虑使用 `GroupCommitWriter` 替代每次 commit 都 flush。

---

## 相关文件

- `crates/storage/src/wal.rs` - WAL 实现
- `crates/storage/src/wal_storage.rs` - WAL 存储封装
- `crates/observability/src/tables/wal_stats.rs` - WAL 统计
