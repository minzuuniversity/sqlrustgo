# SQLRustGo v4.1.0 重构准入整备设计（R1 整改 · 方案 A）

> design_status: **PROPOSED — 待属主批准后转为实施计划**
> revision: 2（已并入 builder check 的 18 项复核结论）
> prepared_by: DSH-B 主导（本会话）
> date: 2026-11-23
> 依据: Codex Desktop R1 独立审核（`/Volumes/workspace/dev/workspaces/codex-review/sqlrustgo-v4.1.0/reports/round-01/`）
> 范围: 本文只解决「4.1.0 如何整备到可以开始重构的程度」。**本文本身不授权任何重构执行。**

---

## 0. 总体裁决（校准后）

| 事项 | Codex R1 裁决 | 本文裁决 | 差异原因 |
|---|---|---|---|
| 发布 | NO-GO | **NO-GO** | 一致。75 场景与 SOAK 无可继承的当前树证据 |
| 阶段跃迁 ALPHA→BETA | NO-GO | **NO-GO** | 一致。阶段出口证据不足以支撑跃迁 |
| 启动正式重构 | NO-GO | **NO-GO，但理由不同** | 见 §0.1 |

### 0.1 为什么理由不同

Codex 把阻断归因于「执行控制面不可信」（Harness 接线、门禁吞退出码、证据绑定未闭环）。
本文核验后认为：**阻断是「料」的问题，不是「设计不可信」的问题。**

1. **重构材料根本不在 develop 上**（`docs/refactor/v4.1.0-c/` 在 develop tip 上 0 个文件；DAG/ADR/matrix/tools 只在 `45da6968ae`，该提交**不是 develop 祖先**）
2. 三层基线互不相同，无权威 base
3. 10 份 ADR 已被属主裁决，但**机器可核字段仍是 `PROPOSED` / `[]`**
4. handoff manifest 3 文件漂移；`verify_handoff.py --check` 因硬编码基线与硬断言无法跑完
5. 75 场景矩阵 75/75 为 `NOT_RUN` + `UNREVIEWED`，且其测试来源多数指向 242 提交前的清单快照
6. 18 包 DAG 内**没有任何核心编码包**，全部是 `preparation_test_or_analysis`

因此整备路径是「让材料自洽 + 让门禁说真话」，而非「重建执行控制面」。
**这意味着整备成本远低于 Codex 设想的 G0–G6 七阶段路径。**

### 0.2 反目标（明确不做）

- **不新建第二套 Harness / 调度器 / 账本**。复用 `deepseek-schedule` 现有 dispatcher 与 `tools/test_factory.py`。
- **不批量处置 83 道 dead/orphan 门禁**。门禁清单变更需属主另行批准，不属本次准入前置。
- **不物理拆分 T0–T5 六层**。T0–T6 仅作为测试分类标签。
- **不修改 `STAGE_CONFIG.yaml` 的阶段清单语义**。
- **不因整备而放宽任何门禁阈值**。
- **不改写冻结的历史证据绑定**（见 §5.1.4 的前向/历史分界规则）。

---

## 1. Codex R1 的校准结果

我对 15 条 findings 逐条独立复现（可访问仓库与全部 worktree，Codex 只有 ZIP）。

### 1.1 成立（复现一致）

| ID | 严重度 | 复现证据 |
|---|---|---|
| R1-F-001 | HIGH | `EVIDENCE-INDEX.json`=24/41/6=71；`EVIDENCE-COVERAGE.md`=21/41/6=68（止于 E-068） |
| R1-F-002 | HIGH | `01-EXECUTIVE-SUMMARY.md`=13/45/6=64 |
| R1-F-003 | HIGH | `01-EXECUTIVE-SUMMARY.md:54`「61 道…全部仍有文档引用」↔ `04-GATE-GOVERNANCE.md:36-37`「61 无引用 / 22 有引用」。错误侧共 4 处 |
| R1-F-004 | HIGH | `15-DEAD-GATE-DISPOSITION.md:16` 称 134 道 stages-empty「无执行点」，其自表列 `ci_reachable=51`。真实「既无阶段又无 CI」=83 |
| R1-F-005 | MEDIUM | ZIP 20 / MANIFEST 18，隐式自排除未声明 |
| R1-F-009 | HIGH | 159 为静态候选计数；负向验证 NOT_RUN |
| R1-F-010 | HIGH | `03-HARNESS-ARCHITECTURE.md:56` 自述「39/39 pass，但正式派发回路尚未接线」 |

### 1.2 需要修正

**① R1-F-011（CRITICAL）—— 事实对，归因错。**
Codex 写「18 包 DAG、10 ADR、75 场景矩阵缺失」。它们**在仓库中完整存在**：

```
.worktrees/codex-calpha01-20261120/docs/releases/v4.1.0/refactoring/   @ 45da6968ae
├── REFACTOR_TASK_DAG.json        18 包 C01-01..C01-18
├── TEST_COVERAGE_MATRIX.json      75 场景
├── adrs/ADR-410-001.md .. 010.md  10 份
└── tools/verify_handoff.py        --finalize / --check
```

Codex 只拿到 ZIP，其措辞「absent from the approved review inputs」技术上无误，
但整改动作会被误导为「从零造材料」。实际动作是**导入基线 + 填字段 + 重签**。

更重要的是：实测这 18 包 `task_type` 全为 `preparation_test_or_analysis`、
`milestone_id` 全为 `C-ALPHA-01`——**当前 DAG 内本就没有核心重构包**。
所以真正阻断不是「看不见重构顺序」，而是「重构对象尚未进入 DAG」。

**② R1-F-006 / R1-F-007（基线）—— UNKNOWN 实为更严重。**
Codex 无仓库访问权，只能标 UNKNOWN。实测三层基线互不相同：

| SHA | 身份 | 相对 `origin/develop/v4.1.0` |
|---|---|---|
| `cd8cefe623` | 外审包采集树（`fix/tpch-fixture-handoff-20261117`） | 分叉，落后 **242**，独有 5 个未合并提交 |
| `e9ff21fca3` | 18 包 DAG `base_sha`；HANDOFF G5 认可基线 | 祖先，落后 **138** |
| `8cbde5b6d3` | develop/v4.1.0 tip（2026-11-22） | 最新 |

包内声明的 "behind 104" 已过期 6 天，实测 **242**。且包采在**特性分支**上，不是 develop。

**③ R1-F-012（GPT-6 报告 / G0–G6 任务书缺失）—— 措辞误判。**
Codex 把 `REVIEW-PLAN.md` 的 G0–G6 当成「GPT-6 任务书」并否定它，
但 G0–G6 本身就是外审包流程，不是整改序列。Codex 自己给出的 G0–G6 替代表才是可用序列。
真实缺口是：**没有一份可核的「GPT-6 独立评审全文」**，需属主指认。

### 1.3 Codex 未发现、本文新增的三点

**① 门禁编目自相矛盾（新增，MEDIUM-HIGH）**

`docs/releases/v4.1.0/GATE_CATALOG.jsonl` 168 行：

