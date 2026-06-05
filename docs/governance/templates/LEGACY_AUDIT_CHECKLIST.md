<!-- env:blocked:no-ci -->

# LEGACY_AUDIT_CHECKLIST.md (Template)

> **Template**: Reusable checklist for cross-version legacy issue audit
> **Source**: v3.8.0 GA 治理示范 (2026-06-04~05, ~7 hours)
> **Status**: ACTIVE — 后续版本 (v3.9.0+, v4.0.0) 直接复用
> **Cross-references**: `PATTERN_LEGACY_AUDIT_FOLLOWUP.md`, `GA_GOVERNANCE_DEMO_v3.8.0.md`, `DOC_CHECK_CORRECTION_RULES.md`, `ISSUE_CLOSING_VERIFICATION.md`

---

## 使用说明

每个版本 GA 前 1-2 周执行本 checklist。**预计时间**: 1 session (5-8h)

**复制本文件到**:
- `docs/releases/v<X.Y.Z>/historical/LEGACY_AUDIT_<YYYY-MM-DD>.md`
- 填入实际数据
- 完成后归档为 GA 治理示范

---

## Phase 0: 准备 (Pre-Audit)

### 0.1 规则确认

- [ ] 阅读 `docs/governance/AGENTS.md`（强制规则）
- [ ] 阅读 `docs/governance/ISSUE_CLOSING_VERIFICATION.md`（Issue 关闭规则）
- [ ] 阅读 `docs/governance/DOC_CHECK_CORRECTION_RULES.md`（5 步流程）
- [ ] 阅读 `docs/governance/ADR-001-truthfulness-framework.md`（真实数据原则）
- [ ] 阅读上一版治理示范（`GA_GOVERNANCE_DEMO_v<X.Y-1>.md`）

### 0.2 仓库同步

```bash
git fetch origin develop/v<X.Y.Z>
git status  # 确认本地无未提交修改
git stash list  # 确认无 stash
git worktree list  # 确认无遗留 worktree
git checkout develop/v<X.Y.Z>
git pull --ff-only
```

- [ ] 本地 `develop/v<X.Y.Z>` 与 `origin/develop/v<X.Y.Z>` 同步
- [ ] 工作树 clean
- [ ] 无 stash
- [ ] 无遗留 worktree

### 0.3 Gitea 访问验证

```bash
# 测试 Gitea API
curl -s -X GET "http://<gitea>/api/v1/repos/<org>/<repo>" | jq .name

# 测试 PR 创建（用 --dry-run 不会真创建）
curl -s -X POST "http://<gitea>/api/v1/repos/<org>/<repo>/pulls" \
    -H "Content-Type: application/json" -d '{}'
```

- [ ] Gitea HTTP API 可访问
- [ ] 认证 token 有效
- [ ] 可以创建 Issue + PR

---

## Phase 1: 调研 (Investigation)

### 1.1 Subagent 并行调研 (推荐)

3 subagent 并行调研，节省 ~5h 人工时间。

#### Subagent #1: F-XX + I-12 实际状态

**Prompt 模板**:
```
你是 SQLRustGo 仓库的代码分析 subagent。请独立调研 v<X.Y.Z> 中 [16/32] F-XX + I-12
功能的实际实现状态。

工作目录: /home/openclaw/workspace/dev/sqlrustgo

**功能列表**: [F-XX 列表]

**对每个功能核对**:
1. 设计: docs/releases/v<X.Y.Z>/specs/debt/FXX_*.md 是否真实存在？
2. 测试代码: tests/<fxx>_test.rs 存在？测试名与 SPEC 一致？
3. 测试是否真实运行: git log --oneline -- <test_file> 最近 30 天有 commit？
4. 门禁集成: scripts/gate/check_*.sh 中验证？
5. 集成到主路径: src/execution_engine.rs 真调用？还是孤立测试？

**输出格式** (Markdown 表格):
对每个 F-XX:
| 维度 | 状态 | 证据 |
| 真实评级 | ✅ TRUE 100% / ⚠️ PARTIAL / ❌ STALE |

**最后汇总**:
- 真实 100% CLOSED: N 个
- PARTIAL: M 个
- STALE: K 个
- 真实问题清单
```

