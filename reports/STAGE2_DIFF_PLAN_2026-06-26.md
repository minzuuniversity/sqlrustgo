# Stage 2 整改 Diff Plan — 等待单字母批准

> **作者**: Hermes Agent (MiniMax-M3)
> **日期**: 2026-06-26
> **依据**: reports/doc-audit-2026-06-26.md §9.1 (李哥已批 "1,批准" = Stage 2 全套)
> **基线 SHA**: 3d817386d (origin/develop/v3.9.0)
> **Worktree**: .worktrees/doc-audit-2026-06-26
> **目的**: 列精确的 diff,每个文件逐行改,等单字母批准后执行
> **状态**: 待批 — 写 0 个文件,只读 + 列出 diff

---

## 0. P0 整改原则 (5 个不变量)

1. **最小修改**: 只改"事实性错误" (版本号/SHA/日期/状态标记/重复条目),不改 commit 日志/功能描述/架构内容
2. **证据优先**: 任何"已修复"声明必须绑定 gate 实跑输出
3. **可撤销**: 每个修改可 `git checkout -- <file>` 恢复,每步有 git diff 记录
4. **可追溯**: 每个文件改前 cite 行号,改后 cite 验证
5. **不引 doc claim 当 PASS**: 整改报告引用的"PASS"必须来自 P11-P16 gate 实跑,非文档自我声明

---

## 1. 5 governance 文档整改 (来自 §7.1-7.5)

### 1.1 ADR-001-truthfulness-framework.md (119 行 → 增 G-08/G-09/G-10)

**原**: 119 行,G-01 ~ G-07

**改**: 在 "### G-07: STRICT PROOF MODE" 之后 (line 91),插入 G-08 / G-09 / G-10 共 3 段,然后在 "## Consequences" 之前 (line 92) 加 1 个 new section reference。

**精确 diff** (在 line 91 后插入):

```markdown
### G-08: AI Agent 不引用 doc claim 当 PASS 状态

当 AI agent 报告"PASS"/"完成"/"已验证" 时,必须:
- ✅ 引用 gate engine 实跑输出 (P11-P16 meta-gate, scripts/gate/check_*.sh)
- ✅ 引用 `cargo test` / `cargo clippy` 实际输出
- ✅ 引用 git SHA + commit log
- ❌ 禁止引用其它 doc 的 badge / status 字段
- ❌ 禁止引用 "现状描述" 当 "已验证" 状态

**违反例子** (2026-06-26 实查):
- README badge "5/5 meta-gates PASS" 无对应 P11-P16 实跑输出 = 违反 G-08
- README "TPC-H 22/22 ✅" 在 in-process 是,wire/mysql-server 路径未验证 = 违反 G-08 (P0 严重)

### G-09: Gate FAIL 时,文档 badge 立即降级

当任何 meta-gate (P11-P16) FAIL 时:
- README.md / CURRENT_VERSION.md / CONVERGENCE_TRACKER.md 的对应 badge **必须**在 24h 内降级
- CHANGELOG.md **必须**加 "## Gate FAIL disclosure" 段
- 文档作者 **必须** 在 PR description 引用 gate FAIL 输出

**当前状态** (2026-06-26): README badge 仍是 "5/5 PASS" 但 4/6 meta-gate FAIL = G-09 违规 6+ 天

### G-10: Cross-remote discrepancy 必须声明

当 5 个 remote (origin / gitea / gitea250 / gitcode / gitee) 显示不同 HEAD / branch state 时:
- 文档 **必须** 列出 "Cross-remote State Matrix" 表格
- 不允许"挑一个最乐观的" 写文档
- 实际以 origin (252 Gitea) 为 SSOT

**当前状态** (2026-06-26): 本地 develop/v3.9.0 落后 origin 9800 commits,doc-audit worktree HEAD = origin HEAD,本地主仓未更新 — 文档全部按本地 develop 写,严重违反 G-10
```

