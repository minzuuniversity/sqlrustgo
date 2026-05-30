# ADR-002: Claim Registry

## Status

**Accepted** — v3.7.0 GA (2026-05-30)

## Context

v3.7.0 GA 过程中发现多个 Claim 争议案例：

1. **Issue #2580**: 用户声称"Parser coverage 47% 是结构性缺陷"
   - 初始判断：未实测就下结论
   - 纠正后：历史 RC2 commit (aa830bcd) 已确立综合方法

2. **Issue #2582**: 用户声称"Executor coverage 需要从 72% 提升到 85%"
   - 初始判断：85% 目标不现实
   - 纠正后：78.66% 是执行模型表达力上限

3. **EX-v350-006**: 用户创建"覆盖率测量方法不一致"
   - 错误指控：GA_GATE_REPORT 造假
   - 纠正后：综合方法在 RC2 就已确立

问题的根因是：Claim 缺乏统一的记录、追踪和验证机制。

## Decision

建立 **Claim Registry**，作为 Claim 的中央记录系统：

### Claim Registry 结构

每条 Claim 记录包含：

```yaml
claim_id: CLAIM-{YYYY}-{NNN}
title: "Parser coverage structural deficiency"
version: v3.7.0
status: [Active|Resolved|Retracted|Superseded]
claimant: [User|Hermes Agent|System]
created: YYYY-MM-DD
evidence:
  - type: [实测|SSOT引用|历史文档]
    content: "..."
    source: "..."
verdict:
  - by: Hermes Agent
    date: YYYY-MM-DD
    decision: [Accepted|Rejected|Retracted]
    reasoning: "..."
linked_issues:
  - #2580
  - #2582
linked_adr: ADR-001-truthfulness-framework
```

### Claim 生命周期

```
Created → Active → Resolved/Rejected/Retracted
                        ↓
                  Superseded (by new Claim)
```

### Claim 争议解决流程

1. **Claim Created**: 记录 Claim 内容和 Evidence
2. **Investigation**: 验证 Claim 的 Evidence 是否充分
3. **Verdict**: Hermes Agent 给出判断
4. **Linked Issue**: 将 Claim 关联到相关 Issue
5. **ADR Update**: 必要时更新 ADR

## Consequences

### Positive

- Claim 有唯一的 ID，可追溯
- Claim 争议有明确的解决流程
- 支持跨版本 Claim 对比
- 为 Replay Graph 提供输入数据

### Negative

- 需要维护 Claim Registry 的工具/流程
- Claim 数量可能增长较快
- 部分历史 Claim 可能需要回溯记录

### Neutral

- Claim Registry 不替代 Issue Tracker，两者互补
- Claim Registry 主要用于 Governance 相关的 Claim

---

## Metadata

- **Author**: Hermes Agent
- **Date**: 2026-05-30
- **Related ADRs**: ADR-001 (Truthfulness Framework), ADR-003 (Decision Registry), ADR-004 (Negative Evidence)
- **Related PRs**: v3.7.0 GA (dd1cfdbd)
- **Supersedes**: N/A (new ADR)