# BASELINE-RECONCILIATION.md — SQLRustGo v4.1.0 基线重新绑定（G0）

> **source_agent**: DSH-B (opencode / MiniMax-M3)
> **source_run**: `SQLRUSTGO-V410-REFACTOR-HARNESS-R1-20261123` · G0 轮
> **timestamp**: 2026-11-23T08:00:00+08:00
> **base_sha**: `8cbde5b6d33c876fa1197474c46d50142a9823bd`（`origin/develop/v4.1.0` tip，本轮全部实测的来源树）
> **evidence_hash**: 见同目录 `manifest.json` 的 `files_digest`（本包 6 份内容文件的 sha256 排序摘要）；`SHA256SUMS` 逐文件列出 sha256。
> **conflict_resolution**: 本轮**只读**调查优先于任何既有文档的说法；凡与既有文档冲突之处，以本文件的实测为准并逐条列出被推翻的说法。涉及 Owner 裁定的条目一律标 `REPORTED` / `UNKNOWN`，不代裁。
> **review_verdict**: `REQUEST_CHANGES`（评审包绑定 `cd8cefe623`，本文件给出更正后的绑定）
> provenance: source_agent=DSH-B (opencode / MiniMax-M3); generated_by=DSH-B/opencode; generated_at=2026-11-23T10:00:00+08:00; base_sha=8cbde5b6d33c876fa1197474c46d50142a9823bd; package=docs/releases/v4.1.0/refactoring/gpt-r1/

---

## 0. 一句话结论

评审包绑定的是**过期基线**：它声称的 `behind 104` 是在一个**已经落后的远端 tip** 上量出来的；重新 fetch 后，真实差距是 **5 ahead / 242 behind**，涉及 **1074 个文件 / +79,865 / −850**。以 `cd8cefe623` 为依据的任何重构结论都不成立。

---

## 1. 权威开发分支的完整 SHA（VERIFIED）

| 项 | 值 | 证据 |
|---|---|---|
| 权威分支 | `origin/develop/v4.1.0` | `git remote -v` → `origin http://192.168.0.252:3000/openclaw/sqlrustgo.git` |
| 完整 SHA | `8cbde5b6d33c876fa1197474c46d50142a9823bd` | `git rev-parse origin/develop/v4.1.0` |
| tip 提交 | `Merge pull request 'fix(storage): 修复合并残留 —— develop/v4.1.0 的 storage crate 编译不过' (#5925) from fix/merge-artifacts-5760-5785` | `git log -1 --pretty=fuller origin/develop/v4.1.0` |
| tip 时间 | Sun Nov 22 19:01:17 2026 +0000 | 同上 |
| 本地 `develop/v4.1.0` | `7fe24a86368f8921d19cc7a6e2b8f2f86144551c` —— **落后 104** | `git rev-list --left-right --count develop/v4.1.0...gitea252/develop/v4.1.0` → `0  104` |
| fetch 时点 | 2026-11-23T07:31+08:00（`git fetch origin` 拉取 200+ 新分支，`develop/v4.1.0` 由 `e9ff21fca3..8cbde5b6d3` 前移） | fetch 输出 |

**为什么要重取**：`origin/develop/v4.1.0` 在本轮之前停在 `e9ff21fca3`，本地 worktree `srg-gagate-r2` 停在更早的 `7fe24a8636`。评审包显然量的是后者。

---

## 2. 评审包基线与权威分支的差异（VERIFIED，并更正评审说法）

评审包写：`所在工作分支 fix/tpch-fixture-handoff-20261117，ahead 5 / behind 104`。

| 对比对象 | ahead | behind | 判定 |
|---|---|---|---|
| `fix/tpch-fixture-handoff-20261117` vs `gitea252/develop/v4.1.0`（= `7fe24a8636`，评审当时用的远端 tip） | 5 | 104 | **VERIFIED — 评审数字本身没算错，但参照物已过期** |
| `fix/tpch-fixture-handoff-20261117` vs `origin/develop/v4.1.0`（= `8cbde5b6d3`，当前权威） | 5 | **242** | **VERIFIED — 真实差距** |
| `cd8cefe623`（评审包基线） vs `origin/develop/v4.1.0` | 5 | 242 | 同上 |

