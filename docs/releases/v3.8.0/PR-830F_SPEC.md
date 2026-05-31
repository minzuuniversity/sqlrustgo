# PR-830F SPEC — WAL Lifecycle Controller (Checkpoint + Truncation)

> **PR Number**: PR-830F
> **PR Title**: WAL Lifecycle Controller — Checkpoint-Based WAL Truncation
> **Version**: v3.8.0 Alpha
> **Branch**: `develop/v3.8.0`
> **Auditor**: [Pending]
> **Created**: 2026-05-31
> **Status**: DRAFT — For Review

---

## 1. 概述

### 1.1 PR 目标

PR-830F 实现 WAL 生命周期闭环控制，通过 CheckpointManager 提供的 `safe_truncate_lsn` 实现 WAL 安全截断。

**核心约束**: WAL 是 Event Log，不是 Redo Log — recovery 后不自动 truncate，必须通过 CheckpointManager 显式控制。

### 1.2 当前状态

| 组件 | 状态 | 说明 |
|------|------|------|
| `RecoveryEngine` trait | ✅ 已有 (PR-830C) | 定义在 `crates/storage/src/recovery_engine.rs` |
| `RecoveryEngineImpl` | ✅ 已有 (PR-830D) | 默认实现 |
| `RecoveryState` | ✅ 已有 (PR-830E) | One-shot guard |
| `StatefulRecoveryEngine` | ✅ 已有 (PR-830E) | 状态封装 |
| `CheckpointManager` | ✅ 已有 | 定义在 `crates/storage/src/checkpoint.rs` |
| CheckpointManager → Recovery 集成 | ❌缺失 | 无 WAL truncation gate |
| WAL truncation API | ❌ 缺失 | WAL 永远增长 |

### 1.3 架构决策（已确认）

**WAL 是 Event Log，不是 Redo Log**：
- WAL = authoritative event history（所有操作的完整记录）
- RecoveryEngine replay WAL entries 到 storage
- **恢复后禁止自动 truncation**，因为：
  - CheckpointManager 未与 RecoveryEngine 集成
  - 无 durable page tracking
  - Crash after recovery but before persistence 可能导致数据丢失

**正确架构**：
```
WAL → RecoveryEngine (830E one-shot guard)
         ↓
CheckpointManager (tracks safe_truncate_lsn)
         ↓
WAL Truncation Gate (lsn <= safe_truncate_lsn → safe to delete)
```

---

## 2. 功能范围

### 2.1 PR-830F 必须做（Must Do）

| 功能 | 实现位置 | 说明 |
|------|----------|------|
| CheckpointManager → ExecutionEngine 集成 | `src/execution_engine.rs` | 添加 checkpoint field |
| WAL Truncation Gate trait | `crates/storage/src/wal.rs` | `WalTruncationGate` trait |
| `safe_truncate_lsn()` API | `crates/storage/src/checkpoint.rs` | 返回可安全截断的 LSN |
| WAL truncation trigger | `src/engine_builder.rs` | commit 后检查并 truncate |
| Checkpoint advance on commit | `src/execution_engine.rs` | commit hook 推进 checkpoint |

### 2.2 PR-830F 禁止做（Must NOT Do）

| 功能 | 原因 |
|------|------|
| Parallel replay | PR-830F-C 范围（可选） |
| Background checkpoint scheduler | PR-830F-C 范围（可选） |
| DML replay safety fix | PR-840 范围 |
| Checkpoint 文件持久化 | PR-830F 简化版不包含 |

---

## 3. 技术设计

### 3.1 WalTruncationGate Trait

```rust
/// WAL truncation safety gate
pub trait WalTruncationGate: Send + Sync {
    /// Returns the LSN below which WAL entries can be safely deleted.
    /// Returns None if no checkpoint has been established.
    fn safe_truncate_lsn(&self) -> Option<u64>;

    /// Check if a given LSN can be truncated
    fn can_truncate(&self, wal_lsn: u64) -> bool {
        self.safe_truncate_lsn().map_or(false, |cp_lsn| wal_lsn <= cp_lsn)
    }
}
```

### 3.2 CheckpointManager 已有实现

```rust
// crates/storage/src/checkpoint.rs

pub struct CheckpointManager {
    config: CheckpointConfig,
    last_checkpoint: Arc<RwLock<Option<CheckpointMetadata>>>,
    last_checkpoint_time: Arc<RwLock<Instant>>,
    checkpoint_dir: PathBuf,
}

impl CheckpointManager {
    /// Get the last checkpoint LSN
    pub fn last_checkpoint_lsn(&self) -> Option<u64> {
        self.last_checkpoint.read().unwrap().as_ref().map(|c| c.lsn)
    }

    /// Record a completed checkpoint
    pub fn record_checkpoint(&mut self, metadata: CheckpointMetadata) {
        *self.last_checkpoint.write().unwrap() = Some(metadata);
    }
}
```

### 3.3 ExecutionEngine 集成

