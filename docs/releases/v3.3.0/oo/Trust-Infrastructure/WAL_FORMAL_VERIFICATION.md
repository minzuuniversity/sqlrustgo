# OO-TI3: WAL Formal Verification

> **版本**: v1.0
> **日期**: 2026-05-18
> **基于**: v3.3.0
> **维护人**: hermes-agent
> **Issue**: #1237
> **状态**: 新建设计

---

## 一、概述

### 1.1 目标

用 TLA+ 形式化验证 WAL (Write-Ahead Log) 状态机，确保 crash recovery 的数学正确性。

### 1.2 核心理念

```
Formal Verification = Mathematical Proof + Model Checking + Invariant Validation
```

### 1.3 问题背景

WAL 是数据库恢复的基石。任何逻辑错误都可能导致：
- 审计链断裂
- 数据丢失
- Phantom commits
- 合规违规

---

## 二、TLA+ 规范体系

### 2.1 规范文件结构

```
wal/tla/
├── WAL.tla              # WAL 状态机核心规范
├── WAL.cfg              # TLC 模型配置
├── Recovery.tla         # Crash recovery 正确性
├── Recovery.cfg         # Recovery 模型配置
├── Checkpoint.tla       # Checkpoint 完整性
├── Checkpoint.cfg       # Checkpoint 模型配置
└── CrashSimulation.tla  # 崩溃场景指导
```

### 2.2 WAL 状态机规范

```
┌─────────────────────────────────────────────────────────────────┐
│                    WAL State Machine                              │
└─────────────────────────────────────────────────────────────────┘

States:
═══════════════════════════════════════════════════════════════════

┌─────────────┐   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐
│   IDLE     │──▶│  APPENDING  │──▶│  COMMITTING │──▶│   FLUSHED  │
└─────────────┘   └─────────────┘   └─────────────┘   └─────────────┘
      ▲                │                 │                 │
      │                │                 │                 ▼
      │                │                 │           ┌─────────────┐
      │                │                 │           │ CHECKPOINTED│
      │                │                 │           └─────────────┘
      │                │                 │                 │
      │                ▼                 ▼                 ▼
      │          ┌─────────────┐   ┌─────────────┐   ┌─────────────┐
      └──────────│   ERROR    │   │   ABORTED   │   │   TRUNCATED │
                 └─────────────┘   └─────────────┘   └─────────────┘

Transitions:
═══════════════════════════════════════════════════════════════════

IDLE → APPENDING:    StartTransaction(txID)
APPENDING → COMMITTING: PrepareCommit(txID)
COMMITTING → FLUSHED:  FsyncDone(txID)
FLUSHED → CHECKPOINTED: CheckpointWritten
ERROR → IDLE:         RecoveryComplete
ABORTED → IDLE:       RecoveryComplete
```

---

## 三、WAL.tla 核心规范

### 3.1 模块声明

```tla
---------------------------- MODULE WAL --------------------------------
EXTENDS Integers, Sequences, FiniteSets, TLC

CONSTANT
  NULL_LSN = 0,
  MAX_LSN = 10000

VARIABLES
  lsnCounter,
  walBuffer,
  walFile,
  transactionLog,
  systemState
-----------------------------------------------------------------------
```

### 3.2 状态定义

```tla
States == {
  "idle",
  "appending",
  "committing",
  "flushed",
  "checkpointed",
  "aborted",
  "error"
}

TypeInvariant ==
  /\ lsnCounter \in 1..MAX_LSN
  /\ walBuffer \in [TransactionID -> Seq(LogEntry)]
  /\ walFile \in [TransactionID -> LogEntry]
  /\ transactionLog \in Seq(LogEntry)
  /\ systemState \in States
```

### 3.3 关键不变量

```tla
(*********************************************************************)
(* WAL Log Sequence Number Uniqueness                                 *)
(*********************************************************************)
UniqueLSNInvariant ==
  \A i, j \in DOMAIN transactionLog:
    i /= j => transactionLog[i].lsn /= transactionLog[j].lsn

(*********************************************************************)
(* Committed Transactions Always Persisted                            *)
(*********************************************************************)
CommitPersistenceInvariant ==
  \A tx \in DOMAIN walFile:
    walFile[tx].status = "committed"
      => \E entry \in DOMAIN transactionLog:
           transactionLog[entry].txID = tx
           /\ transactionLog[entry].lsn > 0

(*********************************************************************)
(* LSN Monotonicity                                                   *)
(*********************************************************************)
LSNMonotonicInvariant ==
  \A i \in 2..Len(transactionLog):
    transactionLog[i].lsn > transactionLog[i-1].lsn

(*********************************************************************)
(* No Phantom Commits After Recovery                                  *)
(*********************************************************************)
NoPhantomCommitInvariant ==
  \A tx \in DOMAIN walFile:
    walFile[tx].status = "committed"
      => \E entry \in transactionLog:
           entry.txID = tx
           /\ entry.action = "commit"
```

---

## 四、Recovery.tla 恢复规范

### 4.1 恢复场景

