<!-- env:blocked:no-ci -->

# PATTERN_LEGACY_AUDIT_FOLLOWUP.md

> **Pattern**: Legacy Issue Audit → Tracked Remediation → Cross-Version Debt Gate Update
> **Trigger**: Major version GA approaching; significant cross-version debt accumulated
> **Source**: v3.8.0 GA 治理示范 (2026-06-04~05, ~7 hours, 13 项治理债务)
> **Author**: Hermes Agent
> **Date**: 2026-06-05
> **Status**: ACTIVE — recommended for v3.9.0+ cycles
> **Cross-references**: `GA_GOVERNANCE_DEMO_v3.8.0.md`, `PATTERN_GATE_FALSE_POSITIVE.md`, `ADR-010-cross-version-debt-governance.md`

---

## Context

Legacy Issue Audit + Follow-up is a governance pattern observed during v3.8.0 GA preparation. The pattern is triggered when:

1. A version has accumulated significant cross-version debt (10+ items) over multiple releases
2. The current GA gate is approaching and debt status is unclear
3. Documentation may be stale relative to actual code reality
4. Gate scripts may be passing on paper but failing in execution

This pattern is the **systematic upgrade** of `PATTERN_GATE_FALSE_POSITIVE.md` (which only addressed gate report false positives). This new pattern adds:
- Multi-dimensional cross-version debt tracking (INT / F / I / T)
- Subagent-driven parallel investigation
- Follow-up Issue creation for items exceeding 1-2h scope
- Code reality check (vs documentation claim) for the "closed" status

---

## Trigger Conditions

This pattern is triggered when ANY of the following are true:

| Signal | Evidence Required |
|--------|-------------------|
| Cross-version debt inventory shows 10+ items | `docs/releases/v<X.Y.Z>/INT5_PLUS_DEBT_INVENTORY.md` (or equivalent) shows >10 items with mixed status |
| Documentation claims "X/X CLOSED" but no recent verification | Gate reports dated >7 days, no recent `cargo test` or `cargo run` output |
| Gate scripts only parse markdown without code check | Grep `grep -c "rg --type rust" scripts/gate/*.sh` (should be 0 for legacy scripts) |
| Multiple subagent reports show contradicting status | Different subagents report different "closed" status for the same debt item |
| A user request explicitly invokes the audit | User message contains keywords: "audit", "legacy", "历史遗留", "STALE", "门禁" |

---

## Architecture (4-Phase Workflow)

```
┌──────────────────────────────────────────────────────────┐
│ Phase 1: 调研 (Investigation)                            │
│   - 3 subagent 并行 (F-XX / INT-ARCH-SEM / F-01~F-36+)  │
│   - 直接读 13 份核心文档                                  │
│   - 5 个 gate 脚本源码审计                                │
│   - 输出: 13+ 项治理债务清单 + 优先级矩阵                │
└──────────────────────────────────────────────────────────┘
                          ↓
┌──────────────────────────────────────────────────────────┐
│ Phase 2: 跟踪 (Tracking)                                  │
│   - 用 Gitea API 创建 N 个 Issue (I#DEBT-v<X.Y.Z>-XXX)   │
│   - 每个 Issue 含: 来源/优先级/真实状态/整改方向/工时    │
│   - 关联 PR #audit (审计报告) → Issue 列表                │
└──────────────────────────────────────────────────────────┘
                          ↓
┌──────────────────────────────────────────────────────────┐
│ Phase 3: 整改 (Remediation)                              │
│   - 按 P0 → P1 → P2 顺序修复                              │
│   - 每个修复: worktree + 5 步流程 + commit + PR + merge  │
│   - 1 周工作超 1-2h 范围: 转 follow-up Issue              │
│   - 测试: 跑 D6 + 真实 cargo test 验证                    │
└──────────────────────────────────────────────────────────┘
                          ↓
┌──────────────────────────────────────────────────────────┐
│ Phase 4: 验证 (Verification)                             │
│   - 跑 8 维门禁 (D1-D8) + Code Reality (新)              │
│   - 对比文档声称 vs 真实结果                              │
│   - 写 GA 治理示范报告                                    │
│   - 列出剩余 follow-up (6-9 周) + v3.9.0+ 计划           │
└──────────────────────────────────────────────────────────┘
```