```rust
// src/execution_engine.rs

pub struct ExecutionEngine<S: StorageEngine> {
    storage: Arc<RwLock<S>>,
    catalog: Option<Arc<RwLock<Catalog>>>,
    stats: Arc<RwLock<ExecutionStats>>,
    cbo_enabled: bool,
    transaction_manager: TransactionManager,
    current_tx_id: Option<u64>,
    tx_status: TxStatus,
    default_isolation: IsolationLevel,
    current_role: Option<String>,
    checkpoint_manager: Option<Arc<RwLock<CheckpointManager>>>, // NEW
}

impl<S: StorageEngine> ExecutionEngine<S> {
    /// Advance checkpoint after commit
    fn advance_checkpoint(&self, lsn: u64) {
        if let Some(cp) = &self.checkpoint_manager {
            if let Ok(mut guard) = cp.write() {
                guard.record_checkpoint(CheckpointMetadata {
                    lsn,
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64,
                    tx_count: 1,
                    dirty_pages: 0,
                    file_path: PathBuf::new(),
                });
            }
        }
    }

    /// Try to truncate WAL up to checkpoint
    fn try_truncate_wal(&self, wal: &mut dyn WalManager) {
        if let Some(cp) = &self.checkpoint_manager {
            if let Ok(guard) = cp.read() {
                if let Some(lsn) = guard.last_checkpoint_lsn() {
                    wal.truncate_before(lsn).ok(); // Ignore errors
                }
            }
        }
    }
}
```

### 3.4 WAL Truncation Flow

```rust
/// Commit hook in ExecutionEngine
fn commit_transaction(&mut self) -> SqlResult<TxId> {
    let tx_id = self.transaction_manager.commit()?;
    let lsn = self.wal().append_commit(tx_id)?;

    // Advance checkpoint
    self.advance_checkpoint(lsn);

    // Try to truncate WAL
    self.try_truncate_wal();

    Ok(tx_id)
}
```

### 3.5 FileBackedWalManager Truncation

```rust
// crates/storage/src/wal.rs

pub trait WalManager: Send + Sync {
    // ... existing methods ...

    /// Truncate WAL entries with LSN < `lsn`
    fn truncate_before(&mut self, lsn: u64) -> SqlResult<()>;
}
```

---

## 4. 文件清单

### 4.1 修改的文件

| 文件 | 修改内容 |
|------|----------|
| `crates/storage/src/wal.rs` | 添加 `WalTruncationGate` trait 和 `truncate_before` method |
| `crates/storage/src/checkpoint.rs` | 添加 `last_checkpoint_lsn()` method |
| `src/execution_engine.rs` | 添加 `checkpoint_manager` field 和 truncation hook |
| `src/engine_builder.rs` | 传递 CheckpointManager 到 ExecutionEngine |

### 4.2 新增的文件

| 文件 | 内容 |
|------|------|
| `docs/releases/v3.8.0/PR-830F_CONTRACT.md` | WAL Lifecycle Contract |

---

## 5. 测试策略

### 5.1 单元测试

```rust
#[test]
fn test_truncation_gate_blocks_before_checkpoint() {
    let gate = CheckpointManager::default();
    assert!(!gate.can_truncate(1000)); // No checkpoint yet
}

#[test]
fn test_truncation_gate_allows_after_checkpoint() {
    let mut gate = CheckpointManager::default();
    gate.record_checkpoint(CheckpointMetadata {
        lsn: 1000,
        timestamp: 0,
        tx_count: 1,
        dirty_pages: 0,
        file_path: PathBuf::new(),
    });
    assert!(gate.can_truncate(500));  // Below checkpoint
    assert!(gate.can_truncate(1000)); // At checkpoint (inclusive)
    assert!(!gate.can_truncate(1500)); // Above checkpoint
}
```

### 5.2 集成测试

```bash
# Test WAL lifecycle: commit → checkpoint → truncation
cargo test -p sqlrustgo-storage --test wal_lifecycle
```

---

## 6. 依赖关系

### 6.1 依赖的 PR（前置）

| PR | 说明 |
|----|------|
| PR-830E | RecoveryState + StatefulRecoveryEngine |

### 6.2 被依赖的 PR（后续）

| PR | 说明 |
|----|------|
| PR-840 | DML Transaction Interception（修复 delete/update replay） |
| PR-830F-C | Parallel replay + background checkpoint scheduler |

---

## 7. 验收标准

| 标准 | 要求 |
|------|------|
| WalTruncationGate trait | 存在且可调用 |
| can_truncate logic | LSN <= checkpoint_lsn 时返回 true |
| ExecutionEngine checkpoint_manager | 正确集成 |
| Commit hook advances checkpoint | transaction commit 后推进 checkpoint |
| WAL truncation trigger | commit 后尝试 truncate |
| Clippy | `cargo clippy --all-features -- -D warnings` → 0 warnings |

---

## 8. SSOT 引用

- `crates/storage/src/checkpoint.rs` — CheckpointManager 实现
- `crates/storage/src/wal.rs` — WalManager trait
- `crates/storage/src/recovery_engine.rs` — RecoveryEngine（PR-830E）
- `src/engine_builder.rs:155-161` — `with_wal_recovery()`
- `docs/releases/v3.8.0/PR-830E_SPEC.md` — PR-830E 规格说明
- `docs/releases/v3.8.0/PR-830E_CONTRACT.md` — RECOVERY Contract v1