# ADR-011: Cross-Version Debt State Machine

> **Status**: PROPOSED
> **Date**: 2026-06-05
> **Author**: Hermes Agent
> **Source**: GA 治理示范 v3.8.0 评审建议 (2026-06-05)
> **Supersedes**: 隐式状态分散在 `INT5_PLUS_DEBT_INVENTORY.md`, `CROSS-VERSION-DEBT.md`, `ARCH_SEM_DEBT_REMEDIATION_PLAN.md` 等多处
> **Cross-references**: ADR-001 (Truthfulness), ADR-010 (Cross-Version Debt Governance), `GA_GOVERNANCE_DEMO_v3.8.0.md`, `PATTERN_FOLLOWUP_SLA.md`

---

## Context

v3.8.0 GA 治理审计发现债务状态命名混乱（OPEN / PARTIAL / ACTIVE / CLOSED / DEFERRED / PASS-WITH-DRIFT / STALE）混用，导致：

1. **Issue / PR / Gate 报告状态不一致** — Issue 标 CLOSED 但 PR 标 SUPERSEDED
2. **PARTIAL 缺乏具体进度信息** — INT-3 实际 1/15 branches 委托，无量化
3. **PASS-WITH-DRIFT 不是契约** — 只是描述
4. **Follow-up 缺乏 SLA 字段** — owner / target_release / review_date / blocking 缺失

本 ADR 统一 7 状态 + 强制 SLA 字段。

---

## Decision

### 7-State Machine

| 状态 | 含义 | 进入条件 | 退出条件 | 必备字段 |
|------|------|----------|----------|----------|
| **OPEN** | 债务发现登记，未开始 | 审计发现或 Issue 创建 | 分配 owner 进入 IN_PROGRESS | id, title, source, discovered_at |
| **IN_PROGRESS** | 已分配 owner，正在修复 | 分配 owner + target_release | 进入 VERIFIED 或 BLOCKED | + owner, target_release |
| **BLOCKED** | 修复中遇到外部阻塞 | 阻塞描述 | 阻塞解除回到 IN_PROGRESS | + blocked_reason, blocking[] |
| **VERIFIED** | 修复完成 + 门禁验证通过 | 跑门禁脚本 + 测试 PASS | 文档同步后 CLOSED | + verification_evidence, verified_at |
| **CLOSED** | 债务已解决，PR 合并 | VERIFIED + PR 合并 + Issue 关闭 | (终态) | + closing_pr, closed_at |
| **SUPERSEDED** | 被其他修复取代 | 标记替代源 | (终态) | + superseded_by, reason |
| **REJECTED** | 决定不修复（设计决策） | 评审 + 决议 | (终态) | + rejection_reason, decision_evidence |

### 状态转移图

```
    ┌──────┐
    │ OPEN │
    └──┬───┘
       ↓ (分配 owner)
    ┌──────────────┐
    │ IN_PROGRESS  │────┐
    └──┬──────┬────┘    │ (阻塞)
       │      │         ↓
       │      │   ┌─────────┐
       │      │   │ BLOCKED │────┐
       │      │   └─────────┘    │
       │      │   (解除阻塞回 IN_PROGRESS)
       │      ↓
       │   ┌──────────┐
       │   │VERIFIED  │────┐
       │   └──────────┘    │ (发现新问题回 IN_PROGRESS)
       ↓ (PR 合并 + Issue 关闭)
    ┌───────┐
    │CLOSED│
    └───────┘

    OPEN ──→ SUPERSEDED (被其他修复取代)
    OPEN ──→ REJECTED   (设计决策不修复)
```

### 强制 SLA 字段 (每个非 OPEN 状态)

```yaml
debt_id: INT-2
state: IN_PROGRESS
owner: openclaw  # 个人或团队
target_release: v3.9.0
review_date: 2026-09-01  # 季度审查
blocking:  # 阻塞此债务修复的依赖
  - MemoryStorage::in_transaction (需先修)
  - INT-3 完整合并 (1 周)
```

### 进度量化（仅 IN_PROGRESS / VERIFIED）

PARTIAL CLOSED 状态必须附进度量化（避免"PR 声称 50% 完成"）：

```yaml
debt_id: INT-3
state: VERIFIED
progress: 1/15  # 1/15 branches 完成委托
progress_metric: "FunctionCall 分支委托完成 (1/15)"
```

---

## 应用示例

### 示例 1: INT-3 (本次报告关键)

```yaml
debt_id: INT-3
title: "expr 双实现合并 (src/expr_utils.rs vs crates/executor/src/expr/)"
state: IN_PROGRESS  # PARTIAL CLOSED 升级为 IN_PROGRESS
owner: openclaw
target_release: v3.9.0
review_date: 2026-09-01
blocking:
  - MemoryStorage::in_transaction (跨 INT-2)
progress: 1/15  # FunctionCall 分支已委托
progress_metric: "FunctionCall 分支委托 (eval_fn via dispatch_fn)"
verification_evidence: |
  PR-3019 修复 DML TX (相关但未涉及 expr)
  2026-06-05 调查: src/expr_utils.rs::evaluate_expression 14/15 分支自实现
related_followup: 3146
related_issue: 3108
```

