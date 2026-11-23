# REQUIREMENT-TEST-COVERAGE-MATRIX（DSH-B 绑定版）

- source_run: dshb-first-round-20261120-4
- 权威源：`docs/releases/v4.1.0/refactoring/TEST_COVERAGE_MATRIX.json`（Codex，C-TF-0.1-PLAN，哈希绑定见 manifest）
- DSH-B 不另建第二份矩阵主账；本文件记录绑定与执行缺口。

## 现状（实读权威矩阵）

| 项 | 值 |
|---|---|
| 场景总数 | 75（source_pilot_count） |
| 试点声明 | PASS=68, KNOWN_FAIL=4, NOT_RUN=3 |
| **复跑通过数** | **42**（reexecution_pass_count=42；2026-11-23 起，L1 切片 `feature∈{T1,T2,T3}` 的 44 条在冻结基线 fd849ae20541 上实跑：PASS=42 / NOT_RUN=2，证据见 `docs/releases/v4.1.0/refactoring/evidence/L1-RERUN-fd849ae20541.json` 与 `L1-DIFF-fd849ae20541.md`。余下 31 条仍为 `NOT_RUN`） |
| 矩阵立场 | `source_claims_are_not_acceptance: true`（来源声明不等于验收） |
| 必需字段 | case_id/feature/invariant_ids/source/seed/backend/assembly/base_sha/expected/actual/execution_status/artifact_sha256 |
| 装配状态 | 全部 `UNVERIFIED_DO_NOT_INFER_FROM_DEFAULT_FACTORY` |

## DSH-B 执行计划（把 0 → 75）

1. **复跑顺序**：先 4 个 KNOWN_FAIL（D-001~004 候选）→ 3 个 NOT_RUN → T1/T3/T6 事务族 → 其余。
2. **复跑执行器**：确定性脚本（pilot test_factory.py 修复零测试计数后），不逐条调 LLM。
3. **每条落 12 要素**：backend 分列（MemoryStorage/FileStorage/BinaryTableStorageV2/WAL 并行）实测。
4. **验收口径**：execution_status 只能来自真实运行；assembly=UNVERIFIED 的不得记 PASS。

> **2026-11-23 Task 6 状态注记（不改写本条规则，只登记冲突）**：L1 切片（`feature∈{T1,T2,T3}` 的 44 条）在冻结基线 fd849ae20541 上实跑，得到 PASS=42 / NOT_RUN=2，并按「execution_status = 执行状态」的读法回写进了权威矩阵 —— 而 75 条全部 `assembly=UNVERIFIED_DO_NOT_INFER_FROM_DEFAULT_FACTORY`，按本条第 4 项的**字面**读法这 42 条 PASS 不能作为验收证据。两种读法（字面 vs 矩阵自带的软化口径：`required_for_certification=false` + `activation_requires=[ASSERTION_REVIEW, INPUT_HASH_BINDING, REAL_EXECUTION]`）**尚未由属主裁决**，登记在 `docs/releases/v4.1.0/refactoring/evidence/L1-DIFF-fd849ae20541.md` §五-5 与 §六 属主裁决项 ②。裁决前，这 42 条 PASS 不得被当作「已验收」的凭据。

## D-001~004 映射状态（诚实记录）

属主指令引用"D-001～004 缺陷反例"，**未在仓库找到该精确编号**（全树 grep 无 D-001 定义）。最强候选 = 试点矩阵 4 个 KNOWN_FAIL/特殊场景：

| 候选 | case_id | 缺陷族 | 与 HARA 对应 |
|---|---|---|---|
| D-001? | PILOT-T4-104 `tf_t4_04_non_tx_backend_must_reject_or_document` | BinaryStorage 名义事务/无事务语义 | HARA B 族 |
| D-002? | PILOT-T5-101 `(odku-rollback-noop)` | ODKU 回滚无效撤销（#5905 同族） | undo 恢复族 |
| D-003? | PILOT-T6-001 `(crash-recovery-kill-9)` | 崩溃恢复 | WAL/Recovery 族 |
| D-004? | PILOT-T6-002 `(checkpoint-torn-write)` | Checkpoint 撕裂写 | Checkpoint/Recovery 族 |

**待属主确认**：此映射是否即 D-001~004，或另有权威定义文件。确认前不得据此进入代码修改（遵守"只允许候选 Issue 和测试设计"约束）。

---

[![](https://img.shields.io/badge/powered_by-dsh-4D6BFE?style=flat-square&logo=deepseek&logoColor=white)](https://github.com/deepseek-ai/deepseek-harness)
