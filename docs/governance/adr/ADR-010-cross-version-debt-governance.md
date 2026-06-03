# ADR-010: Cross-Version Debt Governance (G-02 follow-up)

## Status

**Accepted** — v3.8.0 GA (2026-06-03)

## Context

CROSS-VERSION-DEBT.md 框架已建（Issue #2585 已完成 2026-05-31），但**无自动化**：
- INT-1~INT-4 跨 6 版本未根治
- ARCH-1~ARCH-3 跨 3 版本未根治
- SEM-1~SEM-4 跨 3 版本未根治
- 共 11/11 debt items ACTIVE
- GA Gate 报告无 "Cross-Version Debt Delta" 章节
- 历史上累积的债务无人问津

**Issue #2773 (Task #2748) 任务**：自动化跟踪 + GA Gate Delta 章节。

## Decision

**强制执行 Cross-Version Debt 跟踪**：每个版本门禁必须包含 "Cross-Version Debt Delta" 章节。

### Decision-1: 自动化门禁

`scripts/gate/check_cross_version_debt.sh` 解析 CROSS-VERSION-DEBT.md，统计 ACTIVE/CLOSED/DEFERRED 数量，输出 delta 报告。

**检查项**：
- **CV-1**: 所有 debt item 状态合法 (ACTIVE/CLOSED/DEFERRED)
- **CV-2**: CROSS-VERSION-DEBT.md 至少 1 个 item
- **CV-3**: 没有新增跨版本债务 (delta ≤ 0)
- **CV-4**: ACTIVE debt 有 progress plan (CLOSED 或 DEFERRED 至少 1)

**Verdict**：
- `STABLE` — 没有进展但也无新债务
- `PROGRESS` — 有 CLOSED 或 DEFERRED
- `NEEDS_ATTENTION` — 11 ACTIVE / 0 progress
- `FAIL_NEW_DEBT` — delta > 0（新增跨版本债务）

### Decision-2: GA Gate 报告章节

在 `RC_GA_GATE_REPORT.md` 和 `INTEGRATION_GATE_REPORT.md` 中新增章节：

```markdown
## Cross-Version Debt Delta

| 类别 | ACTIVE | CLOSED | DEFERRED |
|------|--------|--------|----------|
| Integration | N | N | N |
| Architecture | N | N | N |
| Semantic | N | N | N |
| **Total** | **N** | **N** | **N** |

**Delta (vs v3.7.0)**: +X ACTIVE, +Y CLOSED, +Z DEFERRED

**Verdict**: <STABLE|PROGRESS|NEEDS_ATTENTION|FAIL_NEW_DEBT>
```

### Decision-3: 状态机规则

每个 debt item 必须有明确状态：
- **ACTIVE** — 未修复，需要修复计划
- **CLOSED** — 已修复
- **DEFERRED** — 推迟到具体版本

**过期规则**：
- 任何 ACTIVE 跨 3 个版本 → **自动升级为 P0**
- 任何 ACTIVE 跨 5 个版本 → **GA 阻断**

### Decision-4: 与现有规则关系

| 现有规则 | 关系 |
|----------|------|
| ADR-001 Truthfulness | CV 报告债务状态必须真实 |
| ADR-005 Legacy Gate Retirement | 旧 gate 规则不再用，CV 是新机制 |
| GATE_CONDITIONS.md | CV 是新增章节（CV-Alpha / CV-Beta / CV-RC / CV-GA）|
| LEGACY_ISSUES.md | 12 个可关闭 issue 已记录，但 CV 持续跟踪 |

## Consequences

### Positive

1. **跨版本债务可见** — 11/11 ACTIVE 状态机
2. **GA Gate 报告有 Delta** — 每版本可对比
3. **债务不可能"藏起来"** — CI 强制检查
4. **过期升级** — 长期未修复自动升级 P0

### Negative

1. **首次运行 NEEDS_ATTENTION** — 0 progress 可视化
2. **历史债务多** — 11 个 ACTIVE 需分批解决
3. **SEM table 格式不同** — §3.3 用 "规则" 列而非 "状态" 列（需特殊处理）

### Risks

- **R-1**：CV-4 (0 progress) 长期 WARN — 11 ACTIVE 债务可能需要跨版本
- **R-2**：delta 0 长期存在 — 表示没解决也没恶化
- **R-3**：债务来源未追溯 — 早期开发债务无 issue 关联

## Implementation Roadmap

| 任务 | 状态 |
|------|------|
| `check_cross_version_debt.sh` 创建 | ✅ SPEC-008 |
| GA Gate 模板更新 | ⏳ 本 PR |
| Beta/RC/GA Gate 接入 | ⏳ 后续 PR |
| 11 ACTIVE debt 修复计划 | ⏳ v3.8.0+1 / v3.9.0 |

## Current State (2026-06-03)

```
Verdict: NEEDS_ATTENTION
  ACTIVE:   11
  CLOSED:   0
  DEFERRED: 0
  Delta:    0 (vs v3.7.0)
```

## Related

- Source ISSUE: Issue #2585 (已完成的框架)
- Parent issue: #2773 (Task #2748)
- Source PR: PR-XXX (SPEC-008 execution)
- Related SPEC: SPEC-008
- Refs:
  - `docs/releases/v3.8.0/CROSS-VERSION-DEBT.md`
  - `docs/audit/V380_RECTIFICATION_PLAN_2026-06-03.md` §3.2
  - `docs/governance/adr/ADR-005-legacy-gate-retirement.md`

## Change History

| 版本 | 日期 | 作者 | 说明 |
|------|------|------|------|
| 1.0 | 2026-06-03 | claude-macmini (governance-engineer) | 初始版本：跨版本债务治理规则 |
