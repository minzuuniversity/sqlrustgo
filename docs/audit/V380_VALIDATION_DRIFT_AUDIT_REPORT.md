# v3.8.0 Validation Drift Audit Report

**Audit Framework**: P0 工程事实优先 (Facts Over Artifacts)
**Date**: 2026-06-01
**Auditor**: Hermes Agent
**Branch**: develop/v3.8.0

---

## Executive Summary

v3.8.0 存在严重的 **Validation Drift（验证漂移）** 问题：WAL Recovery 测试声称验证了 Crash Recovery，但实际验证的是 WAL 自身（内存中的日志条目），而非经过持久化→重启→恢复整个闭环后用户能观察到的数据。

**核心发现**：
- 3个 RECOVERY 测试声称验证 Crash Recovery，实际只验证 WAL recover() 读取自己写的日志的能力
- `integration_wal.rs` 是唯一接近真实场景的测试，但没有跨越 FileStorage + WAL 的完整端到端验证
- 存在 `wal_storage.rs` (StorageEngine wrapper) 和 `file_storage.rs` (独立 StorageEngine) 两个执行逻辑违反 G-SSOT-01
- 没有 Test Design Review 记录，测试先于需求定义（逆向）
- 3个 audit 报告描述了验证目标，但没有一张截图、一个可重放命令、一份测试结果

**结论**：v3.8.0 WAL Integration 声称完成了 Crash Recovery 验证，但整个验证链路从未完成过一次端到端闭环。

---

## 一、P0 Audit 框架检查

### P0-1: Claim ≠ Fact 检查（最高优先级）

| Claim | Evidence | Status |
|---|---|---|
| "CRASH_RECOVERY_001 通过" | 无测试结果、无日志 | **BLOCKED** |
| "CRASH_RECOVERY_002 通过" | 无测试结果、无日志 | **BLOCKED** |
| "CRASH_RECOVERY_003 通过" | 无测试结果、无日志 | **BLOCKED** |
| "test_wal_recovery_uncommitted_transaction 验证了未提交事务回滚" | 只验证 WAL recover() 输出类型，未验证数据是否真的被丢弃 | **BLOCKED** |
| "WAL invariant 已验证" | 3份 audit 报告，零可重放测试证据 | **BLOCKED** |

**判定**：所有 Claim 均无可重放证据。按 P0 原则，视为未验证。

---

### P0-2: Requirement → Observable Behavior 链路分析

#### REQ-WAL-001: Crash Recovery

```
Requirement（需求）:  数据库崩溃后，已提交事务的数据必须存在
                      未提交事务的数据必须不存在

Observable Behavior（可观察行为）:
  1. INSERT + COMMIT → crash → 重启 → SELECT 返回该行
  2. INSERT + 无 COMMIT → crash → 重启 → SELECT 不返回该行

Test Design（测试设计）:
  1. 写数据 + COMMIT + 模拟进程终止 + 重启 + 验证数据存在
  2. 写数据 + 无 COMMIT + 模拟进程终止 + 重启 + 验证数据不存在

Assertion（断言）:
  1. row.value == "committed_data"
  2. row == None

Evidence（证据）:
  cargo test crash_recovery_persistence_committed
  cargo test crash_recovery_persistence_uncommitted
```

**现状**：
- crash_recovery_test.rs 中的测试使用的是 MemoryStorage
- MemoryStorage 崩溃后数据不存在 → 无法验证持久化数据的恢复
- REQ-WAL-001 的 Observable Behavior 从未被验证

**判定**：链路断裂。Requirement 未转化为任何有效测试。

---

### P0-3: 间接断言检查

| Test | Assertion | 需求 | 问题 |
|---|---|---|---|
| `test_wal_recovery_uncommitted_transaction` | `commits == 1` | 未提交事务不恢复 | 验证 WAL 输出 ≠ 验证数据库状态 |
| RECOVERY_005 (行数验证) | `COUNT(*) == 1` | UPDATE 存活 | 行数 ≠ 值内容 |
| RECOVERY_006 (行数验证) | `COUNT(*) == 1` | DELETE 不恢复 | 行数 ≠ 数据缺失 |
| integration_wal.rs (行数验证) | `entries.len() == 3` | 事务存在 | 验证 WAL 读出内容 ≠ 验证数据库正确 |