`cd8cefe623` = `fix/tpch-fixture-handoff-20261117` 的 HEAD，提交 `fix(gate): TPC-H 夹具的两套解析链必须指向同一个目录`（MCode，2026-11-17 07:01:28 +0800）。

**diffstat**（`git diff --shortstat cd8cefe623 origin/develop/v4.1.0`）：

```
1074 files changed, 79865 insertions(+), 850 deletions(-)
```

按顶层目录分布（`git diff --name-only` 聚合）：

| 路径 | 文件数 | 说明 |
|---|---|---|
| `audit/v4.1.0/` | 822 | 审计 run / 交接包 / 能力评测工件 —— **评审包完全没覆盖** |
| `scripts/gate/` | 92 | **门禁本体已变 92 个文件** —— 直接否定"基于 `cd8cefe623` 审门禁" |
| `docs/releases/` | 83 | 发布线文档 |
| `tests/integration/` | 19 | 含 `tests/integration/test_factory/`（评审基线上不存在） |
| `crates/storage/` | 19 | 存储层（含 #5925 合并残留修复） |
| `src/` | 8 | 引擎顶层 |
| `scripts/soak/` | 8 | |
| `crates/transaction/` | 3 | |
| `docs/governance/` | 2 | |
| `tools/` | 3 | `verify_srg_d004_acceptance.py`、`test_factory.py`、`test_test_factory_counterexample.py` |
| `evidence/` | 6 | 75 场景矩阵与 RERUN 工件 |

**结论（VERIFIED）**：`cd8cefe623` 与权威 tip 之间隔着 **138 个提交**（`e9ff21fca3` 是 `8cbde5b6d3` 的祖先，`git rev-list --left-right --count e9ff21fca3...origin/develop/v4.1.0` → `0  138`），其中 `scripts/gate/` 单项就有 92 个文件变更。**以 `cd8cefe623` 为基线的门禁结论、覆盖矩阵、能力结论一律作废。**

---

## 3. Codex 交接包的实际落点（VERIFIED）

| 项 | 值 |
|---|---|
| 落点目录 | `docs/releases/v4.1.0/refactoring/test-factory-design/` |
| 文件数 | 16（`00_` … `15_`）+ `SHA256SUMS` |
| 与 `origin/codex/test-factory-design-20261121`（`cf7e9de880`）的关系 | `git diff origin/develop/v4.1.0...origin/codex/test-factory-design-20261121 -- docs/releases/v4.1.0/refactoring/` → **空**（内容逐字节相同） |
| 进入 develop 的路径 | PR #5912（`srg-test-factory-pilot` → `develop/v4.1.0`），合并提交 `462ffb3529` |
| `SHA256SUMS` 校验 | `shasum -a 256 -c SHA256SUMS` → **16/16 OK** |
| `14_HANDOFF_MANIFEST.json` 的 `base_sha` | `e9ff21fca3aa945f6fe543ce8f91dfc19e6aaed0` —— **落后权威 tip 138 个提交（STALE）** |
| `14_HANDOFF_MANIFEST.json` 的 `final_head_sha` | `null` |
| `authority.design_manifest_resign` | `true`（设计侧允许重签） |
| `authority.upstream_c01_manifest_resign` | **`false`**（上游 C01 manifest 未重签） |

**判定：内容完整性 VERIFIED（16/16），绑定 STALE（base_sha 过期 138 提交，final_head 未填）。** 这正是任务书 §6 第 1 项「基线绑定修正」要处理的；`dsh-ops` 修订包时应把 `base_sha` 改成最终 PR HEAD + manifest SHA256 双锚（包内 `binding.head_binding_mode` 已声明该模式，但从未落地）。

---