#### Subagent #2: INT / ARCH / SEM 集成债务

**Prompt 模板**:
```
你是 SQLRustGo 仓库的代码分析 subagent。请独立调研 v<X.Y.Z> 中跨版本集成债务。

工作目录: /home/openclaw/workspace/dev/sqlrustgo

**范围**: 
- INT-XX 集成债务 (来自 docs/releases/v<X.Y.Z>/debt/CROSS-VERSION-DEBT.md)
- ARCH-X 架构债务
- SEM-X 语义债务

**对每项核对**:
1. 代码现状: 关键符号 (rg "<symbol>" crates/ src/)
2. PR 状态: git log --all --oneline --grep="<keyword>"
3. 门禁脚本: scripts/gate/check_int_debt.sh, check_arch_sem_debt.sh
4. 文档冲突: 多个 .md 文档是否一致？

**输出格式**:
对每个 INT/ARCH/SEM:
| ID | 主题 | 文档 | 真实 | 门禁 | 真实结论 |

**最后汇总**:
- 真实 CLOSED: N 个
- PARTIAL: M 个
- OPEN: K 个
- 文档冲突清单
- 门禁 BUG 清单
```

#### Subagent #3: 旧债务 F-01~F-36 + I-XX + T-XX

**Prompt 模板**:
```
你是 SQLRustGo 仓库的代码分析 subagent。请独立调研 v<X.Y.Z> 中来自 v<X.Y-2>.0 era 的旧债务。

工作目录: /home/openclaw/workspace/dev/sqlrustgo

**范围**: 
- F-01~F-36 (36 项功能缺口)
- I-01~I-12 (12 项集成缺口)
- T-01~T-20 (20 项测试缺口)

**对每项核对**:
1. 代码现状: rg "<symbol>" crates/ src/
2. 测试代码: tests/<fxx>_test.rs
3. SPEC: docs/releases/v<X.Y.Z>/specs/debt/FXX_*.md
4. 门禁: scripts/gate/check_*.sh

**输出格式**:
| ID | 主题 | 文档声称 | 代码实际 | 真实评级 |
| 评级**: ✅ CLOSED / ⚠️ PARTIAL / ❌ STALE / ❌ OPEN

**最后汇总**:
- 真正 CLOSED: N
- PARTIAL: M
- STALE: K (声称 PASS 实际不达标)
- OPEN: W
- 完全无实现: P (0 commits / 0 files)
```

### 1.2 直接读核心文档

| # | 文档 | 必读章节 | 关注 |
|---|------|----------|------|
| 1 | `docs/releases/v<X.Y.Z>/FEATURE_MATRIX.md` | §11 (v<X.Y.Z> 新增), §14 (F-XX 完整列表) | 内部矛盾, 100% CLOSED 声称 |
| 2 | `docs/releases/v<X.Y.Z>/debt/INT5_PLUS_DEBT_INVENTORY.md` | §F-XX / §I-XX / §T-XX | Status Summary 是否真实 |
| 3 | `docs/releases/v<X.Y.Z>/debt/CROSS-VERSION-DEBT.md` | §INT-1~4 | 状态机字段 vs 实际 |
| 4 | `docs/releases/v<X.Y.Z>/ga/GA_GATE_CHECKLIST.md` | §7.5 Cross-Version Debt Summary | INT 状态 vs 最新 PR |
| 5 | `docs/releases/v<X.Y.Z>/V380_COMPREHENSIVE_ASSESSMENT.md` (或类似) | §1.2 D7 row | D7 ACTIVE 数量 |
| 6 | `docs/releases/v<X.Y.Z>/rc/RC_GA_GATE_REPORT.md` | §D6 Cross-Version Debt Delta | 11 ACTIVE / 0 CLOSED 声称 |
| 7 | `docs/releases/v<X.Y.Z>/archived/INT_DEBT_REMEDIATION_PLAN.md` | §1 INT-1, §4 INT-4 | ACTIVE vs CLOSED |
| 8 | `docs/releases/v<X.Y.Z>/archived/ARCH_SEM_DEBT_REMEDIATION_PLAN.md` | §1 ARCH-1, §5 SEM-2 | 行数 / 状态 |
| 9 | `docs/releases/v<X.Y.Z>/FEATURE_MATRIX.md` | §1.5 vs §11 vs §14 | F-11 矛盾 |