**改后预期**: 119 → ~155 行
**验证**: `wc -l docs/governance/adr/ADR-001-truthfulness-framework.md` 应该 ≈155

---

### 1.2 ANTI_FABRICATION_POLICY.md (278 行 → 增 5.4/7.4/8.4)

**原**: 278 行,1.0.0 2026-05-30

**改 1**: 5.1 Hard Gate 表格后 (line 167), 增 5.4

**精确 diff** (在 line 167 后插入,在 5.3 之前):

```markdown
### 5.4 Cross-agent claim 链 (多 AI 协作时强制)

当多个 AI agent (Hermes / Claude Code / opencode) 同时工作时,所有 claim 必须带 source AI 标识:

```yaml
claim:
  text: "TPC-H 22/22 in-process PASS"
  source_agent: hermes-ai | claude-code | opencode | human
  source_run: ci_run_id | cargo_test_output | gate_engine_output
  timestamp: ISO8601
  evidence_hash: sha256:...
```

**禁止**: 1 个 AI 引用另一个 AI 的 claim 当自己的 evidence
**当前违规** (2026-06-26): 3 AI 同时在跑,均未标识 source agent
```

**改 2**: 7.3 之后 (line 240), 增 7.4

**精确 diff** (line 240 后插入):

```markdown
### 7.4 P16 Gate Test Integrity FAIL 时处理流程

当 `check_gate_test_integrity.sh` FAIL 时:
1. 立即停止所有"5/5 PASS" 类声明
2. 列出 "no gate-referenced tests found" 的具体原因 (gate 脚本 path 错了? 脚本 grep pattern 错了?)
3. 修复 P16 gate 脚本本身,再 re-run 验证
4. 修复后**必须**加一条 `p16-baseline.json` 记录 PASS 时间
5. README badge 才能恢复 "5/5 PASS"

**当前违规** (2026-06-26): P16 FAIL 但 README badge 仍 "5/5 PASS" ≥ 6 天
```

**改 3**: 8.2 之后 (line 264), 增 8.4

**精确 diff** (line 264 后插入):

```markdown
### 8.4 AFP violation 自我发现流程 (类似 6/22 Z6G4 调查)

当 AI agent 自己发现 AFP 违规时 (例如 6/22 硬件盲区 / 6/26 doc 自我矛盾):
1. **立即停止** 当前 task, 不要"先干完再修"
2. **写** `reports/<date>-afp-self-discovery.md` (参照 reports/doc-audit-2026-06-26.md 格式)
3. **不要** 在 doc 中隐藏 / 美化 / "事后补" 修正声明
4. **不要** 等用户要求才自查 (6/22 Z6G4 调查是 6/22 panic 之后才补,违反此条)
5. 实查 + 实跑, 不引用 doc claim 当 evidence
```

**改后预期**: 278 → ~340 行
**验证**: `wc -l docs/governance/ANTI_FABRICATION_POLICY.md` 应该 ≈340
**版本号**: 1.0.0 → 1.1.0 (在 line 3 改)

---

### 1.3 ISSUE_CLOSING_VERIFICATION.md (192 行 → 增 2.3/2.4/2.5)

**原**: 192 行, v1.0 2026-04-23

**改**: 在 2.2 之后 (line 41), 增 2.3 / 2.4 / 2.5

**精确 diff** (line 41 后插入):

