# AI Agent 任务认领协议

> **版本**: v1.0
> **日期**: 2026-06-03
> **适用范围**: SQLRustGo v3.8.0 整改任务
> **关联报告**: `V380_RECTIFICATION_PLAN_2026-06-03.md`

---

## 一、协议目的

建立 AI Agent 在 SQLRustGo v3.8.0 整改任务中的**标准化认领、推进、交付**流程，避免任务重复执行、所有权混乱、状态失同步。

---

## 二、任务池

当前 v3.8.0 整改包含 **9 个 Gitea ISSUEs**（#2744~#2752）：

| Task # | 标题 | 优先级 | 建议 Agent 角色 | 依赖 |
|--------|------|--------|----------------|------|
| #2744 | ADR-007 WAL 架构澄清 | 🔴 P0 | architect | 无 |
| #2745 | F-06 TransactionalFacade STUB | 🟡 P1 | backend-engineer | #2744 |
| #2746 | F-09 PR-840 完整 DML 截获 | 🟡 P1 | storage-engineer | #2744 |
| #2747 | G-01 验证链强制门禁 | 🟡 P1 | governance-engineer | 无 |
| #2748 | Cross-Version Debt 自动化 | 🟡 P1 | governance-engineer | 无 |
| #2749 | F-07~F-15 Ghost PR 决策 | 🟢 P2 | release-manager | #2744 |
| #2750 | ISSUE-2740 Crash Recovery 实证 | 🔴 P0 | qa-lead / recovery-engineer | 无 |
| #2751 | ISSUE-2743 v3.8.0+1 计划 | 🟢 P2 | architect + qa-lead | #2744 |
| #2752 | 文档同步（CURRENT_VERSION.md / ROADMAP.md） | 🟢 P2 | doc-writer | 无 |

---

## 三、认领流程

### 步骤 1：扫描任务池

AI Agent 在 session 开始时检查 Gitea Issues 标签为 `ai-task`、`v3.8.0`、对应 priority 的 open issues。

### 步骤 2：匹配 Agent 角色

每个 Agent 应有明确定义的角色（如 `architect`、`qa-lead`），匹配任务的"建议 Agent 角色"。

**不允许**：
- 同一 Agent 角色同时认领 > 3 个 P0 任务
- Agent 认领超出自身权限（如 backend-engineer 改 governance 脚本）

### 步骤 3：声明认领

在对应 Gitea Issue 添加 comment（**不是修改 body**）：

```markdown
<!-- claim-start -->
**Agent**: claude-macmini (role: architect)
**Session ID**: <session-id>
**认领时间**: 2026-06-03 14:30 UTC
**预计完成**: 2026-06-04
**工作分支**: feature/task-2744-wal-architecture-adr
<!-- claim-end -->
```

### 步骤 4：创建工作分支

```bash
git fetch origin
git worktree add .worktrees/task-2744 -b feature/task-2744-wal-architecture-adr
cd .worktrees/task-2744
```

### 步骤 5：执行任务

按任务的"完成标准"清单逐项执行。

### 步骤 6：提交 + PR

```bash
git add -A
git commit -m "fix(adr): add ADR-007 WAL architecture clarification

Resolves #2744

Decisions:
- DDL goes through WAL
- is_wal_enabled() exposed as public API
- DDL bypass: reject in production, allow in migration scripts
- Readonly mode: returns Err for any DML

Refs: ISSUE-2742"

git push -u origin feature/task-2744-wal-architecture-adr
```

通过 Gitea API 创建 PR：

```bash
curl -X POST "$GITEA_URL/api/v1/repos/openclaw/sqlrustgo/pulls" \
  -H "Authorization: token $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "head": "feature/task-2744-wal-architecture-adr",
    "base": "develop/v3.8.0",
    "title": "docs(adr): ADR-007 WAL architecture clarification (#2744)",
    "body": "..."
  }'
```

### 步骤 7：关闭 Issue

**仅在 PR 合并后**才能关闭 Issue。

```bash
curl -X PATCH "$GITEA_URL/api/v1/repos/openclaw/sqlrustgo/issues/2744" \
  -H "Authorization: token $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"state": "closed"}'
```

