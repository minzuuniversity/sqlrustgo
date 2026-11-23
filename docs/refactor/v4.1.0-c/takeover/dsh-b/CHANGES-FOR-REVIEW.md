# CHANGES-FOR-REVIEW — DSH-B 会话产生的全部变更清单（供属主审阅/提交决策）

- source_run: goal-R6 变更盘点
- timestamp: 2026-11-20T17:31:03+08:00
- 原则：所有变更均为测试/工具/控制面/证据文件；**零产品源码修改**；未提交部分留在工作树待属主处置。

## 1. deepseek-schedule（✅ 已由 DSH-A 代为提交）

| 文件 | 变更 | 状态 |
|---|---|---|
| scheduler/gatekeeper.py | 反例#2 修复（非 success 测试需归因） | **已包含在 DSH-A commit `485f0aa`（16:46，v3.0 multi-project + LOCAL_ONLY P0 fixes）** |
| tests/test_gatekeeper.py | +4 回归测试 | 同上（随 commit 进入） |

跨会话协作事实：DSH-B 修复 → DSH-A 验证并纳入 v3.0 提交；DSH-A 同时交付 `scheduler/handoff.py`（14 测试绿）与 `docs/multi-project/SQLRUSTGO-TASK-HANDOFF-SPEC.md`（DSH-B→DSH-A 14 字段 digest 交接规范）。

## 2. srg-test-factory-pilot（⚠️ 未提交，工作树留待属主）

| 文件 | 变更 |
|---|---|
| tools/test_factory.py | 反例#1 修复（零测试→NOT_RUN、summary 交叉校验、should_panic 解析、--test 模式、raw 留档、CLI --test 参数） |
| tools/test_test_factory_counterexample.py | 新增 8 项回归（全绿） |
| crates/storage/tests/tf_t4_stub_silent_success.rs | pilot 侧已有（试点产物，未跟踪） |
| crates/storage/tests/tf_t4_ai_candidate_v1.rs | **新增**：AI 候选测试（终态 4/4 RED，Task C 实证） |
| tests/integration/test_factory/ | 试点产物目录（含 tf_t1_undo_invariants 4/4 PASS） |
| evidence/* | 复跑证据 ~20 个文件（RERUN-*、rerun-*、case-mapping、rerun-summary） |
| Cargo.toml | pilot 侧既有本地改动（非本轮产生） |

## 3. deepseek-harness（⚠️ 未提交）

| 文件 | 变更 |
|---|---|
| python/control/ | 新增控制面包（lease/dispatch/domains/ledger/storage + 40 测试全绿，含 Task E 多进程恢复演练） |
| docs/sqlrustgo-takeover/README.md | 指针文档 |
| AGENTS.md | **既有本地改动（非本轮产生，属主自查）** |

## 4. sqlrustgo 主工作树（⚠️ 未提交）

| 文件 | 变更 |
|---|---|
| docs/refactor/v4.1.0-c/takeover/ | 本目录全套证据/报告/账本（DSH 预检 + control-plane + dsh-b 交付物） |
| .worktrees/codex-calpha01-20261120/docs/refactor/… | Codex 交接包（Codex 产物，属 Codex 提交流程） |
| .aoci/baseline.json、.gitignore、AGENTS.md、AGENTS.md.backup.* | **既有本地改动（非本轮产生，属主自查）** |
| /tmp/srg-d002-base | 临时基线 worktree（D-002 基线复现用，可删除：`git worktree remove /tmp/srg-d002-base`） |

## 建议处置（属主裁决）

1. pilot 工具修复 + AI 候选测试 + 证据 → 建议单独提交（Test Factory 反例清除批次）
2. deepseek-harness python/control → 由 DSH-A/属主决定并入 v3.0 线或单提交
3. sqlrustgo takeover 证据目录 → 建议纳入 docs（作为接管证据链）
4. Codex 交接包 → Codex 重签 manifest 后自行提交

---

[![](https://img.shields.io/badge/powered_by-dsh-4D6BFE?style=flat-square&logo=deepseek&logoColor=white)](https://github.com/deepseek-ai/deepseek-harness)
