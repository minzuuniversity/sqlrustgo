# openspec/3174 - P1-2 Crash Test Framework (100+ scenarios)

> **Issue**: #3174
> **作者**: Hermes Agent
> **日期**: 2026-06-05
> **Phase**: 3 (W5-6)
> **工作量**: 40h (按 V390_DEVELOPMENT_PLAN)
> **状态**: 部分已存在,本次做统一 + 补缺 + G8 gate

## 一、问题分析

### 1.1 现状审计 (2026-06-05)

仓库已有 **113 个 crash/fault/wal 相关测试** 分布在 10 个文件中:

| 文件 | 测试数 | 覆盖类别 |
|------|--------|----------|
| tests/memory_fault_injection_test.rs | 7 | OOM (memory) |
| tests/network_fault_injection_test.rs | 7 | Network fault |
| tests/e2e_trigger_wal_recovery.rs | 3 | WAL recovery (trigger path) |
| tests/wal_integration_test.rs | 16 | WAL 集成 |
| tests/double_write_buffer_test.rs | 6 | DWB |
| tests/wal_tx_contract_test.rs | 26 | WAL/TX 契约 |
| tests/exp_g_wal_contracts_verified.rs | 5 | exp-g 验证 |
| tests/tx_wal_contract_tests.rs | 31 | TX/WAL 契约 |
| crates/storage/tests/e2e_crash_recovery_proof.rs | 4 | E2E file storage crash |
| crates/transaction/tests/deadlock_injection_test.rs | 8 | Deadlock injection |
| **TOTAL** | **113** | 已超过 100+ 目标 |

### 1.2 #3174 8 类场景的覆盖映射

| #3174 类别 | 已有覆盖 | 缺口 |
|------------|----------|------|
| INSERT 时崩溃 (page write) | memory_fault (OOM during insert) | **缺真实进程崩溃测试** |
| UPDATE 时崩溃 (WAL flush) | memory_fault (OOM commit) | **缺 UPDATE-specific 崩溃** |
| DELETE 时崩溃 (B+tree rebalance) | ❌ | **完全缺** |
| COMMIT 前崩溃 (TX state) | e2e_crash_recovery (filestorage persistence) | **缺 TX-level crash (WAL replay)** |
| COMMIT 后崩溃 (WAL fsync) | wal_integration (16 tests) | **部分** |
| WAL 写一半崩溃 (partial WAL) | ❌ | **缺 (binary corruption)** |
| CHECKPOINT 时崩溃 (page flush) | ❌ | **完全缺** |
| Recovery 时崩溃 (WAL replay) | e2e_trigger_wal_recovery (3) | **部分** |

### 1.3 P1-2 任务真正需要补的 (按治理最小修改)

**A. 缺失类别补全** (新增 ~15-25 tests):
- DELETE 时崩溃 (B+tree rebalance 中)
- WAL 写一半崩溃 (binary corruption)
- CHECKPOINT 时崩溃 (page flush 中)
- 真实进程崩溃模拟 (std::process::exit 在关键点)

**B. G8 Gate 统一** (新):
- 验证 8 类场景都有 ≥1 test
- 验证总测试数 ≥100
- 验证 fault-injection feature flag 体系

**C. CrashTest Harness** (新):
- 测试框架包装 spawn + signal + timeout
- 跨进程验证 recovery

## 二、实施方案

### 2.1 范围限定

按治理 §2.1 最小修改 + 复用现有基础:

**本次 PR 范围 (3 大块)**:

1. **新文件**: `tests/crash_test_framework.rs` (新增 ~15 tests,补缺口)
2. **新文件**: `scripts/gate/check_p12_crash_test.sh` (G8 gate)
3. **新文件**: `tests/crash_test_harness.rs` (跨进程 harness, 共享 helper)

**延后 (推 v3.10+ 或后续 Phase 4)**:
- 完整 Chaos Monkey 框架 (随机故障注入)
- 跨节点 crash 协调 (distributed cluster crash)
- 持续 chaos testing (CI scheduled)

### 2.2 Crash Test Harness 设计

```rust
// tests/crash_test_harness.rs (共享, non-test helper)
// - spawn_child_db(): 启动 db 子进程, 返回 (Child, port)
// - kill_gracefully(child, signal): 优雅关闭 (SIGTERM)
// - kill_hard(child): 强制杀死 (SIGKILL)
// - recover_and_verify(): 重新连接 + 验证 state
// - crash_point_inject(env_var): 通过环境变量触发 crash point
```