### 1.3 Gate 脚本源码审计

5 个核心 gate 脚本:

| 脚本 | 检查内容 | BUG 检测点 |
|------|----------|-----------|
| `check_int_debt.sh` | INT-1~4 状态追踪 | 路径失效（PR reorg 后）→ 假 PASS |
| `check_arch_sem_debt.sh` | ARCH-1~3 + SEM-1~4 | 硬编码状态（未跟 PR 更新）|
| `check_arch2_no_bypass.sh` | ARCH-2 storage.* bypass | 21+ bypass 是真实 regression |
| `check_cross_version_debt.sh` | F-01~F-36 + I-XX + T-XX | 只解析 markdown，不查代码 |
| `check_execution_semantics.sh` | SGL-001~005 | 主动排除主路径文件 |

### 1.4 实际跑门禁脚本

```bash
# 跑所有相关 gate
for s in check_int_debt check_arch_sem_debt check_arch2_no_bypass \
         check_cross_version_debt check_execution_semantics \
         check_alpha_v380 check_rc_ga_gate check_full_gate_verification; do
    echo "=== $s ==="
    bash scripts/gate/$s.sh 2>&1 | tail -10
    echo "EXIT: $?"
done
```

- [ ] 记录每个脚本的真实 EXIT code（不是文档声称）
- [ ] 对比 vs 文档声称（PASS/FAIL/DRIFT）
- [ ] 标记"文档 vs 实际"差异

---

## Phase 2: 跟踪 (Tracking)

### 2.1 审计报告

写入 `docs/releases/v<X.Y.Z>/historical/LEGACY_ISSUES_<YYYY-MM-DD>_AUDIT.md`，参考 `docs/releases/v3.8.0/historical/LEGACY_ISSUES_2026-06-05_AUDIT.md` 模板：

- 271 行结构
- 6 节 + 1 节 (跨版本引用)
- 13+ 项治理债务清单
- 30+ 文件路径 + 100+ 代码引用

### 2.2 创建 Gitea Issues

每项治理债务创建 1 个 Issue:

```bash
API="http://<gitea>/api/v1/repos/<org>/<repo>/issues"

# 模板函数
create_issue() {
    local id="$1"
    local title="$2"
    local priority="$3"  # P0/P1/P2
    local body="..."
    curl -s -X POST "$API" \
        -H "Content-Type: application/json" \
        -d "{\"title\": \"[$priority] $title\", \"body\": \"$body\"}"
}
```

**Body 模板**:
```markdown
## 来源

v<X.Y.Z> 严重遗留问题审计 (<date>) — PR #<audit>

审计文件: `docs/releases/v<X.Y.Z>/historical/LEGACY_ISSUES_<date>_AUDIT.md`

## 优先级

**[🔴 P0 / 🟡 P1 / 🟢 P2]** (GA 阻断 / v<X.Y.Z+1> 修复 / 清理)

## 问题描述

[2-3 段真实描述，含具体行号 / commit / 测试名]

## 整改方向

[1-2 段具体修复任务]

## 估计时间

[1h / 1d / 1w]

## 关联

- 报告章节: §<N>
- 审计 PR: #<NNNN>
```

- [ ] 创建 I#DEBT-v<X.Y.Z>-001 ~ 013 (13 个 P0/P1 治理债务)
- [ ] 每个 Issue 包含 5 要素（来源/优先级/描述/方向/时间）
- [ ] 优先级标记（🔴 P0 / 🟡 P1 / 🟢 P2）

### 2.3 审计 PR 关联

合并审计 PR 后，添加评论列出所有 Issue:

```bash
curl -s -X POST "<api>/issues/<audit_pr>/comments" \
    -H "Content-Type: application/json" \
    -d "{\"body\": \"## 关联 N 个治理 Issue\\n\\n本审计报告已开 N 个 I#DEBT-v<X.Y.Z>-XXX...\"}"
```

---

## Phase 3: 整改 (Remediation)

### 3.1 优先级排序

