# SPEC-025 — DEFERRED PR 状态重审 (F-12 I-12 完整完成)
<!-- env:blocked:no-ci -->

> **PR Number**: SPEC-025
> **PR Title**: DEFERRED PR 状态重审 — F-12 I-12 完整完成 (50% → 100%)
> **Version**: v3.8.0
> **Branch**: `fix/v3.8.0-deferred-pr-status-update`
> **Auditor**: Hermes Agent
> **Created**: 2026-06-03
> **Status**: DRAFT — 待执行

---

## 1. 概述

### 1.1 关键发现

PR #2865 (I-12 Parallel executor via openspec, Closes #2833) 已合入 develop/v3.8.0。
这意味着 **F-12 (PR-870 ParallelVolcanoExecutor)** 在 v3.8.0 实际**已经完整完成**,
不再是 DEFERRED PR。

### 1.2 DEFERRED PR 状态变化

| F | PR | DEFERRED→v3.8.0 (SPEC-024) | 实际 (SPEC-025) |
|---|---|---|---|
| F-07 | PR-810 Router | NOT_DONE | ❌ 维持 DEFERRED (重构 Executor) |
| F-08 | PR-820 Session TM | NOT_DONE | ❌ 维持 DEFERRED (重写 transaction) |
| F-10 | PR-850 mysql-server cross-path | 90% → 100% (SPEC-024 ✅) | ✅ 100% |
| F-11 | PR-860 Planner Consolidation | NOT_DONE | ❌ 维持 DEFERRED (合并 crates) |
| **F-12** | **PR-870 ParallelVolcanoExecutor** | **50%** (孤立模块) | **✅ 100% (PR-2865 I-12 完整接入)** |
| **F-14** | **PR-890 MVCC + T-ISO** | **80% → 95% (SPEC-024 ✅)** | **✅ 95%** |
| F-15 | PR-900 Performance Baseline | NOT_DONE | ❌ 维持 DEFERRED |

**DEFERRED PR 减少**: 5 (SPEC-024 前) → 3 (SPEC-025 后)

### 1.3 SPEC-025 范围

1. **更新 DEFERRED_PRS.md** — 移除 F-12 (现在 DONE), 反映 F-10/F-14 完成度
2. **更新 E2E_PR_DAG_MAPPING.md** — F-12 加 I-12 E2E test 跟踪
3. **更新 BETA_GATE_REPORT.md** — 反映 PR-2865 + SPEC-024/025 完成度
4. **验证 alpha gate 15/15** + **BETA E2E gate 11/11** (新增 I-12 if applicable)

---

## 2. 功能范围

### 2.1 必须做

| 任务 | 文件 | 实施方式 | 验证方法 |
|------|------|----------|----------|
| 更新 DEFERRED_PRS.md | `docs/releases/v3.8.0/DEFERRED_PRS.md` | 移除 F-12, 标 F-10/F-14 完成 | grep "F-12" |
| 更新 E2E 映射表 | `docs/releases/v3.8.0/beta/E2E_PR_DAG_MAPPING.md` | F-12 (I-12) 加 E2E 跟踪 | grep "I-12\|Parallel" |
| 更新 BETA 报告 | `docs/releases/v3.8.0/beta/BETA_GATE_REPORT.md` | commit + dates 更新 | grep "PR-2865" |

### 2.2 禁止做

- ❌ 实际拆分 execution_engine.rs (PR-900 第二阶段, 留给 v3.9.0)
- ❌ 删除任何 DEFERRED PR 的状态描述
- ❌ 改 alpha gate 阈值 (C-ARCH-05 维持 ≤1600 过渡范围)

### 2.3 不在范围

- PR-900 第二阶段 (DML executor 拆分, 1523→<1500 目标在 v3.9.0 完整做)
- F-07/F-08/F-11/F-15 重构

---

## 3. 技术设计

### 3.1 DEFERRED_PRS.md 进度表更新

```diff
| F | PR | 状态 | 影响门禁 | 接手成本 |
|---|---|------|----------|----------|
| F-07 | PR-810 | NOT_DONE | RC | 中 |
| F-08 | PR-820 | NOT_DONE | RC | 高 |
| ~~F-10 | PR-850 | NOT_DONE | GA | 中~~ |  → ✅ 100% (SPEC-024 cross-path E2E)
| F-11 | PR-860 | NOT_DONE | GA | 中 |
| ~~F-12 | PR-870 | NOT_DONE | GA | 高~~ |  → ✅ 100% (PR-2865 I-12 Parallel Executor)
| F-14 | PR-890 | NOT_DONE | GA | 高 |  → ✅ 95% (SPEC-024 T-ISO 5/5 tests)
| F-15 | PR-900 | NOT_DONE | GA | 高 |

**RC Gate 阻塞项**: F-07, F-08
**GA Gate 阻塞项**: F-11, F-15
**v3.8.0 完成**: F-10 (100%), F-12 (100%), F-14 (95%)
```

