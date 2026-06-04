# v3.8.0 RC1 阶段 — 后续工作分析报告 (2026-06-04)

> **Date**: 2026-06-04 14:50
> **Author**: Hermes Agent
> **状态**: 实时状态快照 (基于 Gitea API 拉取的最新数据)
> **Gitea 状态**: ✅ 已恢复, broken ref 已修复, develop/v3.8.0 = a77b7cda

---

## 0. TL;DR

| 维度 | 数据 |
|------|------|
| **Open Issues (真)** | **2** (#3058 等待 close, #2948 待修) |
| **Open PRs (真)** | **1** (PR #3058 `fix/v380-rc1-tpch-22of22`, mergeable: false) |
| **Closed Issues (recent 24h)** | 35+ (本会话 + 之前 sessions) |
| **Merged PRs (recent 24h)** | 30+ (含本会话 PR #3060 #3061) |
| **Active worktrees** | 2 (`.worktrees/cli3` + `.worktrees/resolve` 其他 sessions) |
| **冲突 PR** | 1 (PR #3058 vs develop, 唯一文件 parser.rs) |

---

## 1. PR #3058 冲突分析 (当前唯一阻塞项)

### 1.1 状态
- **PR**: #3058 (openclaw 拥有)
- **Title**: "v3.8.0-rc1 Stage 2: Corpus 89.3% → 92.7% + TPCH-01 Q1 PASS verified"
- **Branch**: `fix/v380-rc1-tpch-22of22` (head `824edca2`, base `10f982e2`)
- **关联 Issue**: #3058 (state=open, 等 close)
- **Worktree**: `.worktrees/resolve` @ `624fccdc` (其他 session 拥有)
- **mergeable**: **false** ❌
- **additions**: 222, **deletions**: 5, **changed_files**: 5

### 1.2 PR #3058 实际改的 5 files
| File | Changes | Status |
|------|---------|--------|
| `crates/parser/src/parser.rs` | +43 行 (Token::If + POSITION 特殊 form) | ❌ CONFLICT |
| `docs/releases/v3.8.0/STAGE2_TPCH_CORPUS_REPORT.md` | +163 行 (新文件) | ✅ auto-merge OK |
| `sql_corpus/DML/SELECT/join_combinations.sql` | +7 (SETUP block) | ✅ auto-merge OK |
| `sql_corpus/DML/SELECT/outer_join.sql` | +6 (SETUP block) | ✅ auto-merge OK |
| `sql_corpus/DML/SELECT/self_join.sql` | +8 (SETUP block) | ✅ auto-merge OK |

### 1.3 冲突精确定位 (parser.rs 2 区域)
**Gitea 算的 merge_base**: `504b4f796be43a6d9aff1c90cb398eb3686b7cc7` (PR #3048 merge 后)

#### 冲突 1: line 1768-1782 (14 lines)
- **PR #3058 改**: 在 `parse_select_statement` main column list match 加 `Token::If` (LEFT/RIGHT/INSERT/REPLACE/IF)
- **Develop 改** (PR #3060 + 之前 sessions): `Token::Level` bare column accept (同位置)
- **不重叠**: 一个改 If, 一个改 Level
- **修法**: 两边都保留 (合并 5 个 token 一起处理)

#### 冲突 2: line 3232-3327 (95 lines)
- **PR #3058 改**: 在 `parse_primary_expression` 加 `Token::If` + `POSITION(x IN y)` 特殊 form
- **Develop 改** (PR #3060): `Token::Level` 兜底 (line 3547)
- **不重叠**: 改不同 token + 不同函数
- **修法**: 两边都保留 (加 IF/POSITION/Level 各自处理)

### 1.4 为什么 PR #3058 跟 develop 差异巨大 (35 files, 1828 deletions)
之前 `git diff` 显示 35 files 改, **但实际 PR #3058 只改 5 files**. 30 files 差异来自 **base 10f982e2 跟当前 develop 差距**:
- base = 2026-06-04 11:01:26 (PR #3048 之后)
- develop = 2026-06-04 14:38 (PR #3061 之后)
- 20 commits 在 base 之后 merged: #3049 #3050 #3051 #3052 #3053 #3054 #3055 #3056 #3057 #3059 #3060 #3061

PR #3058 看到 develop 推进, 想要 rebase 但 base SHA `10f982e2` 已不在 Gitea (Gitea 算 merge base = `504b4f79`).

### 1.5 修法 (供 openclaw 决策)

#### Option A: 在 `.worktrees/resolve` rebase (推荐)
```bash
cd .worktrees/resolve
git fetch origin develop/v3.8.0
git rebase origin/develop/v3.8.0
# 修 parser.rs 2 区域冲突 (保留 PR #3058 的 If+POSITION + develop 的 Level)
# 4 corpus + 1 doc auto-merge 无需修
git add crates/parser/src/parser.rs
git rebase --continue
git push --force-with-lease
```
- **ETA**: 30-60 min (人手修 conflict markers)
- **风险**: 中 (force-push 自己的 branch OK)

#### Option B: 关闭 PR #3058, 重提新 PR
```bash
# close PR #3058
gh pr close 3058 --comment "Superseded by rebase; new PR coming"
# 在新 branch 上重做 (从 develop 拉新基线 + 应用 PR #3058 5 files 改动)
git checkout -b fix/v380-rc1-tpch-22of22-retry origin/develop/v3.8.0
# apply 5 files 改动
git commit -m "..."
git push origin fix/v380-rc1-tpch-22of22-retry
# create PR
```
- **ETA**: 1-2h (重做 + 验证)
- **风险**: 低 (clean base, 无 conflict)

#### Option C: 管理员 force_merge (跳过 conflict 检查)
```bash
curl -X POST "$GITEA/api/v1/repos/openclaw/sqlrustgo/pulls/3058/merge" \
  -H "Authorization: token $TOKEN" -d '{"Do":"merge","force_merge":true}'
```
- **ETA**: 1 min
- **风险**: **高** (会丢 develop 上 20 commits 的改动! 因为 git 不知道怎么合并, 会用 PR 侧的版本覆盖)

### 1.6 关联 #3058 close
修完 PR 之后:
```bash
gh issue close 3058 --comment "Closed via PR #3058 (merged: <merge_sha>)"
```
per `docs/governance/ISSUE_CLOSING_VERIFICATION.md §3.1` (有 PR 关联)

---

## 2. 全部 Open Issues (2 个)

### 2.1 #3058 [P1-pending] - PR 合并即 close
- **Title**: v3.8.0-rc1 Stage 2: Corpus 89.3% → 92.7% + TPCH-01 Q1 PASS verified
- **State**: open
- **关联**: PR #3058 (open + mergeable=false)
- **实际工作**: 已完成 (92.7% corpus, TPC-H Q1 PASS, 22/22 deferred to rc2)
- **下一步**: 修 PR #3058 conflict → merge → close #3058

### 2.2 #2948 [P1] - 待修 8h+
- **Title**: Track 3: Real-data wire-protocol TPC-H at SF>=1 (server-side bulk loader)
- **State**: open
- **ETA**: 8h+ (server-side bulk loader 实现)
- **依赖**: Track 1 (PR #2929 ✅), Track 2 (parser fixes 进行中)
- **关联**: #2977 TPC-H 22/22 配套 (Track 1+2+3 = 22/22)
- **下一步**: 
  - 先 close #2977 (per timeline 关联是 PR #3059 parser arithmetic, 不是 full 22/22) — per §3.3 应保持 open, 不应 close
  - 决定: reopen #2977 + new sub-issue for Track 3
  - 或: 让 #2948 单独推进, 完成后一起处理 #2977

---

## 3. 全部 Open PRs (1 个)

| PR | Title | Status | Owner | mergeable | Conflict |
|----|-------|--------|-------|-----------|----------|
| #3058 | Stage 2 Corpus 92.7% + TPC-H Q1 | open | openclaw | **false** | parser.rs 2 区域 |

**唯一阻塞项**: PR #3058 conflict. 修法见 §1.5.

---

## 4. Closed Issues 验证 (§3.1 PR 关联)

| # | Title | Closed at | State | Closing PR (from PR list) | 验证 |
|---|-------|-----------|-------|---------------------------|------|
| #2987 | CTE-01 10 fails | 2026-06-04 12:55:15 | closed | PR #3052 (squash) | ✅ merged (本会话辅助) |
| #2973 | INT-4 VtuGuard | 2026-06-04 12:39:48 | closed | PR #3051 (squash) | ✅ merged (本会话 PR) |
| #2974 | ARCH-2 DML | 2026-06-04 10:09:56 | closed | PR #3038 (merge) | ✅ merged |
| #2874 | P0-1 35 missing tests | 2026-06-03 15:49:53 | closed | PR #2897 + #2899 | ✅ merged (前 sessions) |
| #3031 | Bata 完成公告 | 2026-06-04 09:28:49 | closed | commit 34cd64d7 (bata branch) | ✅ |
| #2937 | F-32 mysqladmin | 2026-06-04 00:32:34 | closed | PR #2848 | ✅ |
| #2743 | R5 Coverage | 2026-06-04 09:28:49 | closed | (c2919595b) | ✅ |
| #2763 | 进展报告 | 2026-06-04 09:28:49 | closed | (Beta 公告取代) | ✅ |
| #2988 | MySQL-01 26 fails | 2026-06-04 13:16:16 | closed | PR #3057 (#2988 sub-A) | ⚠️ partial (15/26 仍 failing, 但 issue closed) |
| #2977 | TPCH-01 22/22 | 2026-06-04 13:36:06 | closed | PR #3059 (parser arithmetic) | ⚠️ partial (8/22 → 12/22, 22/22 deferred) |
| #3047 | MySQL-01 phase-3c | 2026-06-04 12:13:37 | closed | PR #3047 (merged) | ✅ |

**所有 closed 都有 PR 关联** (per §3.1). 注意 #2988 #2977 是 partial close (issue state=closed 但实际工作 partial, 这是历史 reopen/close 决策, 跟本会话无关).

---

## 5. Recent Merged PRs (last 24h, 30+ 个)

| 时段 | PR | Title | 类别 |
|------|-----|-------|------|
| **本会话** | **#3061** | **V380 RC1 Handover Report** | **docs** |
| **本会话** | **#3060** | **Issue #2987 CTE-01 10/10** | **fix(parser+corpus)** |
| 同期 | #3059 | tpch-01 parser arithmetic | fix(parser) |
| 同期 | #3057 | MySQL 5.7 TRIM | fix(parser+executor) |
| 同期 | #3056, #3055, #3041 | Phase 2a wire protocol tests | feat(tests) |
| 同期 | #3054 | Bata → Beta spelling | docs |
| 同期 | #3053, #3049 | mysql-server EphemeralConfig | fix |
| 同期 | #3052 | 10/10 CTE-01 (openclaw) | fix(parser+corpus) |
| 同期 | #3051, #3050 | INT-4 explicit TX | fix(executor) |
| 同期 | #3048, #3042, #3062 | CLI-01 REPL Stages 2/3 | feat(cli) |
| 同期 | #3047, #3045, #3043 | MySQL-01 phases 3b/3c/follow-up | feat(parser+executor) |
| 同期 | #3046, #3044 | SERVER-01 stages | feat(server) |
| 同期 | #3040 | 3 of 10 CTE-01 (openclaw) | fix(parser+corpus) |
| 同期 | #3039 | OpenSpec planning entry | chore |
| 同期 | #3038 | ARCH-2 Stage 1 | fix(arch) |
| 同期 | #3037, #3036, #3035, #3034, #3033, #3032, #3031, #3030, #3029 | docs + tests | various |

---

## 6. 后续工作优先级建议

### 6.1 P0 立即 (1-2h) — 修 PR #3058 conflict

**根因**: PR #3058 5 files 中 parser.rs 跟 develop 重叠改动 (Token::If/POSITION vs Token::Level)

**修法** (建议 Option A, in `.worktrees/resolve`):
```bash
cd .worktrees/resolve
git fetch origin develop/v3.8.0
git rebase origin/develop/v3.8.0
# 修 parser.rs 2 冲突区 (保留 If+POSITION from PR + Level from develop)
# 4 corpus + 1 doc auto-merge OK
git add crates/parser/src/parser.rs
git rebase --continue
git push --force-with-lease
```

**结果**:
- PR #3058 mergeable: true
- merge squash → develop HEAD advance
- close #3058 (per §3.1)
- 累计 RC1 PRs = 28+

### 6.2 P1 短期 (1-3 天) — RC1 backlog 推进

#### 6.2.1 TPC-H 22/22 (#2977 partial)
- 当前: 12/22 (Q1 PASS, 11 deferred)
- 已知 parser fix 路径 (per Bata 报告):
  - Q3, Q5, Q7, Q8, Q9, Q10, Q14, Q19: 已知 executor fix (PR #3059 修了 parser arithmetic)
  - Q2, Q7, Q9, Q13, Q21: qualified-alias join (n1.n_name = ...)
  - Q8, Q17, Q20, Q22: nested subquery with outer reference
  - Q15: comma-list + subquery in FROM (建议改写为 JOIN)
- ETA: 40h+ (per issue)
- Track 3 (#2948) 配套: 8h+

#### 6.2.2 MySQL 5.7 5/26 → 26/26 (#2988 partial)
- phase 3c (PR #3047) 已 merge
- PR #3057 (TRIM LEADING/TRAILING/BOTH) 已 merge
- 剩余 15 cases: DATE_ADD INTERVAL, GROUP_CONCAT (已), ROLLUP/CUBE (已部分), ST_*, CTE DML
- ETA: 15h+

#### 6.2.3 Keyword-as-identifier 批量修 (3-4h, 净 corpus +30 cases)
- 同 #2987 (Token::Level) 处理方式
- Token::Group, Token::Date, Token::Position (注意: PR #3058 修了 POSITION), Token::Left, Token::Json, Token::Nulls
- 一次性在 parse_primary_expression + parse_select_statement main match 加兜底
- 跟踪: 新增 `scripts/gate/check_keyword_column.sh` (监控 keyword-column fallback 数量)

### 6.3 P2 中期 (1-2 周) — Frozen to v3.9.0+ 之外
per `docs/releases/v3.8.0/V380_FROZEN_TO_V390.md`:
- SIMD / Parallel 主路径: FROZEN
- Vector SQL: FROZEN
- 新优化器: FROZEN
- ROLLUP/CUBE/INSERT 函数: FROZEN

### 6.4 P3 长期 — GA 阶段准备 (RC1+RC2 完整后)
- 72h 稳定性验证 (gate 已有 `long_run_stability_test`)
- D6b 88% → 95% → 100%
- Bata 标 v3.8.0-beta → rc1 → rc2 → ga
- Bata 公告 `34cd64d7` bata branch → 主线 (develop/v3.8.0) 推进

---

## 7. 工具/资源

| 资源 | 路径/命令 |
|------|-----------|
| 当前 Gitea HEAD | `a77b7cda` (develop/v3.8.0) |
| 本地 HEAD | `9cfe482a` (1 commit ahead of gitea, squash-equivalent) |
| 冲突 PR | #3058 (`fix/v380-rc1-tpch-22of22`) |
| 其他 session worktree | `.worktrees/cli3` (cli3-file-storage), `.worktrees/resolve` (corpus-tpch-stage2-v2) |
| Recovery scripts | `/tmp/gitea_recovery_actions.sh` (历史) + 本报告 (新) |
| Bata 完成报告 | `docs/releases/v3.8.0/V380_BATA_COMPLETION_REPORT.md` |
| RC1 Handover | `docs/releases/v3.8.0/V380_RC1_HANDOVER_REPORT.md` (本会话 PR #3061) |
| Issue 关闭规范 | `docs/governance/ISSUE_CLOSING_VERIFICATION.md` |
| Frozen 列表 | `docs/releases/v3.8.0/V380_FROZEN_TO_V390.md` |
| Gitea 完整 repo bundle | `/tmp/gitea_repo_backup/repo_complete.bundle` (139MB, 备份) |

---

## 8. 引用资源

- **Gitea**: http://192.168.0.252:3000/openclaw/sqlrustgo
- **本会话 PR #3060**: merged @ dd7d7fb0 (Issue #2987 CTE-01 10/10)
- **本会话 PR #3061**: merged @ a77b7cda (V380 RC1 Handover Report)
- **PR #3052** (openclaw): 10/10 CTE-01 base (含 Token::Level accept)
- **PR #3059** (claude-macmini): tpch-01 parser arithmetic
- **PR #3057** (claude-macmini): MySQL 5.7 TRIM
- **PR #3047** (claude-macmini): MySQL-01 phase-3c aggregates
- **PR #3048** (claude-macmini): CLI-01 Stage 2 REPL 持久化
- **PR #3062** (claude-macmini): CLI-01 Stage 3 cross-session

---

## 9. 建议下一步 (本会话已完成, 下个 session 接续)

1. **用户 (openclaw) 决策**: 选 Option A (rebase) / B (重提) / C (force_merge) 修 PR #3058
2. **下个 session 修 PR #3058 conflict** (推荐 Option A in `.worktrees/resolve`, 30-60min)
3. **close #3058** per §3.1
4. **继续 RC1 backlog**: TPC-H 22/22 (#2977) + MySQL 5.7 26/26 (#2988)
5. **新增 keyword-as-identifier 批量修** (净 corpus +30 cases, 3-4h)
6. **72h 稳定性验证** (rc1 → rc2 → ga 推进)

**当前状态**: ✅ RC1 阶段启动就绪, 仅 1 阻塞项 (PR #3058 conflict), 1 真 open issue (#2948).
