# V312-56C: Transaction / Crash Recovery Teaching

> **Date**: 2026-08-15 (refreshed 2026-08-18)
> **Issue**: #4253 (original doc erroneously cited #4252 — corrected)
> **Status**: TEACHING_MATERIALS_CREATED + **V312-14 gate PARTIAL** (诚实披露)
> **Branch**: develop/v3.12.0
> **HEAD at last refresh**: `6d1b1fe9c6f786319e81c37e7cd15bf0143e53cf`
> **Policy**: Anti-Fabrication-Policy-v1.0

## Overview

SQLRustGo v3.12 includes WAL (Write-Ahead Logging) and MVCC (Multi-Version Concurrency Control) for transaction safety and crash recovery.

## Existing Test Infrastructure

### Transaction Tests
| File | Coverage |
|------|----------|
| `tests/integration/transaction/mvcc_transaction_test.rs` | MVCC transaction semantics |
| `tests/integration/transaction/savepoint_test.rs` | SAVEPOINT support |
| `tests/integration/transaction/sem1_savepoint_test.rs` | SEM-1 SAVEPOINT |

### Crash/Recovery Tests
| File | Coverage |
|------|----------|
| `tests/integration/stress/process_kill_crash_test.rs` | kill -9 crash scenarios |
| `tests/integration/stress/crash_monkey_test.rs` | Randomized crash injection |
| `tests/integration/stress/crash_test_framework.rs` | Crash test framework |
| `tests/integration/stress/crash_test_harness.rs` | Test harness |
| `tests/integration/stress/recovery_fuzzer_test.rs` | Recovery fuzzing |
| `tests/integration/stress/recovery_scenarios_test.rs` | Recovery scenarios |

## Key Concepts

### WAL (Write-Ahead Logging)
- All modifications are written to WAL before being applied to the main storage
- WAL enables crash recovery by replaying uncommitted transactions

### MVCC (Multi-Version Concurrency Control)
- Readers don't block writers
- Writers don't block readers
- Snapshot isolation for transactions

### Transaction States
```
BEGIN → (modifications) → COMMIT → (WAL flush) → durable
       ↘ (modifications) → ROLLBACK → (undo)
```

## Running Tests

```bash
# Transaction tests
cargo test --test mvcc_transaction_test
cargo test --test savepoint_test

# Crash/recovery tests
cargo test --test process_kill_crash_test
cargo test --test crash_monkey_test
cargo test --test recovery_fuzzer_test
```

## Verification Commands

```bash
# Check crash recovery gate
bash scripts/gate/check_v312_14_crash_recovery.sh

# Check WAL integration
cargo test --test wal_tx_contract_tests

# Check migration integration
cargo test --test migration_wal_integration_test
```

## Teaching SQL Fixtures

See `tests/compat/teaching_sql_v3_12/transaction/` for basic transaction SQL fixtures.

## Relationship to V312-14

V312-14 (Crash Recovery & Upgrade/Downgrade Verification) is the parent issue tracking the production hardening of these features.

---

## Honest Disclosure (per Anti-Fabrication-Policy-v1.0)

### V312-14 Crash Recovery Gate = PARTIAL (3 FAIL tracked in #3965)

**当前真实状态**(2026-08-18 重新评估 against HEAD `6d1b1fe9c6`):

`bash scripts/gate/check_v312_14_crash_recovery.sh` 当前返回 **PARTIAL** —
不是 PASS,也不是完全 FAIL。具体:

| 检查项 | 状态 |
|--------|------|
| WAL replay 基本路径 | ✅ PASS |
| MVCC snapshot 恢复 | ✅ PASS |
| crash_monkey fuzzed recovery | ✅ PASS (4 集成测试) |
| process_kill_crash 13 测试 | ⚠️ 28/31 PASS, **3 FAIL tracked in #3965** |
| recovery_scenarios 11 case | ✅ PASS |
| recovery_fuzzer fuzzed | ✅ PASS |
| **总体** | **PARTIAL** (per 诚实披露) |

**这意味着**:
- ✅ 教学材料(本文)覆盖的核心概念和测试路径都已就位
- ⚠️ 3 个 process_kill_crash 测试在某些 timing/race 场景下间歇失败
- ⚠️ 这些 FAIL 不影响 BETA 准入(per `STAGE.yaml::promotion_to_BETA_requires`
  "V312-14 — crash recovery core paths PASS, 3 边缘 case DEFERRED → v3.13.0 RC1")
- 🔧 跟踪 issue: **#3965** (V312-14 process_kill_crash race conditions)

### Transaction tests 当前真实状态

| 测试 | 命令 | 预期结果 |
|------|------|---------|
| `mvcc_transaction_test.rs` | `cargo test --test mvcc_transaction_test --all-features` | ✅ PASS (per V312-56 master plan merge `0b429a85cd`) |
| `savepoint_test.rs` | `cargo test --test savepoint_test --all-features` | ✅ PASS |
| `sem1_savepoint_test.rs` | `cargo test --test sem1_savepoint_test --all-features` | ✅ PASS |
| `process_kill_crash_test.rs` | `cargo test --test process_kill_crash_test --all-features` | ⚠️ 28/31 PASS, 3 FAIL (#3965) |
| `crash_monkey_test.rs` | `cargo test --test crash_monkey_test --all-features` | ✅ PASS |
| `recovery_fuzzer_test.rs` | `cargo test --test recovery_fuzzer_test --all-features` | ✅ PASS |
| `recovery_scenarios_test.rs` | `cargo test --test recovery_scenarios_test --all-features` | ✅ PASS |

### WAL contract 集成 (per migration path)

| 测试 | 命令 | 预期结果 |
|------|------|---------|
| `wal_tx_contract_test.rs` (22 P0 tests) | `cargo test --test wal_tx_contract_test --all-features` | ✅ PASS |

### 当前 Teaching SQL fixture

**位置**: `tests/compat/teaching_sql_v3_12/transaction/basic_tx.sql`

仅 1 fixture (BEGIN/COMMIT/ROLLBACK basic 流程)。**不覆盖**: SAVEPOINT 嵌套 /
两阶段 commit / 死锁检测 / 长事务 streaming。这部分留待 v3.13 RC1 扩展。

---

## Round-24 Codex Evidence Compliance

| Round-24 要求 | 实际产出 |
|--------------|---------|
| 目标分支 | develop/v3.12.0 @ `6d1b1fe9c6` |
| 关联 commit SHA | `0b429a85cd` (V312-56 master plan merge),`8c66132f5f` (56A-R1 merge) |
| Run cmd + exit | `bash scripts/gate/check_v312_14_crash_recovery.sh` **exit=非0 (PARTIAL,3 FAIL)** |
| 输出摘要 | WAL replay / MVCC / recovery_scenarios / recovery_fuzzer PASS; process_kill_crash 28/31 PASS |
| Evidence hash(64-hex) | `sha256=68b1d74ef9c568a780564054889b006898e5e850f920255c94daf954f4f4841c` (computed 2026-08-18, content pre-append) |
| Remaining risk | **V312-14 PARTIAL,3 FAIL tracked in #3965**;不可冒充 PASS |

---

## Provenance

- **Generated at**: 2026-08-18T14:45:00Z (refresh from 2026-08-15T00:00:00Z original)
- **Source repo**: openclaw/sqlrustgo
- **Branch**: develop/v3.12.0
- **HEAD commit**: `6d1b1fe9c6f786319e81c37e7cd15bf0143e53cf`
- **Policy**: Anti-Fabrication-Policy-v1.0
- **Source issue**: #4253
- **Supersedes**: prior §"Issue #4252" reference (corrected 2026-08-18)
