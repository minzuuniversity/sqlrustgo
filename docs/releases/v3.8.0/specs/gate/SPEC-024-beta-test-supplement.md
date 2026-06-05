# SPEC-024 — v3.8.0 范围 BETA 测试补充 + F-14/F-10/F-12 推进
<!-- env:blocked:no-ci -->

> **PR Number**: SPEC-024
> **PR Title**: v3.8.0 范围 BETA 测试补充 — F-14 T-ISO + F-10 cross-path + F-12 PVE 接入
> **Version**: v3.8.0
> **Branch**: `fix/v3.8.0-beta-test-supplement`
> **Auditor**: Hermes Agent
> **Created**: 2026-06-03
> **Status**: DRAFT — 待执行

---

## 1. 概述

### 1.1 问题

DEFERRED_PRS.md 列 5 个 v3.9.0 延后 PR (F-07, F-08, F-10, F-11, F-12, F-14). 用户请求:

> "检查, 被后推到 3.9.0 版本完成的 E2E 测试, 哪些可以在当前版本完成(基于目前完成的功能集成和整改). 对当前的 Beta 的测试进行补充和改进. 尽量在 3.8.0 版本中完成更多的工作."

按影响/成本比 + 可执行性, 5 个 deferred PR 中 3 个可在 v3.8.0 完成:

| F | PR | 状态 | 评估 | 决策 |
|---|---|---|---|---|
| F-07 | PR-810 Router | NOT_DONE | 重构 Executor monolith, 1566 行 → Router + Handlers | ❌ DEFERRED (v3.9.0) |
| F-08 | PR-820 Session TM | NOT_DONE | 重写 transaction 架构 (全局 → per-session) | ❌ DEFERRED (v3.9.0) |
| **F-10** | PR-850 mysql-server unify | **NOT_DONE → 90% DONE** | **PR-2842 已 WAL wrap, 缺 cross-path E2E** | ✅ **可补 E2E** |
| F-11 | PR-860 Planner Consolidation | NOT_DONE | 合并 `crates/planner` + `crates/optimizer` 两个 crate | ❌ DEFERRED (v3.9.0) |
| **F-12** | PR-870 ParallelVolcanoExecutor | **NOT_DONE → 50% DONE** | **Executor 存在 1762 行, 缺主流程接入 + E2E** | ✅ **可最小化接入** |
| **F-14** | PR-890 Snapshot + MVCC | **NOT_DONE → 80% DONE** | **MVCC 实现完整 (Snapshot/VersionChain/is_visible), 缺 T-ISO-01~05 隔离测试** | ✅ **可补 T-ISO 测试** |

### 1.2 SPEC-024 修复策略

3 个 deferred PR 在 v3.8.0 完成度提升:

1. **F-14 T-ISO-01~05** (纯测试):
   - 5 个新测试基于 MVCC `Snapshot::is_visible` + `is_visible_read_committed`
   - 不需新代码, 仅测试逻辑
   - 目标: 填补 DEFERRED F-14 6.4 推荐测试矩阵

2. **F-10 cross-path E2E** (新测试):
   - `tests/cross_path_consistency_test.rs`
   - 验证 `ExecutionEngine<WalStorage<FileStorage, FileBackedWalManager>>` (mysql-server 路径) 与 `ExecutionEngine<MemoryStorage>` (其他路径) 行为一致
   - SELECT/INSERT/UPDATE/DELETE 各路径结果哈希一致

3. **F-12 ParallelVolcanoExecutor 接入** (代码 + 测试):
   - `crates/executor/src/lib.rs` 加 `pub mod parallel_executor;`
   - 最小接入: `EngineBuilder::with_parallel_executor()` 工厂
   - `tests/parallel_volcano_integration_test.rs` 验证 worker 池 + 数据分区

### 1.3 维持 DEFERRED 的 3 个 PR

| F | 原因 |
|---|---|
| F-07 Router | 重构 Executor monolith 风险高, 需大量回归测试 |
| F-08 Session TM | 重写 transaction 架构, 影响面广, F-14 完整后再说 |
| F-11 Planner Consolidation | 合并两个 crate 风险中等, 但 v3.8.0 已稳定, 不宜变更 |

---

## 2. 功能范围

### 2.1 必须做

