# TEST-FACTORY-RECOVERY-PLAN.md — SQLRustGo v4.1.0 Test Factory 恢复（G4）

> **source_agent**: DSH-B (opencode / MiniMax-M3)
> **source_run**: `SQLRUSTGO-V410-REFACTOR-HARNESS-R1-20261123` · G4 轮
> **timestamp**: 2026-11-23T10:00:00+08:00
> **base_sha**: `8cbde5b6d33c876fa1197474c46d50142a9823bd`
> **evidence_hash**: 见同目录 `manifest.json` 的 `files_digest`（本包 6 份内容文件的 sha256 排序摘要）；`SHA256SUMS` 逐文件列出 sha256。
> **conflict_resolution**: 「未实跑不得记 PASS」。本计划里的每一个数字都注明是 **实测** 还是 **REPORTED**；本轮**没有**重跑 75 场景（缺执行准入与机器资源），所以 75 场景的 PASS 状态一律标 **STALE**，不刷新。
> **authorization**: 本轮对 Test Factory 只做**只读核验 + 计划**。重跑 75 场景 / 30 个试点用例属于「大规模执行」，任务书 §7 明确禁止未授权启动；实现新测试文件属于正式实现，需 Owner 授权。
> provenance: source_agent=DSH-B (opencode / MiniMax-M3); generated_by=DSH-B/opencode; generated_at=2026-11-23T10:00:00+08:00; base_sha=8cbde5b6d33c876fa1197474c46d50142a9823bd; package=docs/releases/v4.1.0/refactoring/gpt-r1/

---

## 0. 一句话结论

75 场景矩阵的**内容**在权威 tip 上确实存在且计数一致，但它的**证据绑定是断的**：75 条里 **0 条**指向一个可机器校验的证据文件 —— 71 条写的是占位串 `RERUN-* evidence json/log`，4 条指向一个**根本不存在**的 `.log`（`evidence/` 目录下 `.log` 文件总数 = 0）。真正带 `base_sha` / `artifact_digest` / 逐测试结果的 5 份 RERUN 工件**全部绑在 `e9ff21fca3`**（落后权威 tip 138 个提交）。

---

## 1. 现状核验（VERIFIED，2026-11-23）

| 项 | 值 | 证据 |
|---|---|---|
| 矩阵文件 | `evidence/case-mapping-20261120.json` | 权威 tip 上存在 |
| 条目数 | **75**（`matrix_index` 0…74；`case_id` 有重复 —— `rerun-summary` 自己写明「以 matrix_index 区分」） | `python3` 解析 |
| 状态分布 | `PASS` 68 / `FAILED_RED` 4 / `COVERED_PASS` 2 / `COVERED_RED` 1 | 同上 |
| 汇总 | `evidence/rerun-summary-20261120.json`：`total_cases` 75、`reexecution_pass_count` 70、`red_confirmed` 5、`source_agent = DSH-B (opencode-go/glm-5.3-flash)`、`source_run = goal-R5 rerun-mapping-v2` | 同上 |
| 汇总 `base_sha` | `e9ff21fca3aa945f6fe543ce8f91dfc19e6aaed0` | **STALE（落后 138 提交）** |
| 5 份 RERUN 工件的 `base_sha` | 全部 `e9ff21fca3aa…` | **STALE** |
| 逐测试粒度 | RERUN 文件里 **有**：`tests[]`（`name` / `class` / `raw_status`）、`summary`（`passed`/`failed`/`ignored`/`cargo_status`）、`exit_code`、`duration_s`、`cmd`、`raw_log`、`artifact_digest` | `evidence/RERUN-*.json` |
| 各套件规模 | T1B2 = 218 tests / 1.18 s；T1C = 3 tests / 64.41 s；T2T3 = 222 tests / 1.46 s | 同上 |
| 执行器 | `tools/test_factory.py`（确定性，不依赖 LLM；分类 `PASS/FAIL/KNOWN_FAIL/UNSUPPORTED/DIALECT_DIFFERENCE/NOT_RUN/INFRA_ERROR`；自述「KNOWN_FAIL != PASS；NOT_RUN != PASS」） | `tools/test_factory.py:1-40` |
| 反例自测 | `tools/test_test_factory_counterexample.py` | 权威 tip 上存在 |
| C01 试点测试 | `tests/integration/test_factory/`：`ca01_backend_caps.rs`、`ca01_bufferpool.rs`、`ca01_commit_frontier.rs`、`ca01_complex_dml.rs`、`ca01_file_undo.rs`、`ca01_snapshot_reopen.rs`、`ca01_visibility_oracle.rs`、`tf_t1_undo_invariants.rs` | 目录列表 |
| T4 反例测试 | `crates/storage/tests/tf_t4_ai_candidate_v1.rs`、`crates/storage/tests/tf_t4_stub_silent_success.rs` | 目录列表 |

### 1.1 证据绑定断点（本轮最重要的发现）