- `neg_fixture` 自动派生（`gov_probe.NEG_FIXTURE_RE` 扫 `selftest|self_test|negative|反例|反向变异|canary|负向`），实测 **24 条为 true**
- 但 **7 条** `neg_fixture=false` 的门禁，其人工填写的 `effect.catches` 非空且 reason 写明
  「由夹具 fxNN-… 证明（kind=mutation-verified）」
- 全表 **166/168** 条 `effect.reason` 提到「夹具」

即：`neg_fixture` 字段的自动正则**识别不出真实存在的夹具**，登记表内部数字互不一致。
后果：**不能拿 `neg_fixture=24/168` 论证「只有 14.3% 门禁可证伪」**。
整改：以 `effect` 声明 + 夹具实物双向核对为准，修正自动派生规则。

**② 75 场景并非「从未跑过」—— 存在未被继承的复跑证据（新增，关键）**

`/Volumes/workspace/dev/srg-test-factory-pilot/evidence/rerun-summary-20261120.json`：

```json
{ "base_sha": "e9ff21fca3aa945f6fe543ce8f91dfc19e6aaed0",
  "total_cases": 75, "reexecution_pass_count": 70, "red_confirmed": 5,
  "status_counts": {"PASS":68,"FAILED_RED":4,"COVERED_PASS":2,"COVERED_RED":1} }
```

Codex 的「0 实跑」来自外审包自述（`reexecution=0`），**未看到 pilot 的真实复跑产物**。
但该证据绑定的是 `e9ff21fca3`，且 `TEST_FACTORY_INTEGRATION_PLAN.md` 声明 pilot 早期申报
「经 hold 报告降为 2CONFIRMED+3PARTIAL，不得引用其 919/4/33 为新实跑」。
结论：**证据存在但不可直接继承**——A3 的成本是「重绑到新基线并独立复核」，
不是「从零定义并跑 75 个测试」。

另一份更权威的对照基线本文此前遗漏：
`takeover/dsh-b/RED-LEDGER.md` + `RED-IDENTITY-RECONCILIATION.md`（**17 项 RED 五元组身份**：
测试 fn + package/target + source sha256 + rc + 归属），同样绑定 `e9ff21fca3`，
逐条原始日志在 `takeover/dsh-b/artifacts/`。A3 的差异表须**同时**对这两份。

**③ stage runner 的真实问题不是「不在 CI」，是「ALPHA 没有执行点」**（修正初稿）

初稿误述「`check_stage.sh` 从未被 CI 调用」。实测相反：

```yaml
# .gitea/workflows/ci.yml
- name: Stage gates — BETA list executes for real (beta/v* carrier)
  if: startsWith(github.ref, 'refs/heads/beta/')
  run: bash scripts/gate/check_stage.sh --version v4.1.0 --stage BETA
# RC 同理（rc/v* carrier）、GA 同理（main / release/v* / hotfix/v* carrier）
```

`check_stage.sh` **已在 CI 中**，且在 promotion 时真实执行。但三个 `if:` 条件分别是
`beta/`、`rc/`、`main|release/|hotfix/`——**ALPHA 的 carrier 是 `develop/v*`，不匹配任何一条**。
`ci.yml:1694-1697` 另有一条明确记录：

> 三态 runner 本身**尚未接线到执行门禁**——在 CI 中执行完整阶段清单另行追踪（kanban **IL-7**），
> 因为在已知红的分支上那个 job 会故意变红。

且 `check_gate_executability.sh`（#5461/#5450）**已经存在**并在 CI 中运行，
用于挡住「往 STAGE_CONFIG 加一个没人跑的门禁」。

真实缺口因此收敛为两条，都比初稿窄得多：
- **ALPHA 阶段清单在 CI 中无执行点**（BETA/RC/GA 都有）
- **三态 runner 的执行接线未做**（IL-7），当前只跑自测

---

## 2. 目标 / 非目标 / 反目标

### 目标

G1. 存在唯一权威实施基线，且该基线**包含全部重构材料**
G2. 10/11 份 ADR 的批准状态在机器可核字段上落地，`approved_adr_refs` 绑定完成
G3. handoff manifest 无漂移，`verify_handoff.py --check` 全链通过（含 8 个反向变异）
G4. 75 场景产出绑定新基线的可继承复跑证据，且每个场景有 invariant 映射与真实断言
G5. ALPHA 阶段清单在 CI 中有执行点，且该路径退出码可传播、有负向夹具证明
G6. 属主可在上述条件满足后，依据明确判据决定是否放行 C-ALPHA-02

### 非目标

- 不在本设计内实施 A1–A5 的具体动作（属实施计划与执行阶段）
- 不决定 18 包的具体内容与排期（属 DAG 修订）
- 不做门禁清单裁剪与批量归档
- 不推进 ALPHA→BETA 或任何阶段跃迁

### 反目标（做到以下任一即视为失败）

- 为通过检查而放宽门禁阈值、延长超时、或把 INCOMPLETE 降级为 PASS
- 用 `REPORTED`/`CLAIMED` 状态冒充 `VERIFIED`
- 让 `NOT_RUN` / `KNOWN_FAIL` / `UNSUPPORTED` / `DIALECT_DIFFERENCE` 进入阶段出口计数
- **把已知的 RED 塌陷成普通 FAIL**（会摧毁 RED 五元组身份，破坏 `RED-LEDGER.md` 的 17 项台账）
- 保留第二个调度器、第二个任务状态机或第二份权威账本
- 在 `approved_adr_refs` 仍为空时启动任何核心编码包
- **改写冻结的历史证据绑定**（历史 receipt 的 SHA 是审计事实，不得回填）

---

## 3. 约束

| # | 约束 | 来源 |
|---|---|---|
| C1 | 门禁清单变更、阶段跃迁、ADR 批准、SOAK 启动为属主专属 | `OWNER_DECISIONS_LOG.json` / `ADR-DECISION-RECORD.md` |
| C2 | 不得新建第二套 Harness / 调度器 | `TEST_FACTORY_INTEGRATION_PLAN.md`：`new_scheduler_or_runner: false` |
| C3 | `SQLRUSTGO_LOCAL_ONLY` 材料禁止云端 fallback，分类缺失即 BLOCKED | `LOCAL_ONLY_ATTESTATION.md` / `DATA-CLASSIFICATION.md` |
| C4 | 所有执行必须在隔离 worktree，不在权威工作区直接改 | `IMPLEMENTATION-PLAN-REVIEW.md` §4 |
| C5 | Receipt 必须绑定完整 commit SHA，跨 SHA 证据不可继承 | `05-TESTING-AND-EVIDENCE.md` |
| C6 | ADR 批准指向固定 commit 的正文与 sha256，内容变更须重审 | `ADR_INDEX.md:7` |
| C7 | Oracle 由 Owner/独立 Reviewer 签署，实施者不得自签 | `05_TEST_FACTORY_CONTRACT.json`：`oracle_owner` |
| C8 | ALPHA 阶段门禁不得在每次 push 全量执行 | `ci.yml:1700-1704`：「每次提交都必须跑的门禁清单，就是永远红的门禁清单」 |

