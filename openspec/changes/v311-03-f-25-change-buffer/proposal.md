# V311-03: F-25 Change Buffer 主路径集成

## 目标

将 F-25 Change Buffer 从 ISOLATED 测试代码提升至 `crates/storage/src/` 主路径集成。

## 当前状态

**ISOLATED 位置**: `tests/integration/sql/change_buffer_test.rs`

ChangeBuffer 是内存 mock 实现，验证了以下 API：
- `defer_update(page_id, op)` — 延迟二级索引更新
- `merge_on_read(page_id)` — 读取时合并延迟的更新
- `flush()` — 强制刷新所有延迟更新
- `should_flush()` — 判断是否达到刷新阈值

**问题**: ISOLATED 代码 0 主路径调用。

## 目标架构

```text
StorageEngine.write_row(record)
    ↓
┌─ 如果表有二级索引 ──────────────────────┐
│  1. 主记录写入 ClusteredTable/BTree      │
│  2. 每个二级索引条目 defer_update()      │
│     → 写入 ChangeBuffer (不写磁盘)        │
└─────────────────────────────────────────┘

StorageEngine.read_page(page_id)
    ↓
┌─ Change Buffer merge ───────────────────┐
│  1. BufferPool.read_page(page_id)      │
│  2. 如果有延迟更新 → merge_on_read()   │
│  3. 返回合并后的 page                   │
└────────────────────────────────────────┘

ChangeBuffer 刷新触发:
    - 容量达到 max_entries (should_flush)
    - 后台线程定期 flush
    - 检查点(Checkpoint)前强制 flush
```

## 集成点

1. **StorageEngine trait**: 添加 `change_buffer: Option<Arc<ChangeBuffer>>` 字段
2. **BufferPool**: 在 `read_page()` 调用 `change_buffer.merge_on_read(page_id)`
3. **ChangeBuffer module**: 移入 `crates/storage/src/change_buffer.rs`，导出到 `lib.rs`

## 文件变更

| 操作 | 文件 |
|------|------|
| MOVE | `tests/integration/sql/change_buffer_test.rs` → `crates/storage/src/change_buffer.rs` |
| MODIFY | `crates/storage/src/lib.rs` — 添加 `pub mod change_buffer;` |
| MODIFY | `crates/storage/src/engine.rs` — StorageEngine trait 添加 change_buffer 字段 |
| MODIFY | `crates/storage/src/buffer_pool.rs` — read_page 调用 merge_on_read |
| NEW | `tests/change_buffer_main_path_test.rs` — 主路径集成测试 |
| NEW | `openspec/changes/v311-03-f-25-change-buffer/design.md` |

## 验收标准

- [ ] `ChangeBuffer` 在 `crates/storage/src/change_buffer.rs`
- [ ] `cargo build -p sqlrustgo` 编译通过
- [ ] `cargo test --test change_buffer_main_path_test` PASS
- [ ] `cargo test -p sqlrustgo --lib` PASS
- [ ] 集成测试覆盖: defer_update → merge_on_read 流程

## 依赖

- V311-01 (Clustered Index) — 建议有 ClusteredTable 作为主路径基座
- 无其他代码依赖

## 工作量

- 估算: 40h
- 实际: TBD