| `evidence` 字段形态 | 条数 | 判定 |
|---|---|---|
| 占位/通配（`RERUN-* evidence json/log`） | **71** | 不可机器校验 |
| 具体路径但**文件不存在**（`evidence/D001-red-run-20261120T165449.log`） | **4** | **断链**（`PILOT-T4-101/102/103/104`） |
| 具体路径且存在 | **0** | — |
| `evidence/` 目录下的 `.log` 文件总数 | **0** | — |

**判读**：4 条 RED 用例（`tf_t4_01_scan_after_insert_sees_rows`、`tf_t4_02_delete_reports_real_count`、`tf_t4_03_update_reports_real_count`、`tf_t4_04_non_tx_backend_must_reject_or_document`）**它们的证据文件不在仓库里**。也就是说，**最该被复核的 4 条 RED，恰好是证据最不可信��� 4 条** —— 这正是任务书 §5「优先覆盖：错误证据绑定」所指的风险面。

**好消息**：可校验的绑定材料其实**已经存在** —— 5 份 RERUN JSON 里逐测试的 `name` / `class` / `raw_status` / `artifact_digest` / `base_sha` 都在，只是矩阵没有指过去。**修复是「改指」而不是「重造」。**

---

## 2. 与任务书 §5 优先覆盖面的对应

| 任务书优先项 | 现状 | 判定 |
|---|---|---|
| 零测试假成功 | `tools/test_factory.py` 的分类器里已有 `NOT_RUN` / `INFRA_ERROR`；`crates/storage/tests/tf_t4_stub_silent_success.rs` 存在；`check_anti_fabrication.sh` 已把「validator 报 0 tests ran」判红（本轮又修了 sqlancer 的过期证据通道） | **部分覆盖，REPORTED** |
| 错误证据绑定 | 本轮量化：0/75 可机器校验；4 条 RED 的证据文件不存在 | **未覆盖 —— 本轮的头号发现** |
| FileStorage Undo | `ca01_file_undo.rs`、`tf_t1_undo_invariants.rs` 存在；Issue #5910 / #5911 仍 open（P0） | **部分覆盖；对应 open P0** |
| BinaryStorage 事务语义 | `binary_storage_v2` 在 RERUN-T2T3 里 222 个测试全 PASS；但 `BinaryTableStorageV2::begin_transaction` 返回 `Ok(0)`（Issue #5914，P1）；`tf_t4_04_non_tx_backend_must_reject_or_document` 是 RED | **未覆盖到位** |
| WAL 崩溃恢复 | `PILOT-T6-001` 标 `COVERED_PASS`（`crash_commit_fsync_b1` 2/2 PASS，真实 SIGKILL）；`PILOT-T6-002` `COVERED_PASS`（torn_write 1/1）；证据字段是通配符 | **REPORTED（绑定不可校验）** |
| Storage Backend 差异 | `ca01_backend_caps.rs` 存在；`PILOT-T5-101` `COVERED_RED`（基线 RED，证据指向 `dsh-b/artifacts/…`，**该目录不在权威 tip 上**） | **部分覆盖** |

---

## 3. 恢复计划（分三阶段，每阶段都有可判定的完成条件）

### 阶段 0 — 证据绑定修复（**不改代码，只改绑定**）

| # | 动作 | 完成判据 | 需要什么 |
|---|---|---|---|
| 0.1 | 把矩阵的 `evidence` 字段从占位符/断链改成**具体 RERUN 文件 + 该文件内的测试名** | 75/75 条的 `evidence` 都能解析到一个存在的文件；解析后能定位到该 case 的 `test_name` | 只读 + 改 `evidence/case-mapping-*.json`（**需 Owner 授权改既有工件**） |
| 0.2 | 给每条 case 补 `base_sha` / `artifact_digest` 字段 | 每条 case 的 `base_sha` 等于产生它的那份 RERUN 的 `base_sha`；`artifact_digest` 与 RERUN 一致 | 同上 |
| 0.3 | 4 条 RED 的缺失日志 | 重跑 `tf_t4_*` 并把真实 stdout/stderr 落盘到 `evidence/`，文件名与矩阵一致 | **需要执行准入**（跑测试本身允许；但需重跑 ⇒ 见 §4） |
| 0.4 | 加一条机器检查：**矩阵每条 case 的 evidence 必须可解析且存在** | 新门禁/新检查在断链时 rc≠0 | 新增检查（属 G1 范畴，可并入本分支） |

> 阶段 0.4 建议直接落成 scripts/gate/check_test_factory_matrix_binding.sh（计划名，**未入库**），判据对齐本轮 G1 已确立的「缺证据 ⇒ 不得 PASS」口径。

### 阶段 1 — 基线重绑 + 30 个试点用例重跑

| # | 动作 | 完成判据 |
|---|---|---|
| 1.1 | 在**新**基线（`8cbde5b6d3` 或其后代）上重跑 T1 / T1B / T1B2 / T1C / T2T3 五套 | 5 份新 RERUN 的 `base_sha` = 新基线；每份 `summary.cargo_status` 与 `exit_code` 一致 |
| 1.2 | 重跑 4 条 T4 RED | 仍为 RED（红是被**期望**的 —— 桩静默成功），且每条都有落盘的日志 |
| 1.3 | 重跑 30 个试点用例（选取见 §3.1） | 30/30 产出新的 RERUN 工件 + 逐条 case 绑定 |
| 1.4 | 刷新 `rerun-summary-20261120.json` → 新时间戳新基线 | 汇总里的 `base_sha` 与各 RERUN 一致 |

