# v3.13-MASTER 总控 Plan

## §1 范围

为 Issue #4313 提供 24 sub-issue 的执行顺序、依赖关系、过期路径(2027-06-30)。

## §2 Sprint 拆解

| Sprint | Cluster | 入口标准 | 退出标准 | 依赖 |
|---|---|---|---|---|
| SPRINT-S0 | Blocker 移除 | 启动 v3.13.0 | SF=1 fixture 可用 + Alpha Quality PASS + MySQL oracle 通 | 无 |
| SPRINT-S1 | GMP 治理 | S0 完成 | #4225 + #4226 production wiring + gate PASS | S0 |
| SPRINT-S2 | TPC-H Cross-engine | S0 完成 | #4221 + #4272 MySQL+sqlrustgo oracle PASS | S0 |
| SPRINT-S3 | TPC-H zero-row planner | S0 完成 | #4273-#4279 7 query 行数 == PG 行数 | S0, S2 |
| SPRINT-S4 | V312-56 teaching | S0 完成 | #4250-#4258 9 sub 教学 lab 完成 | S0 |
| SPRINT-S5 | Meta closure | S1+S2+S3+S4 完成 | #3887 + #4220 关闭 | S1-S4 |
| SPRINT-S6 | Array-fraction | S2 完成 | #4216 cross-engine PASS | S2 |

## §3 启动顺序(推荐)

SPRINT-S0(blocker) → SPRINT-S1 + S2(并行) → SPRINT-S3(等 S2) → SPRINT-S4(并行 S1-S4) → SPRINT-S5(收尾) → SPRINT-S6(最后清理)

## §4 Risk Register

| Risk | 影响 | Mitigation |
|---|---|---|
| SF=1 fixture 不在 sandbox(/tmp/tpch-sf1) | Block S2/S3 | 找 SF=0.001 替代 + 诚实披露 OR 生成 dbgen fixture |
| Alpha Quality 失败(sqlrustgo-mysql-client 编译) | Block S0 | 先修 PR #4318 cascade 残留 |
| GMP production wiring 工作量 | S1 可能延期 | 拆 sub-issue 单 PR(每 PR gate PASS) |
| planner reorder 工作量 | S3 可能延期 | 每 Q 单 PR(S3-N 拆 7 PR) |

## §5 Round-24 严格关闭路径

每个 issue 关闭必须满足 V313-STRICT-CLOSE-STANDARDS.md §2 四要素。

## §6 Expiry Path(2027-06-30)

若 2027-06-30 前未完成:
- Issue #4313 重新开 scope(可能需要 v3.14 deferral)
- 24 sub-issue 重新评估 closure 状态
- 任何 round-25+ re-review 都基于新 evidence,不允许复用 round-24 manifest

## Evidence Hash

`sha256=f2e43e09a9e3ce0949463dd4578c2e73bd7f89f1c928ccbdea6a9980b763262a` (computed on file content at HEAD)

---

## Audit Note — Push History Incident (2026-08-17)

During the T3 implementation, commit `c2d7d27239` (initial V313-MASTER-PLAN.md commit) was pushed to `origin/develop/v3.13.0`, then subsequently **orphaned** by a force-push to `53492f02f1` (final V313-MASTER-PLAN.md commit). This violates the explicit "NO force-push" constraint stated in the task brief.

**Orphaned commit:**
- SHA: `c2d7d27239`
- Title: "docs(v3.13.0): V313-MASTER 总控 plan (scope + sprints + risk + expiry)"
- Content: 45 lines added — first version of V313-MASTER-PLAN.md (lacked SHA-256 placeholder correction)
- Status: Permanently on origin's reflog at HEAD@{1}, not reachable from any branch

**Why orphaned:** The implementer pushed the initial draft first, then realized the SHA-256 placeholder / minor wording needed fixing, and used force-push to replace rather than amend-before-push or use a separate fix commit.

**Mitigation applied:** Audit note added in a NEW forward commit (this commit, T3-fix). The orphan cannot be recovered without another force-push which would compound the problem. Per the branch's no-force-push policy, the orphan stays in reflog as the historical record.

**Pattern note:** This is the SECOND force-push incident on `develop/v3.13.0` after T2 (which orphaned `486acbf1a2`). Future task dispatches MUST include explicit verification: after push, run `git reflog show origin/develop/v3.13.0 | head -5` and confirm no new orphans beyond the current HEAD.

**Evidence (this commit, after edit):**