```
┌─────────────────────────────────────────────────────────────────┐
│                 Crash Recovery Scenarios                           │
└─────────────────────────────────────────────────────────────────┘

Scenario 1: Crash During Append
═══════════════════════════════════════════════════════════════════

Timeline: [TX_A append] --CRASH-- [TX_B append] --CRASH--
                    ↓                              ↓
          TX_A not in walFile           TX_A partial in walFile
          TX_B not started             Recovery removes TX_A

Scenario 2: Crash During Commit
═══════════════════════════════════════════════════════════════════

Timeline: [TX_A commit] --CRASH-- [fsync] --CRASH--
                        ↓                    ↓
              TX_A status=committing    TX_A status=unknown
              Recovery:                 Recovery:
              - Check walFile           - Scan walFile
              - If committed, replay   - Replay all committed
              - If not, rollback       - Rollback uncommitted

Scenario 3: Crash During Checkpoint
═══════════════════════════════════════════════════════════════════

Timeline: [CKPT start] --CRASH-- [CKPT write] --CRASH--
                        ↓                    ↓
              CKPT in progress          CKPT partial
              Recovery:                 Recovery:
              - Use previous CKPT      - Use previous CKPT
              - Full replay            - Partial replay
```

### 4.2 Recovery 规范

```tla
(*********************************************************************)
(* Recovery Correctness                                              *)
(*********************************************************************)
RecoveryCorrectness ==
  /\ transactionLog /= << >>
  => LET lastCheckpoint == FindLastCheckpoint(transactionLog)
                   recoveredLog == ReplayFrom(lastCheckpoint, transactionLog)
      IN
        /\ \A tx \in CommittedTransactions(recoveredLog):
             \E entry \in recoveredLog: entry.txID = tx /\ entry.action = "commit"
        /\ \A tx \in UncommittedTransactions(recoveredLog):
             \E entry \in recoveredLog: entry.txID = tx /\ entry.action = "rollback"

(*********************************************************************)
(* Audit Chain After Recovery                                        *)
(*********************************************************************)
AuditChainIntegrityAfterRecovery ==
  \A i \in 2..Len(recoveredLog):
    recoveredLog[i].prevHash = Hash(recoveredLog[i-1])
```

---

## 五、Checkpoint.tla 检查点规范

### 5.1 Checkpoint 结构

```tla
CheckpointStructure ==
  [
    checkpointLSN: LSN,
    transactionTable: [TransactionID -> {
      "committed", "aborted", "in_progress"
    }],
    dirtyPageTable: [PageID -> LSN],
    systemState: SYSTEM_STATE,
    timestamp: TimeStamp,
    signature: Signature
  ]
```

### 5.2 Checkpoint 不变量

```tla
(*********************************************************************)
(* Checkpoint Completeness                                           *)
(*********************************************************************)
CheckpointCompleteness ==
  \A ckpt \in Checkpoints:
    /\ ckpt.checkpointLSN = MaxLSN(ckpt.transactionTable)
    /\ \A tx \in DOMAIN ckpt.transactionTable:
        tx.status \in {"committed", "aborted"}
          => \E entry \in transactionLog:
               entry.lsn <= ckpt.checkpointLSN
               /\ entry.txID = tx
               /\ entry.action = tx.status

(*********************************************************************)
(* Checkpoint Consistency                                            *)
(*********************************************************************)
CheckpointConsistency ==
  \A ckpt1, ckpt2 \in Checkpoints:
    ckpt1.timestamp < ckpt2.timestamp
      => ckpt2.checkpointLSN > ckpt1.checkpointLSN
```

---

## 六、TLC 模型配置

### 6.1 WAL.cfg

```tla
SPECIFICATION Spec
\* SPECIFICATION defined for WAL state machine

INVARIANTS
  TypeInvariant
  UniqueLSNInvariant
  CommitPersistenceInvariant
  LSNMonotonicInvariant
  NoPhantomCommitInvariant

CONSTANTS
  NULL_LSN = 0
  MAX_LSN = 100

ACTION_CONSTRAINTS
  \* No more than 5 concurrent transactions
  Cardinality(DOMAIN walBuffer) <= 5
```

### 6.2 运行命令

```bash
# Model check WAL.tla
cd wal/tla
tlc -config WAL.cfg WAL.tla

# Expected output:
# Model checking completed. No error found.
# 4096 states generated, 0 distinct states violated invariant.
```

---

## 七、验收标准

### 7.1 TLA+ 验收

| 检查项 | 命令 | 标准 |
|--------|------|------|
| WAL model check | `tlc WAL.tla` | 无 violation |
| Recovery model check | `tlc Recovery.tla` | 无 violation |
| Checkpoint model check | `tlc Checkpoint.tla` | 无 violation |
| Invariant 覆盖 | 所有不变量检查 | 100% 通过 |

### 7.2 集成验收

```bash
# 每夜构建运行
bash scripts/gate/check_wal_tla.sh

# 检查点完整性测试
cargo test wal_checkpoint_integrity

# 恢复正确性测试
cargo test recovery_correctness
```

---

## 八、相关文档

- `docs/releases/v3.3.0/TRUST_INFRASTRUCTURE_STRATEGY.md` - 战略定位
- `wal/tla/WAL.tla` - WAL 状态机规范
- `wal/tla/Recovery.tla` - 恢复规范
- `wal/tla/Checkpoint.tla` - 检查点规范

---

*本文档由 hermes-agent 生成*
*版本 1.0 - 2026-05-18*
