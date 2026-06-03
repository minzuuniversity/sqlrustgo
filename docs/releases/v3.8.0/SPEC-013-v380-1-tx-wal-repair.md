# SPEC-013 — v3.8.0+1 TX+WAL Contract Gaps Repair Plan

> **PR Number**: SPEC-013 (v3.8.0 post-GA planning)
> **PR Title**: v3.8.0+1 TX+WAL 12-gap repair roadmap
> **Version**: v3.8.0+1 (post-GA)
> **Branch**: `feature/spec-013-v380-1-plan` (从 `14e8a7744` 切出)
> **Auditor**: Claude (claude-macmini, architect)
> **Created**: 2026-06-03
> **Status**: DRAFT — 待执行

---

## 1. 概述

### 1.1 问题（基于 ISSUE-2743 + ADR-006）

v3.8.0 GA 通过 ADR-006 决定 defer 12 个 TX+WAL contract gap。
- 当前：19/31 contract tests PASS / 12 FAIL
- 12 FAIL 分类：
  - **TX-Lifecycle (4 tests)**: EEK v0 spec 要求 DML 无 active tx → Err；当前是 autocommit
  - **WAL Recovery (8 tests)**: Multi-tx ordering + partial-write 语义与 spec 不一致

### 1.2 依赖状态（2026-06-03）

| 前置 | 状态 | 关联 |
|------|------|------|
| ADR-006 TX+WAL Deferral | ✅ Accepted (2026-06-03) | 已合并 |
| ADR-007 WAL Architecture (4 decisions) | ✅ Accepted (2026-06-03) | 已合并 PR #2783 |
| ADR-007 Decision-4 (read-only mode) | ✅ 选项 A：DML 返回 Err | 已合并 |
| ISSUE-2740 (Crash Recovery Empirical) | ✅ PR #2780 已合并 | 19/31 baseline |

**所有前置已就绪**，可以开始 v3.8.0+1 实施。

## 2. 设计：2 阶段 Spikes

### Phase A: TX-Lifecycle Spike (3 周)

**目标**：从"implicit autocommit"过渡到"EEK v0 strict"模式

| 周 | 任务 | 产出 |
|----|------|------|
| A.1 | TX-lifecycle dual-mode 设计（保留 autocommit + 严格模式） | SPEC |
| A.2 | `SQLRUSTGO_EEK_MODE=strict` 环境变量 | 1 PR |
| A.3 | DML 调用点改造（5 处：DML executor / trigger / LocalExecutor / Parallel / storage facade） | 1-2 PR |
| A.4 | 4 个 TX-Lifecycle test 转为 PASS | 验证 |
| A.5 | 19 个原有 test 仍 PASS | 不退化 |

**风险**：
- 19 个 PASS 可能因 lifecycle 改变而 FAIL
- 缓解：dual-mode 设计，strict 模式 opt-in

### Phase B: WAL Recovery Spike (4 周)

**目标**：实现 multi-tx ordering + partial-write 正确语义

| 周 | 任务 | 产出 |
|----|------|------|
| B.1 | WAL replay order 算法设计 | SPEC |
| B.2 | `WalStorage::log_*` 完整 ordering | 1-2 PR |
| B.3 | `RecoveryEngine` 改写（undo semantics） | 1 PR |
| B.4 | 8 个 WAL Recovery test 转为 PASS | 验证 |
| B.5 | 5/5 exp_g + 8/8 e2e 仍 PASS | 不退化 |

**风险**：
- WAL 是数据库核心，blast radius 极大
- 5/5 exp_g_wal_contracts_verified 必须保持绿色
- 缓解：分步 PR，每步有独立 gate

### Phase C: Integration + GA (2 周)

| 周 | 任务 | 产出 |
|----|------|------|
| C.1 | 31/31 contract tests 全部 PASS | 验证 |
| C.2 | `tests/tx_wal_contract_tests.rs` 从 untracked → tracked | 1 PR |
| C.3 | v3.8.0+1 RC Gate | Gate 报告 |
| C.4 | v3.8.0+1 GA 发布 | Tag + Release Notes |

**总时间表**: ~9 周（2 个月）

## 3. 完成标准

- [ ] POST_GA_PLAN.md 创建（`docs/releases/v3.8.0/POST_GA_PLAN.md`）
- [ ] ADR-011 创建（`docs/governance/adr/ADR-011-v3.8.0-1-tx-wal-repair.md`）
- [ ] 12 gap 分类清晰（TX-Lifecycle 4 / WAL Recovery 8）
- [ ] 时间表 + 依赖图
- [ ] 不退化保证（19+8+5=32 个现有 test）
- [ ] PR 合并 + Issue #2776 关闭

## 4. 不在 v3.8.0+1 范围

- F-07~F-15 Ghost PRs（独立 issue #2774）
- Cross-Version Debt 11 ACTIVE（独立 issue #2773）
- v3.9 MVCC（独立版本）

## 5. Refs

- ISSUE-2743 (12 contract gaps)
- ADR-006 (deferral)
- ADR-007 (WAL architecture 4 decisions)
- ISSUE-2740 (Crash Recovery Empirical)
- Task #2776 (closes)
- 父报告: V380_RECTIFICATION_PLAN_2026-06-03.md §3.3