---

## Phase 1: 调研 (Investigation)

### 1.1 调研方法

| Step | Tool | Time | Output |
|------|------|------|--------|
| 1.1.1 | 3 subagent 并行调研（explore 类型）| ~2h | 13+ 项治理债务 + 优先级 |
| 1.1.2 | 直接读 13 份核心文档（feature matrix, debt inventory, gate reports, etc.）| ~1h | 文档 vs 实际差异 |
| 1.1.3 | 5 个 gate 脚本源码审计（check_int_debt.sh, check_arch2_no_bypass.sh, etc.）| ~30min | Gate BUG 列表 |
| 1.1.4 | 实际跑门禁脚本（`bash scripts/gate/check_*.sh`）| ~30min | 真实状态（非文档声称）|

### 1.2 Subagent 用法

3 个 subagent 并行（节省 ~5h 人工调研）：

| Subagent | 范围 | 输出格式 | Time |
|----------|------|----------|------|
| #1 (explore) | 16 F-XX + I-12 实际状态 | Markdown 表格 per F-XX | ~30min |
| #2 (explore) | 11 INT/ARCH/SEM + 3 gate BUG | 表格 + 真实结论 + 文档冲突 | ~30min |
| #3 (explore) | 36 F-01~F-36 + 12 I-XX + 20 T-XX 旧债务 | 评级 (CLOSED/PARTIAL/STALE/OPEN) | ~30min |

**Subagent Prompt 模板**（参考 v3.8.0 实战）:

```
你是 SQLRustGo 仓库的代码分析 subagent。请独立调研 [具体范围] 的实际状态。

工作目录: /home/openclaw/workspace/dev/sqlrustgo

**范围**: [F-XX 列表 / 债务项列表]
**对每项核对**:
1. 代码现状: [搜索关键词]
2. 测试代码: [tests 路径]
3. SPEC 文档: [docs 路径]
4. 门禁集成: [scripts 路径]
5. 主路径集成: [是否真接入主流程]

**输出格式**: Markdown 表格 (维度 | 状态 | 证据)
**最后汇总**: [N 个真实 CLOSED + M 个 PARTIAL + K 个 OPEN]

不要修改任何文件, 纯调研. 返回详细报告.
```

### 1.3 输出：审计报告

写入 `docs/releases/v<X.Y.Z>/historical/LEGACY_ISSUES_<date>_AUDIT.md`：

- 271 行模板（v3.8.0 实际）
- 6+ 节：背景 / 真实状态总览 / 严重问题 / 集成债务 / 16 F-XX 详细 / 旧债务 / 整改计划 / Issue 清单 / 总结
- 引用 30+ 处文件路径 + 100+ 处代码引用
- 13 项治理债务（I#DEBT-v<X.Y.Z>-001~013）

---

## Phase 2: 跟踪 (Tracking)

### 2.1 Gitea Issue 创建

每个治理债务创建 1 个 Gitea Issue：

```bash
# 模板函数
create_issue() {
    local id="$1"  # 001~013
    local title="$2"  # 🔴 P0 / 🟡 P1 / 🟢 P2 + 标题
    local priority="$3"  # P0 / P1 / P2
    local body="..."  # 来源/优先级/真实状态/整改方向/工时
    curl -s -X POST "https://...api/v1/repos/.../issues" \
        -H "Content-Type: application/json" \
        -d "{\"title\": \"...\", \"body\": \"...\"}"
}
```

### 2.2 Issue Body 模板