---

## 4. 方案选择

| 方案 | 内容 | 取舍 | 结论 |
|---|---|---|---|
| **A（本文）** | 基线归位 → 契约闭环 → 验收基线 → 门禁最小集 → 放行判据 | 5 步；不新建控制面；门禁只做 ALPHA 执行点 + 7 道负向夹具 | **已选** |
| B（Codex 原序 G0–G6） | 先全面治理执行控制面（含 168 门禁重分类、批量处置、Harness 六层评审）再动内核 | 最稳，但 7 阶段且数据库内核一行动不了 | 未选 |
| C 先重构后治理 | 契约未批准即动 Storage/事务/WAL | 违反 C6 与冻结契约 | 排除 |

**A 与 B 的唯一实质差别**：A4 只做「ALPHA 阶段执行点 + ALPHA 7 道负向夹具」，
不碰其余 161 道门禁，也不做 83 道 dead/orphan 处置（需属主单独批准）。

---

## 5. 设计

### A1 基线归位 —— 冻结唯一实施基线

#### A1.1 材料导入（初稿遗漏，builder check P0-Q1）

实测 develop tip 上：

```
docs/releases/v4.1.0/refactoring/   → 只有 evidence/ 与 test-factory-design/ 两个子目录
docs/refactor/v4.1.0-c/             → 0 个文件
REFACTOR_TASK_DAG.json / TEST_COVERAGE_MATRIX.json / ADR_INDEX.md
tools/verify_handoff.py / adrs/ADR-410-001.md  → 全部不在 develop 上
45da6968ae 是否 develop 祖先 → NO
```

即 A1 的产物树默认**不含** A2 要改的任何文件。
因此 A1 的 cherry-pick 清单是 **4 个，不是 3 个**：

| # | 提交 | 带入内容 | 在 develop tip 上的状态 |
|---|---|---|---|
| 1 | `45da6968ae`（`codex/v410-calpha01-docs`） | `docs/releases/v4.1.0/refactoring/{DAG,matrix,adrs,tools,audit}/` + `docs/refactor/v4.1.0-c/handoff/` | **全新**（123 文件，+64,685 行，clean） |
| 2 | `cd8cefe623` | TPC-H 夹具两套解析链修复 | **唯一真 delta**，含 1 处冲突 |
| 3 | `9a61cf59c0` | soak helper 可执行位 | **已为空**（内容与 mode 位 develop 均已有） |
| 4 | `cfd4306b7e` | 两个测试文件 rustfmt | **已为空**（同上） |

3、4 已在 develop 上经 PR 合入，cherry-pick 时用 `--skip`，并在
`BASELINE_FREEZE.json` 的 `cherry_picked` 字段记为 `already-present`（不留空、不伪造）。

#### A1.2 冲突处理

`git merge-tree` 预演：全树**仅 1 个真冲突** —— `scripts/gate/check_g1_tpch_baseline.sh`
（develop 侧与 HEAD 侧都改过同一文件，冲突 hunk 在 177–189 行共 12 行）。
其余 7 个文件自动合并（`stage_test_binaries.py`、`soak_steady_window.py` 的差异是 mode 位，
develop 侧已含相同 mode 与内容）。

#### A1.3 基线身份与落点（初稿方向写反，builder check P0-Q3）

**修正**：BASE = `8cbde5b6d3` + 材料导入（`45da6968ae` 全提交 + `b681b74ca3` 的 `runtime-contract/20261120-pr5915/` 目录）+ 冲突解决后的内容，因此
**develop tip 是 BASE 的祖先**，而非 BASE 是 develop 的祖先。
出口判据相应改为：`git merge-base --is-ancestor origin/develop/v4.1.0 BASE` 成立。

**`BASE` 的准确定义（实施中确立）**：`BASE` 是**内容基线提交** = develop tip + 材料导入 + 冲突解决后的内容。
它是 develop tip 的**后代**、分支 tip 的**祖先**。后续所有任务的 `base_sha` 字段一律填 `BASE`，
**不随每次提交前移**——否则「冻结」失去意义，且 receipt 的 SHA 绑定会逐提交漂移。
（提交无法包含自身 SHA，故 `base_sha` 只能是内容基线，不能是含 freeze 记录的那次提交。）

**落点**：含 `BASE` 的分支必须是**已推送**的，否则所有绑定 BASE 的 receipt 无法被 CI 或其它 agent 校验，
「唯一权威基线」只是本机事实。
落点分支：`base/v4.1.0-frozen`（新建，推送到 Gitea）；分支 tip 前进于 `BASE`。
是否直接推进 `develop/v4.1.0` 到 BASE 属主决定（见 §6 O11）。

冻结产物 `docs/refactor/v4.1.0-c/takeover/BASELINE_FREEZE.json`：

```json
{ "base_sha": "...", "tree_sha": "...", "parent": "8cbde5b6d3...",
  "frozen_at": "...", "source_ref": "base/v4.1.0-frozen",
  "cherry_picked": [{"sha":"45da6968ae","state":"applied"},
                    {"sha":"cd8cefe623","state":"applied","conflict":"scripts/gate/check_g1_tpch_baseline.sh"},
                    {"sha":"9a61cf59c0","state":"already-present"},
                    {"sha":"cfd4306b7e","state":"already-present"}],
  "material_import": {"from":"45da6968ae","files":123} }
```

#### A1.4 SHA 重指范围（前向 vs 历史，builder check P0-Q4）

初稿「所有 `base_sha` 等于 BASE」**不可满足**，且会篡改审计轨迹。
改为分类规则：

| 类别 | 处理 | 实例 |
|---|---|---|
| **前向绑定**（必须改为 BASE） | 改 | `REFACTOR_TASK_DAG.json` `tasks[].base_sha`（18）、`TEST_COVERAGE_MATRIX.json` `cases[].base_sha`（75）、`handoff/03_TASK_MANIFEST.jsonl` `base_commit`（18）、`handoff/10_HANDOFF_MANIFEST.json`、`refactoring/test-factory-design/14_HANDOFF_MANIFEST.json` 的 `base_sha`、handoff 生成物 `01_ARCHITECTURE_BASELINE.json` |
| **历史绑定**（必须保留原值） | 不改 | `audit/v4.1.0/commands_pin_00*.yaml` 与 `runs/*/manifest.yaml` 的 `frozen_commit`、pilot `evidence/RERUN-T1*.json` 与 `rerun-summary-20261120.json` 的 `base_sha`、`docs/releases/v4.1.0/IAIA_410_SSOT.md` 等历史文档 |
| **ADR 正文绑定** | 按裁决记录 | `ADR-DECISION-RECORD.md:11-13` 绑定的 `45da6968ae` / `c722885f…` 是 ADR 原文的提交身份，**不得改**；新批准记录另起字段引用 BASE 下的 ADR sha256 |

**验收方式**：前向绑定集合内 `base_sha` 唯一且等于 BASE；历史绑定集合保持原值不变（逐项 diff 校验）。

#### A1.5 出口判据