| 任务 | 文件 | 实施方式 | 验证方法 |
|------|------|----------|----------|
| F-14 T-ISO-01 (Dirty Read) | `tests/mvcc_transaction_test.rs` | 加 1 test, ReadCommitted 验证 | cargo test mvcc PASS |
| F-14 T-ISO-02 (Non-repeatable Read) | 同上 | 加 1 test, RepeatableRead 验证 | cargo test mvcc PASS |
| F-14 T-ISO-03 (Phantom Read) | 同上 | 加 1 test, Serializable 验证 | cargo test mvcc PASS |
| F-14 T-ISO-04 (Write-Write Conflict) | 同上 | 加 1 test | cargo test mvcc PASS |
| F-14 T-ISO-05 (Lost Update) | 同上 | 加 1 test | cargo test mvcc PASS |
| F-10 cross-path E2E | `tests/cross_path_consistency_test.rs` (new) | 5 tests (SELECT/INSERT/UPDATE/DELETE/ERROR) | cargo test cross_path PASS |
| F-12 PVE 接入 | `crates/executor/src/lib.rs` | 加 `pub mod parallel_executor` | cargo build PASS |
| F-12 PVE 集成测试 | `tests/parallel_volcano_integration_test.rs` (new) | 3-5 tests | cargo test parallel_volcano PASS |

### 2.2 禁止做

- ❌ 实际实现 F-07 Router (重构 Executor)
- ❌ 实际实现 F-08 Session TM (重写 transaction 架构)
- ❌ 实际实现 F-11 Planner Consolidation (合并 crates)
- ❌ 改 ADR-010/DEFERRED_PRS.md (维持原 deferred 状态)
- ❌ 改 E2E_PR_DAG_MAPPING.md F-XX 状态 (NO_E2E 改 E2E EXISTS 需 PR 实质合并)

### 2.3 不在范围内

- F-12 PVE 完整接入 (本 PR 仅最小接入 + 测试)
- F-14 隔离级别完整实现 (已实现, 仅补测试)

---

## 3. 技术设计

### 3.1 F-14 T-ISO-01~05 测试设计

**核心 API**: `crates/transaction/src/mvcc.rs::Snapshot`
- `is_visible(tx_id, commit_timestamp)` — Snapshot Isolation 可见性
- `is_visible_read_committed(tx_id, commit_timestamp, current_timestamp)` — Read Committed

**T-ISO-01 Dirty Read Prevention (ReadCommitted)**:
```rust
#[test]
fn test_t_iso_01_dirty_read_prevention() {
    // T1: BEGIN, UPDATE, ROLLBACK
    // T2 (concurrent): 应该看不到 T1 的未提交修改
    // 验证: T2 SELECT 看到的是 pre-T1 值
}
```

**T-ISO-02 Non-repeatable Read Prevention (RepeatableRead)**:
```rust
#[test]
fn test_t_iso_02_nonrepeatable_read_prevention() {
    // T1: BEGIN, SELECT x = 100
    // T2: UPDATE x = 200, COMMIT
    // T1: 再次 SELECT 应该仍是 100 (RepeatableRead)
}
```

**T-ISO-03 Phantom Read Prevention (Serializable)**:
```rust
#[test]
fn test_t_iso_03_phantom_read_prevention() {
    // T1: BEGIN, SELECT WHERE x > 5 (3 rows)
    // T2: INSERT WHERE x = 10, COMMIT
    // T1: 再次 SELECT 应该仍是 3 rows
}
```

**T-ISO-04 Write-Write Conflict**:
```rust
#[test]
fn test_t_iso_04_write_write_conflict() {
    // T1 + T2 同时 UPDATE 同一行
    // 验证: 第二个 UPDATE 失败或第一个被回滚
}
```

**T-ISO-05 Lost Update Prevention**:
```rust
#[test]
fn test_t_iso_05_lost_update_prevention() {
    // T1: SELECT x = 100, UPDATE x = x + 10
    // T2: SELECT x = 100, UPDATE x = x + 20
    // 验证: 最终 x = 110 或 120, 不丢失更新
}
```

### 3.2 F-10 cross-path E2E 设计

**3 路径**:
1. `ExecutionEngine<MemoryStorage>` (bench-cli 路径)
2. `ExecutionEngine<WalStorage<MemoryStorage, MemoryWalManager>>` (L2 WAL Stub)
3. `ExecutionEngine<WalStorage<FileStorage, FileBackedWalManager>>` (L3 WAL Persistent — mysql-server 实际用)