```markdown
## 来源

v<X.Y.Z> 严重遗留问题审计 (<date>) — PR #<audit>

审计文件: `docs/releases/v<X.Y.Z>/historical/LEGACY_ISSUES_<date>_AUDIT.md`

## 优先级

**🔴 P0** (GA 阻断)

## 问题描述

[2-3 段真实描述，含具体行号 / commit / 测试名]

## 整改方向

[1-2 段具体修复任务，含 1-2 关键命令]

## 估计时间

[1h / 1d / 1w 估时]

## 关联

- 报告章节: §<N>
- 报告 Issue 编号: I#DEBT-v<X.Y.Z>-<NNN>
- 审计 PR: #<NNNN>
- 状态: OPEN
```

### 2.3 关联 PR 评论

审计 PR 合并后，添加评论列出所有 Issue 编号：

```bash
curl -s -X POST ".../issues/<audit_pr>/comments" \
    -d "## 关联 N 个治理 Issue\n\n本审计报告已开 N 个 I#DEBT-v<X.Y.Z>-XXX Gitea Issue..."
```

### 2.4 治理合规

每次创建 Issue 必须遵守 `docs/governance/ISSUE_CLOSING_VERIFICATION.md`：
- 禁止手动关闭没有 PR 合并的 Issue
- 关闭 Issue 前必须验证有 PR 关联

---

## Phase 3: 整改 (Remediation)

### 3.1 优先级排序

| 优先级 | 类别 | 处理 |
|---|---|---|
| P0 | GA 阻断 / ACID 违规 / Gate 假 PASS | 必做，1-2h 内 |
| P1 | 文档 STALE / 集成债务 / 测试缺口 | 必做，1d 内 |
| P2 | 证据刷新 / 性能 / 文档完整性 | 可选 |

### 3.2 5 步文档流程（最小修改原则）

所有文档修改按 `DOC_CHECK_CORRECTION_RULES.md`：

1. **步骤 1: 发现问题** — 列出问题清单（文件/行号/依据）
2. **步骤 2: 改正计划 + Checklist** — oldString → newString 计划 + 核查表
3. **步骤 3: 执行** — `edit` 工具每改一项记录 git diff
4. **步骤 4: Checklist 核查** — 逐项核查
5. **步骤 5: 编写工作报告** — 基本信息/问题/操作/核查/状态/结论

### 3.3 1 周工作 1-2h 拆分模式

遇到 1 周+工作量（如 INT-2/3 集成, ARCH-3 VTU）：

```
调查 5min → 创建 worktree 1min → 写代码 30min → 测试 10min 
→ 发现根本障碍 5min → 撤回 1min → 开 follow-up Issue 3min 
→ 清理 1min → 报告给用户
```

**关键原则**：
- 宁愿撤回也不强推（避免引入新 bug）
- 留 1-2 follow-up Issue 让后续 sprint 处理
- 真实工程节奏优先（7h/会话可接受）

### 3.4 Worktree 流程

每次修复独立 worktree：

```bash
git worktree add .worktrees/<fix-name> -b fix/<NNN>-<topic> develop/v<X.Y.Z>
# ... 修改 + 测试
git add <files>
git commit -m "fix(gate): #<NNN> <topic>"
git push gitea fix/<NNN>-<topic>
# ... 用 Gitea API 创建 PR + merge
git worktree remove .worktrees/<fix-name> --force
git branch -D fix/<NNN>-<topic>
```

### 3.5 Gate BUG 修复模式

gate 脚本 BUG 修复后必须：
1. 实际跑修复后脚本，确认不再假 PASS
2. 修复前后对比（真实状态 vs 文档声称）
3. 跨 8 维门禁跑全套（确认无 cascade 失败）
4. commit message 引用 PR/Issue + 列出修前/修后对比

---

## Phase 4: 验证 (Verification)

### 4.1 8 维门禁

```bash
# 5 维度主 gate
bash scripts/gate/check_rc_ga_gate.sh ga

# 全 8 维度综合验证
bash scripts/gate/check_full_gate_verification.sh

# 单独 audit
bash scripts/gate/audit_testing.sh v<X.Y.Z> ga artifacts/audit/v<X.Y.Z>
```

