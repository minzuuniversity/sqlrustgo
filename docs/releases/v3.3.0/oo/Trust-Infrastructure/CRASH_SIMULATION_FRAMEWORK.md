# OO-TI2: Crash Simulation Framework

> **版本**: v1.0
> **日期**: 2026-05-18
> **基于**: v3.3.0
> **维护人**: hermes-agent
> **Issue**: #1236
> **状态**: 新建设计

---

## 一、概述

### 1.1 目标

验证 WAL/MVCC/SSI/审计链在随机崩溃后的正确性，确保系统在各种故障场景下的可恢复性和数据完整性。

### 1.2 核心理念

```
Crash Simulation = Chaos Injection + Recovery Verification + Correctness Proof
```

### 1.3 问题背景

GMP 系统必须保证：
- 崩溃后审计链不断链
- 签名证明可验证
- WAL replay 正确
- 无 phantom commit

---

## 二、技术架构

### 2.1 组件关系

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    Crash Simulation Framework                                 │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────────┐    ┌───────────────────┐    ┌───────────────────┐  │
│  │  crash-injector  │───▶│  chaos-test suite │───▶│  recovery-verifier │  │
│  │  (kill points)   │    │  (TLA+ guided)   │    │  (data integrity)  │  │
│  └──────────────────┘    └───────────────────┘    └───────────────────┘  │
│                                                                              │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │                      Kill Points                                     │  │
│  │  - wal_append    - page_split    - fsync                          │  │
│  │  - commit        - checkpoint    - buffer_write                   │  │
│  └───────────────────────────────────────────────────────────────────┘  │
│                                                                              │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │                   Verification Targets                              │  │
│  │  - WAL replay correctness                                          │  │
│  │  - Audit chain continuity                                          │  │
│  │  - Signature verifiability                                         │  │
│  │  - MVCC snapshot isolation                                         │  │
│  │  - No phantom commits                                              │  │
│  └───────────────────────────────────────────────────────────────────┘  │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2.2 组件说明

| 组件 | 说明 | 位置 |
|------|------|------|
| `crash-injector` | 随机注入故障点 | `crates/crash-injector/` |
| `chaos-test` | TLA+ 引导的测试序列 | `tests/chaos/` |
| `recovery-verifier` | 验证数据完整性 | `crates/recovery-verifier/` |

---

## 三、Kill Point 设计

### 3.1 WAL 层级

```
WAL Lifecycle Kill Points:
═══════════════════════════════════════════════════════════════════

┌─────────┐   ┌─────────┐   ┌─────────┐   ┌─────────┐   ┌─────────┐
│  BEGIN  │──▶│ APPEND  │──▶│ COMMIT  │──▶│ FLUSH   │──▶│ CKPT    │
└─────────┘   └─────────┘   └─────────┘   └─────────┘   └─────────┘
                   │             │             │             │
                   ▼             ▼             ▼             ▼
              ┌─────────┐   ┌─────────┐   ┌─────────┐   ┌─────────┐
              │wal_     │   │commit_  │   │fsync_   │   │checkpoint_
              │append   │   │syncing  │   │waiting  │   │writing  │
              └─────────┘   └─────────┘   └─────────┘   └─────────┘
```

### 3.2 Buffer Pool 层级

```
Buffer Pool Kill Points:
═══════════════════════════════════════════════════════════════════

┌─────────────┐   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐
│  PAGE_FETCH │──▶│ PAGE_MODIFY │──▶│ PAGE_SPILL  │──▶│ PAGE_EVICT  │
└─────────────┘   └─────────────┘   └─────────────┘   └─────────────┘
       │                 │                 │                 │
       ▼                 ▼                 ▼                 ▼
  ┌─────────┐      ┌─────────┐      ┌─────────┐      ┌─────────┐
  │buffer_  │      │dirty_   │      │spill_   │      │evict_   │
  │reading  │      │marking  │      │writing  │      │selecting │
  └─────────┘      └─────────┘      └─────────┘      └─────────┘
```

### 3.3 审计链层级

```
Audit Chain Kill Points:
═══════════════════════════════════════════════════════════════════

┌─────────────┐   ┌─────────────┐   ┌─────────────┐   ┌─────────────┐
│ SIGN_CREATE │──▶│ CHAIN_LINK  │──▶│ SIG_STORE   │──▶│ HASH_VERIFY │
└─────────────┘   └─────────────┘   └─────────────┘   └─────────────┘
       │                 │                 │                 │
       ▼                 ▼                 ▼                 ▼
  ┌─────────┐      ┌─────────┐      ┌─────────┐      ┌─────────┐
  │sign_gen │      │chain_   │      │store_   │      │hash_    │
  │         │      │attach   │      │write    │      │compute  │
  └─────────┘      └─────────┘      └─────────┘      └─────────┘
```

---

## 四、执行流程

### 4.1 Crash Simulation Test Flow