## 4. 18 个工作包 / 10 份 ADR / 75 场景矩阵（逐项核验）

### 4.1 18 个工作包 — **VERIFIED**

来源：`docs/releases/v4.1.0/refactoring/test-factory-design/11_ADR_PACKAGE_BINDING.json`

| 字段 | 值 | 判定 |
|---|---|---|
| `package_count` | `18` | VERIFIED |
| 实际条目 | `C01-01` … `C01-18`（连续 18 条，无缺号） | VERIFIED |
| `naming_correction_count` | `4`（C01-02 / C01-03 / C01-04 / C01-17） | VERIFIED |
| `implementation_authorized` | 全部 `false` | VERIFIED |
| `human_gates_preserved` | `true`（C01-13、C01-18 带 `human_gate_required`） | VERIFIED |

配套：`09_IMPLEMENTATION_TASK_DAG.jsonl` 是 **7 条**实施任务（`TF-01`…`TF-07`）—— 与 18 个工作包是**两个不同的轴**，不要混为一谈。

**已落地的 C01 证据包**（`docs/releases/v4.1.0/refactoring/evidence/ca01/`）：`C01-05`、`C01-06`、`C01-07`、`C01-09`、`C01-10`、`C01-11`、`C01-14` 共 **7 个**工作包各有 5 份文档（SPEC / TEST_DESIGN / TEST_PLAN / TEST_REVIEW / ACCEPTANCE）。其余 11 个尚无证据包。

### 4.2 10 份 ADR — **REPORTED（文件不在权威树上）**

这是本轮最重要的更正之一。

| ADR 文档 | 实际位置 | 在权威 tip 上？ |
|---|---|---|
| `ADR-410-001` … `ADR-410-010`（10 份） | `docs/releases/v4.1.0/refactoring/adrs/`，存在于 `codex/v410-calpha01-docs`（`45da6968ae`）与 `origin/codex/v410-runtime-5915`（`b681b74ca3`） | **NO** |
| `ADR-410-011` | 仅候选态，`b681b74ca3` 的 `runtime-contract/20261120-pr5915/ADR-410-011-UNIFIED_RUNTIME_CONTRACT.md` | **NO（且只是候选）** |

权威 tip 上 `docs/releases/v4.1.0/refactoring/` 只有两个子目录：`evidence/` 与 `test-factory-design/`。**没有 `adrs/`。**

- `11_ADR_PACKAGE_BINDING.json` 里引用了 11 个 ADR 标识：`ADR-410-001…011`，其中 `APPROVED` 3 份（001/002/005）、`DIRECTION_APPROVED` 6 份（003/004/007/008/009/010）、`CONDITIONAL` 2 份（006/011）、`IMPLEMENTATION_AUTHORIZED` **0 份**。包内 `base_dag_commit = 45da6968ae`、`adr_011_commit = b681b74ca3`。
- **载体是两个未合并的 PR**：
  - PR **#5916** `codex/v410-calpha01-docs` (`45da6968ae`) → `develop/v4.1.0` —— 标题「HARA-410、C 路线架构与 DSH 交接包（待批准）」，**OPEN**
  - PR **#5917** `codex/v410-runtime-5915` (`b681b74ca3`) → `codex/v410-calpha01-docs` —— 「PR5915 运行时证据与统一事务/恢复契约（Owner 待批）」，**OPEN**

**判定**：评审包说的「10 份 ADR」在**设计交接包内部的引用层**成立（VERIFIED 的是 JSON 里的 10 个 `APPROVED`/`DIRECTION_APPROVED` 标识），但**10 份 ADR 文档本体未进入权威基线**（REPORTED，载体是 2 个待批 PR）。任务书 §6 第 2 项要求 dsh-ops 做的「基线绑定修正」必须写清这一点。

### 4.3 75 场景矩阵 — **VERIFIED（内容） / STALE（证据绑定）**

