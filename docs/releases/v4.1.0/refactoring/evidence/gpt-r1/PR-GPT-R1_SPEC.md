# PR-GPT-R1 SPEC — Harness 核心门禁整改（G1）

> **source_agent**: DSH-B (opencode / MiniMax-M3) · **source_run**: SQLRUSTGO-V410-REFACTOR-HARNESS-R1-20261123
> **timestamp**: 2026-11-23T11:00:00+08:00 · **base_sha**: `8cbde5b6d33c876fa1197474c46d50142a9823bd`
> **evidence_hash**: 见 `docs/releases/v4.1.0/refactoring/gpt-r1/manifest.json#files_digest`
> **conflict_resolution**: 凡与既有文档冲突处，以本 PR 的实测为准；属主裁定项标 REPORTED/UNKNOWN，不代裁。

## 1. 范围

评审包 `SQLRUSTGO-V410-REFACTOR-HARNESS-R1-20261123` 判定 `REQUEST_CHANGES`，G1 段要求：

> 优先检查 `check_evidence_binding.sh` / `check_anti_fabrication.sh` /
> `check_gate_test_integrity.sh` / `check_drift_not_pass.sh` /
> `check_architecture_freeze.sh`。先证明实际行为缺陷，再实施最小修复。
> provenance: source_agent=DSH-B (opencode / MiniMax-M3); generated_by=DSH-B/opencode; generated_at=2026-11-23T11:30:00+08:00; base_sha=8cbde5b6d33c876fa1197474c46d50142a9823bd; package=docs/releases/v4.1.0/refactoring/evidence/gpt-r1/

本 PR 覆盖 **G0（只读基线重新绑定）+ G1（5 道门禁整改）** 的**可执行部分**，并交付 G2/G3/G4 的只读核验结论与计划。

## 2. 不在本 PR 范围内

| 项 | 原因 |
|---|---|
| 正式核心源码重构 | 任务书 §7「正式核心源码重构须等待现有准入门禁和Owner授权」 |
| 75 场景重跑 / 30 个试点用例 | 大规模执行未获本轮授权；见 `TEST-FACTORY-RECOVERY-PLAN.md` §4 |
| Swift 能力项 B/C/E | 执行准入与控制面准入未就绪；见 `EXECUTOR-CAPABILITY-PLAN.md` §3 |
| 控制面构件（Attempt/Lease/Receipt） | 本仓不存在这些实体，属主裁定；见 `CONTROL-PLANE-GAP-MATRIX.json` |
| 既有文档的修改 | 交接包与既有发布文档本轮一字未动 |

## 3. 改动的接口

**无产品接口改动。** 改动全部在 harness 层（`scripts/gate/`）与新增测试目标（`tests/integration/gate/`）：

| 产物 | 性质 |
|---|---|
| `scripts/gate/check_drift_not_pass.sh` | 门禁行为收紧（缺基线/零扫描 ⇒ FAIL） |
| `scripts/gate/check_gate_test_integrity.sh` | 门禁行为收紧（引用缺失 ⇒ FAIL） |
| `scripts/gate/check_evidence_binding.sh` | 判据收紧 + 假阳性清除 |
| `scripts/gate/check_architecture_freeze.sh` | A7-1 改用 SPEC-011 判据；ADVISORY 独立计数 |
| `scripts/gate/check_anti_fabrication.sh` | sqlancer 证据必须本次产生 |
| `scripts/gate/architecture_freeze_bypass_detector.py` | **新增**：AD-002 绕过检测的单一实现 |
| `tests/integration/gate/gate_behavior_contract_test.rs` | **新增**：六轴行为契约回归套件 |
| `docs/releases/v4.1.0/refactoring/gpt-r1/` | **新增**：G0/G2/G3/G4 交付包（6 份 + manifest + SHA256SUMS） |

## 4. 新增 CLI 契约

- `check_drift_not_pass.sh --init-baseline` — 显式记录基线（默认**不**自建）。
- `check_gate_test_integrity.sh` — 无新参数；行为变化见 TEST_PLAN。
- `architecture_freeze_bypass_detector.py <repo_root> [crate ...]` — rc 0 = 无绕过，1 = 有绕过，2 = 扫描未完成。

## 5. 验收判据

见 `PR-GPT-R1_ACCEPTANCE.md`。
