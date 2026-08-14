# Tasks — V312-56C: Transaction/crash recovery 教学实验

## Phase 1: 现状调研 ✅

### 1.1 已有事务测试
- [x] mvcc_transaction_test.rs - MVCC 事务测试
- [x] savepoint_test.rs - SAVEPOINT 测试
- [x] sem1_savepoint_test.rs - SEM-1 SAVEPOINT 测试

### 1.2 已有 Crash/Recovery 测试
- [x] process_kill_crash_test.rs - kill -9 场景
- [x] crash_monkey_test.rs - 随机 crash 测试
- [x] crash_test_framework.rs / harness.rs - crash 测试框架
- [x] recovery_fuzzer_test.rs - 恢复 fuzzer
- [x] recovery_scenarios_test.rs - 恢复场景测试

### 1.3 缺失项识别
- ❌ 没有统一的教学实验文档
- ❌ 没有 before/after count/hash 验证 fixture
- ❌ 没有 GMP-specific backup/restore 验证

## Phase 2: 教学实验文档

- [ ] 2.1 创建 `docs/teaching/transaction-basics.md` - BEGIN/COMMIT/ROLLBACK 教学指南
- [ ] 2.2 创建 `docs/teaching/savepoint.md` - SAVEPOINT 实验
- [ ] 2.3 创建 `docs/teaching/crash-recovery.md` - crash recovery 实验

## Phase 3: GMP Backup/Restore 验证

- [ ] 3.1 GMP documents backup/restore count/hash 验证
- [ ] 3.2 embeddings backup/restore 验证
- [ ] 3.3 graph edges backup/restore 验证
- [ ] 3.4 audit chain 完整性验证

## Phase 4: 必跑命令

- [ ] 4.1 `bash scripts/gate/check_v312_14_crash_recovery.sh` PASS
- [ ] 4.2 `cargo test --test wal_tx_contract_test -- --nocapture` PASS
- [ ] 4.3 `cargo test --test compatibility_harness -- --nocapture` PASS
- [ ] 4.4 `cargo test --test process_kill_crash_test -- --nocapture` PASS

## Acceptance Criteria

- [ ] 教学实验文档能逐步展示事务提交、回滚、savepoint 和可见性
- [ ] crash/recovery fixture 能生成 before/after count/hash
- [ ] backup/restore 后 GMP documents、embeddings、graph、audit chain 有 checksum
- [ ] 所有实验命令可在本地复跑
- [ ] 失败不得被文档描述为 PASS