**判定**：所有测试均使用间接断言。无一直接证明 Requirement。

---

### P0-4: Single Source of Truth 检查

发现两个独立的存储执行路径：

```
Path A: FileStorage (crates/storage/src/file_storage.rs)
  ├── pub fn new(data_dir) → 自管理的 JSON 文件持久化
  ├── 自实现 insert/get/update/delete/batch_insert
  └── 无 WAL 集成

Path B: WalStorage<S, T> (crates/storage/src/wal_storage.rs)
  ├── pub fn new(inner: S, wal: T)
  ├── 包装另一个 StorageEngine (S) + WalManager (T)
  └── 所有操作通过 WAL 记录

问题：
  - FileStorage 和 WalStorage 都实现了 StorageEngine trait
  - FileStorage 绕过 WAL，违反 G-SSOT-01
  - 不清楚 v3.8.0 WAL Integration 的实际入口是哪个
```

**判定**：违反 G-SSOT-01。同一业务能力（持久化存储）存在两个实现。

---

### P0-5: 可重放证据检查

| 声称 | 要求提供 | 实际提供 |
|---|---|---|
| RECOVERY 测试通过 | 命令 + 日志 + 输出 | 无 |
| WAL invariant 验证 | 测试命令 + 测试结果 | 无 |
| integration_wal.rs 通过 | cargo test 输出 | 无 |

**判定**：无可重放证据。Claim 视为未验证。

---

## 二、测试设计 Review

### 2.1 现有测试清单

| 测试文件 | 测试数 | 验证对象 | 直接验证 Requirement？ |
|---|---|---|---|
| tests/crash_recovery_test.rs | 3 | 内存存储 + WAL recover() | 否 |
| tests/wal_integration_test.rs | 4 | WAL + FileStorage（独立） | 否 |
| crates/storage/tests/integration_wal.rs | 4 | WAL 自身 | 否 |
| crates/storage/tests/storage_e2e_test.rs | 5+ | FileStorage 端到端 | 部分 |

**关键问题**：
1. crash_recovery_test.rs 使用 MemoryStorage，无法验证持久化
2. wal_integration_test.rs 测试 WAL 对接 FileStorage，但没有完整闭环
3. 没有一个测试真正做到：写数据 → COMMIT → 进程终止 → 重启 → 验证数据存在

---

### 2.2 测试设计逆向问题

正确顺序：
```
Requirement
↓
Test Design Review（设计评审）
↓
Implementation
```

实际顺序：
```
Implementation（MemoryStorage + WAL）
↓
Test（验证内存中 WAL recover）
↓
声称验证了 Crash Recovery
```

**根因**：AI 快速生成了"看起来合理"的测试，但没有经过需求到测试设计的评审流程。

---

## 三、文件系统状态

### 3.1 分支状态

```
develop/v3.8.0 @ 474b36502
  ├── docs/audit/wal_invariant_report.md
  ├── docs/audit/tx_invariant_report.md
  ├── docs/audit/capability_contract_report.md
  ├── docs/governance/wal/MISSING_TESTS.md
  ├── docs/verification/RECOVERY_TEST_MIGRATION_PLAN.md
  ├── tests/crash_recovery_test.rs (119 行)
  ├── tests/wal_integration_test.rs (316 行)
  ├── crates/storage/tests/integration_wal.rs (123 行)
  └── crates/storage/tests/storage_e2e_test.rs (289 行)
```

### 3.2 本地状态

```
git status: 干净
```

---

## 四、证据链分析

### 4.1 完整验证链路应该是什么样的

```
Step 1: 创建真实数据库文件 + WAL 文件
Step 2: 通过 StorageEngine 接口写入数据
Step 3: COMMIT（触发 WAL 刷盘 + 数据刷盘）
Step 4: 进程终止（模拟 crash）
Step 5: 新进程启动 + 加载 StorageEngine
Step 6: SELECT 查询
Step 7: 断言返回值 == expected
```

