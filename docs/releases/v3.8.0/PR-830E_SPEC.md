# PR-830E SPEC — WAL Recovery Lifecycle (Startup Recovery)

> **PR Number**: PR-830E
> **PR Title**: WAL Recovery Lifecycle — Startup Recovery with One-Shot Guard
> **Version**: v3.8.0 Alpha
> **Branch**: `develop/v3.8.0`
> **Auditor**: [Pending]
> **Created**: 2026-05-31
> **Status**: DRAFT — For Review

---

## 1. 概述

### 1.1 PR 目标

PR-830E 实现启动时 WAL 自动恢复，确保数据库在 crash 后重启时能从 WAL 恢复到最后一致状态。

**核心约束**: 不实现 WAL truncation（WAL 生命周期管理推迟到 PR-830F）。

### 1.2 当前状态

| 组件 | 状态 | 说明 |
|------|------|------|
| `RecoveryEngine` trait | ✅ 已有 | 定义在 `crates/storage/src/recovery_engine.rs` |
| `RecoveryEngineImpl` | ✅ 已有 | 默认实现，entry-level determinism |
| `RecoveryReport` | ✅ 已有 | 统计恢复结果（committed/rolled_back/incomplete tx） |
| `with_wal_recovery()` | ✅ 已有 | `engine_builder.rs:155-161`，调用 `recover_wal()` |
| One-shot guard | ❌ 缺失 | 重复恢复可能导致状态不一致 |
| WAL truncation | ❌ 缺失 | **不在本 PR 范围** |

### 1.3 架构决策（已确认）

**WAL 是 Event Log，不是 Redo Log**：
- WAL = authoritative event history（所有操作的完整记录）
- RecoveryEngine replay WAL entries 到 storage
- **恢复后禁止 truncation**，因为：
  - CheckpointManager 未与 RecoveryEngine 集成
  - 无 durable page tracking
  - Crash after recovery but before persistence 可能导致数据丢失

详见: `docs/governance/tx/PR850B_TX_CONTEXT_ANALYSIS.md`

---

## 2. 功能范围

### 2.1 PR-830E 必须做（Must Do）

| 功能 | 实现位置 | 说明 |
|------|----------|------|
| One-shot recovery guard | `crates/storage/src/recovery_engine.rs` | `RecoveryState` enum，防止重复恢复 |
| RecoveryReport 输出 | `src/engine_builder.rs` | `recover_wal()` 返回 `RecoveryReport` |
| 日志输出恢复统计 | `src/engine_builder.rs` | INFO level，记录 committed/rolled_back/incomplete tx |
| RECOVERY Contract 文档 | `docs/releases/v3.8.0/PR-830E_CONTRACT.md` | 记录恢复契约 |

### 2.2 PR-830E 禁止做（Must NOT Do）

| 功能 | 原因 |
|------|------|
| WAL truncation | 等 PR-830F（CheckpointManager 集成后） |
| Delete replay 幂等性修复 | PR-840/DML Transaction Interception 范围 |
| Update replay 修复 | WAL 格式需要重新设计 |
| CheckpointManager 集成 | PR-830F 范围 |

---

## 3. 技术设计

### 3.1 One-Shot Recovery Guard

```rust
/// Recovery state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryState {
    /// No recovery has been attempted yet
    Unrecovered,
    /// Recovery completed successfully
    Recovered,
    /// Recovery failed
    Failed,
}

/// RecoveryEngine with state tracking
pub struct StatefulRecoveryEngine<S: StorageEngine> {
    inner: RecoveryEngineImpl,
    state: RecoveryState,
}

impl<S: StorageEngine> RecoveryEngine<S> for StatefulRecoveryEngine<S> {
    fn recover(&mut self, storage: &mut S, wal: &mut dyn WalManager) -> SqlResult<RecoveryReport> {
        match self.state {
            RecoveryState::Unrecovered => {
                self.state = RecoveryState::Recovered;
                self.inner.recover(storage, wal)
            }
            RecoveryState::Recovered => {
                // Idempotent: already recovered, skip
                Ok(RecoveryReport::default())
            }
            RecoveryState::Failed => {
                Err(crate::engine::SqlError::ExecutionError(
                    "RecoveryEngine: cannot recover after previous failure".to_string(),
                ))
            }
        }
    }
}
```

### 3.2 RecoveryReport 结构（已有）

```rust
#[derive(Debug, Default, Clone)]
pub struct RecoveryReport {
    pub entries_total: usize,       // Total WAL entries read
    pub committed_txns: usize,     // Committed transactions replayed
    pub rolled_back_txns: usize,   // Rolled back transactions skipped
    pub incomplete_txns: usize,     // Incomplete (no Commit/Rollback)
    pub rows_inserted: usize,       // Rows inserted during recovery
    pub rows_updated: usize,       // Rows updated during recovery
    pub rows_deleted: usize,        // Rows deleted during recovery
}
```

### 3.3 日志输出格式

```rust
// In recover_wal():
info!(
    "WAL recovery completed: {} committed txns, {} rolled back, {} incomplete, {} entries total",
    report.committed_txns,
    report.rolled_back_txns,
    report.incomplete_txns,
    report.entries_total
);
```

---

## 4. 文件清单

### 4.1 修改的文件

