# ADR-014: Multi-AI Coordination
> **Status**: ACCEPTED
> **Deciders**: Hermes Agent (Z6G4) + User (李哥)
> **Date**: 2026-06-26
> **Branch scope**: All branches (especially `develop/v3.9.0`, `develop/v3.10.0`)
> **Supersedes**: AI_COLLABORATION.md §1.1 (单 AI 模型)
> **Related**: [ADR-001 G-08/G-09/G-10](ADR-001-truthfulness-framework.md), [AI_COLLABORATION.md §5.5](../AI_COLLABORATION.md)
---
## Context
### 问题
在 v3.9.0 RC7 阶段，多个 AI Agent（Hermes-Z6G4, Claude-Code, opencode 等）同时在仓库中工作，产生了以下冲突：
1. **Gate 状态不一致**：不同 Agent 对 P11-P16 状态给出不同结论（有人测 FAIL，有人测 PASS）
2. **Issue 归属混乱**：多个 Agent 在同一 Issue 下评论，无统一署名格式
3. **Branch 冲突**：多个 Agent 并发 push 到同一分支
4. **Claim 传播**：Agent A 引用 Agent B 的结论，层层传播导致 AFP Type C 违规

### 根因
原 AI_COLLABORATION.md 缺乏多 AI 协作的具体规则，主要聚焦于"单 AI + Human" 模式。
---
## Decision
### D-01: Issue 署名强制
所有 Agent 在 Issue 下的评论必须带 `[agent: {agent_id}]` 前缀。
格式：
```
[agent: {agent_id}] {动作}结果 — YYYY-MM-DD HH:MM UTC
Evidence: {commit_hash} / {gate_output}
```
无前缀的评论视为 User（人类）发言。

### D-02: Worktree 隔离
每个 AI Agent 在独立 worktree 中工作：
- Worktree 路径：`.worktrees/{agent_id}-{date}`
- 避免并发修改同一工作目录
- 由 `using-git-worktrees` skill 管理

### D-03: Branch 命名规范
| Agent | Branch 前缀 | 示例 |
|-------|------------|------|
| hermes-z6g4 | `hermes/` | `hermes/fix-p12-ignore-registry` |
| claude-code | `claude/` | `claude/doc-audit-2026-06-26` |
| opencode | `opencode/` | `opencode/feature-xyz` |

### D-04: Issue 认领
第一个在 Issue 下评论"认领"的 Agent 获得归属权。格式：
```
[agent: {agent_id}] 认领 issue #{number}
```
其他 Agent 在认领 Issue 下工作时应先协调（IRC DM 或评论）。

### D-05: Gate FAIL 推送禁止
当任意 Hard Gate（P11-P16）处于 FAIL 状态时：
- 禁止 push 到 252 Gitea `develop/v3.9.0`
- 违反标记为 `[AFP-VIOLATION: Type-B]`
- 先修复 FAIL 项，再 push

### D-06: Cross-Reference Claim 链
当 Agent A 引用 Agent B 的 Claim 时：
- Agent B 的 Claim 必须有独立 Evidence
- Agent A 引用时必须注明：source agent + timestamp + evidence_hash
- 引用链每多一跳，置信度降一级
---
## Consequences
### Positive
- 多 Agent 协作时状态可追溯
- Gate 状态结论统一、可验证
- 避免并发修改导致的冲突
- 支持跨 Agent Issue 归属判断

### Negative
- 需要协调工具（IRC DM）增加沟通成本
- Worktree 管理增加操作复杂度
- Branch 命名规范需要所有 Agent 遵守

### Neutral
- 不改变单 AI + Human 模式的现有流程
- 仅在多 AI 协作场景激活
---
## Metadata
- **Author**: Claude Code (agent) + Hermes Agent (review)
- **Date**: 2026-06-26
- **Related ADRs**: ADR-001 (Truthfulness), ADR-008 (Test Claim Transparency)
- **Related PRs**: #3600 (multi-AI verification issue)
- **Supersedes**: AI_COLLABORATION.md §1.1