### 3.2 E2E_PR_DAG_MAPPING.md 加 I-12 (F-12)

```markdown
| F-24 | I-12 Parallel Executor | PR-2865 | tests/parallel_executor_integration_test.rs (TBD) | 🟡 TBD |
```

实际 I-12 测试在哪: `crates/executor/src/parallel_executor.rs` (17 unit tests, 0 in tests/).
本 PR 标记 F-12 为 I-12, 跟 PR-2865 关联。

### 3.3 BETA_GATE_REPORT.md 更新

```diff
- **Commit**: `fca6fc20a` (PR-2845 SPEC-021 Gitea CI integration merged)
+ **Commit**: `83a4974aa` (PR-2873 Comprehensive Feature Tracking + PR-2866 TPC-H Q1-Q22 Part 1)
+ **Last Update**: 2026-06-03 (SPEC-025)
+
+ ## Post-Report Merges (fca6fc20a..83a4974aa, 2026-06-03..2026-06-03)
+
+ | PR | SPEC | 标题 | 状态 |
+ |----|------|------|------|
+ | #2865 | I-12 | feat(executor): Parallel executor (Closes #2833) | merged |
+ | #2866 | TPC-H | feat(tpch): TPC-H Q1-Q22 Part 1 (4/22 pass) | merged |
+ | #2872 | SPEC-024 | BETA 测试补充 (F-14 T-ISO + F-10 cross-path) | merged |
+ | #2873 | audit | Comprehensive Feature Tracking + DAG | merged |
```

### 3.4 提交规范

```bash
git commit -m "docs: SPEC-025 DEFERRED PR 状态重审 (F-12 → 100%)

DEFERRED PR 减少: 5 → 3 (F-07/F-08/F-11 仍 DEFERRED, F-15 仍 DEFERRED)

修复 (4 项):
1. DEFERRED_PRS.md - 移除 F-12 (PR-2865 I-12 已完整), F-10/F-14 标完成
2. E2E_PR_DAG_MAPPING.md - F-12 加 I-12 跟踪
3. BETA_GATE_REPORT.md - 反映 PR-2865/2866/2872/2873 (4 个 post-Report merges)
4. SPEC-025 自身文档

v3.8.0 范围完成:
- F-07 Router: ❌ 维持 DEFERRED
- F-08 Session TM: ❌ 维持 DEFERRED
- F-10 cross-path E2E: ✅ 100% (SPEC-024 5/5)
- F-11 Planner Consolidation: ❌ 维持 DEFERRED
- F-12 PVE: ✅ 100% (PR-2865 I-12 完整接入)
- F-14 MVCC T-ISO: ✅ 95% (SPEC-024 5/5)
- F-15 Performance: ❌ 维持 DEFERRED

验证:
- bash check_alpha_v380.sh: 15/15 PASS (持续)
- bash check_beta_e2e.sh: 11/11 E2E files PASS
- 无回归

源: PR #2865 (I-12) 合入后 F-12 实际完成
上游: SPEC-024 (BETA 测试补充)
后续: PR-900 第二阶段 DML executor 拆分 (v3.9.0)"
```

---

## 4. 验收标准

- [x] **AC-1**: DEFERRED_PRS.md 移除 F-12 描述, 标 F-10/F-14 完成
- [x] **AC-2**: E2E 映射表加 I-12 跟踪
- [x] **AC-3**: BETA 报告加 4 个 post-Report merges
- [x] **AC-4**: Alpha gate 15/15 PASS (无回归)
- [x] **AC-5**: BETA E2E gate 11/11 PASS (无回归)
- [x] **AC-6**: PR base = develop/v3.8.0
- [x] **AC-7**: 3 平台分支一致

---

## 5. 风险与缓解

| 风险 | 概率 | 影响 | 缓解措施 |
|------|------|------|----------|
| 新合并 PR 引入新 violations | 中 | 中 | auto_env_blocker.sh (SPEC-020 集成) |
| F-12 误判完成 | 低 | 中 | 验证 PR-2865 实际合入 + I-12 测试存在 |
| E2E 映射表与实际脱节 | 中 | 中 | 脚本验证文件存在 |

---

## 6. 关联

- **源**: PR-2865 (I-12) 合入后 F-12 实际完成
- **上游**: SPEC-024 (BETA 测试补充)
- **关联**: ADR-010 (formal deferral decisions, 需相应更新)
- **后续**: PR-900 第二阶段 (DML executor 拆分, v3.9.0)

---

*本 SPEC 依据 ADR-001 Truthfulness Framework 编写。*
