# ADR-004: Negative Evidence

## Status

**Accepted** — v3.7.0 GA (2026-05-30)

## Context

在 v3.7.0 GA 过程中，我错误地产生了多个 Negative Evidence Case：

### Case 1: EX-v350-006 (2026-05-29)

**错误**: 用户用 `--lib only` 测得 84.98%，创建"覆盖率测量方法不一致"的 EX 条目，指控 GA_GATE_REPORT 造假。

**问题**:
- Evidence 不充分：仅用单一测量方法，未检查历史文档
- Claim 未经核实：综合方法在 RC2 commit (aa830bcd) 就已确立
- User 纠正："必须先检查历史文档再判定方法对错"

### Case 2: v3.6.0 Beta Gate False Positive (2026-05-30)

**错误**: Beta Gate Checklist 声称 7/9 PASS，但实际执行 0/8 通过。

**问题**:
- Document State ≠ Execution State
- 文档声称 PASS，但实际未执行
- 缺乏实际命令输出作为 Evidence

### Case 3: mysql-server tests2 43 errors (2026-05-30)

**错误**: 初始判断为"新增代码导致 regression"。

**问题**:
- 实际问题：行 174 和 1538 重复定义 `mod tests`
- 根因：历史遗留问题，非 v3.7.0 引入

## Decision

建立 **Negative Evidence 处理规范**：

### Negative Evidence 定义

Negative Evidence 是指：
1. **未经充分调查的 Claim**: 未检查 SSOT/历史文档就下结论
2. **不充分的 Evidence**: 仅有单一数据点，无交叉验证
3. **错误的 Evidence**: 证据与结论之间缺乏因果关系
4. **过时 Evidence**: 使用陈旧数据未标注 Freshness

### Negative Evidence 处理流程

```
Negative Evidence Detected
    ↓
Self-Correction (如果能自行发现)
    ↓
User Correction (如果无法自行发现)
    ↓
Root Cause Analysis
    ↓
Pattern Documentation
    ↓
Prevention Rule Updated
```

### Negative Evidence vs False Positive

| 属性 | Negative Evidence | False Positive |
|------|------------------|----------------|
| 来源 | Claim/Evaluation | Gate Report |
| 性质 | Evidence 不足 | Result 错误 |
| 检测 | 自检或用户纠正 | 实际执行 |
| 处理 | 更新 Pattern Library | 更新 Gate Script |

## Consequences

### Positive

- 减少 Negative Evidence 的产生
- Negative Evidence 可追溯
- 支持 Root Cause 分析
- Pattern Library 持续完善

### Negative

- Agent 需要更多时间收集 Evidence
- Self-Correction 可能不及时
- 部分 Negative Evidence 依赖 User 纠正

### Neutral

- Negative Evidence 不等于失败，是学习机会
- Pattern Library 持续更新

---

## Metadata

- **Author**: Hermes Agent
- **Date**: 2026-05-30
- **Related ADRs**: ADR-001 (Truthfulness Framework), ADR-002 (Claim Registry), ADR-005 (Legacy Gate Retirement)
- **Related PRs**: v3.7.0 GA (dd1cfdbd)
- **Supersedes**: N/A (new ADR)