**禁止手动关闭**（参考 `docs/governance/ISSUE_CLOSING_VERIFICATION.md`）。

---

## 四、并发处理规则

### 4.1 锁机制

当 Agent A 认领 Task X 后，Task X 在 Gitea 上添加 label `claimed-by-claude-macmini`。

其他 Agent **必须**：
- 检查 task 的 `claimed-by` label
- 等待 24 小时无响应后方可"抢占"（添加 comment 通知原 Agent）

### 4.2 抢占流程

```markdown
<!-- claim-overwrite -->
@<原 Agent> 您的认领已超过 24 小时无更新，请确认是否继续。
如不继续，我将在 24 小时后接管。
**新 Agent**: <新 Agent>
**新 Session ID**: <新 Session ID>
<!-- claim-overwrite-end -->
```

### 4.3 协同任务

对于 #2751 (v3.8.0+1 计划) 等需要多 Agent 协同的任务：

1. 主 Agent 认领并创建框架
2. 子任务在 comment 中拆解
3. 子 Agent 各自认领子任务
4. 主 Agent 整合

---

## 五、状态字段

每个 Issue body 维护一个**状态机**（由认领 Agent 更新）：

```markdown
## Status

- [x] Task claimed by <agent>
- [x] Working branch created: <branch>
- [ ] Implementation complete
- [ ] Tests pass
- [ ] PR opened (#<pr>)
- [ ] PR merged
- [ ] Issue closed
```

**禁止跳过任何状态**。每个状态变更在 comment 中记录时间和原因。

---

## 六、Truthfulness 原则

参考 `docs/governance/DOC_CHECK_CORRECTION_RULES.md` 的 Truthfulness 原则：

- **禁止**伪造测试通过声明
- **禁止**伪造 cargo test 输出
- **禁止**虚构证据 commit SHA
- **必须**链接到真实 `git log`、`cargo test` 输出
- **必须**保留原始失败记录（不能删除）

---

## 七、失败处理

### 7.1 任务失败

如 Agent 无法完成认领的任务：

```markdown
<!-- task-failed -->
**Agent**: <agent>
**失败原因**: <具体原因，如 "依赖的 PR #2755 被 revert">
**尝试方案**: <已尝试的方法>
**建议**: <推荐后续 Agent 的处理方式>
<!-- task-failed-end -->
```

然后**重新打开**（不是关闭）Issue，等待其他 Agent。

### 7.2 与其他 PR 冲突

如发现任务范围已在他处完成：

```markdown
<!-- task-superseded -->
**已由 PR #<X> 完成**（commit <sha>）
**关闭原因**: 重复工作
<!-- task-superseded-end -->
```

关闭 Issue 时引用新 PR。

---

## 八、报告与审计

### 8.1 每日报告

每个 Agent 在当日工作结束时，在最新认领的 Issue comment：

```markdown
<!-- daily-report-2026-06-03 -->
**进度**: 60% (3/5 决策完成)
**今日完成**: ADR-007 4 个决策中的 2 个
**遇到问题**: 需要 DDL WAL 设计的额外调研
**明日计划**: 完成 ③ ④ 决策
<!-- daily-report-end -->
```

### 8.2 周报

每个周五在 v3.8.0 整改主 Issue（#2753）汇总：

- 已完成 Tasks
- 进行中 Tasks
- 阻塞 Tasks
- 新发现问题

---

## 九、关键文件路径速查

| 内容 | 路径 |
|------|------|
| 整改主报告 | `docs/audit/V380_RECTIFICATION_PLAN_2026-06-03.md` |
| 整改 ISSUEs | Gitea #2744 ~ #2752 |
| ADR 目录 | `docs/governance/adr/` |
| Issue 目录 | `docs/audit/issues/` |
| Gate 脚本 | `scripts/gate/` |
| 测试目录 | `tests/` |
| Worktree 目录 | `.worktrees/` |

---

## 十、变更历史

| 版本 | 日期 | 作者 | 说明 |
|------|------|------|------|
| v1.0 | 2026-06-03 | Claude (claude-macmini) | 初始版本：v3.8.0 整改任务 AI Agent 认领协议 |