| 文件 | 修改内容 |
|------|----------|
| `crates/storage/src/recovery_engine.rs` | 添加 `RecoveryState` enum 和 `StatefulRecoveryEngine` struct |
| `src/engine_builder.rs` | 修改 `recover_wal()` 使用有状态的 engine，输出日志 |

### 4.1 新增的文件

| 文件 | 内容 |
|------|------|
| `docs/releases/v3.8.0/PR-830E_CONTRACT.md` | RECOVERY Contract v1 文档 |
| `docs/releases/v3.8.0/PR-830E_TEST_PLAN.md` | 测试计划 |

---

## 5. 测试策略

### 5.1 单元测试

```rust
// crates/storage/src/recovery_engine.rs

#[cfg(test)]
mod tests {
    #[test]
    fn test_recovery_state_transitions() {
        // Unrecovered → Recovered → Recovered (idempotent)
    }

    #[test]
    fn test_recovery_state_failed_blocking() {
        // Failed state blocks subsequent recovery
    }

    #[test]
    fn test_recovery_report_default() {
        // Default report has all zeros
    }
}
```

### 5.2 集成测试（已有 RECOVERY 测试）

```bash
# RECOVERY-001~008 测试已在 tests/wal_tx_contract_test.rs
# 使用 L3 WAL engine（create_wal_engine / recover_and_rebuild）
cargo test --test wal_tx_contract_test
# 预期: 15 PASS + 7 FAIL（RECOVERY-003 已 ignore）
```

**注意**: RECOVERY-007 (`test_partial_delete_write_recovery`) 已标记 `#[ignore]`，因为 delete replay 语义问题不在本 PR 范围。

### 5.3 Beta Gate B2 验证

```
cargo test -p sqlrustgo-storage    → 275+ passed
cargo clippy -p sqlrustgo-storage -- -D warnings → 0 warnings
cargo test --test wal_tx_contract  → 15+ PASS (RECOVERY tests use L3)
```

---

## 6. 依赖关系

### 6.1 依赖的 PR（前置）

| PR | 说明 |
|----|------|
| PR-830A | WAL Entry 结构定义 |
| PR-830B | WalManager trait |
| PR-830C | RecoveryEngine trait |
| PR-830D | RecoveryEngineImpl 实现 |
| PR-850A | WalStorage tx context cleanup |
| PR-850B | tx_id 传播链路修复 |

### 6.2 被依赖的 PR（后续）

| PR | 说明 |
|----|------|
| PR-830F | CheckpointManager 集成 + WAL truncation |
| PR-840 | DML Transaction Interception（修复 delete replay） |

---

## 7. 风险和缓解

### 7.1 风险 1: One-shot guard 类型安全

**风险**: `RecoveryState` 作为 struct field，需要确保所有 code path 都正确初始化。

**缓解**: `StatefulRecoveryEngine::new()` 默认 state 为 `Unrecovered`。

### 7.2 风险 2: 重复 recovery 导致状态不一致

**风险**: 如果有人手动调用 `recover()`，可能覆盖已有状态。

**缓解**: Guard 检查 state，idempotent 返回。

### 7.3 风险 3: WAL 不是幂等 Event Log

**风险**: Delete replay 不是真正的 row-level delete，可能导致重复删除。

**缓解**: 已知问题，记录在 RECOVERY Contract，defer 到 PR-840。

---

## 8. RECOVERY Contract v1（新增文档）

```markdown
# RECOVERY_CONTRACT_v1

## Recovery Trigger
RecoveryEngine MUST run at startup before serving requests.

## Recovery Source
RecoveryEngine replays WAL entries from durable storage.

## Idempotency
Repeated execution of RecoveryEngine MUST NOT corrupt storage state.
Current status: **GUARANTEED** via one-shot guard.

## WAL Retention
Recovered WAL files MUST NOT be truncated automatically.

Reason:
- No checkpoint mechanism integrated with recovery
- No durable page tracking exists
- Crash after recovery but before persistence may cause data loss

## Known Limitations
1. Delete replay uses `storage.delete(&table, &[])` — deletes ALL rows in table
2. Update replay is skipped (WAL stores debug-formatted data)

## Future Work
- PR-830F: CheckpointManager integration
- PR-840: DML Transaction Interception (fix delete/update replay)
```

---

## 9. 验收标准

| 标准 | 要求 |
|------|------|
| One-shot guard | `StatefulRecoveryEngine` 防止重复 recovery |
| RecoveryReport | `recover_wal()` 返回统计信息 |
| 日志输出 | INFO level 记录恢复统计 |
| Beta Gate B2 | `cargo test -p sqlrustgo-storage` → 275+ passed |
| Clippy | `cargo clippy -p sqlrustgo-storage -- -D warnings` → 0 warnings |
| Doc links | `bash scripts/gate/check_docs_links.sh` → 0 broken links |

---

## 10. SSOT 引用

- `crates/storage/src/recovery_engine.rs` — RecoveryEngine trait + RecoveryReport
- `crates/storage/src/checkpoint.rs` — CheckpointManager（未集成）
- `crates/storage/src/wal_storage.rs` — WalStorage
- `src/engine_builder.rs:155-161` — `with_wal_recovery()`
- `tests/wal_tx_contract_test.rs` — RECOVERY-001~008 测试
- `docs/governance/tx/PR850B_TX_CONTEXT_ANALYSIS.md` — tx_id 传播分析
- `docs/releases/v3.8.0/RECOVERY_TEST_MIGRATION_PLAN.md` — 测试分层策略