预期：每个 exit code 与文档一致（如 0 = PASS, 1 = FAIL, 2 = DRIFT）。

### 4.2 Code Reality Check（v3.8.0 新增）

`scripts/gate/check_cross_version_debt.sh` Part 5（PR #3141）：
- 检测 10 孤岛 F-XX（`use sqlrustgo_` 数量 = 0）
- 检测 5 无实现（关键符号 rg 检查）
- 输出 ❌/⚠️/✅ 状态
- 不阻断 gate（info-level）

### 4.3 文档一致性

```bash
bash scripts/gate/check_docs_links.sh
bash scripts/gate/check_docs_consistency.sh
```

### 4.4 写 GA 治理示范报告

参考 `GA_GOVERNANCE_DEMO_v3.8.0.md` 模板：
- 7 节：背景 / 调研方法 / 整改工作流 / 产出物 / 真实状态 / 规范化 / 教训
- 100+ 行
- 引用所有 PR + Issue + commit

---

## Failure Modes

### Wrong Action 1: 跳过 5 步文档流程

```
❌ 错误做法: 直接 git add + commit + push 修改 7 份文档
```

**Why it fails**: 
- 无法回滚（没有 oldString 记录）
- 过度修改可能破坏原文意图
- 违反 `DOC_CHECK_CORRECTION_RULES.md`（强制）

**Correct**: 步骤 1-5 走完，最小修改，每步记录 diff。

### Wrong Action 2: 文档声称 vs 实际差异时编造数据

```
❌ 错误做法: 发现 D6 报告声称 "0 failed" 但实际有 14 FAIL → 重写 d6_test_inventory.json 写 "0 failed"
```

**Why it fails**:
- 违反 `ADR-001-truthfulness-framework.md`
- 引入新 false positive
- 破坏审计链

**Correct**: 如实记录 14 FAIL，状态记为 ❌ FAIL。如果 issue 严重就开新 issue 修复（如 #3111 d6 证据刷新）。

### Wrong Action 3: 1 周工作强行 1-2h 完成

```
❌ 错误做法: INT-2/3 集成 1 周 → 1-2h 写 5 处代码 → 引入新 bug
```

**Why it fails**:
- 时间不足导致未充分测试
- 引入 cascade 失败（如 #3109 撤回案例）
- 损坏代码质量

**Correct**: 调查 + 最小可行 + follow-up Issue。30min 实际有效工作时间 + 3min follow-up + 1min 撤回（如果发现障碍）。

### Wrong Action 4: 不创建 worktree

```
❌ 错误做法: 在主 develop/v3.8.0 分支直接修改 + commit
```

**Why it fails**:
- 违反 `AGENTS.md` 强制规则
- 多个修复混在一起，无法独立 review
- 出错时回滚困难

**Correct**: 每个修复独立 worktree + 独立分支 + 独立 PR。

### Wrong Action 5: Subagent 不明确输出格式

```
❌ Subagent prompt: "检查 F-09 状态"
```

**Why it fails**:
- Subagent 输出模糊（"已检查"）
- 无法验证 Subagent 工作质量
- 输出难以聚合到审计报告

**Correct**: 明确输出格式（Markdown 表格 per F-XX），明确证据要求（行号/命令/commit），明确汇总要求（N CLOSED + M PARTIAL）。

---

## Validation

### Pre-conditions (Before Starting Phase 1)

- [ ] 本地 `develop/v<X.Y.Z>` 与 `origin/develop/v<X.Y.Z>` 同步
- [ ] Worktree 清理（无遗留 worktree）
- [ ] Stash 列表为空
- [ ] AGENTS.md 阅读并遵守
- [ ] `docs/governance/ISSUE_CLOSING_VERIFICATION.md` 流程理解
- [ ] `docs/governance/DOC_CHECK_CORRECTION_RULES.md` 5 步流程理解
- [ ] `ADR-001-truthfulness-framework.md` 原则理解

