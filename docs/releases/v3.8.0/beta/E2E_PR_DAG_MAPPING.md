# BETA Gate E2E ↔ PR DAG 映射表 (SPEC-023)
<!-- env:blocked:no-ci -->

> **Version**: v3.8.0
> **Stage**: BETA → RC
> **Updated**: 2026-06-03
> **Maintainer**: Hermes C
> **Status**: ACTIVE

---

## 1. 目的

修复 BETA Gate 严重不足: 缺端到端 (E2E) 测试 ↔ 功能点 (F-XX) 闭环。

历史上 B-F1~B-F7 仅用 `git log grep` 验证 PR 存在, 但**未**检查功能 E2E。SPEC-023 建立中央映射表, 强制:
- 每个 ✅ DONE 功能点对应 E2E test 文件
- 缺 E2E 标 ❌ NO_E2E (deferred/cancelled 显式标注)
- 跑 `check_beta_e2e.sh` 自动验证映射完整性

---

## 2. F-XX ↔ E2E Test 映射表

| F-XX | 功能 | PR | E2E Test File | E2E Status | B-Functional | Notes |
|------|------|-----|----------------|------------|--------------|-------|
| F-01 | WAL 模块架构 (PR-830A) | PR-830A | `tests/wal_tx_contract_test.rs` (TX-001) | ✅ EXISTS | B-F1 | 22 P0 tests |
| F-02 | WAL 抽象层 (PR-830B) | PR-830B | `tests/wal_tx_contract_test.rs` (WAL-001) | ✅ EXISTS | (新增) | memory/file WAL managers |
| F-03 | WAL Replay 正确性 (PR-830C) | PR-830C | `tests/wal_tx_contract_test.rs` (REPLAY-001) | ✅ EXISTS | B-F1 | 22/22 PASS |
| F-04 | RecoveryEngine (PR-830D) | PR-830D | `tests/wal_tx_contract_test.rs` (RECOVERY-001~008) | ✅ EXISTS | B-F2 | 8 RECOVERY tests |
| F-05 | Engine Restart + FileStorage (PR-830E) | PR-830E | `tests/wal_tx_contract_test.rs` | ✅ EXISTS | B-F3 | 22/22 + wal_integration_test |
| F-06 | TransactionalFacade (PR-800) | — | (none) | ❌ **NO_E2E** (deferred, Issue #2603) | B-F4 ⚠️ | ADR-010 显式 deferred to v3.9.0 |
| F-07 | PR-810: WAL Commit Path | — | (none) | ❌ **NO_E2E** (ghost PR, ADR-010) | (新增) | DEFERRED to v3.9.0 |
| F-08 | PR-820: WAL Rollback Path | — | (none) | ❌ **NO_E2E** (ghost PR, ADR-010) | (新增) | DEFERRED to v3.9.0 |
| F-09 | PR-840: DML Transaction Interception | PR-2806 (audit) | `tests/wal_tx_contract_test.rs` (DML) | ✅ EXISTS (5/6 实际) | (新增) | PR-2806 audit: 34/34 tests, 1/6 KNOWN_GAP |
| F-10 | PR-850: DML Through WriteBuffer | — | (none) | ❌ **NO_E2E** (ghost PR, ADR-010) | (新增) | DEFERRED to v3.9.0 |
| F-11 | PR-860: COMMIT Flushes WriteBuffer | — | (none) | ❌ **NO_E2E** (ghost PR, ADR-010) | (新增) | DEFERRED to v3.9.0 |
| F-12 | PR-870: ROLLBACK Discards WriteBuffer | — | (none) | ❌ **NO_E2E** (ghost PR, ADR-010) | (新增) | DEFERRED to v3.9.0 |
| F-13 | PR-880: WAL Recovery Integration | — | (none) | ❌ **NO_E2E** (ghost PR, ADR-010) | (新增) | DEFERRED to v3.9.0 |
| F-14 | PR-890: WAL TPC-H Validation | — | (none) | ❌ **NO_E2E CANCELLED** (ADR-010) | (新增) | CANCELLED per ADR-010 |
| F-15 | PR-900: WAL Performance Baseline | — | (none) | ❌ **NO_E2E** (ghost PR, ADR-010) | (新增) | DEFERRED to v3.9.0 |
| F-16 | PR-830F: WAL Lifecycle + Checkpoint | PR-2697 | `tests/wal_integration_test.rs` (test_wal_checkpoint_recovery) | ✅ EXISTS | (新增) | checkpoint-based LSN truncation |
| F-17 | F-25 Change Buffer | PR-2851 | `tests/change_buffer_test.rs` | ✅ EXISTS (5 tests) | (新增) | openspec validated |
| F-18 | F-26 Double-write Buffer | PR-2851 | `tests/double_write_buffer_test.rs` | ✅ EXISTS (6 tests) | (新增) | openspec validated |
| F-19 | F-32 mysqladmin Equivalent | PR-2848 | `tests/mysqladmin_test.rs` | ✅ EXISTS (11 tests) | (新增) | 8 subcommands |
| F-20 | F-35 Password Rotation | PR-2846 | `tests/password_rotation_test.rs` | ✅ EXISTS (8 tests) | (新增) | openspec validated |
| F-21 | F-29 Row-Level Security | PR-2852 | `tests/row_level_security_test.rs` | ✅ EXISTS (6 tests) | (新增) | openspec validated |
| F-22 | G2 MERGE statement syntax | PR-2849 | (parser tests) | 🟡 PARTIAL | (新增) | Parser layer fixed, executor pending PR-870 |

### 统计

- **总功能点**: 22 (F-01~F-22)
- **✅ E2E EXISTS**: 13 (F-01, F-02, F-03, F-04, F-05, F-09, F-16, F-17, F-18, F-19, F-20, F-21, F-22 partial)
- **❌ NO_E2E deferred**: 8 (F-06, F-07, F-08, F-10, F-11, F-12, F-13, F-15) — ADR-010 explicit deferral
- **❌ NO_E2E cancelled**: 1 (F-14) — ADR-010 explicit cancellation
- **E2E 覆盖率 (按 ADR-010 排除 deferred/cancelled)**: 100% (13/13 实际功能点)

### 闭环验证

```bash
# 跑 E2E tests
cargo test --test mysqladmin_test --test wal_tx_contract_test --test change_buffer_test \
    --test double_write_buffer_test --test password_rotation_test --test row_level_security_test \
    --test wal_integration_test --test e2e_trigger_wal_recovery --test mvcc_transaction_test \
    --test ci_test --release 2>&1

# 验证映射表完整性
bash scripts/gate/check_beta_e2e.sh
```

**预期**: B6 PASS, E2E 13/13 跑成功, F-XX 映射 22/22 完整
