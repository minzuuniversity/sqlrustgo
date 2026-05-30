# ISSUE #2626: Storage Layer 独立集成测试链路

## 目标

建立 Storage Layer 独立集成测试体系，实现 sqlrustgo-storage crate 的完整测试覆盖。

## 当前状态

- **Lib tests**: 186 passed (all in-tree)
- **Integration tests**: 10 new (5 BufferPool + 5 WAL)
- **总计**: 196 tests
- **Integration tests 目录**: `crates/storage/tests/` (新建)

## Top 5 缺失测试区域

### 1. replication 模块 (replication.rs)
- **原因**: `mod tests { ... }` 块存在但测试函数未被 cargo test 发现（模块非 pub）
- **当前有**: `test_binlog_event_serialize`, `test_binlog_event_ddl` 等基础测试
- **缺失**: 主从复制流程、binlog 同步、故障转移场景
- **优先级**: HIGH - 复制是 HA 核心功能
- **下一步**: 将 `replication` 改为 `pub mod` 或在 lib.rs 中重新导出测试

### 2. failover_manager 模块 (failover_manager.rs)
- **原因**: `mod tests { ... }` 存在但未注册
- **当前有**: `test_node_info_creation`, `test_failover_config_default` 等
- **缺失**: 自动故障转移流程、健康检查、节点状态转换
- **优先级**: HIGH - 直接影响系统可用性

### 3. checkpoint 模块 (checkpoint.rs)
- **原因**: `mod tests { ... }` 存在但未被 cargo test 发现
- **当前有**: `test_checkpoint_metadata_serialization`, `test_checkpoint_manager_default`
- **缺失**: checkpoint 触发条件、增量 checkpoint、checkpoint 与 WAL 联动
- **优先级**: MEDIUM - 影响恢复时间目标 (RTO)

### 4. wal_storage 模块 (wal_storage.rs)
- **原因**: `mod tests { ... }` 存在但未被 cargo test 发现
- **当前有**: `test_wal_storage_basic_insert`, `test_wal_storage_rollback`
- **缺失**: WAL 与存储引擎的完整集成测试、崩溃恢复
- **优先级**: HIGH - WAL 是事务核心

### 5. Buffer Pool + File Storage 集成测试
- **已完成**: ✅ 5 个集成测试 (crates/storage/tests/integration_buffer_pool.rs)
- **待补充**: Buffer Pool + FileStorage 联动、页面置换与持久化联动

## 阶段计划

### 阶段 S1: Buffer Pool + File Storage + WAL ✅ (已完成)
- [x] BufferPool 基本操作测试
- [x] BufferPool 容量/长度测试
- [x] BufferPool prefetch 配置测试
- [x] BufferPool stats 初始状态测试
- [x] WAL 单事务测试
- [x] WAL 多事务测试（commit + rollback）
- [x] WAL 崩溃恢复测试（未提交事务不恢复）
- [x] WAL checkpoint 测试

### 阶段 S2: WAL Verification 强化
- [ ] WAL Verification 与 WalManager 集成测试
- [ ] Checkpoint atomicity 验证测试
- [ ] LSN monotonicity 验证测试

### 阶段 S3: End-to-End Storage Integration
- [ ] BufferPool + FileStorage 联动测试
- [ ] WAL + Checkpoint 联动测试
- [ ] 故障转移场景测试

## 已创建的测试文件

| 文件 | 测试数 | 内容 |
|------|--------|------|
| `crates/storage/tests/integration_buffer_pool.rs` | 5 | BufferPool 基本操作、容量、prefetch、stats、get |
| `crates/storage/tests/integration_wal.rs` | 5 | WAL 创建、单事务、多事务、崩溃恢复、checkpoint |

## 测试验证

```bash
$ cargo test -p sqlrustgo-storage --lib
test result: ok. 186 passed; 0 failed

$ cargo test -p sqlrustgo-storage --tests
test result: ok. 10 passed; 0 failed

$ cargo test -p sqlrustgo-storage
test result: ok. 196 passed; 0 failed
```

## 关键发现

1. **私有模块测试不可见**: `checkpoint`, `failover_manager`, `replication`, `wal_storage` 在 lib.rs 中是 `mod`（私有）而非 `pub mod`，导致 `#[cfg(test)]` 下的测试对 `cargo test` 不可见
2. **WalManager API**: `new(PathBuf)` 而非 `new(&PathBuf)`; 无 `current_lsn()` 方法
3. **BufferPool API**: `new(capacity: usize)` 无 storage backend 参数；页面操作使用 `u32` page_id

## 与 #2625 关系

无代码依赖，可完全并行。sqlrustgo-storage 编译通过，可独立开发和测试。

## 验收标准

- [x] 所有 Storage Layer 测试通过 (196+ tests)
- [ ] 私有模块测试可见化（pub mod 或重新导出）
- [ ] WAL Verification 集成测试补充
- [ ] End-to-End Storage Integration 测试补充
- [ ] 独立 CI 配置