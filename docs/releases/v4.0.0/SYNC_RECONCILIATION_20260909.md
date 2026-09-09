# SQLRustGo Local/Remote Sync Reconciliation Report

> **日期**: 2026-09-09
> **作者**: yinglichina
> **范围**: sqlrustgo develop/v3.12.0, main, release/v3.12.0, develop/v4.0.0
> **目的**: 记录 2026-09-09 sync 状态排查 + 修复 + 后续建议

---

## 1. 同步状态 (2026-09-09 排查时)

| 分支 | Local | .252 | .250 | gitcode | 状态 |
|---|---|---|---|---|---|
| develop/v3.12.0 | `cc19f779ad` | `9febebb255` | `9febebb255` | `9febebb255` | ❌ Local 落后 1 commit |
| main | `77f5dbb920` | `9febebb255` | `9febebb255` | `9febebb255` | ❌ Local 落后 1149 commits |
| release/v3.12.0 | `9febebb255` | `9febebb255` | `9febebb255` | `9febebb255` | ✅ SYNC |
| develop/v4.0.0 | `53d0fd7e0` | `967e95a81a` | `53d0fd7e0` | `53d0fd7e0` | ⚠️ .252 多 1 commit |

## 2. Divergence 根因分析

### 2.1 develop/v3.12.0 (Local 落后 1 commit)

**缺失 commit**: `9febebb25 fix(v312 / GA post-cut verification): refresh gate evidence at HEAD 355b5a3837`

**根因**: Hermes Agent 8-step cron 在 Phase 0 commit (`53d0fd7e0`) 后由 cron 自动 push 了 GA post-cut verification commit。这是 hermes-gateway 自动运行行为,即使我已 stop service, 之前的 in-flight cron run 已经产生了该 commit。

### 2.2 main (Local 落后 1149 commits)

**Local main HEAD**: `77f5dbb920` (V312-55A Procedure DDL lifecycle merge)
**.252 main HEAD**: `9febebb255` (V312 GA post-cut)
**Merge-base**: `9419a6fc4e` (V312-26 wire_smoke backpressure fix)

**Unique local**: 144 commits
**Unique .252**: 1149 commits

**根因**:
- Local main 走 **PR-merge chain** (V312-26 → V312-55 → V312-56 → V312-59 → GA)
- .252 main 走 **force-push + fast-forward chain** (因为 `main enable_push=false`)
- 两个 chain 的最终 source 一样, 但 local 的 chain 通过 `Merge PR #4259 from fix/v312-4019-3943-evidence-refresh` 等 PR merge commits 表达
- .252 的 chain 通过 GA 期间的 force-push (`git push gitea252 release/v3.12.0:main --force`) 一次性推进到 9febebb255

### 2.3 develop/v4.0.0 (.252 多 1 commit)

**Extra on .252**:
- `2baeee9ae docs(v4.0.0): record GMP-Platform integration verification`
- `967e95a81 Merge pull request #4866 from work/gmp-adapter-v400 into develop/v4.0.0`

**根因**: claude-z6g4 (.252 Claude Code CLI, PID 2334203) 自动 merge 了 #4866

**触发源**: PR #4866 由 yinglichina 创建 (2026-09-09 11:04:59 UTC), claude-z6g4 在 11:05:11 UTC merge — **claude-z6g4 是 active AI process 在 .252 上跑**

## 3. 修复动作

### 3.1 develop/v3.12.0

**动作**: `git fetch gitea252 develop/v3.12.0` + `git merge gitea252/develop/v3.12.0 --ff-only`

**结果**: Local = `9febebb255`, sync ✅

### 3.2 main

**方案选择**: 
- ❌ Cherry-pick 144 local commits 到 .252 main: 失败, 因为 **144 commits 全部已经在 develop/v3.12.0** (通过 PR merge chain), 没有 missing work
- ❌ Force-push .252 main → local main (77f5dbb920): 失去 GA promotion commits (1149 个) — bad
- ✅ Force-reset local main → .252 main (9febebb255): 144 local-only commits 全部已 reachable via develop/v3.12.0, 无 work loss

**动作**: `git reset --hard gitea252/main` (本地 main 强制对齐 .252)

**结果**: Local main = `9febebb255`, sync ✅

**关键 insight**: Local 的 144 commits 不是 "未推送的工作", 而是 local main 走 PR-merge chain 的历史, 全部内容已经 merge 到 develop/v3.12.0。force-reset main 是安全的。

### 3.3 develop/v4.0.0

