# ADR-003: Decision Registry

## Status

**Accepted** — v3.7.0 GA (2026-05-30)

## Context

v3.7.0 GA 过程中产生多个关键 Governance 决策：

1. **DML 路径决策**: DML 直接调用 storage 是架构违规（AV-001~AV-007），需在 v3.8.0 解决
2. **Coverage Ceiling 决策**: 78.66% 是执行模型表达力上限，非测试缺口
3. **WAL 集成决策**: WAL 集成是 INT-1，延期 v3.8.0 PR-830~PR-840
4. **ParallelVolcanoExecutor 决策**: 存在但未接入主路径，延期 v3.8.0 PR-870

这些决策缺乏统一的记录机制，导致后续版本无法追溯决策理由。

## Decision

建立 **Decision Registry**，作为关键 Governance 决策的中央记录：

### Decision Registry 结构

```yaml
decision_id: DEC-{YYYY}-{NNN}
title: "DML 不经过 WAL 是架构违规，延期 v3.8.0"
version: v3.7.0
status: [Accepted|Overturned|Superseded]
context: |
  v3.7.0 GA 审计发现 DML 直接调用 storage，跳过 TransactionManager + WAL
  导致无 crash recovery，非预期终止导致数据丢失
decision: |
  1. 将 DML 不经过 WAL 标记为 CRITICAL 架构违规 (AV-001~AV-007)
  2. 延期到 v3.8.0 PR-830~PR-840 解决
  3. v3.7.x stabilization 中不修复（影响范围太大）
alternatives_considered:
  - alternative: "在 v3.7.0 中修复"
    rejected_reason: "影响范围太大，可能引入 regression"
  - alternative: "在 v3.9.0 中修复"
    rejected_reason: "架构债务已跨越 7 个版本，不能再延期"
outcome:
  expected: DML 经过 WAL，crash recovery 可用
  actual: Pending (v3.8.0)
stakeholders: [Hermes Agent, User]
date: 2026-05-30
linked_decisions:
  - DEC-v3.7.0-002 (Coverage Ceiling 决策)
  - DEC-v3.7.0-003 (WAL 集成决策)
linked_issues:
  - #2588 (INT-1: DML 不经过 WAL)
linked_adr: ADR-003-decision-registry
```

### Decision 生命周期

```
Proposed → Accepted → [Implemented|Overturned|Superseded]
                                ↓
                          Superseded (by new Decision)
```

### Decision vs Claim

| 属性 | Decision | Claim |
|------|----------|-------|
| 性质 | 行动或方向 | 判断或结论 |
| 约束力 | 高（影响执行） | 低（仅供参考） |
| 可追溯性 | 必须记录 | 可选记录 |
| 争议解决 | 明确的裁决流程 | 依赖 Truthfulness Framework |

## Consequences

### Positive

- 决策有唯一的 ID，可追溯
- 决策理由和上下文被记录
- 支持跨版本决策对比
- 为 Replay Graph 提供决策节点

### Negative

- 决策数量可能增长较快
- 部分历史决策可能需要回溯记录
- 需要维护 Decision Registry 的工具/流程

### Neutral

- Decision Registry 不替代设计文档（ADR）
- Decision Registry 主要用于 Governance 相关的决策

---

## Metadata

- **Author**: Hermes Agent
- **Date**: 2026-05-30
- **Related ADRs**: ADR-001 (Truthfulness Framework), ADR-002 (Claim Registry), ADR-004 (Negative Evidence)
- **Related PRs**: v3.7.0 GA (dd1cfdbd)
- **Supersedes**: N/A (new ADR)