- `origin/develop/v4.1.0` 是 `BASE` 的祖先；含 `BASE` 的 `base/v4.1.0-frozen` 已推送（分支 tip 是 `BASE` 的后代）
- `BASELINE_FREEZE.json` 落盘；其 `base_sha` 是实际 HEAD 的**祖先**（内容基线），`tree_sha` == `git rev-parse <base_sha>^{tree}`
- 前向绑定字段扫描：唯一值 = BASE；历史绑定字段扫描：逐项未变
- `45da6968ae` 引入的 123 个文件全部在 BASE 树内

---

### A2 契约闭环 —— ADR 批准落地 + manifest 重签

属主已于 2026-11-21 01:28 裁决 11 份 ADR（`ADR-DECISION-RECORD.md:35-47`）：

| 档位 | ADR | 前置条件 |
|---|---|---|
| 无条件批准 | 001、002、005 | — |
| 条件批准 | 006、011 | 006 需「五窗口故障语义表冻结」；011 需「7 项补充验收契约冻结」。**两者前置均未完成** |
| 仅批准方向 / 实施另审 | 003、004、007、008、009、010 | — |

**关于「10 份还是 11 份」（builder check P0-Q6）—— 已裁决（O7）**：
`adrs/` 目录原本只有 ADR-410-001..010；ADR-410-011 位于
`refactoring/runtime-contract/20261120-pr5915/ADR-410-011-UNIFIED_RUNTIME_CONTRACT.md`
（commit `b681b74ca3`，分支 `codex/v410-runtime-5915`）。
裁决账本明写「011 在 45da 交接包 manifest 之外……Codex 重签时应并入」。

**属主裁决：并入 `adrs/`，编目为 11 份；但因 011 属「条件批准、7 项验收契约冻结前不授权」，
本轮不得进入任何 task 的 `approved_adr_refs`。**
故：`verify_handoff.py:122` 改为 `assert len(baseline["adrs"])==11`；
`adrs/` 目录含 001..011；`approved_adr_refs` 的取值集合仍限 001..010。
011 在 `ADR_INDEX.md` 中状态列标为「条件批准（前置未满足）」，与 006 同档。

#### A2.1 完整改动闭包（初稿只列了 3 行，实际远不止，builder check P0-Q5）

**`refactoring/tools/verify_handoff.py`：**

| 行 | 现状 | 动作 |
|---|---|---|
| 16 | `BASE = "e9ff21fca3…"` 硬编码 | 改为从 `BASELINE_FREEZE.json` 读取 |
| 60 | `assert t["approved_adr_refs"]==[]` | 改为「非空 ∧ 每项为真实存在的 ADR id」 |
| 65 | base_sha 断言 | 随 16 参数化 |
| 89 | `status:"PROPOSED", approval_refs:[]` | 按裁决档位填 `status`，填 `approval_refs` |
| 90 | `"tree":"4194fc97…"` 硬编码 + `"adr_approval":"PENDING"` | 改为实际 tree + 已批准档位 |
| 116 | `assert sha(HARA MANIFEST.json)=="4649fccc…"` | 改为读取实际值比对 |
| 122 | `assert len(baseline["adrs"])==10` | 改为 `==11`（O7：011 并入编目） |
| 123 | `assert d["approval_refs"]==[]` | 改为「非空 ∧ ⊆ 真实 ADR id 集合」 |
| 132-133 | `assert all(p.startswith(("docs/releases/v4.1.0/refactoring/","docs/refactor/v4.1.0-c/")) for p in changed)` | 见 A2.4 的 commit 边界约束 |

**其余文件：**

| # | 文件 | 改动 |
|---|---|---|
| 1 | `refactoring/REFACTOR_TASK_DAG.json` | `tasks[i].approved_adr_refs` 18 处（行 38/176/320/…/2561） |
| 2 | `refactoring/ADR_INDEX.md` | 状态列 10/11 行 + 第 7 行说明 + 第 3-4 行 `evidence_hash` |
| 3 | `refactoring/adrs/ADR-410-001..010.md` 第 6 行 | `Status:` / `Approvals:` 从 `PROPOSED`/`[]` 改为档位 + 裁决引用 + 绑定 sha256 |
| 4 | `refactoring/runtime-contract/20261120-pr5915/ADR-410-011-*.md` | **移入 `adrs/`**（O7），第 6 行 `Status:` 标「条件批准（前置未满足）」、`Approvals:` 填裁决引用 |
| 5 | `handoff/03_TASK_MANIFEST.jsonl` | 18 行 `base_commit` → BASE |
| 6 | `refactoring/test-factory-design/14_HANDOFF_MANIFEST.json` | 第二份 manifest 的 `base_sha`/`adr_base_commit`/`files_digest`；其 `authority.upstream_c01_manifest_resign:false` 需显式裁决（O12） |

#### A2.2 ADR→task 映射规则（初稿只说「填入对应 ADR id」，无来源，builder check P2-Q16）

来源：`ADR_INDEX.md` 的依赖图（001/002→003/004；005/006/007；008←002/006；009←all；010←005/007）
+ DAG 的 `invariant_ids`/`epic_id`/`target_files`/`risk_level`。

**不得**采用「非空即可」的宽松规则——那会让反目标形同虚设。规则为：

1. 每个 task 的 `approved_adr_refs` **至少 1 项**
2. 引用项必须是该 task 的 `invariant_ids` / `target_files` 实际关联的 ADR
3. **档位为「仅批准方向/实施另审」的 ADR（003/004/007/008/009/010）不得被
   `authorized_for_execution=true` 的 task 引用**（实施另审即不得实施）
4. 档位为「条件批准」的 ADR（006/011）**不得被引用**，直至其前置冻结物存在并被记录
5. 规则由属主在 §6 O10 确认；执行者不得自拟映射

#### A2.3 顺序与重签

顺序不可颠倒：

```
1. ADR 正文状态更新（A2.1 第 2/3/4 项）
2. DAG approved_adr_refs 填充（A2.1 第 1 项）
3. verify_handoff.py 断言放宽（A2.1 表 9 行）+ 生成器改写（第 89/90 行）
4. ADR_INDEX.md 同步
5. python3 tools/verify_handoff.py --finalize   ← 重签 10_HANDOFF_MANIFEST.json
6. python3 tools/verify_handoff.py --check      ← 全链校验
```

第 5 步必须在 1–4 之后：manifest 绑定所有文件 sha256，任何改动后必须重算。

**反向变异必须仍被拒**：`verify_handoff.py:106` 的 `"unapproved_adr"` 变异臂语义是
「凭空写不存在的 ADR 名必须被拒」。放宽断言时不得放宽这条，
并需新增一条变异：「引用「仅批准方向」档位的 ADR 被执行授权 task 引用 → 必须被拒」。

#### A2.4 manifest 漂移的结构性成因与修复（builder check P0-Q14）

`verify_handoff.py:132-133` 断言：工作树相对 HEAD 的改动只允许落在
`docs/releases/v4.1.0/refactoring/` 与 `docs/refactor/v4.1.0-c/` 两个前缀内。
这带来两条约束：