| 优先级 | 处理 | 时间 |
|---|---|---|
| P0 (GA 阻断) | 必做 | 1-2h 每个 |
| P1 (文档/集成) | 必做 | 1d |
| P2 (清理) | 可选 | 时间允许 |
| 1 周+ 工作量 | 转 follow-up Issue | - |

### 3.2 5 步文档流程（每个修复）

参考 `DOC_CHECK_CORRECTION_RULES.md`：

- [ ] **步骤 1: 发现问题** — 列出 oldString → newString
- [ ] **步骤 2: 改正计划 + Checklist**
- [ ] **步骤 3: 执行** — `edit` 工具最小修改
- [ ] **步骤 4: Checklist 核查**
- [ ] **步骤 5: 编写工作报告**

### 3.3 Worktree 流程

```bash
# 1. 创建 worktree
git worktree add .worktrees/<fix-name> -b fix/<NNN>-<topic> develop/v<X.Y.Z>

# 2. 在 worktree 中修改
cd .worktrees/<fix-name>
# ... 修改代码

# 3. 测试
cargo check --lib  # 快速编译检查
cargo test --test <test_name>  # 实际跑

# 4. commit
git add <files>
git commit -m "fix(gate): #<NNN> <topic>

[详细 commit body, 含 P0/P1 标签, PR 关联, 证据]"

# 5. push + PR + merge
git push gitea fix/<NNN>-<topic>
curl -X POST "<api>/pulls" -d '{...}'
curl -X POST "<api>/pulls/<pr>/merge" -d '{"Do": "merge"}'

# 6. 主 worktree 同步
cd /home/openclaw/workspace/dev/sqlrustgo
git fetch origin develop/v<X.Y.Z>
git merge --ff-only FETCH_HEAD

# 7. 清理
git worktree remove .worktrees/<fix-name> --force
git branch -D fix/<NNN>-<topic>
```

### 3.4 1 周工作 1-2h 拆分

```
调查 5min → 创建 worktree 1min → 写代码 30min → 测试 10min 
→ 发现根本障碍 5min → 撤回 1min → 开 follow-up Issue 3min 
→ 清理 1min → 报告给用户

# 总: 1h
# 输出: 1 follow-up Issue 跟踪实际工作
```

**关键原则**:
- 宁愿撤回也不强推
- 留 1-2 follow-up Issue 给后续 sprint
- 真实工程节奏优先

### 3.5 关闭 Issue

按 `ISSUE_CLOSING_VERIFICATION.md`:

```bash
# 1. PR 合并后自动 close (if PR body has "Closes #<id>")
# 2. 或手动 close (需 PR 关联证据)
curl -X PATCH "<api>/issues/<id>" -d '{"state": "closed"}'

# 3. 添加 cross-reference 评论
curl -X POST "<api>/issues/<id>/comments" -d '{
    "body": "## ✅ 已被 PR #<PR> 修复 (merged <date>)\n\n..."
}'
```

- [ ] 每个 P0/P1 Issue 都有 PR 关联
- [ ] PR 合并后 Issue 自动/手动关闭
- [ ] cross-reference 评论加好

---

## Phase 4: 验证 (Verification)

### 4.1 8 维门禁

```bash
# 主 5 维 + 1 测试 + 1 全验证
bash scripts/gate/check_rc_ga_gate.sh ga
bash scripts/gate/check_full_gate_verification.sh
bash scripts/gate/audit_testing.sh v<X.Y.Z> ga artifacts/audit/v<X.Y.Z>
```

- [ ] D1-D5 + D6a + C-ARCH-05 + D6b + D7 + D8 + D9 全部跑过
- [ ] 每个 EXIT code 记录
- [ ] 对比修前/修后状态

### 4.2 Code Reality Check

```bash
bash scripts/gate/check_cross_version_debt.sh
```

- [ ] Part 5 检测 N 孤岛 F-XX
- [ ] Part 5 检测 M 无实现债务
- [ ] 输出真实状态（非文档声称）

### 4.3 D6 证据刷新

