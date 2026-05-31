# ADR-005: Legacy Gate Retirement

## Status

**Accepted** — v3.7.0 GA (2026-05-30)

## Context

v3.7.0 GA 过程中产生大量 Legacy Issues：

- **跨版本债务**: INT-1~INT-4 跨越 v1.2.0~v3.7.0（7 个版本）
- **历史架构违规**: AV-001~AV-010 首次记录于 v3.7.0，但实际存在多年
- **覆盖率债务**: Parser 47% 和 Executor 72% 首次记录于 v3.5.0

这些 Legacy Issues 的处理规则不明确：
- Issue 可以在 GA 后关闭吗？
- Issue 关闭需要什么 Evidence？
- Legacy Issue 如何追踪跨版本债务？

## Decision

建立 **Legacy Gate Retirement** 规范：

### Legacy Issue 定义

Legacy Issue 满足以下任一条件：
1. **跨版本**: 存在超过 3 个版本未解决
2. **架构债务**: 涉及核心架构变更，非简单修复
3. **影响范围**: 影响多个子系统
4. **根因复杂**: 需要多 PR 才能解决

### Legacy Issue 标记

所有 Legacy Issue 必须使用 `LEGACY` 标记：

```markdown
# Issue Title
> Status: LEGACY
> First Appeared: v1.2.0
> Affected Versions: v1.2.0 ~ v3.7.0 (7 versions)
> Root Cause: DML 直接调用 storage，跳过 TransactionManager + WAL
> Planned Resolution: v3.8.0 PR-830~PR-840
```

### Legacy Issue 关闭流程

```
Legacy Issue Created
    ↓
Planned Resolution Version Assigned
    ↓
Resolution PRs Merged
    ↓
Integration Test Passed
    ↓
Gate Retired (Legacy Issue 正式关闭)
```

### Legacy vs Normal Issue

| 属性 | Legacy Issue | Normal Issue |
|------|-------------|-------------|
| 标记 | `LEGACY` | `P0/P1/P2` |
| 关闭证据 | Integration Test + Gate | PR merged + Test |
| 追踪周期 | 跨版本 | 单版本 |
| GA 后处理 | 允许在 GA 后继续 | 应在 GA 前解决 |

## Consequences

### Positive

- Legacy Issue 有明确的处理流程
- 跨版本债务被显式追踪
- GA 后 Legacy Issue 继续处理不违规
- 支持 Replay Graph 追踪

### Negative

- Legacy Issue 数量可能增长较快
- 需要维护 Legacy Issue 列表
- 部分历史 Issue 可能需要回溯标记

### Neutral

- Legacy Issue 不等于"未完成"，是"延期处理"
- GA 后继续处理 Legacy Issue 是预期行为

---

## Metadata

- **Author**: Hermes Agent
- **Date**: 2026-05-30
- **Related ADRs**: ADR-001 (Truthfulness Framework), ADR-002 (Claim Registry), ADR-003 (Decision Registry)
- **Related PRs**: v3.7.0 GA (dd1cfdbd)
- **Supersedes**: N/A (new ADR)