1. **commit 边界**：A1 的 cherry-pick 改 `scripts/`、`tests/`；
   A4 的 CI 接线改 `.gitea/`、`scripts/`。因此 `--check` 必须在
   **A1/A4 已 commit（或在独立 worktree 中）** 的状态下运行。
   建议顺序：先 A1（commit）→ A2（docs 内改动，commit）→ A4（commit）→ 最后跑 `--check`。
2. **漂移的结构性成因**：`finalize()` 哈希整个 `docs/refactor/v4.1.0-c/`，
   而该目录包含**活跃写入的 `takeover/`**（本设计文档也在其中）。
   DSH-B 每写一份治理文档，manifest 就漂移一次——这正是「3 文件漂移」的成因之一。

   **修复**：把 `finalize()` 的哈希范围收窄为
   `docs/refactor/v4.1.0-c/handoff/` + `docs/releases/v4.1.0/refactoring/`，
   **排除 `takeover/`**（活跃工作区，非交付物）。
   需同步更新 `--check` 的对应断言与 `SOURCE_BINDINGS.json` 的输入集合定义。

#### A2.5 依赖声明（builder check P1-Q12）

`verify_handoff.py:49` `from jsonschema import Draft202012Validator`。
当前机器 python3（3.15.2）只装了 `PyYAML`，**`jsonschema` 缺失**，
仓库无 `requirements.txt` 钉版本。且脚本用裸 `python3`，而 `ci.yml` 用 `/usr/bin/python3`——两个解释器。

处理：
- 在 `refactoring/tools/` 增加 `requirements.txt` 钉 `jsonschema` 版本
- 统一解释器（CI 内显式 `/usr/bin/python3` + 依赖预装步骤）
- 若属主认为「加依赖」违反 C2，则改为用仓库内既有校验器替代 JSON Schema 校验（§6 O13）

#### A2.6 出口判据

- `python3 tools/verify_handoff.py --check` 全链通过
- 8 个原有反向变异全部被拒 + 新增 1 条「档位越权引用」变异被拒
- `assert len(cases)==75` 成立；HARA manifest 哈希对质一致；Markdown 链接全量校验通过
- 10/11 份 ADR 的 `Status` / `Approvals` 与裁决账本逐条一致
- DAG 18 个 `approved_adr_refs` 满足 A2.2 全部 5 条规则

---

### A3 验收基线 —— 75 场景重绑到新基线

#### A3.1 先修矩阵本身的缺陷（`TEST_COVERAGE_MATRIX.json`，75 条）

| 字段 | 现状 | 目标 |
|---|---|---|
| `invariant_ids` | 75/75 为 `[]` | 逐条填入实际不变量 id（取自 `TRANSACTION_INVARIANTS.md` / `STORAGE_CAPABILITY_CONTRACT.md`） |
| `invariant_mapping_status` | 75/75 `UNREVIEWED` | 逐条改为已复核状态 |
| `required_for_certification` | 75/75 `false` | 由属主标记阶段出口必需项（O2，**不由执行者代填**） |
| `execution_status` | 75/75 `NOT_RUN` | 由复跑填充（词汇见 A3.3） |
| `actual` | 75/75 `null` | 由复跑填充 |
| `case_id` | 重复（`PILOT-T6-001`、`PILOT-T6-002` 各 ×2） | 以 `matrix_index` 唯一化 |

#### A3.2 切片策略与出口判据一致（初稿不自洽，builder check P2-Q17）

初稿切片取 T1/T2/T3（44 条）但出口判据写「75 条全部有终态」——不自洽。

**改为分两级**：

| 级别 | 范围 | 出口判据 | 用途 |
|---|---|---|---|
| L1 风险切片（必经） | T1+T2+T3 = 44 条 | 44 条全部有终态 + 差异表 | 存储与事务语义面，阶段出口证据 |
| L2 全量（可选） | + T4/T5/T6 = 31 条 | 75 条全部有终态 | 完整基线 |

**默认执行 L1**。L2 的价值在 T4/T5/T6 含已知 RED（见 A3.5），
是 RED 身份核验的最好载体，但成本更高、且 bespoke 探针最多。
属主标定的 `required_for_certification` 子集确定后，作为**附加**门槛并入 A5 #4。

#### A3.3 状态词汇三套映射（初稿混用，builder check P0-Q8）

现存三套词汇：

| 来源 | 词汇 |
|---|---|
| `05_TEST_FACTORY_CONTRACT.json` / 矩阵 | `PASS` `FAIL` `KNOWN_FAIL` `UNSUPPORTED` `NOT_RUN` `DIALECT_DIFFERENCE` `INFRA_ERROR`（7 类，互斥） |
| pilot `rerun-summary` / `case-mapping` | `PASS` `FAILED_RED` `COVERED_PASS` `COVERED_RED` |
| 初稿出口判据 | `failed=0, infra_error=0, not_run=0`（runner summary 计数器，非矩阵字段） |

**映射表（必须落盘，作为矩阵 schema 的一部分）：**

| pilot 词汇 | 契约 7 类 | 含义 | 备注 |
|---|---|---|---|
| `PASS` | `PASS` | 真通过 | |
| `COVERED_PASS` | `PASS` | 由已有测试覆盖而通过 | 需标注覆盖来源 |
| `FAILED_RED` | `KNOWN_FAIL` | **故意保持红**（缺陷已复现） | **必须带 RED 五元组身份** |
| `COVERED_RED` | `KNOWN_FAIL` | 由已有覆盖复现的红 | 同上 |
| — | `FAIL` | 真缺陷（新发现，非已知 RED） | |
| — | `NOT_RUN` | 未执行 | 不得进阶段出口计数 |
| — | `INFRA_ERROR` | 基础设施故障 | 同上 |

**红线**：`FAILED_RED` → `KNOWN_FAIL` 的映射**不得丢失 RED 身份**。
`RED-LEDGER.md` 的 17 项 RED 各带五元组（测试 fn + package/target + source sha256 + rc + 归属），
映射后必须保留这五项，否则等于把已知缺陷降级为普通失败。
初稿的 `failed=0` 判据应改写为矩阵 schema 上的字段判定，而非 runner 内部计数器。

**另注**：`TEST_EXECUTION_CONTRACT_DECISION.md` 是一份**仍未裁决**的申请
（方案 A/B/C：RED 契约测试如何执行）。若方案 B 胜出，需在 7 类之外增加
`EXPECTED_RED` 类——该裁决属主专属（O14）。

#### A3.4 Oracle 签名（初稿只写「不得自签」，无人签则不可达，builder check P0-Q7）

矩阵每条的 `activation_requires` = `["ASSERTION_REVIEW","INPUT_HASH_BINDING","REAL_EXECUTION"]`；
契约规定 `oracle_owner: "Owner/Independent Reviewer not implementer"`、`mock_vs_live_required: true`。

**若实施者不得自签、又没有指定签名人，75 条将全部停在 `NOT_RUN`——正是反目标所列。**

**属主裁决（O6）：分层签署 —— 先签 T1 试点，再批量签 T2/T3。**