### Post-conditions (After Phase 4)

- [ ] 13+ 治理债务全部 Gitea Issue 跟踪
- [ ] 12+ Issue 关闭（PR 关联）
- [ ] 3+ Follow-up Issue 创建（1 周+ 工作量）
- [ ] 14+ PR 合并到 develop/v<X.Y.Z>
- [ ] 8 维门禁通过（PASS 或 PASS-WITH-DRIFT）
- [ ] Code Reality Check 10 孤岛 + 5 无实现检测到（如属实）
- [ ] D6 证据刷新（时间戳 < 7 天）
- [ ] 7+ 份 STALE 文档同步
- [ ] GA 治理示范报告写入 `docs/governance/`

---

## Reuse for v3.9.0+

### Checklist (Copy-Paste)

```markdown
# v3.9.0 Legacy Audit Checklist

## Pre-Audit
- [ ] Read `AGENTS.md` and `ISSUE_CLOSING_VERIFICATION.md`
- [ ] Sync local with `origin/develop/v3.9.0`
- [ ] Clean worktrees + stash
- [ ] Verify Gitea access (http://...:3000/api/v1)

## Phase 1: Investigation
- [ ] 3 subagent parallel: F-XX, INT-ARCH-SEM, F-01~F-36+
- [ ] Read 13+ core documents directly
- [ ] Audit 5+ gate scripts source code
- [ ] Run all gate scripts, capture actual exit codes

## Phase 2: Tracking
- [ ] Create N Gitea Issues with I#DEBT-v3.9.0-XXX pattern
- [ ] Each Issue: source / priority / real status / remediation / estimate
- [ ] Add comment to audit PR linking all Issues

## Phase 3: Remediation
- [ ] P0 issues: fix in 1-2h each, worktree + 5-step process
- [ ] P1 issues: 1d, same process
- [ ] 1-week+ items: open follow-up Issue, don't force complete
- [ ] Real data only (no fabrication, ADR-001)

## Phase 4: Verification
- [ ] 8-dimension gate check passes
- [ ] Code Reality Check detects N isolated + M unimplemented
- [ ] 7+ STALE documents synchronized
- [ ] D6 evidence refreshed (timestamp < 7 days)
- [ ] GA governance demo report written to docs/governance/

## Output
- [ ] N PRs merged
- [ ] M Issues closed (PR-linked)
- [ ] K follow-up Issues for 1-week+ work
- [ ] GA demo report for v3.9.0 (similar to v3.8.0)
```

---

## Time Budget (Reference from v3.8.0)

| Phase | Subagent 调研 | 直接读文档 | Gate 审计 | 修复 + PR | 报告 |
|--------|---------------|-----------|-----------|-----------|------|
| Time | ~2h | ~1h | ~30min | ~3h (12 issues) | ~30min |
| % | 28% | 14% | 7% | 43% | 7% |

Total: ~7h (1 session)
Tools: subagent-driven + Gitea API + worktree + 5-step docs flow

---

## Related Patterns

- `PATTERN_GATE_FALSE_POSITIVE.md` — v3.6.0 触发，本 pattern 的前置
- `PATTERN_EVIDENCE_CHAIN.md` — Truthfulness 原则
- `PATTERN_LEGACY_RETIREMENT.md` — 旧代码/文档退役
- `PATTERN_COVERAGE_DISPUTE.md` — 覆盖率争议处理

---

## Maintenance

| 项目 | 值 |
|------|-----|
| 文档版本 | PATTERN_LEGACY_AUDIT_FOLLOWUP-1.0 |
| 最后更新 | 2026-06-05 |
| 维护者 | Hermes Agent |
| 状态 | ACTIVE |
| 下次审查 | v3.9.0 GA 前 (~3 months) |
| 复用案例 | v3.8.0 GA (2026-06-04~05, 7h, 12 issues closed) |