```bash
# 跑核心 16 个 F-XX + I-12 测试
for t in aggregate_functions_test distinct_test gap_locking_test \
         adaptive_hash_index_test clustered_index_test change_buffer_test \
         double_write_buffer_test table_compression_test row_level_security_test \
         performance_schema_test mysqladmin_test password_rotation_test \
         parallel_executor_test f11_f12_executor_test savepoint_test \
         wal_integration_test; do
    timeout 30 cargo test --test $t 2>&1 | grep "test result"
done
```

- [ ] 16 个核心 F-XX 测试全 PASS（130+ tests）
- [ ] 更新 `artifacts/gate/v<X.Y.Z>/d6_test_inventory.json` 时间戳

### 4.4 文档同步

```bash
bash scripts/gate/check_docs_links.sh
bash scripts/gate/check_docs_consistency.sh
```

- [ ] 7 份 STALE 文档同步
- [ ] doc 链接 PASS
- [ ] 一致性 PASS

---

## Phase 5: 报告 (Reporting)

### 5.1 GA 治理示范报告

写入 `docs/governance/GA_GOVERNANCE_DEMO_v<X.Y.Z>.md`，参考 `GA_GOVERNANCE_DEMO_v3.8.0.md` 模板：

- 400+ 行
- 7-10 节：背景 / 调研方法 / 整改工作流 / 产出物 / Gate 真实状态 / 规范化 / 教训
- 引用所有 PR + Issue + commit
- 中文 + 表格 + 一句话总结

### 5.2 更新现有治理文档

- [ ] `docs/governance/DEBT_TRACKING.md`（如存在）— 加上 v<X.Y.Z> 关闭记录
- [ ] `docs/governance/lessons/LESSON_v<X.Y.Z>_GA.md`（如模式）— 写本版本教训

---

## 输出清单

完成所有阶段后，输出：

- [ ] 1 份审计报告（`docs/releases/v<X.Y.Z>/historical/LEGACY_ISSUES_<date>_AUDIT.md`，~270 行）
- [ ] 1 份治理示范报告（`docs/governance/GA_GOVERNANCE_DEMO_v<X.Y.Z>.md`，~400 行）
- [ ] 1 份本 checklist 归档（`docs/releases/v<X.Y.Z>/historical/LEGACY_AUDIT_<date>.md`）
- [ ] 13+ Gitea Issue（I#DEBT-v<X.Y.Z>-001~013+）
- [ ] 12+ PR 合并到 develop/v<X.Y.Z>
- [ ] 3+ Follow-up Issue（1 周+ 工作量）
- [ ] 8 维门禁 PASS（含 Code Reality 新增）
- [ ] D6 证据刷新（130+ tests PASS）
- [ ] 7+ STALE 文档同步

---

## 时间预算

| Phase | Subagent 调研 | 直接读文档 | Gate 审计 | 修复 + PR | 报告 |
|--------|---------------|-----------|-----------|-----------|------|
| Time | ~2h | ~1h | ~30min | ~3h (12 issues) | ~30min |
| % | 28% | 14% | 7% | 43% | 7% |

Total: ~7h (1 session)

**节省**: 3 subagent 并行节省 ~5h 调研时间
**风险**: 1 周+ 工作强行 1-2h 拆分 → 撤回 + follow-up（避免赶工 bug）

---

## 复用案例

| 版本 | 文档 | 工作量 | 状态 |
|------|------|--------|------|
| v3.8.0 | `LEGACY_ISSUES_2026-06-05_AUDIT.md` (271 行) + `GA_GOVERNANCE_DEMO_v3.8.0.md` (400 行) | 7h, 12 issues closed, 3 follow-up | ✅ COMPLETED 2026-06-05 |
| v3.9.0 | (待复用) | (预估 5-7h) | 🔜 GA 前 1-2 周 |
| v4.0.0 | (待复用) | (预估 5-7h) | 🔜 大版本前 1-2 月 |

---

## 维护

| 项目 | 值 |
|------|-----|
| 文档版本 | LEGACY_AUDIT_CHECKLIST-1.0 |
| 最后更新 | 2026-06-05 |
| 维护者 | Hermes Agent |
| 状态 | ACTIVE |
| 关联 | `PATTERN_LEGACY_AUDIT_FOLLOWUP.md`, `GA_GOVERNANCE_DEMO_v3.8.0.md` |