**属主已明确不在签署环节介入**（仅在最终放行裁决时介入），故两段的签署人统一为
**属主授权的独立 Reviewer**（Codex 或 DSH Reviewer 池），且**不得是该场景的实现者**。

| 段 | 范围 | 签署人 | 前置 |
|---|---|---|---|
| 第 1 段（试点） | T1 = 30 条 | 独立 Reviewer（具名，≠ 实现者） | 断言文本 + 测试源就绪，流程跑通 |
| 第 2 段（批量） | T2 + T3 = 14 条 | 同一独立 Reviewer 或另一名独立 Reviewer | 第 1 段流程验证通过且无 `ASSERTION_UNREVIEWABLE` |

分段的意义：第 1 段用最贵的人工成本换取「签署流程本身可行」的证据；
若流程有缺陷，只在 30 条上暴露，不会在 44 条上全面返工。

签署规则（无论哪一段）：

- **签名对象**：断言文本 + 测试源（**不是**执行日志）
- **签署人资格**：不得是该场景的实现者
- **记录位置**：`TEST_COVERAGE_MATRIX.json` 每条新增
  `assertion_reviewed_by` / `assertion_reviewed_at` / `assertion_source_sha256`
- **A5 #4 的可核方式**：每条 `PASS` 具备非空签署人 + 签署时间 + 绑定的 source sha256，
  且签署人 ≠ 该场景实现者

若第 1 段暴露流程缺陷（如「断言文本无法在不读实现的前提下独立判定」），
则 T1 的 `PASS` 不得计入阶段出口，须先修正断言的可判定性。

#### A3.5 执行层的真实缺口（初稿高估了「复用 test_factory.py」的可行性，builder check P1-Q13）

`tools/test_factory.py` 是 **filter-based**：一次调用 = 一个 package + 一个 target + N 个 `--filter`；
其 `list` 子命令期望的 manifest schema（`cases[].id/suite/status/desc`）与
`TEST_COVERAGE_MATRIX.json`（`case_id`/`feature`/…）**不匹配**，**没有 manifest-driven 模式**。

三类缺口：

| 缺口 | 数量 | 事实 | 需要的动作 |
|---|---|---|---|
| **无矩阵驱动层** | 75 | 需要一个 driver 把 matrix → 调用 → 回写 `execution_status`/`actual`/`artifact_sha256` | 判定是否算「新建 runner」（O15）——本文倾向判定为**既有 runner 的适配层**，因为它不引入新执行语义、不新建队列与账本（C2 允许） |
| **非 cargo 场景** | 3 | `PILOT-T6-001`(kill-9)、`PILOT-T6-002`(checkpoint-torn-write)、`PILOT-T5-101`(odku-rollback-noop) | test_factory 跑不了，需 bespoke 探针（原在 pilot 仓库）。须决定是否迁入基线 worktree 及归属 crate/target |
| **测试源不在本仓库** | ~6 | `tf_t1_*` / `tf_t4_*` 的源在 `/Volumes/workspace/dev/srg-test-factory-pilot/tests/integration/test_factory/`，且 pilot 的 `Cargo.toml` 处于未提交修改态 | 迁入 or 明确排除（排除则这些场景转为 `NOT_RUN` 并说明） |
| **清单快照过期** | ~66 | 引用 `cargo test --list @ e9ff21fc`，即 **242 提交前**的测试清单 | 须在 BASE 上**重新 `--list`**；若某测试已不存在，7 类契约中**没有「测试消失」类别**——需新增或映射（O16） |

#### A3.6 对照基线（两份，不是初稿的一份，builder check P2-Q18）

| 基线 | 内容 | 同 base |
|---|---|---|
| pilot `evidence/rerun-summary-20261120.json` | 75 案例，70 PASS / 5 red_confirmed；逐条 `case-mapping-20261120.json`；原始日志 `RERUN-T1*.json` | `e9ff21fca3` |
| DSH-B `takeover/dsh-b/RED-LEDGER.md` + `RED-IDENTITY-RECONCILIATION.md` | **17 项 RED** 五元组身份；逐条日志 `takeover/dsh-b/artifacts/` | `e9ff21fca3` |

差异表须**同时**对这两份，且须解释：基准的 70/5 与本轮结果的任何偏离
（新增 RED、消失的 RED、状态降级）。

#### A3.7 出口判据

- 已执行切片（L1 = 44 条）全部有终态；未执行部分显式 `NOT_RUN` 并说明原因
- 每个 `PASS` 同时满足：`failed=0 ∧ infra_error=0 ∧ not_run=0`（在矩阵 schema 上判定）、
  有 `artifact_sha256`、有 `base_sha=BASE`、有 Oracle 签署（C7/O6）
- 每个 `KNOWN_FAIL` 保留 RED 五元组，且能在 `RED-LEDGER.md` 中定位
- 与 A3.6 两份基准的逐条差异表已出，偏离均已解释
- 未出现「RED 被塌陷为 FAIL」或「FAIL 被记为 PASS」

---

### A4 门禁最小集 —— 只做「ALPHA 执行点 + ALPHA 7 道」

**不碰** 83 道 dead/orphan 的归档与重挂（需属主单独批准门禁清单变更，O3）。

#### A4.1 ALPHA 阶段清单的执行点（修正初稿，见 §1.3③）

**已具备**（无需再建）：

- `check_stage.sh` 已在 CI 中，在 promotion 时真实执行 BETA/RC/GA 清单
- `check_gate_executability.sh`（#5461/#5450）已在 CI 中运行，挡住「加了没人跑的门禁」
- `run_gates.py` + `gate_report.py` 三态语义正确，C6 优先级「有 INCOMPLETE 则总判 INCOMPLETE」
- `gate_registry.py check`（KR-3）已接线

**缺口（两条，都比初稿窄）：**

| # | 缺口 | 事实 | 动作 |
|---|---|---|---|
| G1 | **ALPHA 阶段清单无执行点** | `ci.yml` 三个 `if:` 分别是 `beta/`、`rc/`、`main\|release/\|hotfix/`；ALPHA carrier 是 `develop/v*`，**不匹配任何一条** | 为 ALPHA 建立执行点。**受 C8 约束**：不能在每次 push 跑全量 ALPHA（已知红分支会永远红），应在 promotion 或周期性触发 |
| G2 | **三态 runner 的执行接线未做（IL-7）** | `ci.yml:1694-1697` 明确记录「runner 本身尚未接线到执行门禁……追踪于 kanban IL-7」 | 决定是否在 A4 内一并接线，或移交 IL-7。**本文倾向：A4 只做 ALPHA 执行点，三态 runner 接线移交 IL-7**，因为 IL-7 涉及「已知红分支」的策略取舍，属策略决定 |

补充：`check_stage.sh` 只区分 exit 0 / 非零（超时、信号、脚本缺失全塌成 FAIL）；
`run_gates.py` 区分 PASS/FAIL/INCOMPLETE。若 ALPHA 执行点采用 `check_stage.sh`，
则 INCOMPLETE 语义丢失——需明确 ALPHA 执行面采用哪一个（O17）。