| 项 | 值 | 判定 |
|---|---|---|
| 文件 | `evidence/case-mapping-20261120.json` | VERIFIED |
| 条目数 | **75**（`matrix_index` 0…74，`case_id` 有重复，以 `matrix_index` 为准 —— 这一点 `rerun-summary-20261120.json` 自己写明了） | VERIFIED |
| 状态分布 | `PASS` 68 / `FAILED_RED` 4 / `COVERED_PASS` 2 / `COVERED_RED` 1 | VERIFIED |
| 4 条 `FAILED_RED` | `PILOT-T4-101…104`（`tf_t4_01_scan_after_insert_sees_rows` 等），证据 `evidence/D001-red-run-20261120T165449.log` | VERIFIED |
| 汇总文件 | `evidence/rerun-summary-20261120.json`，`source_agent = DSH-B (opencode-go/glm-5.3-flash)`，`source_run = goal-R5 rerun-mapping-v2` | VERIFIED |
| 汇总的 `base_sha` | `e9ff21fca3aa945f6fe543ce8f91dfc19e6aaed0` —— **落后 138 提交** | **STALE** |

**判定**：75 场景矩阵的**内容**在权威 tip 上确实存在且计数一致；**证据绑定过期**。RERUN 工件（`RERUN-T1*.json`、`RERUN-T2T3*.json`）同样落在 `e9ff21fca3` 的基线上。

### 4.4 交接包 manifest 重签状态 — **STALE**

| 检查项 | 结果 |
|---|---|
| `SHA256SUMS` 覆盖 | 16 个条目，覆盖 `00_`…`15_` 全部 16 份文档（含 `14_HANDOFF_MANIFEST.json` 自身） |
| `shasum -a 256 -c SHA256SUMS` | **16/16 OK** |
| `files_digest`（`44c72a4b…`）自洽性 | 未在本轮独立复算（manifest 声明的算法是 "sha256(sorted sha256sum lines for 00-13 and 15)"）→ **NOT_RUN** |
| `base_sha` | `e9ff21fca3…` → **STALE（138 提交）** |
| `final_head_sha` | `null` → **STALE** |
| 上游 C01 manifest 重签 | `false` → **未做** |

**任务书 §6 第 3 项「SHA256SUMS 实际 16/16」—— 在权威 tip 上实测为 16/16 OK，该项已满足，不需要再修。** 需要修的是 `base_sha` / `final_head_sha` 与上游 C01 manifest 重签。

---

## 5. Issue / PR / 门禁 / 本地能力评测状态刷新（VERIFIED，2026-11-23T07:31+08:00 取）

| 项 | 值 | 取数方式 |
|---|---|---|
| Open Issue 总数 | **105** | `GET /api/v1/repos/openclaw/sqlrustgo/issues?state=open&type=issues` → `X-Total-Count: 105` |
| Open PR 总数 | **2** | `GET .../pulls?state=open` → `X-Total-Count: 2` |
| Open PR 清单 | #5916（head `45da6968ae` → base `develop/v4.1.0`）、#5917（head `b681b74ca3` → base `codex/v410-calpha01-docs`），**均 WIP / 待 Owner 批** | 同上 |
| P0 open issue | #5911 / #5910 / #5909 / #5908 / #5906 / #5905 / #5897 / #5896 / #5895 / #5891 | issue 列表 |
| P1 open issue | #5914 / #5913 / #5836 / #5835 / #5797 / #5794 / #5793 / #5768 / #5735 / #5717 / #5702 / #5698 / #5688 / #5603 / #5599 / #5598 | issue 列表 |
| `STAGE.yaml` `current_stage` | `ALPHA` | `grep current_stage docs/releases/v4.1.0/STAGE.yaml` |
| `merge_policy` | `reviewers: 2`，`approval: codeowner + governance`，`ci_required: [lint, build, test, full-gate, anti-fab]` | `docs/releases/v4.1.0/STAGE.yaml` |

**未合并即未生效**：#5916 / #5917 未合并 ⇒ ADR 文档与 ADR-410-011 候选**不在权威基线上**；#5910 / #5909 / #5908 / #5906 / #5905 未关闭 ⇒ GA 仍被阻断。

