# PR-5785 TEST_REVIEW — clippy 全仓清债（LINT-2）

> Issue: #5362 · PR: #5785 · 分支: `opencode-macmini/clippy-clean`
> 冻结基线: `1cb5d8889e` · 阶段: ALPHA
> 配套: `PR-5785_SPEC.md` / `PR-5785_TEST_PLAN.md` / `PR-5785_TEST_DESIGN.md` / `PR-5785_ACCEPTANCE.md`

---

## 1. 独立审核状态

**尚未取得独立 Reviewer 回执。**

- Hub `ai-project-hub` #133：独立 Reviewer 派单权不在 Controller，转 Owner 决策；
  截至本 PR 提交时未回。
- 依据 `STAGE.yaml merge_policy`（reviewers: 2，approval: codeowner + governance），
  本 PR **不自签放行**、不声明 ALPHA→BETA、不声明 GA。
- 看板 `LINT-2` 状态固定为**待复核**，不写成「完成」。

## 2. 自审清单（执行者视角，不是独立审核）

| 检查项 | 结果 | 证据 |
|---|---|---|
| 是否存在「删断言换绿」 | 删除的 4 条全是恒真断言（`assert!(true)`）；无行为断言被删 | `git show 9eaf55cf63` |
| 是否存在「新增 `#[ignore]` 换绿」 | 无。原计划挂账的 0xfd 测试改为真修 + 常驻；anti-ignore 136/136 一致 | run 002 臂 7 |
| 是否存在「调低门禁阈值」 | 无。`check_test_target_lint_budget` 用 compare 模式，输出明确 “baseline was not modified” | run 002 臂 5/6 |
| 是否存在「新增 check_* 改变分母」 | 无。未新增门禁脚本；`gate_registry` 仍 168 行 | run 002 臂 8 |
| 是否把失败 run 抹掉 | 否。`20261114-clippy2-001` 原样入仓，作为真红取证 | `audit/.../runs/20261114-clippy2-001/` |
| 是否有行为变化未声明 | 两处真修（0xfd、VtuGuard）均在 SPEC 与 PR 正文声明 | `PR-5785_SPEC.md` §2.2 |
| 是否声明了未测的东西 | 未声明全仓测试绿；未声明 CI 绿灯；未声明阶段晋级 | 本文件 §3 |

## 3. 已知偏差与未覆盖

1. **CI 执行面**：#5597（runner-03 缺 `sqlrustgo-runner:v1`）仍不可用 ⇒ 本 PR 的
   证据是本地冻结 worktree 归档 run，不是 CI run。
2. **独立 Reviewer**：未回 ⇒ 状态「待复核」。
3. **未跑**：`cargo test --workspace` 全量。只跑了触碰范围（见 TEST_PLAN 层 3）。
4. **未做**：1.97.0 工具链 lint 预算基线录入（`--update`）；SEC-1 的 12 处
   `always-evaluates-to-false`；LocalExecutorDml 退役线 8 处 deprecated。
5. **`physical_backup` 负载 flaky**：既有存量，本 PR 不认领。

## 4. 复核者需要看什么

1. `audit/v4.1.0/runs/20261114-clippy2-001/`：先确认这条红是真的、可 verify。
2. `audit/v4.1.0/runs/20261114-clippy2-002/`：确认 16 臂 rc=0 且第 1 臂是 clean。
3. `git show 9eaf55cf63 -- crates/security/tests/v400_acl_audit.rs \
   crates/executor/src/local_executor_dml.rs`：确认 allow 的理由成立。
4. `git show 056ac98736`：确认 0xfd 修复与测试转常驻。
5. `git show 1cb5d8889e`：确认 VtuGuard 透传。
