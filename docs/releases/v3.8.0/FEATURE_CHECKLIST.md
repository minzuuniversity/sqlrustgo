# v3.8.0 Feature Checklist

> **版本**: v3.8.0  
> **阶段**: Alpha → Beta  
> **更新日期**: 2026-05-31  
> **gate_policy_eval_id**: `run_20260601_010`  
> **维护者**: Hermes C  

---

## 功能清单

| ID | 功能 | PR | 状态 | 备注 |
|----|------|-----|------|------|
| F-01 | WAL 模块架构 (PR-830A) | PR-830A | ✅ DONE | FileStorage production ready |
| F-02 | WAL 抽象层 (PR-830B) | PR-830B | ✅ DONE | memory/file WAL managers |
| F-03 | WAL Replay 正确性 (PR-830C) | PR-830C | ✅ DONE | delegated to storage engine |
| F-04 | RecoveryEngine (PR-830D) | PR-830D | ✅ DONE | deterministic WAL replay |
| F-05 | Engine Restart + FileStorage (PR-830E) | PR-830E | ✅ DONE | boot sequence documented |
| F-06 | TransactionalFacade (PR-800) | — | ❌ NOT DONE | 代码未实现，DEFERRED |
| F-07 | PR-810: WAL Commit Path | — | ❌ NOT DONE | 未合并，无 Issue 追踪 |
| F-08 | PR-820: WAL Rollback Path | — | ❌ NOT DONE | 未合并，无 Issue 追踪 |
| F-09 | PR-840: WriteBuffer Integration | — | ❌ NOT DONE | 未合并 |
| F-10 | PR-850: DML Through WriteBuffer | — | ❌ NOT DONE | 未合并 |
| F-11 | PR-860: COMMIT Flushes WriteBuffer | — | ❌ NOT DONE | 未合并 |
| F-12 | PR-870: ROLLBACK Discards WriteBuffer | — | ❌ NOT DONE | 未合并 |
| F-13 | PR-880: WAL Recovery Integration | — | ❌ NOT DONE | 未合并 |
| F-14 | PR-890: WAL TPC-H Validation | — | ❌ NOT DONE | 未合并 |
| F-15 | PR-900: WAL Performance Baseline | — | ❌ NOT DONE | 未合并 |
| F-16 | PR-830F: WAL Lifecycle + Checkpoint Truncation | PR-2697 | ✅ DONE | checkpoint-based LSN truncation |

---

## 完成标准

### ✅ DONE

功能已完成并合并到 develop/v3.8.0。

### ❌ NOT DONE

功能未完成。有两种情况：

1. **有明确计划**：在 LEGACY_ISSUES.md 或对应 Issue 中记录，预计在后续版本完成
2. **无追踪**：未合并且无任何说明，属于"幽灵 PR"

### ⚠️ DEFERRED

功能已规划但延后到下一版本，不影响当前版本 Beta Gate。

---

## Beta Gate 功能状态

**B-Functional 要求**：Beta Gate 必须验证所有计划 PR 的状态。

| 检查 | 状态 | 说明 |
|------|------|------|
| B-F1 (F-03) | ✅ PASS | PR-830C merged |
| B-F2 (F-04) | ✅ PASS | PR-830D merged |
| B-F3 (F-05) | ✅ PASS | PR-830E merged |
| B-F4 (F-06) | ⚠️ DEFERRED | TransactionalFacade deferred, tracked in #2603 |
| B-F5 | ✅ PASS | PR-DAG in DEVELOPMENT_PLAN.md matches actual |
| B-F6 | ✅ PASS | FEATURE_CHECKLIST.md created |
| B-F7 | ✅ PASS | No orphan PRs (all merged PRs have corresponding issues) |

---

## 功能缺失说明

### F-06: TransactionalFacade (PR-800)

**状态**: DEFERRED  
**原因**: PR-800 Foundation 已合并（DriftGate, TransactionContext, WriteOp），但 TransactionalFacade 接口本身未实现。  
**追踪 Issue**: #2603 (R2: 执行引擎统一)  
**影响**: 影响 RC 阶段（不是 Beta 阶段），已在 DEVELOPMENT_PLAN.md 中说明。

### F-07 ~ F-16: PR-810/820/840/850/860/870/880/890/900

**状态**: NOT DONE, 无追踪  
**原因**: 这些 PR 是 PR-800 chain 的后续扩展，在 PR-800 核心架构完成前未开始。  
**影响**: 影响 RC 阶段（不是 Beta 阶段），需要在 RC Gate 前完成或 Deferred。

---

## 根因分析 (Issue #2682)

### 问题

v3.8.0 Beta Gate 通过了，但实际上大量功能（PR-810~PR-900）未完成。

### 根因

1. **门禁定义漏洞**: GATE_CONDITIONS.md 中 Beta Gate 只有 B1-B4 基础设施检查，没有功能追踪
2. **文档与脚本不一致**: `verify_beta_entry.sh` 只检查 B1-B4，未检查 PR-DAG 或功能清单
3. **功能追踪缺失**: 没有 FEATURE_CHECKLIST.md 或等效文档追踪功能状态

### 修复

1. GATE_CONDITIONS.md v2.0 新增 B-Functional 检查（已在本次更新）
2. `scripts/gate/check_beta_gate.sh` 新增功能追踪检查（已在本次创建）
3. `FEATURE_CHECKLIST.md` 创建，记录所有功能状态（本次创建）

---

## 关联文档

- [GATE_CONDITIONS.md](../governance/GATE_CONDITIONS.md) — 门禁条件定义
- [BETA_GATE_CONTRACT.md](./BETA_GATE_CONTRACT.md) — Beta Gate 合同
- [BETA_GATE_REPORT.md](./BETA_GATE_REPORT.md) — Beta Gate 报告
- [DEVELOPMENT_PLAN.md](./DEVELOPMENT_PLAN.md) — 开发计划（含 PR-DAG）
- [LEGACY_ISSUES.md](./LEGACY_ISSUES.md) — 历史遗留问题

---

**最后更新**: 2026-06-01  
**更新者**: Hermes C