#### A4.2 ALPHA 7 道负向夹具

`STAGE_CONFIG.yaml:88-121` 的 `stages.ALPHA.required_gates` 实为 10 条 = 7 个 `check_*.sh` + 3 条 cargo。

**部分可复用**：`audit/v4.1.0/neg/` 已有 **14** 个负向夹具（fx01…fx14），
其中 `fx02-anti-ignore-registry-drift`、`fx08-alpha-entry-doc-missing`、`fx14-stage-version-uninferable`
**已瞄准 ALPHA 阶段门禁**。故 A4.2 是「一部分重新指向 + 一部分新造」，不是全量新写。

`swallow_hits` 定位（`gov_probe.SWALLOW_TOKENS = ("|| true","|| :","||  true")`）：
`arch3_no_bypass`=2、`arch_invariants`=2、`sql_corpus_gate`=1，其余 4 道为 0。

**判据**：7 道各有一个坏产物夹具，且在**门禁层与阶段汇总层同时观测到 RED**——
两层须是同一次执行的两级（夹具经该门禁产生输出，再由阶段汇总判红），否则不是同一证据。

#### A4.3 门禁编目修正（§1.3①）

`GATE_CATALOG.jsonl` 的 `neg_fixture` 自动派生规则识别不出真实夹具。
修正方向：与人工 `effect` 声明双向核对，以夹具实物为准；
`gate_registry.py check` 的 C2/C3/C5 判据需同步从「派生值非空」改为「派生值与人工声明一致」。

#### A4.4 出口判据

- ALPHA 阶段清单在 CI 中存在可指认的执行点（记录触发条件）
- 7 道各有一个坏产物夹具，在门禁层与阶段汇总层同次执行中观测到 RED
- `GATE_CATALOG.jsonl` 内部数字自洽（`neg_fixture` 与 `effect` 声明一致）
- 未放宽任何门禁阈值

---

### A5 重构启动判据 —— 属主放行清单

以下**全部**满足才允许启动 C-ALPHA-02：

| # | 判据 | 可核方式 |
|---|---|---|
| 1 | `BASE`（内容基线）唯一，develop tip 是其祖先，含 BASE 的分支已推送 | `git merge-base --is-ancestor origin/develop/v4.1.0 BASE` 成立；`git ls-remote` 可见 `base/v4.1.0-frozen` 且其 SHA 是 BASE 的后代 |
| 2 | 45da6968ae 的 123 个材料文件全在 BASE 树内 | `git ls-tree` 对比 |
| 3 | 前向绑定 `base_sha` 唯一且等于 BASE；历史绑定逐项未变 | 前向/历史两张扫描表 |
| 4 | ADR 机器字段非空、档位正确、映射满足 A2.2 五条规则 | `verify_handoff.py --check` + 变异臂 |
| 5 | manifest 零漂移 | 同上（且 `finalize()` 范围已按 A2.4 收窄） |
| 6 | 已执行切片全部有终态 + Oracle 签署 + 双向差异表 | 矩阵 + 差异表 + `RED-LEDGER.md` 对照 |
| 7 | ALPHA 阶段清单有 CI 执行点 | `ci.yml` 断言 |
| 8 | ALPHA 7 道负向夹具两层 RED | 夹具记录 |
| 9 | 门禁编目内部自洽 | `gate_registry.py check` |
| 10 | 外审包三套统计一致（71/71/71） | **发布 R2 包**（R1 已冻结，见下） |
| 11 | dead 门禁口径唯一化 | R2 包内不得再出现「61 道全部有文档引用」 |
| 12 | ~~`SQLRUSTGO_LOCAL_ONLY` 路由 fail-closed~~ → **本次准入显式豁免** | 放行裁决记录中必须写明豁免及其理由，不得静默跳过（见下） |

**关于 #10/#11（R1 包已冻结，builder check P1-Q10）**：

R1 包位于 `…/DSH-EXTERNAL-REVIEW-001/sqlrustgo-v4.1.0/package/`，有 `FREEZE.json`
（`frozen_at: 2026-11-23T07:20`、逐文件 sha256 含 MANIFEST 自身哈希）、
`MANIFEST.json`（`approval_state: NOT_APPROVED`、`status: DRAFT`、`source_commit: cd8cefe623`）、
以及属主批准文件 `SQLRUSTGO-V410-REFACTOR-HARNESS-R1-20260723-v1.json`。
且 `EVIDENCE-INDEX.json`(71) 与 `01-EXECUTIVE-SUMMARY.md`(64) 在 freeze **内**，
而 `EVIDENCE-COVERAGE.md`(68) 在 freeze **外**。

**因此本设计不修改 R1**，而是**发布 R2 包**（**已裁决，O8**）：
修正计数漂移、修正 dead 口径、重指 `source_commit` 到 BASE、显式声明 MANIFEST/SHA256SUMS 自排除规则。
A5 #10/#11 的可核对象是 R2 包，不是被改写的 R1。
R1 保持原样，作为历史审计快照。

**关于 #12 —— 已裁决（O9）：跨仓库变更不纳入本计划，显式豁免。**

事实记录（供放行裁决引用）：

`SQLRUSTGO_LOCAL_ONLY` 在 sqlrustgo 仓库中**零命中**，它存在于 DSH 仓库
（`deepseek-schedule/docs/test-factory/LOCAL_MODEL_ROUTING_POLICY.md`）。
DSH 路由器的 LOCAL_ONLY 是**策略级**标志：

```python
scheduler/router.py:277  def local_only(...): ... routing["strategies"][name]["local_only"]
scheduler/router.py:290  if local_only and str(rid).startswith("cloud"): continue   # 绝不回落云端
```

**不存在 per-task 的数据分类维度**（`scheduler/` 内 `data_classification` / `DATA_CLASS` 零命中）。
`blockers.py` 定义了 `DATA_BLOCKED` / `SECURITY_BLOCKED`，但无人从分类字段填充它们。
DSH 已有 `docs/test-factory/hold/LOCAL-ONLY-FAIL-CLOSED-TESTS.md`，
且它是 `verify_handoff.py --finalize` 哈希的文件之一。

**豁免内容**：本计划不修改 `deepseek-schedule` 仓库的路由器，不引入 per-task 分类字段。
**豁免后果（必须写入放行裁决记录，不得静默跳过）**：

1. 「缺分类即拒绝」在本次准入范围内**未成立**；
2. DSH 路由器的云端 fallback 拦截仅依赖**策略级标志 + 代码路径**，
   不依赖 per-task 数据分类，因此**单个任务的分类缺失不会导致自动 BLOCKED**；
3. 本次准入期间的任何跨仓库派发，仍须由调度侧显式指定 LOCAL_ONLY 策略；
4. 缺口转为独立待办，移交 DSH 仓库治理，不阻断 C-ALPHA-02。

**放行裁决记录必须包含的字段**：`waiver_id` / `waived_criterion: 12` /
`reason` / `scope` / `compensating_control` / `expires_at` / `approved_by`。

---

## 6. 属主裁决台账

### 6.1 已裁决（阻塞项，2026-11-23）