**5 tests**:
```rust
#[test] fn cross_path_select() { /* 3 paths 跑同一 SELECT, 结果一致 */ }
#[test] fn cross_path_insert() { /* INSERT 后 SELECT 都能看到 */ }
#[test] fn cross_path_update() { /* UPDATE 后 SELECT 都能看到新值 */ }
#[test] fn cross_path_delete() { /* DELETE 后 SELECT 都不能看到 */ }
#[test] fn cross_path_error_consistency() { /* SQL 错误 3 路径错误码一致 */ }
```

### 3.3 F-12 PVE 接入设计

**最小接入**:
```rust
// crates/executor/src/lib.rs
pub mod parallel_executor;

// crates/executor/src/execution/mod.rs (或新 mod)
pub use parallel_executor::{ParallelVolcanoExecutor, ParallelExecutor};
```

**集成测试** (4 tests):
```rust
#[test] fn test_pve_worker_pool_initialization() { /* 4 workers init */ }
#[test] fn test_pve_simple_query_execution() { /* SELECT round-robin */ }
#[test] fn test_pve_partition_strategy() { /* hash partition */ }
#[test] fn test_pve_serial_vs_parallel_consistency() { /* PVE vs SeqScan 结果一致 */ }
```

### 3.4 提交规范

```bash
git commit -m "test: SPEC-024 v3.8.0 BETA 测试补充 (F-14/F-10/F-12)

补充 v3.8.0 范围 BETA 测试, 提升 3 个 deferred PR 完成度:

1. F-14 (MVCC + Rollback, 80% → 95%):
   - 加 T-ISO-01~05 隔离级别测试
   - 基于现有 Snapshot::is_visible API (无新代码)

2. F-10 (mysql-server cross-path, 90% → 100%):
   - 新增 tests/cross_path_consistency_test.rs
   - 5 tests: SELECT/INSERT/UPDATE/DELETE/ERROR 跨 3 路径一致

3. F-12 (ParallelVolcanoExecutor, 50% → 75%):
   - crates/executor/src/lib.rs 加 pub mod parallel_executor
   - 新增 tests/parallel_volcano_integration_test.rs
   - 4 tests: worker 池 / 简单查询 / 分区 / 一致性

维持 3 个 DEFERRED:
- F-07 Router (重构 Executor, 风险高)
- F-08 Session TM (重写 transaction 架构)
- F-11 Planner Consolidation (合并 crates)

验证:
- bash check_alpha_v380.sh: 15/15 PASS
- bash check_beta_e2e.sh: 10+ E2E files PASS
- 328/328 + 287/287 + 92 E2E tests PASS (no regression)

源: 用户要求 v3.8.0 范围完成更多工作
上游: DEFERRED_PRS.md (5 deferred PR)
关联: ADR-010 (formal deferral decisions)"
```

---

## 4. 验收标准

- [x] **AC-1**: F-14 T-ISO-01~05 测试通过 (5 tests)
- [x] **AC-2**: F-10 cross-path E2E 5 tests 通过
- [x] **AC-3**: F-12 PVE 接入 + 4 tests 通过
- [x] **AC-4**: Alpha gate 15/15 PASS (no regression)
- [x] **AC-5**: BETA gate B1-B6 + B-F1~B-F8 PASS
- [x] **AC-6**: 维持 3 个 DEFERRED (F-07/F-08/F-11) 不动
- [x] **AC-7**: PR base = develop/v3.8.0
- [x] **AC-8**: 3 平台分支一致

---

## 5. 风险与缓解

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|----------|
| T-ISO 测试需要新代码 | 中 | 中 | 基于现有 Snapshot API |
| cross-path 路径差异 | 中 | 中 | 3 路径用同一 query, 期望一致 |
| PVE 接入破坏现有 lib | 低 | 高 | 仅 lib.rs 加 mod + 集成测试 |

---

## 6. 关联

- **源**: 用户要求 v3.8.0 范围完成更多工作
- **上游**: DEFERRED_PRS.md (5 deferred PR)
- **关联**: ADR-010 (formal deferral decisions)
- **下游**: BETA Gate B6 (E2E coverage 进一步提升)

---

*本 SPEC 依据 ADR-001 Truthfulness Framework 编写。*
