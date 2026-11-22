# Test Factory 设计交接说明

provenance: source_agent=Codex; source_run=TF-DESIGN-20261121; timestamp=2026-11-21T03:57:40+08:00; evidence_hash=sha256:d9aa2959618bf348f334fae97e4e5f6492361c41beced7419e673e27a5c4dfdf; conflict_resolution=Owner 仅批准设计与实施计划，未批准实施或合并。

## 状态

- 文档类型：DerivedDoc / 设计交接包。
- 固定产品基线：`e9ff21fca3aa945f6fe543ce8f91dfc19e6aaed0`。
- ADR 交接基线：`45da6968ae2baaade6a31206eeee5854a8514ee2`；ADR-410-011 候选：`b681b74ca3`。
- 当前阶段：ALPHA（本轮只读核对 `docs/releases/v4.1.0/STAGE.yaml`）。
- 本包不构成产品实现、CI/门禁变更、ADR 生效、HUMAN_GATE 解除、PR 合并或发布授权。

## 权威边界

正式 DAG、ADR、Issue、Git 对象和测试源码仍是权威来源。本目录中的 RED 映射、ADR 映射和任务 DAG 是可审核的拟议投影，不得成为第二份事实 SSOT。发生冲突时 fail closed 并回到固定 Git 对象核对。

## 阅读顺序

1. `01_CONCEPTUAL_DESIGN.md`
2. `02_TEST_EXECUTION_CONTRACT.md`
3. `03_TEST_FACTORY_INVARIANTS.md`
4. `04_S1_TEST_SPECIFICATIONS.md`
5. `05_RESULT_CLASSIFIER_SCHEMA.json`
6. `06_RED_IDENTITY_MAPPING.jsonl`
7. `07_CI_AND_RELEASE_GATE_DESIGN.md`
8. `08_BASELINE_CHECK_UPDATE_DESIGN.md`
9. `09_IMPLEMENTATION_TASK_DAG.jsonl`
10. `10_ACCEPTANCE_MATRIX.json`
11. `11_ADR_PACKAGE_BINDING.json`
12. `12_MIGRATION_AND_ROLLBACK.md`
13. `13_IMPLEMENTER_INSTRUCTIONS.md`
14. `14_HANDOFF_MANIFEST.json`
15. `15_R1_ARTIFACT_DRIFT_AND_PROPAGATION.md`

## 已核对与未核对

- 本轮实跑：Phase 0 HEAD/branch/status、STAGE `current_stage`、D8。
- 本轮实读：Cargo/CI 测试入口、测试目标执行门禁、manifest honesty、P13 基线脚本、G19 基线、正式候选 DAG、ADR-410-011、S1 两个远端提交。
- 未运行：全量 Cargo 测试、17 项 RED 全量复跑、PR/CI 独立验收。因此本包不声称这些项目 PASS。

## 形式冻结规则

只有同时满足以下条件才称为正式设计交接：设计分支已提交并推送至 `gitea252`、Gitea PR 可读取、最终 PR HEAD 与交接回执一致、`SHA256SUMS` 全量通过、Manifest SHA256 可复算、DSH-A 已接收 `HANDOFF_READY`。自动巡检的形式 `APPROVED` 不构成技术 Reviewer 验收。