**当前测试状态**：
- Step 1-3: integration_wal.rs 部分覆盖
- Step 4-7: **完全缺失**

### 4.2 WalStorage 是否被测试过？

```
grep "WalStorage" tests/*.rs crates/storage/tests/*.rs
```

结果：没有任何测试导入或使用 WalStorage。

---

## 五、结论

### 5.1 Validation Drift 确认

```
需求: Crash Recovery
  ↓
测试设计: MemoryStorage + WAL recover()
  ↓
测试实现: 验证 WAL 输出类型和数量
  ↓
AI 报告: "CRASH_RECOVERY 测试通过"
  ↓
事实: 从未真正完成一次端到端重启验证
```

### 5.2 阻塞项（按优先级）

| # | 阻塞项 | 影响 |
|---|---|---|
| 1 | 所有 Crash Recovery 测试使用 MemoryStorage，无法验证持久化 | REQ-WAL-001 完全未验证 |
| 2 | 无任何端到端闭环测试（写→刷盘→重启→验证） | WAL 功能未验证 |
| 3 | WalStorage 未被任何测试使用 | 集成点未验证 |
| 4 | 无可重放证据 | 所有 Claim 无效 |
| 5 | G-SSOT-01 违反 | 两个执行路径存在 |

---

## 六、整改计划

### Phase 1: 创建可验证的 WAL Recovery 测试（本周）

**目标**：让 REQ-WAL-001 真正可验证

**Task 1.1**: 创建端到端 WAL Recovery 测试
```
文件: tests/wal_e2e_recovery_test.rs
内容:
  - test_wal_recovery_with_file_storage_persistence
  - test_wal_recovery_uncommitted_data_not_survived
  - test_wal_recovery_multiple_transactions
  - test_wal_recovery_restart_after_checkpoint
```

**Task 1.2**: 测试必须验证可观察行为
```
断言规则:
  - 验证值内容 (value == expected)，不验证行数
  - 未提交数据: row == None（不是 COUNT(*) == 0）
  - 已提交数据: row.value == committed_value（不是 WAL entry 数量）
```

**Task 1.3**: 提供可重放证据
```
每个测试必须输出:
  - cargo test 命令
  - 测试结果日志
  - 证据截图（如需要）
```

### Phase 2: SSOT 修复（下周）

**目标**：消除两个执行路径违反 G-SSOT-01

**Task 2.1**: 确定 WalStorage 是唯一入口
```
决策: WalStorage<FileStorage, FileBackedWalManager> 是唯一存储路径
删除或标记 FileStorage 独立路径为 legacy（如果确实如此）
```

**Task 2.2**: 删除 MemoryStorage 在 recovery 测试中的使用
```
替换: MemoryStorage → FileStorage + WalStorage
```

### Phase 3: Knowledge OS 证据链建立（持续）

**目标**：每个 Claim 都有可重放证据

**Task 3.1**: 建立 Claim Registry
```
格式:
  CLAIM-XXX: [描述]
  Evidence: [命令]
  Result: [PASS/FAIL]
  Date: [日期]
```

---

## 七、Example 验证（本报告核心产出）

下面创建一个最小可验证的 WAL Recovery 测试，证明我的分析框架有效：

### 目标

验证 REQ-WAL-001：`COMMIT 后的数据 crash 重启后必须可读`

### 测试设计

```
Requirement: INSERT + COMMIT 后 crash 重启，数据仍存在
Observable Behavior: SELECT 返回该行，且 value == 'Alice'
Test Design:
  1. 用 WalStorage<FileStorage, FileBackedWalManager> 写入数据
  2. COMMIT（触发 WAL + Storage 刷盘）
  3. drop storage（模拟进程终止，不触发 destruct 等）
  4. 重新打开 WalStorage
  5. SELECT + assert_eq!(value, "Alice")
```

### 断言设计

```
好: assert_eq!(row.get("name"), Value::Text("Alice"))
坏: assert!(entries.len() > 0)
坏: assert_eq!(table_rows.len(), 1)
```

---

**Report Status**: DRAFT - 待李哥确认方向后开始实施 Phase 1
**Next Action**: 等待李哥确认是否按此计划进行 WAL Recovery 测试整改