```markdown
### 2.3 HTTP 405 Gitea API Rate Limit workaround

当 `gh pr merge` / Gitea API 返回 HTTP 405 (rate limit) 时:

**禁止**:
- 跳过 PR 直接 push 到目标分支
- 标记 issue "closed (via PATCH)" 绕过 PR 关联验证

**允许 workaround** (memory 6/22 经验):
1. 等待 rate limit reset (默认 60s,可用 `gh api /rate_limit` 查)
2. 如急需: 用 `git push <target_branch> <commit>` + 在 Gitea issue 评论 "manual merge due to API rate limit, commit=<sha>"
3. **必须**在 commit message 加 "MANUAL-MERGE-FIX:" 前缀,便于审计

**当前违规** (2026-06-22): PR #3323/#3324 "closed (via PATCH)" 但无 workaround 文档,违反 2.1 条件 1

### 2.4 多 AI 协作时 Issue 归属

当 ≥ 2 AI agent (Hermes / Claude Code / opencode) 协作时:
- Issue 创建者: 哪个 AI 创建的,后续 verification 由该 AI 执行
- AI Maintainer 关闭 AI Developer 提的 Issue 前,**必须** re-run verification
- 不允许 "看似完成" 就关闭 (Type D 伪任务完成)

### 2.5 AI claim 的 PR 必须是同 AI 提交

约束:
- AI agent 在自己创建的 Issue 上声明的 "fix"，对应的 PR **必须** 由同 AI 提交
- 跨 AI claim 必须显式声明 (例如 "Claude Code #1 claim, Hermes verified")
- 防止"AI A claim → AI B 提 PR 验证" 的伪证据链

**当前违规** (2026-06-26): 3 AI 协作但无归属规则,违反 2.4 + 2.5
```

**改后预期**: 192 → ~250 行
**版本号**: v1.0 → v1.1 (line 2)

---

### 1.4 DOC_CHECK_CORRECTION_RULES.md (241 行 → 增步骤 6/7 + 5.3)

**原**: 241 行, v1.0.0 2026-05-30

**改 1**: 步骤 5 之后 (line 142) 增步骤 6/7

**精确 diff** (line 142 后插入, 在 "## 四、复核审查 Checklist" 之前):

```markdown
### 步骤 6：改完后实跑 gate 验证 (强制)

**操作**：
1. 重跑所有受影响的 `scripts/gate/check_*.sh` (至少 P11-P16)
2. 对比修改前 vs 修改后 gate 输出
3. 在工作报告记录 gate 输出 diff
4. **禁止**: 只跑 "我改的 gate" 而跳过交叉 gate (本次 6/26 调查发现 P14 修完后 P16 才被发现)
5. 如新增 gate 脚本: 必须先在 `tests/baseline/` 加 baseline 再跑

**输出**：
| Gate | 改前 | 改后 | 差异 |
|------|------|------|------|

### 步骤 7：改完后实跑 git push + CI 验证 (强制, 防本地 fix 远端 regression)

**操作**：
1. 推 252 Gitea 触发 CI
2. 等 CI run 完成后,**必须** 在 Gitea commit page 记录 CI run ID + log hash
3. CI FAIL 时, 立即 revert (按 §2.3 可撤销原则)
4. 严禁: 本地 fix 但不 push (导致远端 regression 持续 N 天)

**当前违规** (2026-06-26): 远端 252 多次 gate 修复未 push CI 验证
```

**改 2**: 在 5.2 之后 (line 227) 增 5.3

**精确 diff** (line 227 后插入):

```markdown
### 5.3 Don't claim "5/5 PASS" without re-running gates

约束:
- 文档 badge / status 字段改动前,**必须** 重跑 P11-P16
- 上一份 P11-P16 输出 ≤ 24h 才允许写 PASS
- 超过 24h: 必须加 "stale: last verified <timestamp>" 标记
- 违反: README badge 长期声称 PASS 但实际 FAIL (AFP Type B)
```

**改后预期**: 241 → ~290 行
**版本号**: v1.0.0 → v1.1.0

---

### 1.5 AI_COLLABORATION.md (356 行 → 关键改写: 增 §5.5 多 AI 协调)

**原**: 356 行, 1.0 2026-03-07

**改**: 在 §5.2 审查输出格式 之后 (line 224), 增 §5.5 "多 AI 协调" 完整新章节

**精确 diff** (line 224 后插入,在 §六 之前):

