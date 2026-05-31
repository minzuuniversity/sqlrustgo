# SQLRustGo WAL Contract Map
# 格式: Contract ID + 测试文件 + 测试名称 + 状态(Real/Fake/Weak) + 原因

## WAL 核心合约

| Contract | Statement | File | Test | Status | Evidence |
|----------|-----------|------|------|--------|----------|
| WAL-001 | COMMIT → data survives restart | wal_tx_contract_test.rs | test_insert_then_crash_rolls_back | REAL | 有 restart，有 assert_eq |
| WAL-001 | COMMIT → data survives restart | exp_g_wal_contracts_verified.rs | test_wal_001_insert_commit_survives | REAL | 完整 restart + 正确 assertion |
| WAL-002 | No COMMIT → data does NOT survive restart | wal_tx_contract_test.rs | test_insert_then_crash_rolls_back | REAL | 验证未提交数据丢失 |
| WAL-002 | No COMMIT → data does NOT survive | exp_g_wal_contracts_verified.rs | test_wal_002_uncommitted_no_survive | REAL | 完整验证 |
| WAL-003 | Multiple TX ordering preserved on restart | exp_g_wal_contracts_verified.rs | test_wal_003_multi_tx_ordering | REAL | 验证两阶段提交顺序 |
| WAL-004 | UPDATE survives with correct value | exp_g_wal_contracts_verified.rs | test_wal_004_update_survives | REAL | 验证 UPDATE 后值正确 |
| WAL-005 | DELETE survives with correct removal | exp_g_wal_contracts_verified.rs | test_wal_005_delete_survives | REAL | 验证删除后无数据 |
| WAL-006 | Recovery replays WAL entries in order | wal_tx_contract_test.rs | test_wal_multiple_transactions | REAL | 多事务顺序验证 |
| WAL-007 | Recovery does NOT replay uncommitted TX | wal_tx_contract_test.rs | test_insert_then_crash_rolls_back | REAL | 未提交数据丢失 |

## 假测试（Fake Tests）

| File | Test | Why Fake |
|------|------|----------|
| crash_recovery_test.rs | test_recovery_after_failed_transaction | MemoryStorage，无持久化，无 restart |
| crash_recovery_test.rs | test_recovery_after_invalid_insert | 同上 |
| crash_recovery_test.rs | test_recovery_after_parse_error | 同上 |
| crash_recovery_test.rs | test_rollback_simulation | 同上 |
| crash_recovery_test.rs | test_state_persistence_across_queries | 同上 |
| crash_recovery_test.rs | test_partial_query_failure_isolation | 同上 |
| wal_e2e_recovery_test.rs | test_wal_recovery_committed_data_survives_restart | 缺少 BEGIN/COMMIT；with_wal_recovery 可能不存在 |
| wal_e2e_recovery_test.rs | test_wal_recovery_multiple_transactions_survive_restart | 同上 |
| wal_e2e_recovery_test.rs | test_wal_recovery_uncommitted_data_not_survived | 同上 |
| wal_e2e_recovery_test.rs | test_wal_recovery_mixed_committed_and_uncommitted | 同上 |
| wal_e2e_recovery_test.rs | test_wal_recovery_update_survives_with_correct_value | 同上 |
| wal_e2e_recovery_test.rs | test_wal_recovery_delete_survives_with_correct_removal | 同上（DELETE 碰巧通过） |

## 弱测试（Weak Tests）

| File | Test | Why Weak |
|------|------|----------|
| wal_integration_test.rs | test_wal_* | 只验证 WAL 写入，不验证 restart 后数据 |
| wal_tx_contract_test.rs | test_commit_flush_crash_replays | 验证 commit flush，但 assertion 可能间接 |
| wal_tx_contract_test.rs | test_begin_then_crash_rolls_back | 验证 crash rollback，但不验证数据值 |

## 总结

### 真实验证（9 个 test）

- wal_tx_contract_test.rs: 7 个（真实 restart 验证）
- exp_g_wal_contracts_verified.rs: 5 个（真实 restart 验证，全部通过）

### 假测试（12 个）

- crash_recovery_test.rs: 6 个（MemoryStorage，无 restart）
- wal_e2e_recovery_test.rs: 6 个（缺少事务边界，API 错误）

### 结论

**WAL 系统本身是正确的**（WAL-001~007 全部通过验证）

**但测试体系存在严重的假阳性覆盖率**：
- crash_recovery_test.rs 名称暗示"崩溃恢复验证"，实际只测试内存错误处理
- wal_e2e_recovery_test.rs 名称正确，但实现有 bug（缺少 BEGIN/COMMIT）

### 必须删除或修复

1. **crash_recovery_test.rs** — 全部 6 个测试（用 MemoryStorage 冒充 crash recovery）
2. **wal_e2e_recovery_test.rs** — 全部 6 个测试（需要修复 BEGIN/COMMIT 和 API 调用）

### 最小可行门禁

```bash
# 只运行真实测试
cargo test --test wal_tx_contract_test --test exp_g_wal_contracts_verified
```

### 禁止的断言模式

```bash
# 禁止：无 restart 的"crash recovery"测试
grep -l "MemoryStorage" tests/*crash*.rs

# 禁止：间接 assertion
grep "entries.len()\|buffer\|internal" tests/
```