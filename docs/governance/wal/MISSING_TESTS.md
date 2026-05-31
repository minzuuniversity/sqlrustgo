# MISSING TESTS — Contract Validation Gaps

> Hermes A: Contract Guardian
> 核心价值：防止 opencode 做出"能跑但不对"的实现。

---

## 1. TX Lifecycle Tests（事务生命周期测试）

### 1.1 require_tx Invariant Tests

| Test ID | Description | Expected | Priority | File Location |
|---------|-------------|----------|----------|---------------|
| `tx_lifecycle::test_insert_without_tx_panics` | 无事务执行 INSERT | panic "DML requires active transaction" | P0 | `crates/executor/src/execution/` |
| `tx_lifecycle::test_update_without_tx_panics` | 无事务执行 UPDATE | panic | P0 | `crates/executor/src/execution/` |
| `tx_lifecycle::test_delete_without_tx_panics` | 无事务执行 DELETE | panic | P0 | `crates/executor/src/execution/` |
| `tx_lifecycle::test_insert_after_commit_panics` | COMMIT 后执行 INSERT | panic | P0 | `crates/executor/src/execution/` |
| `tx_lifecycle::test_insert_after_rollback_panics` | ROLLBACK 后执行 INSERT | panic | P0 | `crates/executor/src/execution/` |
| `tx_lifecycle::test_dml_in_readonly_tx_panics` | 只读事务执行 DML | panic 或返回错误 | P1 | `crates/executor/src/execution/` |
| `tx_lifecycle::test_double_commit_panics` | 重复 COMMIT | panic | P0 | `crates/transaction/src/` |

### 1.2 State Transition Tests

| Test ID | Description | Expected | Priority |
|---------|-------------|----------|----------|
| `tx_state::test_idle_to_modifying_invalid` | IDLE 状态下直接 MODIFYING | panic | P0 |
| `tx_state::test_committed_to_dml_invalid` | COMMITTED 后执行 DML | panic | P0 |
| `tx_state::test_aborted_to_dml_invalid` | ABORTED 后执行 DML | panic | P0 |
| `tx_state::test_active_to_committed_valid` | ACTIVE → COMMIT | 进入 COMMITTED | P1 |
| `tx_state::test_modifying_to_aborted_valid` | MODIFYING → ROLLBACK | 进入 ABORTED | P1 |

---

## 2. WAL Contract Tests（WAL 契约测试）

### 2.1 WAL Order Tests（WAL 顺序测试）

| Test ID | Description | Expected | Priority |
|---------|-------------|----------|----------|
| `wal_contract::test_data_page_before_wal_panics` | 数据页写入早于 WAL 写入 | panic | P0 |
| `wal_contract::test_commit_without_wal_entry_panics` | COMMIT 前未写 WAL entry | panic | P0 |
| `wal_contract::test_wal_entry_out_of_order_panics` | WAL entry 顺序违反 | panic | P0 |
| `wal_contract::test_insert_without_wal_panics` | INSERT 前未写 WAL | panic | P0 |
| `wal_contract::test_update_without_wal_panics` | UPDATE 前未写 WAL | panic | P0 |
| `wal_contract::test_delete_without_wal_panics` | DELETE 前未写 WAL | panic | P0 |

### 2.2 WAL LSN Ordering Tests

| Test ID | Description | Expected | Priority |
|---------|-------------|----------|----------|
| `wal_lsn::test_page_lsn_must_be_ge_wal_lsn` | 数据页 LSN >= WAL LSN | true | P0 |
| `wal_lsn::test_lsn_monotonic_increasing` | LSN 单调递增 | true | P1 |
| `wal_lsn::test_tx_id_uses_correct_lsn` | 每个 TX 第一个 LSN < 最后 LSN | true | P1 |

---

## 3. WAL Replay Idempotency Tests（WAL 重放幂等性测试）

### 3.1 Entry Type Replay Tests

| Test ID | Description | Expected | Priority |
|---------|-------------|----------|----------|
| `wal_replay::test_commit_twice_second_ignored` | Commit replay 两次 | 第二次忽略 | P0 |
| `wal_replay::test_insert_twice_duplicate_ignored` | Insert replay 两次 | duplicate 忽略 | P0 |
| `wal_replay::test_update_twice_idempotent` | Update replay 两次 | 结果一致 | P0 |
| `wal_replay::test_delete_twice_second_ignored` | Delete replay 两次 | 第二次忽略 | P0 |
| `wal_replay::test_rollback_twice_second_ignored` | Rollback replay 两次 | 第二次忽略 | P0 |
| `wal_replay::test_commit_without_begin_panics` | 无 Begin 的 Commit | panic | P0 |

### 3.2 Replay Order Tests

