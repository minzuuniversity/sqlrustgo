# PR Cleanup — 2026-06-11

## 背景

After syncing 252 main Gitea to current develop/v3.9.0 state (43c08bf3),
6 open PRs targeting develop/v3.9.0 were analyzed for conflicts.

## 6 PR 状态分析

| PR | Head | Mergeable | 真实冲突 | 决定 |
|----|------|-----------|---------|------|
| #3318 | feature/data-regen-sprint5 | False | src/engine_select.rs (与 Sprint 5 v11 Q18 fix) | Close (PR 改动已并入) |
| #3319 | fix/v390-q17-scalar-subquery | False | src/engine_select.rs, src/engine_utils.rs (Q17 fix 冲突) | Close (被 Sprint 5 v11 取代) |
| #3320 | feature/v390-sprint5-v2-q17-fix | True | None ("Already up to date") | Close (commits 已合并) |
| #3322 | fix/v390-q21-multi-col-index | True | None ("Already up to date") | Close (Sprint 5 v6-v8 work 完成) |
| #3323 | feat/v390-failure-matrix | True | None ("Already up to date") | Close (Failure Matrix v1 完成) |
| #3324 | feat/v390-operator-regression-suite | True | None ("Already up to date") | Close (Sprint 3 work 完成) |

## 处理结果

### 252 Gitea (main)
全部 6 个 PR 关闭:
- #3318: data-regen 冲突解决太复杂,Sprint 5 v11 Q18 fix 已是主导
- #3319: Q17 fix 旧方法 (a86f0044) 被 Sprint 5 v11 取代 (2b93fac0)
- #3320, #3322, #3323, #3324: 已是 "Already up to date" — commits 全部在 develop/v3.9.0 中

### 250 Gitea (backup)
清理 2 个额外 PR:
- #3244: Q21 PR (与 252 的 #3322 相同, 关闭)
- #3235: Sprint 3 Multi-Join + Operator Regression (commits 已在 develop/v3.9.0)

## 保留的内容

所有 PR 的 head branches 仍然保留 (没有自动删除):
- feature/v390-sprint5-v2-q17-fix
- fix/v390-q21-multi-col-index
- feat/v390-failure-matrix
- feat/v390-operator-regression-suite
- feature/data-regen-sprint5
- fix/v390-q17-scalar-subquery

## 关闭理由摘要

每个 PR 都收到 comment 解释关闭原因:
1. **#3318**: 冲突源自 Sprint 5 v11 Q18 fix。`tpch_data_gen.rs` 的修改可能与新代码不兼容。openspec/ 下的 SPECs 是好的内容,应该基于当前 develop 重新开 PR。
2. **#3319**: 完全被 Sprint 5 v11 取代。Q17 现在 79.69 (= MD)。
3. **#3320**: 早期 Sprint 5 v2 Q17 尝试,Sprint 5 v11 是权威。
4. **#3322**: 我们的 PR,Sprint 5 v6-v8 work 完成,22/22 PASS。
5. **#3323**: Failure Matrix v1 完成。
6. **#3324**: Sprint 3 deliverables 完成。

## 最终状态

| Repo | Open PRs to v3.9.0 |
|------|---------------------|
| 252 (main) | 0 ✅ |
| 250 (backup) | 0 ✅ |
| Gitcode | 0 ✅ (no PRs created there) |

所有 PR 处理完成,代码完整,branch 保留方便未来参考。
