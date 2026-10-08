# v4.1.0 Alpha→Beta 门禁与验收标准

> **制定日期**: 2026-10-08
> **文档性质**: 门禁规范，不是执行报告。
> **当前阶段**: `ALPHA`，以 `STAGE.yaml` 为 SSOT。
> **source_agent**: codex
> **source_run**: v410-alpha-beta-gates-20261008
> **assessed_commit**: `d312683244`（2026-10-08 rebase 后快照）
> **evidence_hash**: 待冻结提交实跑后生成

## 1. 判定原则

Alpha→Beta 只允许 `PASS` 或 `FAIL`。AB-01..AB-10 必须在同一冻结提交上全部 PASS；
任何 required check 的缺失、跳过、超时、workflow 无法解析或证据不可校验均判 FAIL。
文档中的历史 PASS、合并记录和 Issue 评论不能替代本次执行证据。

## 2. 硬门禁

| ID | 审核域 | 必须满足 | FAIL 条件 | 责任 Issue |
|---|---|---|---|---|
| AB-01 | SSOT/证据一致性 | 版本、阶段、commit、Issue、报告和 manifest 一致 | 陈旧结论、跨 commit 拼接、dirty checkout 未披露 | #5117 |
| AB-02 | 基础质量 | fmt、clippy `-D warnings`、all-features build/test 成功 | 任一非零退出、测试失败或警告降级 | #5113 |
| AB-03 | 门禁可信度 | workflow 可解析且 fail-closed；P11-P16、anti-ignore 和 mutation probe 成功 | invalid/missing/skipped、退出码吞掉、registry 与源码不一致 | #5113 |
| AB-04 | 事务/会话正确性 | rollback `0/300`、8000 事务无丢失、savepoint/MVCC/多库/wire 通过 | 间歇失败、状态跨连接泄漏、模型不一致 | #5112 |
| AB-05 | 外部工作负载 | BustubX B 轨必测 100%；SQL corpus `>=80%`；oracle 无差异 | 只检查未报错、忽略必测项、存在 P0 正确性失败 | #5114 |
| AB-06 | 覆盖率 | L1_8 平均 `>=80%`；关键 crate 结果可追溯 | 测试失败仍产覆盖率、零样本、低于阈值 | #5113/#5114 |
| AB-07 | 稳定性/恢复 | 1h SOAK 完成，heartbeat 连续，恢复后数据一致；24h runner 可执行 | 进程中止、心跳缺口、证据缺失、数据不一致 | #5115 |
| AB-08 | 性能基线 | 固定环境 A/B；>=5 次；正确性校验；无明显回退 | 吞吐降幅 >10% 或 p99 增幅 >20% 且超出噪声 | #5116 |
| AB-09 | 安全/依赖 | 安全扫描、依赖审计和恢复相关 gate 成功 | hard gate 降级、`|| true`、未处置高危项 | #5113/#5115 |
| AB-10 | 发布治理 | 所有 P0/P1 发布阻断项关闭且有合并 PR；人工双审通过 | open blocker、无 PR 关闭、无 codeowner/governance 批准 | #5117 |

覆盖率采用 `GATE_CONDITIONS.md` G17 的 Beta 80% 阈值；它高于
`STAGE_CONFIG.yaml` 当前的 75%。在治理文件统一前，执行较严格值。v4.1.0 GA 的本地目标
仍为 85%，不因本次 Beta 晋级而降低。

## 3. 审核运行流程

1. 冻结候选 commit，确认工作树干净并记录所有 submodule/LFS 状态。
2. 创建唯一 `source_run`，记录环境指纹和门禁版本。
3. 按 AB-01 到 AB-10 执行；每条命令保留原始 stdout/stderr 和退出码。
4. 生成 SHA-256 manifest，并验证所有引用文件存在且哈希匹配。
5. 由非执行者复核 Issue/PR、日志、失败计数和变异测试结果。
6. 全部通过后创建独立阶段晋级 PR；该 PR 只更新阶段和冻结证据引用。

修复任一失败后，必须创建新 `source_run` 并重跑受影响门禁；AB-02、AB-03、AB-04
以及最终汇总必须全部重跑。

## 4. 证据结构

建议每次执行写入 `docs/releases/v4.1.0/evidence/alpha-to-beta/<source_run>/`：

```yaml
source_agent: "<agent-or-human>"
source_run: "<unique-run-id>"
timestamp: "<RFC3339>"
ref: "develop/v4.1.0"
commit: "<40-char-sha>"
worktree_clean: true
environment_hash: "sha256:<hash>"
gate:
  id: "AB-04"
  command: "<exact command>"
  exit_code: 0
  verdict: "PASS"
  log: "logs/AB-04.log"
  artifact_hash: "sha256:<hash>"
review:
  reviewer: "<independent reviewer>"
  decision: "APPROVED"
```

`verdict: PASS` 必须由退出码和结构化断言共同导出，禁止手工覆盖失败结果。

## 5. Issue 审核与关闭合同

每个子 Issue 的最终评论必须包含：

1. 合并 PR URL 和 merge commit；
2. 修复前可复现证据与修复后证据；
3. 完整命令、退出码、测试计数和日志路径；
4. `source_agent`、`source_run`、timestamp、evidence hash；
5. 对验收条目逐项勾选，未满足项保持 Issue OPEN。

总控 #5117 只有在 #5112..#5116 和既有依赖全部满足关闭合同时才能关闭。

## 6. 阶段验收记录

晋级报告必须明确列出：

| 项目 | 允许值 |
|---|---|
| 候选 commit | 单一 40 位 SHA |
| AB-01..AB-10 | 全部 PASS |
| P0/P1 blockers | 0 OPEN |
| required checks | 全部 success |
| 独立审核 | codeowner + governance 批准 |
| STAGE 修改 | 独立 PR，合并后再创建 Beta tag |

本文件发布时不填写 PASS；实际结果写入后续审核报告和 evidence 目录。

## 7. 引用

- `docs/releases/v4.1.0/TEST_PLAN.md`
- `docs/releases/v4.1.0/DEV_PLAN.md`
- `docs/releases/v4.1.0/STAGE.yaml`
- `docs/governance/GATE_CONDITIONS.md`
- `docs/governance/ANTI_FABRICATION_POLICY.md`
- `docs/governance/ISSUE_CLOSING_VERIFICATION.md`
- `docs/governance/adr/ADR-008-test-claim-transparency.md`
- `docs/governance/adr/ADR-014-multi-ai-coordination.md`