### 示例 2: INT-1 (本次报告 P0)

```yaml
debt_id: INT-1
title: "DML 不经过 WAL/TransactionManager"
state: CLOSED
owner: openclaw
target_release: v3.8.0  # 已集成
closed_at: 2026-06-04
closing_pr: [3019, 3050]
verification_evidence: |
  PR-3019 修复 DML → TM.begin/commit 路径
  PR-3050 修复 commit_transaction/rollback_transaction tx_status reset
  6/6 INT-1 fix tests PASS
  docs/releases/v3.8.0/ga/GA_GATE_CHECKLIST.md §7.5 状态同步
```

### 示例 3: ARCH-3 (本次报告 ARCH-3)

```yaml
debt_id: ARCH-3
title: "VTU 主路径集成"
state: BLOCKED
owner: openclaw
target_release: v3.9.0
review_date: 2026-09-01
blocked_reason: "MemoryStorage::in_transaction 永远 false 触发 panic"
blocking:
  - INT-2 主路径集成 (依赖此)
  - MemoryStorage::in_transaction stub
related_followup: 3129  # 修复路径
```

---

## 与现有治理的整合

### 替代现有状态命名

| 旧命名 (隐式) | 新命名 (本 ADR) |
|--------------|------------------|
| ❌ 未实现 | OPEN |
| ⚠️ 部分 | IN_PROGRESS (必须附 progress_metric) |
| ❌ OPEN (INT 上下文) | ACTIVE 改为 IN_PROGRESS |
| ✅ CLOSED | CLOSED |
| ❌ DEFERRED | BLOCKED (附 blocked_reason) |
| PARTIAL CLOSED with progress | IN_PROGRESS + progress_metric |
| PARTIAL 实际 CLOSED 但声称 100% | (禁止: ADR-001 违反) |
| PASS-WITH-DRIFT | 不在本 ADR 范围 (gate 概念) |

### 替代现有文档

| 旧文档 | 新文档 |
|--------|--------|
| `INT5_PLUS_DEBT_INVENTORY.md` (Markdown 表格) | `debt/debt-registry.yaml` (机器可读) + 派生 Markdown 表格 |
| `CROSS-VERSION-DEBT.md` | (并入 debt-registry.yaml) |
| `ARCH_SEM_DEBT_REMEDIATION_PLAN.md` | (并入 debt-registry.yaml) |
| 各 Issue body 中的状态描述 | debt-registry.yaml + Issue 评论 (SLA 字段) |

### 与 ADR-010 (Cross-Version Debt) 整合

ADR-010 决策 3: "ACTIVE 跨 3 个版本 → P0 升级, 跨 5 版本 → GA 阻断"

新规则（按本 ADR）:
- 跨 3 版本: state = IN_PROGRESS + review_date ≤ 3 月内
- 跨 5 版本: state = BLOCKED (或自动升级 P0 阻断 GA)

### 门禁契约 (D7/D8 改进)

按评审建议，强化 PASS-WITH-DRIFT 定义:

```bash
# scripts/gate/check_int_debt.sh 改进
if [ $IN_PROGRESS_COUNT -gt 3 ]; then
    echo "❌ FAIL: >3 IN_PROGRESS without target_release"
fi
for id in "${IN_PROGRESS_IDS[@]}"; do
    if ! has_sla "$id" owner target_release; then
        echo "❌ FAIL: $id missing SLA fields"
    fi
done
```

---

## v3.9.0 实施计划

### 阶段 1: 立即 (本次)
- ✅ 本 ADR 通过 (本次会话)
- ✅ debt-registry.yaml 创建 (本次会话)
- ✅ 5 follow-up Issue 加 SLA 字段 (本次会话)

### 阶段 2: v3.9.0 RC
- 重写 `INT5_PLUS_DEBT_INVENTORY.md` 等为 debt-registry.yaml 派生
- 门禁脚本读 debt-registry.yaml 验证 SLA 完整
- 报告 / Gate 全部用 debt-registry.yaml 生成

### 阶段 3: v3.9.0 GA
- 全量债务 (含跨版本) 迁移到 debt-registry.yaml
- 8 维门禁读 debt-registry.yaml 自动判定 PASS / PASS-WITH-DRIFT / FAIL

---

## 风险与回退

| 风险 | 缓解 |
|------|------|
| 状态命名迁移混乱（多文档不一致）| 阶段 1 不删除旧文档，只新增 debt-registry.yaml；阶段 2 统一 |
| debt-registry.yaml 维护成本 | 用 Gitea API 自动同步 Issue 状态（v3.9.0+） |
| 字段定义过严 | v3.9.0 RC 后回顾 + 简化 |
| 跨版本债务 owner 模糊 | 强制 owner (个人或 team) 字段；缺 owner = OPEN (不分配) |

---

## 维护信息

| 项目 | 值 |
|------|-----|
| 文档版本 | ADR-011-1.0 |
| 最后更新 | 2026-06-05 |
| 维护者 | Hermes Agent |
| 状态 | PROPOSED → 阶段 1 立即 |
| 关联 | ADR-001, ADR-010, `GA_GOVERNANCE_DEMO_v3.8.0.md`, `PATTERN_FOLLOWUP_SLA.md` |