#### 3.1 30 个试点用例的选取口径

从 75 条里挑 30 条，**按任务书 §5 的六个优先面加权**，而不是按 `case_id` 顺序：

| 优先面 | 建议条数 | 来源 |
|---|---|---|
| 零测试假成功 | 6 | T4 组（`tf_t4_01…04`）+ `tools/test_test_factory_counterexample.py` 的反例 + 一条 `NOT_RUN` 样例 |
| 错误证据绑定 | 5 | 当前 4 条断链 RED + 1 条占位 PASS |
| FileStorage Undo | 6 | `ca01_file_undo` / `tf_t1_undo_invariants` / #5910 / #5911 对应用例 |
| BinaryStorage 事务语义 | 5 | `binary_storage_v2` 相关 + `tf_t4_04` + #5914 对应用例 |
| WAL 崩溃恢复 | 4 | `PILOT-T6-001/002` + `crash_commit_fsync_b1` + torn-write |
| Storage Backend 差异 | 4 | `ca01_backend_caps` + `PILOT-T5-101` + #5913 的 scan/delete/update |

### 阶段 2 — 扩到 75 + 接入 CI

| # | 动作 | 完成判据 |
|---|---|---|
| 2.1 | 75 条全量重跑 | 75/75 有具体、可解析、存在的证据 |
| 2.2 | 75 条全量重跑 | 75/75 的 `base_sha` = 同一基线 |
| 2.3 | 矩阵绑定检查接入 `required_gates`（GA） | 断链 ⇒ 门禁红 |
| 2.4 | 矩阵与 `GATE_CATALOG` 的关系登记 | 门禁台账里能查到这条新检查 |

---

## 4. 本轮**没有**做的事，以及为什么

| 没做 | 原因 | 状态 |
|---|---|---|
| 重跑 75 场景 | 属「大��模执行」；任务书 §7 禁止未授权启动；且 4 条 RED 需要 `sqlrustgo-mysql-server` 的 `--all-features` 产物（当前树无 `target/`） | **NOT_RUN** |
| 30 个试点用例 | 同上 | **NOT_RUN** |
| 改 `evidence/case-mapping-20261120.json` | 改既有工件需 Owner 授权（交接包把它列为只读投影） | **NOT_RUN**（方案见 §3 阶段 0） |
| 补 `evidence/D001-red-run-*.log` | 需要重跑才能产出真实日志 | **NOT_RUN** |
| 新增 `check_test_factory_matrix_binding.sh` | 本轮 G1 已交付 5 个门禁修复 + 20 条回归；再加一道门禁需要先确认基线口径（§4） | **可做，待批** |
| 实现 18 个工作包里的任何测试 | `11_ADR_PACKAGE_BINDING.json` 里 18 个包的 `implementation_authorized` **全为 false**；7 个已有证据包（C01-05/06/07/09/10/11/14） | **BLOCKED（属主门禁）** |

---

## 5. 与交接包的一致性检查（VERIFIED）

| 交接包的承诺 | 实测 | 判定 |
|---|---|---|
| `01_CONCEPTUAL_DESIGN.md` 承诺「不依赖任何 LLM、确定性执行」 | `tools/test_factory.py` 的分类器与解析全是确定性正则 + json | **一致** |
| `10_ACCEPTANCE_MATRIX.json` | 存在 | 存在 |
| 交接包 `00_README`：「本目录中的 RED 映射、ADR 映射和任务 DAG 是可审核的拟议投影，不得成为第二份事实 SSOT」 | 本计划的所有结论都指向 git 对象与 RERUN 工件，没有另立事实 | **一致** |
| 交接包「未实跑不得声称 PASS」 | 本文件对 75 场景一律标 STALE，未刷新 | **一致** |

---

## 6. 给属主的三个决策点

1. **矩阵的 `evidence` 字段改成什么形状？** 建议：`{"file": "<repo-relative path>", "test_name": "<exact name>", "artifact_digest": "<sha256>"}` —— 机器可校验，且能反查 RERUN 工件。
2. **`evidence/` 目录要不要收 `.log`？** 当前 0 个 `.log`。建议**不收**（日志会漂移、难校验），改成把原始 stdout 摘要进 RERUN JSON，矩阵只指 JSON + digest。
3. **4 条 T4 RED 的证据缺失是「工件没提交」还是「当时就没生成」？** 这决定它是补工件（阶段 0.3）还是重新复跑（阶段 1.2）。需要 #5912 的原始 run 目录才能判定 —— 而那些 run 目录不在权威 tip 上。

---

*本文件所有数字来自 2026-11-23 在 `8cbde5b6d3` 基线上的实测。75 场景的 PASS 状态一律 STALE（未重跑）。*
