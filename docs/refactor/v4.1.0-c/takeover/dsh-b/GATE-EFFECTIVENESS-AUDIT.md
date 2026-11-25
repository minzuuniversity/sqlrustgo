# GATE-EFFECTIVENESS-AUDIT（门禁有效性审计 · DSH-B）

- source_run: dshb-first-round-20261120-4
- 结论：**2 项确定性反例中 1 项已修复并回归，1 项待修**；D8 门禁 PASS 不推导为语义正确（属主令）。

## 反例 #1：pilot test_factory 零测试 exit0→PASS ✅ 已修复

- 证据：contract-observations.json（MOCK 实录：`tests:[] + exit_code:0 → class:PASS`）。
- 根因（tools/test_factory.py 实读）：`run_suite` 末行 `class = FAIL if failed else PASS`，零测试 + exit 0 命中 PASS。
- 修复（本轮完成，2026-11-20 16:53）：
  1. 无测试行（即使 exit 0）→ `NOT_RUN`，note 注明"空跑不是验收证据"；
  2. `summary.passed` 与解析出的测试行数交叉校验，不一致 → `INFRA_ERROR`（防静默无匹配被记 PASS）。
- 回归：`tools/test_test_factory_counterexample.py` **6/6 通过**（零测试两种形态 NOT_RUN、summary 不一致 INFRA_ERROR、正常 PASS/FAIL/KNOWN_FAIL 不回归）。
- 真实冒烟：真实 cargo 零测试运行 40.2s（exit 0）→ 实测输出 `class=NOT_RUN`，证据 `/tmp/tf-smoke/ZEROTEST-SMOKE-20261120T165325.json` + 原始 log。
- 修改文件：`srg-test-factory-pilot/tools/test_factory.py`（+修复）、`tools/test_test_factory_counterexample.py`（新增）；未提交，git 工作树留待属主查看。

## 两反例状态汇总

| 反例 | 状态 | 回归证据 |
|---|---|---|
| #2 gatekeeper 失败测试→PASS_TO_REVIEW | ✅ 已修复 | 4 新测试，11/11；全量 143+1s |
| #1 pilot 零测试 exit0→PASS | ✅ 已修复 | 6 新测试 6/6；真实 cargo 冒烟 NOT_RUN |

## 反例 #2：deepseek-schedule gatekeeper 失败测试→PASS_TO_REVIEW

- 证据（源码级）：gatekeeper.py `evaluate()` 原先仅校验状态字符串 ∈ 四态，不检查值。
- 修复（本轮完成）：非 success 测试必须带显式 `attribution`（如属主预批准的 known_fail），否则 REJECTED。
- 回归：新增 4 测试（failed_required_test_rejected / failed_with_attribution_reaches_review / inconclusive_without_attribution / skipped_without_attribution）；**11/11 通过，全量 143 passed + 1 skipped**。
- 修改文件：`deepseek-schedule/scheduler/gatekeeper.py`、`tests/test_gatekeeper.py`（未提交，待属主查看后处理提交）。

## D8 门禁与语义正确性的边界

- `check_arch_sem_debt.sh`：PASS（7/7 CLOSED，本轮实测 exit 0）——仅证明架构/语义债台账项关闭，**不证明**：
  - 75 场景复跑（=0 未跑）
  - FileStorage buffer_pool 绕过 Undo 已消解
  - BinaryStorage 事务语义已定义
  - 36 条 open P0 已清零
- 负向门禁验证计划：对每个 gate 脚本注入已知坏产物，确认会 RED（属主指令"CI 失败传播及门禁负向验证"）；纳入 Task B-E 之后的 S4。

## CI 运行时验证状态

- ASSET_VERIFIED（.gitea/workflows 8 个配置在位）；CI_RUNTIME_NOT_VERIFIED（未实测失败传播与独立复跑）——保持诚实标注。

---

[![](https://img.shields.io/badge/powered_by-dsh-4D6BFE?style=flat-square&logo=deepseek&logoColor=white)](https://github.com/deepseek-ai/deepseek-harness)