```markdown
### 5.5 多 AI 协调 (Multi-AI Coordination, ADR-014 配套)

当 ≥ 2 AI agent 同时在 SQLRustGo 工作时 (2026-06-26 实测: Hermes + Claude Code #1 + Claude Code #2 + opencode):

**5.5.1 物理隔离 (强制)**

| 隔离维度 | 实现方式 | 验证命令 |
|----------|----------|----------|
| 文件系统 | `git worktree` 分离 | `git worktree list` |
| 进程 | 不同 PID, 不同 cwd | `ps -o pid,cwd,cmd` |
| 端口 | 不同 server port (默认 13306-13399) | `netstat -tlnp` |
| 数据库 data dir | 每个 worktree 独立 `data/` 目录 | `ls <worktree>/data` |
| Gitea identity | 不同 SSH key (hermes / claude-code / opencode) | `git config --get user.email` |

**禁止**:
- 2 AI 共用同一 worktree (会产生 uncommitted 冲突)
- 2 AI 共用同一 server port (会 EADDRINUSE)
- 2 AI 共用同一 data dir (会 data corruption)

**5.5.2 共享资源协调**

| 资源 | 风险 | 处理 |
|------|------|------|
| Gitea issue 评论 | race condition | 用 Gitea server-side lock (issue #X 分配给 AI A) |
| Gitea PR review | 重复 review | AI 提 PR 时必须在 description 标 `agent: <name>` |
| `governance/*` 目录 | 任何 AI 改前必须 review | `governance/adr/ADR-014-multi-ai-coordination.md` 维护者审批 |
| 顶层 `*.md` (README/CHANGELOG/CURRENT_VERSION) | 同上 | 改前必须 review |
| Cargo.lock | 同时改会冲突 | `cargo update` 必须串行, 不能 2 AI 并行 |

**5.5.3 标签系统 (Gitea Issue 标签)**

| 标签 | 含义 | 谁贴 |
|------|------|------|
| `agent:hermes` | Hermes Agent 创建/认领 | Hermes |
| `agent:claude-code` | Claude Code 创建/认领 | Claude Code |
| `agent:opencode` | opencode 创建/认领 | opencode |
| `agent:multi` | 多 AI 协作 | 任意 AI 提议 + 人类审批 |
| `conflict:potential` | 检测到潜在冲突 | 任何 AI |
| `soak:running` | 跑 SOAK 测试中 | 启动者 |

**5.5.4 调度规则**

- 1 个 task 同一时刻只允许 1 AI 持有 (避免 2 AI 重复实现)
- SOAK 测试: 任何时刻只允许 1 个 (端口/资源冲突)
- `cargo test --all-features`: 任何时刻只允许 1 个 (CPU 抢占)
- 1 个 PR 只允许 1 AI 提, 其他 AI 可 review

**5.5.5 ADR-014 配套**

- 新建 `docs/governance/adr/ADR-014-multi-ai-coordination.md`
- 状态: ACCEPTED (2026-06-26)
- Deciders: Hermes Agent + User (李哥)
- 相关: ADR-001 (Truthfulness), AFP (Anti-Fabrication), ISSUE_CLOSING (HTTP 405 workaround)

**当前违规** (2026-06-26): 0 协调规则, 4 AI 物理隔离靠运气
```

**改后预期**: 356 → ~470 行
**版本号**: 1.0 → 1.1 (line 2)
**变更历史**: line 351 加 1.1 行

---

## 2. 新建 ADR-014-multi-ai-coordination.md

**新文件** (跟 §1.5 §5.5.5 配套)

**模板**: 复用 ADR-008 模板结构 (line 1-50 已读)

**完整内容草案** (待执行时生成):

```markdown
# ADR-014: Multi-AI Coordination Policy

> **Status**: ACCEPTED (2026-06-26)
> **Deciders**: Hermes Agent + User (李哥)
> **Date**: 2026-06-26
> **Supersedes**: 部分覆盖 AI_COLLABORATION.md §1.1 (单 AI 模型)
> **Related**: [ADR-001 — Truthfulness](../docs/governance/adr/ADR-001-truthfulness-framework.md), [ADR-008 — Test-Claim-Transparency](../docs/governance/adr/ADR-008-test-claim-transparency.md), [AFP — Anti-Fabrication](../docs/governance/ANTI_FABRICATION_POLICY.md), [ISSUE_CLOSING — HTTP 405 workaround](../docs/governance/ISSUE_CLOSING_VERIFICATION.md), [AI_COLLABORATION — §5.5](../docs/governance/AI_COLLABORATION.md)

## Context

2026-06-26 实测发现: 4 个 AI agent (Hermes / Claude Code #1 / Claude Code #2 / opencode) 同时在 SQLRustGo 工作, 物理隔离靠 worktree 隔离, 但 Gitea 共享资源无协调规则。具体问题:

1. **CLAIM 归属**: 谁 claim 什么, 谁 verification, 跨 AI claim 怎么传递, 0 规则
2. **Gitea API 限流**: 多个 AI 同时打 Gitea API, 触发 405 rate limit (6/22 已发生过)
3. **PR 重复 review**: 2 AI 提相似 PR, review 重复
4. **数据 corruption 风险**: 2 AI 共用 data dir (目前靠 worktree 隔离但无强制)

## Decision

采用 ADR-014 规范:

### 14.1 AI Agent Identity

每个 AI agent 必须:
- 独立 Gitea SSH key
- 独立 `git config user.email`
- 独立 worktree (no sharing)
- 独立 server port (13306-13399 范围)
- 独立 `data/` 目录

### 14.2 Claim Provenance

任何 AI claim 必须带 (引用 AFP §5.4):

```yaml
claim:
  text: "..."
  source_agent: hermes | claude-code | opencode | human
  source_run: ...
  timestamp: ISO8601
  evidence_hash: sha256:...
```

### 14.3 Resource Lock

| 资源 | Lock 机制 |
|------|----------|
| 1 个 Issue | 1 AI 持有 (comment "claim" 后其他人等 24h) |
| 1 个 PR | 1 AI 提 (description 标 `agent: <name>`) |
| SOAK test | 1 个时刻只 1 个 (用 PID 锁) |
| `cargo test --all-features` | 1 个时刻只 1 个 |
| `governance/*` 改 | 必须 1 AI 提议 + 1 AI review + 1 human 批 |

### 14.4 Conflict Resolution

- 检测到冲突: 立即 stop, 写 `reports/<date>-multi-ai-conflict.md`
- 冲突升级: 人类 (李哥) 仲裁
- 48h 无仲裁: 自动按 "最小修改" 原则回退 (git revert)

## Consequences

### Positive

- 4 AI 协作可治理
- 数据 corruption 风险 → 0
- 重复 review → 0
- Claim 链可追溯

### Negative

- AI agent 工作量增加 (必须带 provenance)
- 人类审批负担增加 (governance/* 必审)

### Neutral

- 物理隔离机制 (worktree) 保持不变
- 单 AI 工作流不变

## Metadata

- **Author**: Hermes Agent
- **Date**: 2026-06-26
- **Related ADRs**: ADR-001, ADR-008
- **Related PRs**: 暂无, 待 Stage 3 整改 PR
- **Supersedes**: AI_COLLABORATION.md §1.1 (部分)
```

**新文件大小**: 约 90 行

---

## 3. INDEX.md 刷新

**改**: 76 个 governance 文件的索引, 当前 12 个 ADR (跳 008, 错标 009/010/011)

**改前**: (还没读全文, 摸底时发现版本 "v3.8.0" 错, 最后更新 2026-06-05 错)

**待执行时做**:
- 改 "版本: v3.8.0" → "版本: v3.9.0-rc7"
- 改 "最后更新: 2026-06-05" → "最后更新: 2026-06-26"
- 列 ADR-001 到 ADR-014 (14 个, 补 ADR-008 跳过 + ADR-014 新增)
- 修 ADR-009/010/011 标题错 (按实际文件名)
- 5 套规范加 cross-link (互相引用)

**注**: INDEX.md 还没细读, 计划: 先读全文, 列精确 diff, 报"等单字母" 二轮, 再写。

---

## 4. 顶层 5 文档修正

### 4.1 README.md

**改前** (line 3-6):
```
Last updated: 2026-06-13
Current dev branch: 8a83e2553 @ develop/v3.9.0
Latest stable: v3.8.0 (GA, 2026-06-08) | v3.9.0-rc7 (in soak, GA target 2026-12-15)
Latest RC: v3.9.0-rc7 (RC, 2026-06-12, G1-G16 PASS, awaiting 24h real soak)
```

**改后** (line 3-6):
```
Last updated: 2026-06-26
Current dev branch: 3d817386d @ develop/v3.9.0
Latest stable: v3.8.0 (GA, 2026-06-08) | v3.9.0-rc7 (in soak, GA target TBD, NOT 2026-12-15 — 24h/72h/168h real soak 0 完成)
Latest RC: v3.9.0-rc7 (RC, 2026-06-12, 6 meta-gate 实跑 1/6 PASS, 详见 reports/doc-audit-2026-06-26.md)
```

**改前** (line 14-17 badge):
```
<img src="...9--Dim%20Gate-8%2F8%20PASS...">
<img src="...INT--1%20(P0)-CLOSED...">
```

**改后**:
```
<img src="...9--Dim%20Gate-1%2F6%20PASS-yellow..." alt="D9 (1/6 meta-gate PASS, 4 FAIL)">
<img src="...P12%2FP14%2FP15%2FP16-FAIL-red..." alt="P12/P14/P15/P16 FAIL">
```

**改后预期**: 451 → ~455 行
**验证**: `git diff --stat README.md` 应该 ~10-15 行变更

### 4.2 CURRENT_VERSION.md (84 行, 改 9 处)

**改前** (line 3-12):
```
alpha/v3.8.0
## 阶段信息
- **阶段**: Alpha (功能开发阶段)
- **当前里程碑**: Execution Semantics Freeze → TransactionManager 集成
- **开始日期**: 2026-05-28
- **开发分支**: develop/v3.8.0
- **目标**: WAL + MVCC 事务 + TransactionManager → GA
```

**改后** (line 3-12):
```
v3.9.0-rc7
## 阶段信息
- **阶段**: RC7 (Release Candidate 7, in soak)
- **当前里程碑**: 真实 24h/72h/168h SOAK 验证 (0 完成, GA 不可达)
- **开始日期**: 2026-05-28 (develop/v3.9.0)
- **开发分支**: develop/v3.9.0
- **当前 HEAD**: 3d817386d (origin/develop/v3.9.0 @ 252 Gitea, 2026-06-26 06:30 验证)
- **目标**: 24h/72h/168h SOAK PASS → GA, GA target TBD (NOT 2026-12-15)
```

**改后预期**: 84 → ~90 行
**验证**: `git diff --stat CURRENT_VERSION.md` 应该 ~10 行变更

### 4.3 CONVERGENCE_TRACKER.md (405 行, 改 1-5 行基线 + 加新段)

**改前** (line 3-4):
```
**基线**: develop/v3.9.0 @ 3afe482f6
**目标**: 真实运行验证 → RC3 → RC4 → GA
```

**改后** (line 3-4):
```
**基线**: develop/v3.9.0 @ 3d817386d (origin HEAD @ 2026-06-26 06:30 验证, 旧基线 3afe482f6 落后 510+ commits)
**目标**: 24h/72h/168h SOAK 完成 (当前 0 完成) → GA
```

**加新段** (在 line 53 "### 待办" 之后):
```markdown
## 6/26 实跑重整

- 6/26 06:30 验证 252 Gitea origin HEAD = 3d817386d
- 6/26 06:30 跑 6 meta-gate (P11-P16): 1/6 PASS, 1 WARN, 4 FAIL
- 6/26 06:33 写 reports/doc-audit-2026-06-26.md
- 6/26 06:45 写 reports/STAGE2_DIFF_PLAN_2026-06-26.md (Stage 2 diff 等批)
- 6/26 06:50 待李哥批 Stage 2 5 governance 改写 + 新 ADR-014 + 顶层 5 doc
```

**改后预期**: 405 → ~420 行

### 4.4 CHANGELOG.md (404 行, 加 1 个 [Unreleased] 子段)

**改前** (line 8-15):
```
## [Unreleased] - 2026-06-05
### v3.9.0 Production Readiness Release 启动
...
**当前状态**: RC7 (2026-06-12), GA 目标 2026-12-15 (per Hermes audit #3252, deferred from 2026-09-23)
```

**改后** (line 8-15):
```
## [Unreleased] - 2026-06-26
### v3.9.0 Production Readiness Release 启动
...
**当前状态**: RC7 (2026-06-12, 6/26 重新验证 1/6 meta-gate PASS)
**GA 目标**: TBD (原 2026-12-15 推迟, 24h/72h/168h real soak 0 完成)
**6/26 整改**: 详见 reports/STAGE2_DIFF_PLAN_2026-06-26.md (5 governance 改写 + ADR-014 新建 + 顶层 5 doc)
```

**加新段** (在文件末尾, "## v3.8.0 GA Final 收口内容" 之前):
```markdown
---

## 2026-06-26 — 文档整改 (本次会话)

### Added
- `docs/governance/adr/ADR-014-multi-ai-coordination.md` (新 ADR)
- §5.5 多 AI 协调 章节 in AI_COLLABORATION.md
- G-08/G-09/G-10 in ADR-001 (Truthfulness 新约束)
- §5.4/§7.4/§8.4 in ANTI_FABRICATION_POLICY.md
- §2.3/§2.4/§2.5 in ISSUE_CLOSING_VERIFICATION.md
- 步骤 6/7 + §5.3 in DOC_CHECK_CORRECTION_RULES.md

### Changed
- 6 meta-gate badge 真实状态: 1/6 PASS (4 FAIL: P12/P14/P15/P16, 1 WARN: P13)
- CURRENT_VERSION.md: alpha/v3.8.0 → v3.9.0-rc7
- README.md: SHA 8a83e2553 → 3d817386d, badge 降级 5/5 → 1/6
- CONVERGENCE_TRACKER.md: 基线 3afe482f6 → 3d817386d

### Gate State (P0 truth)
- P11: PASS
- P12: FAIL
- P13: WARN (gate 自身 syntax error)
- P14: FAIL
- P15: FAIL
- P16: FAIL
```

**改后预期**: 404 → ~440 行

### 4.5 ROADMAP.md (425 行, 改日期 + 加 6/26 段)

**改前** (line 3-5):
```
**版本**: 2.0
**更新日期**: 2026-06-03
**维护人**: yinglichina8848
```

**改后** (line 3-5):
```
**版本**: 2.1
**更新日期**: 2026-06-26
**维护人**: yinglichina8848 + Hermes Agent
```

**加新段** (line 425 之前, 在 "## 二、1.x 系列" 之后):
```markdown
## 零、2026-06-26 整改记录

- 5 governance 文档整改 (ADR-001/AFP/ISSUE_CLOSING/DOC_CHECK/AI_COLLAB) — 增 G-08/9/10 等 9 个新约束
- 新建 ADR-014-multi-ai-coordination.md
- 顶层 5 文档修正 (README/CURRENT_VERSION/CONVERGENCE_TRACKER/CHANGELOG/ROADMAP)
- 6 meta-gate 真实状态: 1/6 PASS, 4 FAIL, 1 WARN (reports/doc-audit-2026-06-26.md)
- v3.9.0 GA target TBD (原 2026-12-15 不可达, 24h/72h/168h real soak 0 完成)
```

**改后预期**: 425 → ~440 行

---

## 5. 修改顺序 (避免冲突)

按以下顺序执行,每改一文件跑对应 check:

1. **新建 ADR-014** (line ~90 行)
2. **改 ADR-001** (G-08/9/10)
3. **改 ANTI_FABRICATION_POLICY** (§5.4/7.4/8.4)
4. **改 ISSUE_CLOSING_VERIFICATION** (§2.3/2.4/2.5)
5. **改 DOC_CHECK_CORRECTION_RULES** (步骤 6/7 + §5.3)
6. **改 AI_COLLABORATION** (§5.5 多 AI 协调)
7. **改 INDEX.md** (待执行时再读全文再列 diff 二轮报)
8. **改 README.md** (badge 降级)
9. **改 CURRENT_VERSION.md** (版本号/SHA/GA target)
10. **改 CONVERGENCE_TRACKER.md** (基线 + 6/26 段)
11. **改 CHANGELOG.md** (Unreleased + 6/26 整改段)
12. **改 ROADMAP.md** (日期 + 6/26 整改段)
13. **跑全套 gate** 验证 (P11-P16 + check_docs_links.sh + check_docs_consistency.sh)
14. **写整改报告** reports/STAGE2_REPORT_2026-06-26.md (3 份: 治理类/入口类/测试报告类)

每步之后:
```bash
git diff --stat <file>
bash scripts/gate/check_docs_links.sh  # 改文档后必跑
```

---

## 6. 不变量 (再强调)

✅ **会做**:
- 改 5 governance 文档 (增 G-08/9/10 等 9 个新约束, + ~150 行)
- 新建 ADR-014 (新文件 ~90 行)
- 改顶层 5 文档 (badge/版本/基线/日期降级, + ~50 行)
- 改 INDEX.md (待二轮报 diff)
- 跑 gate 验证 (P11-P16 + check_docs_links.sh)
- 写整改报告 (3 份)

❌ **不会做**:
- 跑 cargo test (会抢 Claude Code CPU)
- 跑 cargo clippy (同上)
- 跑任何 soak test (会撞 13306)
- 推 252 Gitea (待二轮报)
- 改 `docs/releases/v3.9.0/` 内部 30+ 文档 (超范围, 这轮只改 governance + 顶层 5)
- 改 Cargo.lock / Cargo.toml
- 改任何 *.rs 文件
- 删任何文件
- 改其他 AI agent (Claude Code / opencode) 的 worktree 文件

---

## 7. 风险/边界

- 5 governance 文档 + 1 新 ADR + 5 顶层 doc = 11 文件修改, 0 个代码文件
- 总行数变化: +~290 行 (5 gov) + ~90 行 (新 ADR) + ~50 行 (顶层 5) = +~430 行
- 工作区: doc-audit-2026-06-26 worktree (已物理隔离)
- 不影响: SOAK 测试 (port 13306 仍 occupied by SOAK-3265), Claude Code TPC-H 测试, opencode SOAK

---

## 8. 等你单字母批准

**A** = 全做 (5 governance 改 + 新 ADR-014 + 顶层 5 改)
**B** = 只做 §1 (5 governance 改, 跳过 §4 顶层 5 改)
**C** = 只做 §4 (顶层 5 改, 跳过 §1 governance)
**D** = 只新建 ADR-014, 改其他 (优先级 1)
**E** = INDEX.md 还要再读再列 diff, 现在不批

**默认我会等单字母**。按 6/14 红线 "子任务偏差必须停下+等单字母" — 5 governance 全文 + 顶层 5 全文 + INDEX 全文还没全读, INDEX 摸底要二轮报 diff。

请批 (单字母或具体文件名)。