```
┌─────────────────────────────────────────────────────────────────┐
│              Crash Simulation Test Flow                            │
└─────────────────────────────────────────────────────────────────┘

Start: cargo test --test crash_simulation
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│  1. Initialize Test Database                                     │
│     - Create fresh test DB                                       │
│     - Insert seed data                                           │
│     - Record initial hash                                        │
└─────────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│  2. Execute Transaction Workload                                 │
│     - Mix of: INSERT, UPDATE, DELETE, SELECT                     │
│     - Sign transactions per GMP rules                           │
│     - Record transaction sequence                                │
└─────────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│  3. Select Random Kill Point                                     │
│     - Random from: wal_append, commit, fsync, checkpoint, etc.   │
│     - Suspend execution at kill point                           │
│     - Simulate power loss                                        │
└─────────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│  4. Crash and Recover                                            │
│     - Kill process (SIGKILL)                                    │
│     - Wait 100ms                                                 │
│     - Restart database                                           │
│     - Trigger recovery                                           │
└─────────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│  5. Verify Recovery Correctness                                  │
│     - WAL replay correctness                                     │
│     - No phantom commits                                         │
│     - Audit chain continuity                                     │
│     - Signature verifiability                                    │
└─────────────────────────────────────────────────────────────────┘
        │
        ▼
    ┌────────────┐
    │  Verify    │
    │  Pass?     │
    └─────┬──────┘
          │
    ┌─────┴─────┐
    │           │
   YES          NO
    │           │
    ▼           ▼
┌────────┐  ┌─────────────────────────────────────────────────────┐
│ Next   │  │ FAIL: Log failure details                          │
│ Test   │  │ - Kill point: {point}                              │
│        │  │ - Last valid tx: {tx_id}                            │
│        │  │ - Expected hash: {hash}                              │
│        │  │ - Actual hash: {hash}                                │
└────────┘  └─────────────────────────────────────────────────────┘
```

### 4.2 TLA+ Guided Sequence Generation

```
┌─────────────────────────────────────────────────────────────────┐
│          TLA+ Guided Chaos Sequence Generation                     │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│  TLA+ Model: CrashSimulation.tla                                │
│  ─────────────────────────────────────────                       │
│  VARIABLES                                                       │
│    systemState  ∈ {healthy, crashed, recovering, recovered}      │
│    killPoint    ∈ {none, wal_append, commit, fsync, checkpoint} │
│    txLog        ∈ Seq[Transaction]                               │
│    auditChain  ∈ Seq[AuditEntry]                               │
└─────────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│  TLC Model Checker                                              │
│  - Exhaustively explore state space                              │
│  - Find all possible crash scenarios                             │
│  - Generate crash sequence leading to invariant violation        │
└─────────────────────────────────────────────────────────────────┘
        │
        ▼
┌─────────────────────────────────────────────────────────────────┐
│  Chaos Test Sequences                                            │
│  1. Append → Crash → Recover (no commit)                        │
│  2. Commit → fsync → Crash → Recover (commit persisted?)        │
│  3. Append → Commit → fsync → Crash → Recover                   │
│  ...                                                             │
│  N. All combinations explored                                   │
└─────────────────────────────────────────────────────────────────┘
```

---

## 五、验证目标

### 5.1 WAL Replay Correctness

```
验证条件:
═══════════════════════════════════════════════════════════════════

1. All committed transactions appear in recovered DB
2. All uncommitted transactions do NOT appear in recovered DB
3. Transaction order is preserved
4. No duplicate or missing records
5. LSN sequence is continuous or properly truncated
```

### 5.2 Audit Chain Continuity

```
验证条件:
═══════════════════════════════════════════════════════════════════

1. Chain is unbroken (each entry links to previous)
2. No gaps in sequence numbers
3. Each signature is verifiable against stored public key
4. Hash chain is intact
5. Timestamp ordering is correct
```

### 5.3 No Phantom Commits

```
验证条件:
═══════════════════════════════════════════════════════════════════

1. Transaction not seen in WAL as committed does NOT appear in DB
2. MVCC snapshot isolation is maintained
3. Concurrent transactions do not see each other's uncommitted changes
4. SSI (Serializable Snapshot Isolation) guarantees held
```

---

## 六、验收标准

### 6.1 数量要求

| 场景 | 最小测试次数 | 标准 |
|------|-------------|------|
| 随机崩溃 | 1000 次 | 无数据损坏 |
| Kill point 覆盖 | 所有 12 个 kill points | 每个至少 100 次 |
| 并发压力 | 100 次 | 审计链连续 |

### 6.2 验证命令

```bash
# 运行 1000 次随机崩溃测试
cargo test --test crash_simulation -- --test-threads=4 1000

# 运行特定 kill point 测试
cargo test --test crash_simulation kill_point::wal_append -- --test-threads=4 500

# 运行 TLA+ guided 测试
cargo test --test crash_simulation tla_guided -- --test-threads=4

# 验证审计链连续性
cargo test --test audit_chain_continuity
```

---

## 七、相关文档

- `docs/releases/v3.3.0/TRUST_INFRASTRUCTURE_STRATEGY.md` - 战略定位
- `crates/crash-injector/` - 崩溃注入器实现
- `tests/chaos/` - TLA+ 引导的混沌测试
- `crates/recovery-verifier/` - 恢复验证器实现

---

*本文档由 hermes-agent 生成*
*版本 1.0 - 2026-05-18*
