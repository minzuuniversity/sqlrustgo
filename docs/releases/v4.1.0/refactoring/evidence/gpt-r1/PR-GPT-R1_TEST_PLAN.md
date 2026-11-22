# PR-GPT-R1 TEST_PLAN — Harness 核心门禁整改

> **source_agent**: DSH-B (opencode / MiniMax-M3) · **source_run**: SQLRUSTGO-V410-REFACTOR-HARNESS-R1-20261123
> **timestamp**: 2026-11-23T11:00:00+08:00 · **base_sha**: `8cbde5b6d33c876fa1197474c46d50142a9823bd`
> **evidence_hash**: 见 `docs/releases/v4.1.0/refactoring/gpt-r1/manifest.json#files_digest`
> **conflict_resolution**: 覆盖目标以评审任务书 §2 的六轴为准；未实跑项标 NOT_RUN。
> provenance: source_agent=DSH-B (opencode / MiniMax-M3); generated_by=DSH-B/opencode; generated_at=2026-11-23T11:30:00+08:00; base_sha=8cbde5b6d33c876fa1197474c46d50142a9823bd; package=docs/releases/v4.1.0/refactoring/evidence/gpt-r1/

## 1. 覆盖目标

评审任务书 §2 要求每道门禁在六轴上被测到。本 PR 把这六轴落成**可执行断言**：

| 轴 | 判据 | 落点 |
|---|---|---|
| A 合法输入 PASS | 合规树 / 合规文档 ⇒ rc=0 | `drift_gate_clean_tree_passes`、`architecture_freeze_compliant_tree_passes` |
| B 非法输入 FAIL | 违规树 / 违规文档 ⇒ rc≠0 | `drift_gate_new_antipattern_fails`、`evidence_binding_fabricated_claim_fails`、`evidence_binding_wrong_sha_fails`、`architecture_freeze_missing_drift_gate_fails` |
| C 缺少证据不过 | 缺基线 / 缺工件 ⇒ rc≠0 | `drift_gate_missing_baseline_must_not_pass`、`gate_test_integrity_missing_baseline_must_not_pass`、`anti_fabrication_zero_tests_report_must_not_pass` |
| D 错误 SHA 不过 | 指向不存在的 commit ⇒ rc≠0 | `evidence_binding_wrong_sha_fails` |
| E 依赖异常不过 | 引用不存在的测试 / 缺失的 detector / 缺失的 manifest / 不存在的 log ⇒ rc≠0 | `gate_test_integrity_missing_test_reference_must_not_pass`、`architecture_freeze_missing_detector_is_incomplete`、`anti_fabrication_missing_manifest_must_not_pass`、`evidence_binding_missing_log_file_must_not_pass` |
| F 零执行不得 PASS | 扫 0 个脚本 / 0 个测试 / 空报告 ⇒ rc≠0 | `drift_gate_zero_scanned_scripts_must_not_pass`、`gate_test_integrity_zero_tests_must_not_pass`、`anti_fabrication_zero_tests_report_must_not_pass` |

另加两条**防过度修正**的回归护栏（防止修复把合规输入判红）：
`evidence_binding_negated_statement_is_not_a_violation`（否认句不是声明）、
`evidence_binding_unrelated_negation_does_not_excuse_claim`（无关分句的否定词不豁免）。

## 2. 阶段与命令

```
# 行为契约套件（本 PR 的核心验证）
cargo test --test gate_behavior_contract_test
# 期望：20 passed / 0 failed

# 受影响门禁逐道实跑
bash scripts/gate/check_drift_not_pass.sh          # rc=0
bash scripts/gate/check_gate_test_integrity.sh      # rc=0
bash scripts/gate/check_architecture_freeze.sh      # rc=0
bash scripts/gate/check_gate_script_refs.sh         # rc=0
bash scripts/gate/check_evidence_binding.sh v4.1.0 /tmp/eb  # rc=1，FAIL 29（既有）

# 鉴别力验证（同一套件跑在整改前的脚本上 ⇒ 8 条必须失败）
```

## 3. 鉴别力验证（关键）

**只写「整改后全绿」的测试没有意义。** 本 PR 的套件必须在**整改前**的脚本上失败：

| 脚本状态 | 结果 |
|---|---|
| 整改后（HEAD） | 20 passed / 0 failed |
| 整改前（`8cbde5b6d3` 的 `scripts/gate/`） | **12 passed / 8 failed** |

8 条失败 ⇔ 8 条已证明的行为缺陷。复现方法：把测试文件与 detector 放进一个
`8cbde5b6d3` 的独立 worktree（门禁脚本保持整改前状态）再跑同一套件。

## 4. 未纳入本轮的计划

| 项 | 原因 | 状态 |
|---|---|---|
| 75 场景矩阵重跑 | 大规模执行未授权 | NOT_RUN |
| 30 个试点用例 | 同上 | NOT_RUN |
| `check_anti_fabrication.sh` 在权威树上的全量实跑 | 需要 `sqlancer` / `test-runner` release 产物 | NOT_RUN |
| 全量 `cargo test --all-features` | 与本轮改动无关的既有失败会混淆归因 | NOT_RUN |
| 交接包 `files_digest` 独立复算 | 规范化口径未定义 | NOT_RUN |
