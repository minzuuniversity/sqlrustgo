# Issue #2626: Storage Layer 独立集成测试链路

## 目标

建立不依赖 executor 模块的 Storage Layer 独立集成测试体系，验证存储引擎、Buffer Pool、WAL 等核心组件在隔离环境下正常工作。

## 为什么独立

当前状态：
- `sqlrustgo-executor`: ❌ 无法编译（Issue #2625 修复中）
- `sqlrustgo-storage`: ✅ 编译通过
- `sqlrustgo-types`: ✅ 编译通过
- `sqlrustgo-parser`: ✅ 编译通过

可以完全并行开发和测试。

---

## 阶段 S1: Storage Engine 核心测试

**目标:** 验证基础存储操作

### 1.1 Buffer Pool 测试

- [ ] `test_buffer_pool_allocate_and_evict` — 分配页面，触发驱逐
- [ ] `test_buffer_pool_pin_unpin` — 页面 pin/unpin 引用计数
- [ ] `test_buffer_pool_flush_dirty` — 脏页刷盘

### 1.2 File Storage 测试

- [ ] `test_file_storage_read_write` — 基础读写
- [ ] `test_file_storage_page_lifecycle` — 创建/读取/更新/删除页面
- [ ] `test_file_storage_concurrent_access` — 并发读写冲突

### 1.3 WAL 基础测试

- [ ] `test_wal_append_and_replay` — 追加日志并回放
- [ ] `test_wal_segment_rotation` — 段切换
- [ ] `test_wal_recovery_after_crash` — 崩溃恢复

---

## 阶段 S2: WAL Verification 强化

**目标:** wal-verification 模块已有结构，增强测试覆盖

### 2.1 Verification 模块测试

- [ ] `test_wal_consistency_check` — WAL 内部一致性验证
- [ ] `test_wal_chain_integrity` — WAL 链完整性
- [ ] `test_wal_lsn_ordering` — LSN 顺序保证

### 2.2 Recovery 场景测试

- [ ] `test_recovery_from_partial_write` — 部分写入恢复
- [ ] `test_recovery_txn_atomicity` — 事务原子性恢复
- [ ] `test_recovery_with_checkpoints` — 检查点恢复

---

## 阶段 S3: Storage Integration Tests

**目标:** 完整的存储层集成测试

### 3.1 End-to-End Storage Test

- [ ] `test_storage_full_lifecycle` — 创建数据库 → 建表 → 插入 → 查询 → 更新 → 删除
- [ ] `test_storage_crash_recovery` — 模拟崩溃的数据完整性
- [ ] `test_storage_concurrent_transactions` — 并发事务隔离

### 3.2 Backup/Restore 测试

- [ ] `test_backup_and_restore` — 备份恢复完整性
- [ ] `test_incremental_backup` — 增量备份

---

## 测试文件位置

```
crates/storage/tests/
  ├── buffer_pool_test.rs
  ├── file_storage_test.rs
  ├── wal_test.rs
  ├── wal_verification_test.rs
  └── storage_integration_test.rs
```

---

## 验收标准

1. 所有 Storage Layer 测试通过（不依赖 executor）
2. 测试覆盖率报告生成
3. 独立 CI 流水线验证

---

## 责任人

- Hermes A: 可选（主要精力在 #2625）
- 可分配给其他 Agent 或 Herme C

---

## 与 Issue #2625 的关系

| 依赖 | 说明 |
|------|------|
| ❌ 无依赖 | Storage、Parser、Types、Optimizer 编译通过 |
| ⚠️ 注意 | 最终集成测试需等 #2625 完成后 |

---

## 并行价值

- 缩短整体开发时间（Storage 测试与 Executor 修复并行）
- 提前发现 Storage 层问题
- 建立回归测试基线