---

## 6. 门禁与本地能力评测状态（权威 tip 上的实跑）

### 6.1 门禁清单口径 —— 更正评审的「168 道」与「34 / 85」

| 口径 | 实测值 | 命令 |
|---|---|---|
| `docs/releases/v4.1.0/GATE_CATALOG.jsonl` 行数 | **168** | `wc -l` |
| `docs/releases/v4.1.0/GATE_BASELINE_V410.jsonl` 行数 | **168** | `wc -l` |
| `scripts/gate/` 顶层 `.sh` 文件数 | **196** | `ls scripts/gate/*.sh \| wc -l` |
| `scripts/gate/` 递归 `.sh` 文件数（含 `legacy/`） | **217**（其中 `legacy/` 18 个） | `find scripts/gate -name '*.sh' -type f \| wc -l` |
| `check_drift_not_pass.sh` 实际管辖的门禁数（顶层、排除 `legacy/` 与两个检测器自身） | **194** | 见 §6.1 |
| 分类分布 | `dead` 61 / `ci_reachable` 51 / `stage_required` 34 / `orphan_referenced` 22 | `GATE_CATALOG.jsonl` 的 `class` 字段 |

- **「168 道门禁」= 台账口径，不是脚本文件数**。脚本侧：顶层 `scripts/gate/*.sh` **196** 个；递归（含 `scripts/gate/legacy/` 的 18 个历史实现）**217** 个；`check_drift_not_pass.sh` 实际管辖的是**顶层排除 `legacy/` 与两个检测器自身后的 194** 个。评审要求「不批量删除 168 道门禁」——按台账口径理解是对的，但执行时必须知道这三个数不是同一个数。
- **「34 道阶段绑定」= `stage_required` 34 —— VERIFIED，精确吻合**。
- **「85 道静态可能可达」—— 无法从这两份 168 行台账复算**：可推导的拆分是 `ci_reachable + orphan_referenced = 73`、`dead + orphan_referenced = 83`、`stage_required + ci_reachable = 85`，均与评审表述不吻合。**判定 UNKNOWN**，需 dsh-ops 给出 85 的推导口径。任务书 §6 第 4 项「区分 34 道阶段绑定与 85 道静态可能可达」在拿到口径之前无法执行。

### 6.2 五道候选门禁在权威 tip 上的实跑（整改前）

| 门禁 | rc | 关键输出 |
|---|---|---|
| `check_drift_not_pass.sh` | 0 | `✅ PASS`（14 条已入基线的历史 finding 被吸收） |
| `check_gate_test_integrity.sh` | 0 | `48 gate tests, 0 NEW #[ignore]`（2 条基线豁免） |
| `check_evidence_binding.sh v4.1.0` | 1 | `PASS=286 WARN=301 FAIL=85 UNVERIFIED=84` |
| `check_architecture_freeze.sh` | 0 | `PASS: 5/5` —— 但 A7-1 打了 `INFO (5 calls found)` |
| `check_anti_fabrication.sh` | — | 需真实 cargo/sqlancer，见 §6.4 |

### 6.3 工具链基线状态（权威 tip，未被本轮改动）

| 检查 | 结果 | 判定 |
|---|---|---|
| `cargo fmt --check --all` | **14 个文件有 rustfmt 违规**（`crates/storage/src/engine.rs`、`crates/storage/src/mvcc.rs`、`crates/storage/tests/memory_txlog_db_scope_5897.rs`、`crates/storage/tests/tf_t4_ai_candidate_v1.rs`、`crates/storage/tests/tf_t4_stub_silent_success.rs`、`src/engine_dml.rs`、`src/execution_engine_methods.rs`、`tests/integration/sql/pk_cache_file_backend_test.rs`、`tests/integration/sql/undo_db_scope_merge3.rs`、`tests/integration/storage_engine_api_test.rs`、`tests/integration/test_factory/ca01_commit_frontier.rs`、`ca01_file_undo.rs`、`ca01_snapshot_reopen.rs`、`ca01_visibility_oracle.rs`） | **PRE-EXISTING RED** —— 权威 tip 上 fmt 门禁本身就是红的 |
| `cargo clippy --all-features --all-targets` | **6 条 warning**，全部在 `tests/integration/test_factory/ca01_backend_caps.rs` 与 `ca01_commit_frontier.rs` | **PRE-EXISTING** —— AGENTS.md 要求 0 警告，权威 tip 达不到 |