| Test ID | Description | Expected | Priority |
|---------|-------------|----------|----------|
| `wal_replay::test_entries_replayed_in_lsn_order` | 按 LSN 顺序 replay | 顺序正确 | P1 |
| `wal_replay::test_interleaved_tx_replay_correct` | 交错事务 replay | 每事务独立 | P1 |
| `wal_replay::test_replay_reconstruction_idempotent` | 多次重建结果一致 | 结果相同 | P1 |

---

## 4. Recovery Tests（恢复测试）

### 4.1 Crash Before Commit Tests

| Test ID | Description | Expected | Priority |
|---------|-------------|----------|----------|
| `recovery::test_begin_then_crash_rolls_back` | BEGIN 后 crash | 回滚 | P0 |
| `recovery::test_insert_then_crash_rolls_back` | INSERT 后 crash（未 commit）| 回滚 | P0 |
| `recovery::test_prepare_then_crash_rolls_back` | Prepare 后 crash | 回滚 | P0 |
| `recovery::test_commit_flush_crash_replays` | COMMIT 刷盘时 crash | WAL replay 完成 | P0 |

### 4.2 Partial Write Tests

| Test ID | Description | Expected | Priority |
|---------|-------------|----------|----------|
| `recovery::test_partial_insert_write_recovery` | INSERT 部分写入后 crash | 回滚 | P0 |
| `recovery::test_partial_update_write_recovery` | UPDATE 部分写入后 crash | 回滚 | P0 |
| `recovery::test_partial_delete_write_recovery` | DELETE 部分写入后 crash | 回滚 | P0 |
| `recovery::test_partial_commit_flush_recovery` | COMMIT 部分刷盘 | WAL 重放完成提交 | P0 |

### 4.3 Recovery Scan Tests

| Test ID | Description | Expected | Priority |
|---------|-------------|----------|----------|
| `recovery::test_scan_incomplete_prepared_tx` | 扫描 Prepare 无 Commit 的 tx | 加入 incomplete | P0 |
| `recovery::test_scan_incomplete_begun_tx` | 扫描 Begin 无任何操作的 tx | 加入 incomplete | P1 |
| `recovery::test_recovery_report_accuracy` | RecoveryReport 计数准确 | 数字正确 | P1 |

---

## 5. Integration Tests（集成测试）

### 5.1 End-to-End DML Tests

| Test ID | Description | Expected | Priority |
|---------|-------------|----------|----------|
| `e2e::test_insert_commit_recovery_cycle` | INSERT → COMMIT → crash → recovery | 数据存在 | P0 |
| `e2e::test_update_commit_recovery_cycle` | UPDATE → COMMIT → crash → recovery | 数据更新 | P0 |
| `e2e::test_delete_commit_recovery_cycle` | DELETE → COMMIT → crash → recovery | 数据删除 | P0 |
| `e2e::test_transaction_rollback_isolation` | ROLLBACK 后其他 tx 可见旧数据 | 隔离正确 | P0 |
| `e2e::test_concurrent_tx_conflict_detection` | 并发写冲突检测 | SSI 触发 | P1 |

---

## 6. Summary — Missing Tests Count

| Category | Count | P0 | P1 |
|----------|-------|----|----|
| TX Lifecycle | 7 | 6 | 1 |
| WAL Contract | 9 | 6 | 3 |
| WAL Replay Idempotency | 8 | 6 | 2 |
| Recovery | 10 | 6 | 4 |
| Integration | 5 | 3 | 2 |
| **TOTAL** | **39** | **27** | **12** |

---

## 7. Test Implementation Priority

### Phase 1 — P0（必须实现，防致命错误）

```
tx_lifecycle::test_insert_without_tx_panics
tx_lifecycle::test_update_without_tx_panics  
tx_lifecycle::test_delete_without_tx_panics
tx_lifecycle::test_insert_after_commit_panics
tx_lifecycle::test_insert_after_rollback_panics
tx_lifecycle::test_double_commit_panics
wal_contract::test_data_page_before_wal_panics
wal_contract::test_commit_without_wal_entry_panics
wal_contract::test_insert_without_wal_panics
wal_contract::test_update_without_wal_panics
wal_contract::test_delete_without_wal_panics
wal_replay::test_commit_twice_second_ignored
wal_replay::test_insert_twice_duplicate_ignored
wal_replay::test_commit_without_begin_panics
recovery::test_begin_then_crash_rolls_back
recovery::test_insert_then_crash_rolls_back
recovery::test_prepare_then_crash_rolls_back
recovery::test_commit_flush_crash_replays
recovery::test_partial_insert_write_recovery
recovery::test_partial_update_write_recovery
recovery::test_partial_delete_write_recovery
recovery::test_partial_commit_flush_recovery
```

### Phase 2 — P1（完整性）

剩余 16 个测试。

---

**文档状态**: ACTIVE
**作者**: Hermes A (Contract Guardian)
**日期**: 2026-05-31
**版本**: v0.1
**下一步**: 交给 opencode 实现缺失测试，Hermes B 验证