| # | 事项 | 裁决 | 落点 |
|---|---|---|---|
| O6 | A3 的 Oracle 签名人 | **分层签署**：第 1 段 T1（30 条）由独立 Reviewer 逐条签，流程跑通后第 2 段批量签 T2/T3（14 条）。**属主不介入签署环节**（仅在最终放行裁决时介入）；签署人 ≠ 实现者 | §A3.4 |
| O7 | ADR-410-011 是否并入 | **并入 `adrs/`**（编目 11 份，`verify_handoff.py:122` 改 `==11`）；**但不进 `approved_adr_refs`**（011 条件批准、前置未满足） | §A2 / §A2.1 |
| O8 | R1 包可否修改 | **发 R2，R1 不动**。R1 保持原样作为历史审计快照；R2 修正计数漂移、dead 口径、`source_commit`、自排除规则 | §A5 #10/#11 |
| O9 | A5 #12 跨仓库范围 | **不纳入，显式豁免**。放行裁决记录须写 `waiver_id` / `waived_criterion:12` / `reason` / `scope` / `compensating_control` / `expires_at` / `approved_by`；不得静默跳过 | §A5 #12 |

### 6.2 仍待裁决（不阻塞对应步骤，实施到该步时逐项提请）

| # | 事项 | 为什么我不能代决 | 阻塞哪一步 |
|---|---|---|---|
| O1 | 档位「仅批准方向/实施另审」（003/004/007/008/009/010）是否足够支撑 `approved_adr_refs` 填充 | ADR 批准为属主专属（C1）；且 A2.2 规则 3 会因此禁止执行授权 task 引用它们 | A2.2 映射落地 |
| O2 | 75 场景中哪些标记 `required_for_certification=true` | 属主是阶段出口口径的定义者；执行者代填等于自签 | A3.1 / A5 #6 |
| O3 | 是否授权 83 道 dead/orphan 门禁的批量处置（不在 A4 内） | 门禁清单变更需属主批准 | 不在 A4 内 |
| O4 | 「GPT-6 独立评审报告」指哪一份文件 | 仓库中有多份 Codex 产物，身份需属主指认 | A5 放行记录 |
| O5 | A5 通过后放行哪一批核心重构包 | 需同时确定 DAG 中新增哪些核心编码包 | A5 之后 |
| O10 | ADR→task 映射规则是否采纳 A2.2 的 5 条，或另定 | 该规则是 ADR 门禁的实质，映射由属主确认而非执行者自拟 | A2.2 |
| O11 | BASE 是否直接推进 `develop/v4.1.0`，还是只落在 `base/v4.1.0-frozen` | 涉及权威分支推进节奏 | A1.3 落点 |
| O12 | `14_HANDOFF_MANIFEST.json` 的 `authority.upstream_c01_manifest_resign:false` 如何处置 | 两份 manifest 的权威关系需裁决 | A2.1 第 6 项 |
| O13 | 是否允许为 `verify_handoff.py` 增加 `jsonschema` 依赖，或改用仓库内既有校验器 | 依赖引入是否合规（C2 的适用边界） | A2.5 |
| O14 | `TEST_EXECUTION_CONTRACT_DECISION.md` 的 RED 执行方案（A/B/C） | 该申请仍未裁决；方案 B 需新增 `EXPECTED_RED` 类 | A3.3 |
| O15 | 矩阵驱动适配层是否算「新建 runner」 | C2 的边界需属主划定（本文倾向：算适配层，不算 runner） | A3.5 |
| O16 | 「测试已消失」的分类（7 类契约无此类） | 新增状态类别需属主批准 | A3.5 |
| O17 | ALPHA 执行面用 `check_stage.sh`（两态）还是 `run_gates.py`（三态） | 涉及 INCOMPLETE 语义与 IL-7 边界 | A4.1 |

---

## 7. 留给实施者的部分

- A1.2 冲突的具体三方合并内容（`check_g1_tpch_baseline.sh` 177–189 行）
- A3.1 的 75 条 invariant 映射与断言编写
- A3.5 的矩阵驱动适配层实现细节（矩阵 → 调用 → 回写）
- A4.2 各门禁坏产物的具体构造方式
- 所有实施须在隔离 worktree（C4），不直接在权威工作区改
- A1/A4 的改动须先 commit，再跑 `verify_handoff.py --check`（A2.4 的 commit 边界约束）

---

## 附：证据索引

本文所有事实均可在下列位置复核：

| 事实 | 位置 |
|---|---|
| Codex R1 全部报告 | `/Volumes/workspace/dev/workspaces/codex-review/sqlrustgo-v4.1.0/reports/round-01/` |
| 15 条 findings | `.../findings/ROUND-01-FINDINGS.json` |
| 71 条证据索引 | `.../evidence/ROUND-01-EVIDENCE-INDEX.json` |
| 10 份 ADR | `.worktrees/codex-calpha01-20261120/docs/releases/v4.1.0/refactoring/adrs/` |
| ADR-410-011 | `.worktrees/codex-runtime-5915-20261120/docs/releases/v4.1.0/refactoring/runtime-contract/20261120-pr5915/` |
| 18 包 DAG | `.worktrees/codex-calpha01-20261120/docs/releases/v4.1.0/refactoring/REFACTOR_TASK_DAG.json` |
| 75 场景矩阵 | 同上 `TEST_COVERAGE_MATRIX.json` |
| 门禁编目 168 条 | `docs/releases/v4.1.0/GATE_CATALOG.jsonl` |
| 阶段清单 SSOT | `docs/governance/STAGE_CONFIG.yaml:88-121` |
| stage runner | `scripts/gate/check_stage.sh` / `scripts/gate/run_gates.py` / `scripts/gate/gate_report.py` |
| CI 接线现状 | `.gitea/workflows/ci.yml:1319-1371`（BETA/RC/GA）、`:1660-1712`（IL-7 与 executability） |
| 门禁可执行性检查 | `scripts/gate/check_gate_executability.sh` |
| manifest 重签工具 | `.worktrees/codex-calpha01-20261120/docs/releases/v4.1.0/refactoring/tools/verify_handoff.py` |
| ADR 裁决账本 | `docs/refactor/v4.1.0-c/takeover/dsh-b/ADR-DECISION-RECORD.md:35-47` |
| 75 场景复跑证据 | `/Volumes/workspace/dev/srg-test-factory-pilot/evidence/rerun-summary-20261120.json` |
| RED 身份台账 | `docs/refactor/v4.1.0-c/takeover/dsh-b/RED-LEDGER.md` + `RED-IDENTITY-RECONCILIATION.md` |
| 测试工厂契约 | `.worktrees/codex-calpha01-20261120/docs/refactor/v4.1.0-c/handoff/05_TEST_FACTORY_CONTRACT.json` |
| DSH LOCAL_ONLY 路由 | `deepseek-schedule/deepseek-schedule/scheduler/router.py:277,290` |
| R1 包冻结状态 | `…/DSH-EXTERNAL-REVIEW-001/sqlrustgo-v4.1.0/package/FREEZE.json` |
