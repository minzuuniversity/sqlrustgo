# Proposal — V312-56C: Transaction/crash recovery 教学实验

## Why

Issue #4253: 事务和恢复实验未形成教学闭环。BEGIN/COMMIT/ROLLBACK/SAVEPOINT、隔离可见性、kill -9/WAL replay、checkpoint、backup/restore 需要可验证的实验。

## What Changes

### 1. 事务教学实验

创建教学实验文档，逐步展示:
- BEGIN/COMMIT/ROLLBACK 语义
- SAVEPOINT 和 savepoint rollback
- 隔离级别与可见性 (READ COMMITTED 等)
- 并发事务冲突

### 2. Crash/Recovery Fixtures

- `kill -9` 场景: 生成 before/after count/hash
- WAL replay: 验证 crash 后数据一致性
- dirty page recovery: 验证 buffer pool 恢复

### 3. Backup/Restore 验证

Backup/restore 后验证:
- GMP documents row count/hash
- embeddings count
- graph edges count
- audit chain 完整性

### 4. 必跑命令验证

所有实验命令可在本地复跑，失败不得被文档描述为 PASS。

## Capabilities

### New Capabilities

- **事务教学实验文档** - 可逐步执行的实验指南
- **crash/recovery fixture** - 可验证的 before/after 状态
- **backup/restore 验证** - 包含 checksum 验证

### Modified Capabilities

- 现有 WAL/transaction 实现 → 添加教学实验覆盖

## Non-goals

- 不实现新的事务隔离级别
- 不实现分布式事务
- 不实现多节点 backup

## Acceptance Criteria

- [ ] 新增教学实验文档，展示事务提交、回滚、savepoint 和可见性
- [ ] crash/recovery fixture 能生成 before/after count/hash
- [ ] backup/restore 后 GMP documents、embeddings、graph、audit chain 有 checksum
- [ ] 所有实验命令可在本地复跑
- [ ] 运行 `bash scripts/gate/check_v312_14_crash_recovery.sh` PASS
- [ ] 运行 `cargo test --test wal_tx_contract_test -- --nocapture` PASS
- [ ] 运行 `cargo test --test compatibility_harness -- --nocapture` PASS

## Issue Reference

Issue #4253 (V312-56C)