### 2.3 新增测试 (15 个, 补缺口)

按 #3174 8 类缺口顺序:

| # | Test | 类别 |
|---|------|------|
| 1 | test_crash_during_delete_bretree_rebalance | DELETE / B+tree |
| 2 | test_crash_during_delete_persist_wal | DELETE / WAL |
| 3 | test_wal_partial_write_corruption_recovery | WAL 写一半 |
| 4 | test_wal_truncated_mid_record_recovery | WAL 写一半 |
| 5 | test_wal_zero_length_record_recovery | WAL 写一半 |
| 6 | test_checkpoint_crash_before_complete | CHECKPOINT |
| 7 | test_checkpoint_crash_during_page_flush | CHECKPOINT |
| 8 | test_checkpoint_crash_after_partial_commit | CHECKPOINT |
| 9 | test_recovery_crash_during_wal_replay | Recovery / replay |
| 10 | test_recovery_crash_during_state_restore | Recovery / state |
| 11 | test_update_crash_before_wal_append | UPDATE / WAL |
| 12 | test_update_crash_during_index_update | UPDATE / index |
| 13 | test_insert_crash_during_page_write_slow_io | INSERT / IO |
| 14 | test_commit_crash_during_wal_fsync_sync | COMMIT / fsync |
| 15 | test_recovery_idempotent_after_crash_recovery | Recovery / idempotency |

### 2.4 G8 Gate (scripts/gate/check_p12_crash_test.sh)

7 项检查:
1. `tests/crash_test_framework.rs` 存在
2. `tests/crash_test_harness.rs` 存在
3. 8 类 crash 场景都有 ≥1 test (grep #[test] 名字)
4. 总测试数 ≥100
5. `cargo test --test crash_test_framework` 编译通过
6. `cargo test crash_test --features=fault-injection` 编译通过
7. 已有 test 文件未被破坏 (mem/network/wal)

## 三、风险评估

| 风险 | 影响 | 缓解 |
|------|------|------|
| 真实进程崩溃污染测试环境 | 其他测试失败 | 每个 crash test 用独立 tempfile + cleanup |
| 真实 SIGKILL 不安全 | 平台兼容性 | macOS/Linux 双平台, Windows skip |
| IO 错误注入慢 | CI 超时 | 8 类 × 1-2 test = ~15-20s 目标 |
| 现有 113 测试被破坏 | 回归 | 严格不改现有文件, 只新建 |

## 四、验收标准 (G8 门禁)

```
✅ crash_test_framework.rs: ≥15 tests PASS
✅ 8 类 crash 场景全覆盖 (grep 验证)
✅ Total crash tests: ≥113 (已有) + 15 (新增) = ≥128
✅ G8 gate: 7/7 PASS
✅ 871 L1 tests 不回归
✅ TPC-H 22/22 (G1 维持)
```

## 五、Subsumed Issues

- **#2944 (部分)**: Crash recovery TLA+ 验证 - 已通过 TLA+ PROOF 覆盖,运行时补全在本次
- **#3106 (CLOSED)**: SEM-1 savepoint - 已 closed via #3172

## 六、回滚计划

如 crash_test_framework 编译失败:
1. 删除 `tests/crash_test_framework.rs` + `tests/crash_test_harness.rs`
2. G8 gate 标记 DEFER (因 #3174 暂未 critical)
3. 现有 113 tests 仍然保留

## 七、依赖

**上游**: 无 (TLA+ proofs 已有,运行时补全独立)
**下游**: P1-1 Backup/Restore (#3173) — 需要 recovery 验证
**Subsumed by**: P1-3 Soak Test (#3175) — 长时间运行

## 八、参考资料

- Issue #3174
- V390_DEVELOPMENT_PLAN.md §P1-2
- V390_TEST_PLAN.md §G8
- crates/transaction/tests/deadlock_injection_test.rs (T-15 模型, 8 tests)
- tests/memory_fault_injection_test.rs (OOM 模型, 7 tests)
- crates/storage/tests/e2e_crash_recovery_proof.rs (E2E 模型, 4 tests)
- TLA+ PROOF-026 (Write Skew / SSI formal proof)
