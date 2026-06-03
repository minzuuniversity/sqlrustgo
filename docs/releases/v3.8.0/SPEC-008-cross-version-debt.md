# SPEC-008 — Cross-Version Debt Automation (G-02 follow-up)

<!-- env:blocked:no-ci -->

> **PR Number**: SPEC-008 (governance follow-up)
> **PR Title**: Cross-Version Debt tracking automation + GA Gate Delta section
> **Version**: v3.8.0
> **Branch**: `feature/spec-008-cross-version-debt` (从 `a2d9a7ccb` 切出)
> **Auditor**: Claude (claude-macmini, governance-engineer)
> **Created**: 2026-06-03
> **Status**: DRAFT — 待执行

---

## 1. 概述

### 1.1 问题

CROSS-VERSION-DEBT.md 框架已建（Issue #2585 已完成），但**无自动化**：
- INT-1~INT-4 跨 6 版本未根治，靠人工跟踪
- GA Gate 报告无 "Cross-Version Debt Delta" 章节
- 历史上累积的债务无人问津

### 1.2 调研发现（2026-06-03 develop/v3.8.0 @ a2d9a7ccb）

#### Finding-1: 现有债务分类（来自 CROSS-VERSION-DEBT.md §3）

| 类别 | 数量 | IDs |
|------|------|-----|
| Integration Debt | 4 | INT-1, INT-2, INT-3, INT-4 |
| Architecture Debt | 3 | ARCH-1, ARCH-2, ARCH-3 |
| Semantic Debt | 4 | SEM-1, SEM-2, SEM-3, SEM-4 |
| **总计** | **11** | |

#### Finding-2: 状态机字段

每个 debt item 状态：
- `ACTIVE` — 未修复
- `CLOSED` — 已修复
- `DEFERRED` — 推迟到具体版本

#### Finding-3: 状态分布

| 类别 | ACTIVE | CLOSED | DEFERRED |
|------|--------|--------|----------|
| Integration | 4 | 0 | 0 |
| Architecture | 3 | 0 | 0 |
| Semantic | 4 | 0 | 0 |
| **总计** | **11** | **0** | **0** |

**关键洞察**：11/11 debt items 全部 ACTIVE，零进展。

## 2. 设计

### 2.1 Gate Script 设计

`scripts/gate/check_cross_version_debt.sh`：

**功能**：
1. 解析 `docs/releases/v3.8.0/CROSS-VERSION-DEBT.md` 中所有 debt items
2. 统计 ACTIVE / CLOSED / DEFERRED 数量
3. 与上一个版本（v3.7.0）的 debt delta 对比
4. 输出 Cross-Version Debt Delta section (JSON + text)

**检查项**：
- **CV-Alpha** (v3.8.0):
  - [ ] 每个 ACTIVE debt 有 plan（CLOSED 或 DEFERRED）
  - [ ] 没有"新增"debt 被无记录引入
- **CV-Beta**:
  - [ ] v3.7.0 ACTIVE 债务中至少 1 个转为 CLOSED 或 DEFERRED
- **CV-RC/GA**:
  - [ ] 没有新增跨版本债务

### 2.2 GA Gate 报告新增章节

在 `RC_GA_GATE_REPORT.md` 和 `INTEGRATION_GATE_REPORT.md` 中新增：

```markdown
## Cross-Version Debt Delta

| 类别 | v3.7.0 | v3.8.0 | Delta |
|------|--------|--------|-------|
| Integration ACTIVE | 4 | 4 | 0 |
| Architecture ACTIVE | 3 | 3 | 0 |
| Semantic ACTIVE | 4 | 4 | 0 |
| **Total ACTIVE** | **11** | **11** | **0** |
| Total CLOSED (新增) | 0 | 0 | 0 |
| Total DEFERRED | 0 | 0 | 0 |

**Verdict**: ⚠️ NEEDS_ATTENTION — 11 ACTIVE debt, 0 CLOSED this release
```

### 2.3 接入 Gate

修改 `scripts/gate/check_rc_ga_gate.sh`，在末尾追加：
```bash
bash scripts/gate/check_cross_version_debt.sh || exit 1
```

## 3. 实施

3.1 编写 `check_cross_version_debt.sh` (200 行 bash)
3.2 编写 ADR-010 (跨版本债务治理规则)
3.3 更新 `RC_GA_GATE_REPORT.md` 模板加 Cross-Version Debt Delta 章节
3.4 本地 dry-run + 生成 debt_evidence.json

## 4. 完成标准

- [ ] `scripts/gate/check_cross_version_debt.sh` 创建
- [ ] 解析 CROSS-VERSION-DEBT.md 中 11 个 debt items
- [ ] 输出 debt_evidence.json
- [ ] 接入 `check_rc_ga_gate.sh`
- [ ] ADR-010 创建
- [ ] GA Gate 报告模板更新
- [ ] PR 合并 + Issue #2773 关闭

## 5. Refs

- CROSS-VERSION-DEBT.md (Issue #2585)
- Issue #2773 (Task #2748)
- ADR-005 (legacy gate retirement)
- 父报告: docs/audit/V380_RECTIFICATION_PLAN_2026-06-03.md §3.2