> 这两条是本轮**顺带发现**的基线事实：评审包完全没有提到权威 tip 的 fmt/clippy 是红的。本轮改动**未新增**任何 fmt 或 clippy 违规（见 `HARNESS-P0-FINDINGS.md` §6）。

### 6.4 本地能力评测状态

| 项 | 值 | 判定 |
|---|---|---|
| Swift 端点 `250:8021` | **存活**。`GET /v1/models` → `{"id": "swift-27b-iq3xxs-mtp", "owned_by": "llamacpp", "n_params": 27320697856, "ftype": "BF16", "n_ctx": 180224}` | VERIFIED |
| `250:8011` | HTTP 200，**保持 `GMP_RESERVED`，本轮未做任何调用**（仅一次只读 `/v1/models` 探活） | VERIFIED（边界遵守） |
| Swift 实测能力 | 见 `EXECUTOR-CAPABILITY-PLAN.md` §2（A/D 两项实跑，B/C/E 因准入未就绪为 NOT_RUN） | 见该文件 |

---

## 7. 证据来源 SHA 与真实采集时间（逐条绑定）

| 证据 | 来源 SHA | 采集时间 | 采集方式 |
|---|---|---|---|
| 权威 tip SHA / 提交身份 | `8cbde5b6d3` | 2026-11-23T07:31+08:00 | `git rev-parse` / `git log -1 --pretty=fuller` |
| ahead/behind / diffstat | `cd8cefe623` ↔ `8cbde5b6d3` | 2026-11-23T07:32+08:00 | `git rev-list --left-right --count` / `git diff --shortstat` |
| 交接包 SHA256SUMS 16/16 | `8cbde5b6d3` 工作树 | 2026-11-23T07:33+08:00 | `shasum -a 256 -c SHA256SUMS` |
| 18 工作包 / ADR 绑定 | `8cbde5b6d3` 工作树 | 2026-11-23T07:33+08:00 | `cat 11_ADR_PACKAGE_BINDING.json` |
| ADR 文档实际位置 | `45da6968ae` / `b681b74ca3` / `8cbde5b6d3` | 2026-11-23T07:34+08:00 | `git ls-tree -r <ref>` |
| 75 场景矩阵 | `8cbde5b6d3` 工作树 | 2026-11-23T07:35+08:00 | `python3` 解析 `evidence/case-mapping-20261120.json` |
| Issue / PR 计数 | Gitea API 实时 | 2026-11-23T07:36+08:00 | `X-Total-Count` 响应头 |
| 门禁清单 168（台账）/ 196（顶层 .sh）/ 217（递归 .sh）/ 194（漂移门禁管辖数） | `8cbde5b6d3` | 2026-11-23T07:38+08:00 | `wc -l` / `find` |
| fmt / clippy 基线 | `8cbde5b6d3` | 2026-11-23T08:05+08:00 | `cargo fmt --check --all` / `cargo clippy --all-features --all-targets` |
| Swift 端点探活 | 250:8021 实时 | 2026-11-23T07:40+08:00 | `GET /v1/models` |

---

## 8. 本轮被推翻或需要 dsh-ops 更正的既有说法