**动作**:
1. `git fetch gitea252 develop/v4.0.0`
2. `git merge 967e95a81a --ff-only` (采纳 PR #4866 GMP-Platform integration verification 文档)
3. Push 到 .250 + gitcode (`.252` 已经有 967e95a81a)

**结果**: All 4 sources = `967e95a81a` ✅

## 4. 当前 Sync 状态 (修复后)

| 分支 | Local | .252 | .250 | gitcode | 状态 |
|---|---|---|---|---|---|
| develop/v3.12.0 | `9febebb255` | `9febebb255` | `9febebb255` | `9febebb255` | ✅ SYNC |
| main | `9febebb255` | `9febebb255` | `9febebb255` | `9febebb255` | ✅ SYNC |
| release/v3.12.0 | `9febebb255` | `9febebb255` | `9febebb255` | `9febebb255` | ✅ SYNC |
| develop/v4.0.0 | `967e95a81a` | `967e95a81a` | `967e95a81a` | `967e95a81a` | ✅ SYNC |

**GitHub 仍 blocked**: 251MB server.log 超过 100MB GitHub limit (Phase 0 task)

## 5. Background Process 审计

| Process | 状态 | 备注 |
|---|---|---|
| .252 hermes-gateway.service | failed (已 stop) | 之前 session 已 disable |
| .252 hermes-runner | disabled (dead) | 之前 session 已 disable |
| .250 runner-test container | UP 6d (no jobs) | not registered, no impact |
| .252 cron gitea-startup.sh (*/5) | active (healthcheck only) | no push activity |
| .252 cron gitea-backup.sh (daily 03:00) | active (backup only) | 3T disk backup |
| .252 cron hermes-ops-sync.sh (hourly) | **fails silently** (WIKI_DIR=/Users/liying/hermes-ops-wiki not exist on .252) | no impact to sqlrustgo |
| **claude-z6g4 (.252 Claude Code CLI)** | **ACTIVE** | PID 2334203, started 2026-09-08, auto-merges PR |
| Local launchd: ai.hermes.gateway | active (Hermes Agent backend) | 这是用户启动的 |

## 6. 发现的新问题

### 6.1 claude-z6g4 自动 merge PR

**问题**: `.252` 上 claude-z6g4 (Claude Code CLI) 会自动 merge PR。需要决定是否保留这个行为。

**风险**: 
- 任何带 `work/*` 前缀的 branch 上的 PR 可能被自动 merge
- 如果有 malicious PR, 会被自动 merge

**缓解**:
- 在 claude-z6g4 配置中限定: 只 merge `work/gmp-adapter-*` (这是已知 trusted branch)
- 或 disable claude-z6g4, 改为手动 merge

**当前状态**: 已合并 PR #4866 是合法的 docs(v4.0.0) commit, 内容 verified, 已采纳。

### 6.2 Hermes Agent 8-step cron 自动 push

**问题**: 即使 hermes-gateway service 已被 stop, 之前的 in-flight cron 8-step 已经 push 了 `9febebb25` 到 develop/v3.12.0。

**缓解**:
- 监控: 每次 fetch 后检查 unexpected commits
- 治理: 禁止任何 background process push 到 release/* / ga/* branches (PR-only)

## 7. 后续建议

### 7.1 Phase 0 必做

- [ ] 决定 claude-z6g4 自动 merge 策略 (限定 trusted branches 或 disable)
- [ ] 监控 Hermes Agent background activity (即使 service failed, 旧 cron 可能还在跑)

### 7.2 GitHub unblock

- [ ] git filter-branch 重写 history 删除 log/tbl/json 文件
- [ ] 或 git LFS 迁移 .250 backup 的 251MB server.log
- [ ] 重写后 push 到 GitHub yinglichina8848/sqlrustgo-public

### 7.3 持续 sync 监控

- [ ] 加 `scripts/check_repo_sync.sh` 在 CI 中, 4 remote HEAD 必须 sync
- [ ] 加 audit log: 任何 PR merge 必须有 PR number + author + reviewer trace
- [ ] v4.0.0 file-extension gate 实施 (per user 2026-09-08 rule)

## 8. 不丢失的工作验证

**断言**: Force-reset local main 后, 没有丢失任何代码或文档工作。

**验证方法**: 
```python
local_unique_144 = main_backup ^ remote_main  # 144 commits
all_in_dev = all(sha in rev_list(develop/v3.12.0) for sha in local_unique_144)
assert all_in_dev == True  # confirmed: 0 truly-unique commits
```

**结论**: 所有 144 commits 在 develop/v3.12.0 中已 reachable (通过 PR merge chain), main 的 force-reset 不丢任何东西。

## 9. 时间线

| 时间 (UTC) | 事件 |
|---|---|
| 2026-09-08 (Phase 0) | 创建 develop/v4.0.0 @ `9febebb255`, push 到 3 remote |
| 2026-09-08 (cron) | Hermes Agent 8-step cron push `9febebb25` 到 develop/v3.12.0 (GA post-cut) |
| 2026-09-09 11:04 | yinglichina 创建 PR #4866 (work/gmp-adapter-v400 → develop/v4.0.0) |
| 2026-09-09 11:05 | claude-z6g4 自动 merge PR #4866 (commit `967e95a81a`) |
| 2026-09-09 (this session) | 排查 sync divergence, fast-forward develop/v3.12.0, force-reset main, accept develop/v4.0.0 PR #4866 |
| 2026-09-09 (this session) | All 4 branches sync across 3 remotes ✅ |

## 10. 附录: git commands

```bash
# Fast-forward develop/v3.12.0
git fetch gitea252 develop/v3.12.0
git merge gitea252/develop/v3.12.0 --ff-only

# Force-reset local main to remote main (no work loss)
git fetch gitea252 main
git reset --hard gitea252/main

# Accept develop/v4.0.0 PR #4866
git fetch gitea252 develop/v4.0.0
git merge 967e95a81a --ff-only
git push gitea250 develop/v4.0.0:develop/v4.0.0 --force
git push gitcode develop/v4.0.0:develop/v4.0.0 --force
```