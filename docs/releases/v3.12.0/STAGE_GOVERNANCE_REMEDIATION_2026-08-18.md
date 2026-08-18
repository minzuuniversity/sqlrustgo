# SQLRustGo v3.12.0 阶段治理纠偏报告

> **日期**: 2026-08-18
> **执行人**: codex
> **范围**: 252 Gitea `openclaw/sqlrustgo` open issue、`develop/v3.12.0` 分支文档边界、v3.12 Beta/RC 治理入口
> **依据**: ADR-001 Truthfulness Framework、Anti-Fabrication Policy、Issue Closing Verification、v3.12 `STAGE.yaml`

## 1. 结论

v3.12.0 仍处于 **ALPHA**，不得因为存在 `develop/v3.13.0` 分支或 v3.13 follow-up issue 就视为已经完成 Beta/RC/GA。

2026-08-18 的 252 Gitea 状态显示：

- `docs/releases/v3.12.0/STAGE.yaml` 仍声明 `current_stage: "ALPHA"`。
- v3.12 总控 issue #3887 仍处于 open。
- 多个 `V312-*` open issue 曾被批量改成 `[v3.13 follow-up]`，但这些 issue 的验收边界仍来自 v3.12 的 Beta/RC/GA 条件。
- v3.13 总控 #4313 曾在仍有 V312 open issue 的情况下关闭；本轮已重新打开并标记为冻结。
- PR #4326 曾把 `develop/v3.13.0` 反向合入 `develop/v3.12.0`，导致 v3.12 分支出现 `docs/releases/v3.13.0` 正式发布文档；本轮从 v3.12 修复分支中移除此目录。

## 2. 治理原则

### 2.1 v3.12 是当前主线

在用户没有明确批准 v3.12 阶段裁决前，v3.12 继续按 `STAGE.yaml` 推进：

1. Alpha 质量稳定。
2. Beta 准入。
3. RC 正确性、恢复、兼容性和文档闭环。
4. GA 签核。

任何 v3.13 工作只能作为后续规划或隔离分支工作，不能替代 v3.12 的 Beta/RC/GA 门禁。

### 2.2 V312 issue 不得默认跳转 v3.13

V312 issue 只有在满足以下任一条件时，才可从 v3.12 发布边界中移出：

- 用户明确批准延期或降级。
- v3.12 README、STAGE、TEST_PLAN、ISSUES_PLAN 同步说明该能力不属于 v3.12 产品声明。
- 保留 open follow-up，包含 owner、expiry、关闭边界和可验证证据要求。

否则，V312 issue 必须继续保留在 `v3.12.0` milestone 中，并按 Beta blocker、Beta scope、RC/GA blocker 分类推进。

### 2.3 v3.13 分支冻结规则

在 v3.12 Beta/RC 裁决完成前：

- `develop/v3.13.0` 不得作为 V312 blocker 的默认吸收分支。
- v3.13 PR 合并不自动关闭 V312 issue。
- v3.13 总控 #4313 必须保持 open，直到 #3887 给出 v3.12 阶段裁决。
- 禁止将 `docs/releases/v3.13.0` 正式文档目录合入 `develop/v3.12.0`。

## 3. Issue 纠偏结果

本轮已在 252 Gitea 上执行以下纠偏：

| 类别 | Issue | 当前治理口径 |
|---|---|---|
| v3.12 总控 | #3887 | 恢复为 `[V312-MASTER]`，继续作为 v3.12 主控 |
| v3.13 总控 | #4313 | 重新打开并标为 `[V313-MASTER][FROZEN]` |
| Beta blocker | #4250-#4254 | V312-56A-D 及总控，Beta 准入前必须完成或由用户裁决降级 |
| Beta scope | #4255-#4258 | Beta 阶段必须给出完成、降级或延期证据 |
| RC/GA blocker | #4220、#4221、#4272-#4279 | TPC-H correctness、PARTIAL 收口，RC/GA 前必须闭环或从生产声明剔除 |
| 延期候选 | #4155、#4216 | v3.13 PR 进展不等于 V312 验收；需要回补 v3.12 或用户批准延期 |

## 4. 代码库纠偏结果

本轮在 `develop/v3.12.0` 修复分支执行：

- 移除 `docs/releases/v3.13.0` 目录，避免 v3.13 正式发布文档污染 v3.12 分支。
- 新增 `scripts/gate/check_v312_stage_boundary.sh`，检查 v3.12 分支是否仍保持 v3.12 阶段边界。
- 将边界检查接入 `scripts/gate/check_beta_v3.12.0.sh`。
- 更新根 `README.md` 和 v3.12 README，声明当前主线仍是 v3.12 ALPHA，v3.13 处于冻结 follow-up 状态。

## 5. 后续推进要求

### 5.1 进入 Beta 前

必须完成或获得用户裁决：

- #4250-#4254：V312-56A-D 与总控。
- `bash scripts/gate/check_v312_stage_boundary.sh` 退出 0。
- `bash scripts/gate/check_beta_v3.12.0.sh` 不出现 stage-boundary blocker。

### 5.2 进入 RC 前

必须完成或获得用户裁决：

- #4255-#4258：Beta 阶段范围项。
- #4220：PARTIAL 功能总控。
- `README.md` 与 v3.12 release 文档不能把未闭环 `PARTIAL` 写成生产能力。

### 5.3 进入 GA 前

必须完成或获得用户裁决：

- #4221、#4272-#4279：TPC-H SF=1 correctness、zero-row、cross-engine SHA256。
- SQLLogicTest selected corpus、wire protocol、LOAD DATA、crash recovery、backup/restore、upgrade/downgrade 等 GA 门禁。

## 6. 拒绝规则

以下情况必须阻断 v3.12 Beta/RC/GA：

- `docs/releases/v3.13.0` 出现在 `develop/v3.12.0`。
- 根 README 在 `develop/v3.12.0` 上声明当前开发版为 v3.13.0。
- V312 open issue 被标成 v3.13 follow-up 且没有用户批准的延期裁决。
- v3.13 PR 合并被用作关闭 V312 issue 的唯一证据。
- Issue 评论以 `DEFERRED`、`SUBSTANTIALLY_COMPLETE` 或 `not DONE` 作为关闭理由。

## 7. 结论

v3.12.0 当前应继续作为主线推进，先完成 Alpha 质量稳定与 Beta 准入。v3.13 可以保留规划，但必须冻结为后续跟踪，不能绕过 v3.12 的 Beta/RC/GA 阶段治理。