| # | 既有说法 | 实测 | 处置 |
|---|---|---|---|
| 1 | 评审包基线 `cd8cefe623` 相对 develop「behind 104」 | 相对**当前** `origin/develop/v4.1.0` 是 **behind 242** | **更正**（§2） |
| 2 | 「交接包 manifest 需要重签」「SHA256SUMS 实际 16/16」 | `SHA256SUMS` 在权威 tip 上实测 **16/16 OK** | **该子项已满足**；待修的是 `base_sha` / `final_head_sha` / 上游 C01 重签 |
| 3 | 「18 工作包 / 10 ADR / 75 场景矩阵」在同一基线上 | 工作包 18 与场景矩阵 75 **在**权威 tip；**10 份 ADR 文档不在**，载体是两个未合并 PR | **拆分标注**（§4） |
| 4 | 「168 道门禁」 | 台账 168（`GATE_CATALOG.jsonl`）；脚本顶层 196 / 递归 217 / 漂移门禁管辖 194 | **口径澄清**（§6.1） |
| 5 | 「34 道阶段绑定与 85 道静态可能可达」 | 34 精确吻合 `stage_required`；**85 不可复算** | 34 VERIFIED / 85 UNKNOWN |
| 6 | 权威 tip 的 lint/fmt 是绿的 | **fmt 14 文件红、clippy 6 warning** | **新增基线事实**（§6.3） |
| 7 | `audit/v4.1.0/commit.txt` 是当前冻结点 | 内容 `93c8a9dbbe9f…` ≠ HEAD `8cbde5b6d3…` | **STALE**（合并提交 `712eff7e18` 恢复了它，但相对 HEAD 仍落后） |
| 8 | `audit/v4.1.0/canonical_run_aliases.json` 存在 | **不存在** | canonical run-id 改名机制**当前失效**（见 `CONTROL-PLANE-GAP-MATRIX.json` Q3） |

---

## 9. 证据哈希

本包 6 份内容文件的 sha256 由同目录 `SHA256SUMS` 逐文件列出；覆盖全部 5 份的排序摘要写在 `manifest.json` 的 `files_digest`（该 manifest 自身不含自己的哈希，避免自引用；`SHA256SUMS` 同时覆盖 `manifest.json`）。

复算方法：

```
$ cd docs/releases/v4.1.0/refactoring/gpt-r1 && shasum -a 256 -c SHA256SUMS
```

**注意**：哈希校验只证明「内容没变」，**不证明「内容对应当前基线」** —— 这正是本轮在 G1 修掉的那类缺陷（交接包 16/16 OK 但 `base_sha` 落后 138 提交）的教训。所以本包每份文件的头部都单独写明 `base_sha`，并把「内容完整性」与「基线绑定」分成两件事报告。

---

## 10. 结论与未决项

**VERIFIED**：权威基线 = `8cbde5b6d33c876fa1197474c46d50142a9823bd`；评审包基线落后 242 提交 / 1074 文件；交接包落点 = `docs/releases/v4.1.0/refactoring/test-factory-design/`（SHA256SUMS 16/16 OK，但 base_sha 过期 138 提交）；18 工作包与 75 场景矩阵在权威 tip 上；Open Issue 105 / Open PR 2；Swift 250:8021 存活。

**REPORTED**：10 份 ADR 文档在两个未合并 PR 上（#5916 / #5917），不在权威基线。

**STALE**：交接包 `base_sha` / `final_head_sha`；75 场景矩阵与 RERUN 工件的 `base_sha`；`audit/v4.1.0/commit.txt`；`canonical_run_aliases.json` 缺失。

**UNKNOWN**：「85 道静态可能可达」的推导口径。

**NOT_RUN**：交接包 `files_digest` 的独立复算；`check_anti_fabrication.sh` 在权威 tip 上的完整实跑（需真实 `cargo build --release -p sqlancer` 与 test-runner，已在 `HARNESS-P0-FINDINGS.md` §5 用桩环境证明其过期证据通道，本轮未在权威树上全量实跑）。

**本轮未做（按边界）**：未修改任何既有文档；未改动 `cd8cefe623` 所在工作树；未 reset / 未自动合并；未推进 STAGE；未关闭任何属主确认 Issue；未启动 SOAK；未对生产库或生产服务器做任何写操作。
