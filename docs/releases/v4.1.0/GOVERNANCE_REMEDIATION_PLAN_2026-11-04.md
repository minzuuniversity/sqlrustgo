# v4.1.0 治理整改计划与设施方案（融合 BidSystem 与 BustubX-EDU）

| 字段 | 值 |
|---|---|
| 状态 | **提案**（未提交、未推送；入库需按 `docs/governance/DOC_CHECK_CORRECTION_RULES.md` 七步流程 + owner 批准） |
| 日期 | 2026-11-04 |
| 审计基线 | `develop/v4.1.0` = `b5039acb1c`（与 252 Gitea `refs/heads/develop/v4.1.0` 一致：0 领先 / 0 落后） |
| 融合来源 | BidSystem（v2.3.0 线，11 道门禁 + 台账三件套）、BustubX-EDU（v1.2.0 线，43 道门禁 + IAIA/SSOT 覆盖闭合） |
| 复现 | 本文所有数字由同一次实测注入，命令见附录 C |

---

## 0. 结论先行

sqlrustgo 不缺规则，缺的是**能把规则变成红色的机器**。

本仓库已有 `87` 个 governance 文档、`168` 个 `check_*` 门禁脚本、14 条「严格禁止」、AFP 三类型、五状态实现真相（Implemented / Integrated / Tested / Verified / Release-ready）、IAIA-410 审计 SSOT。同一时刻实测到的却是：

| 缺口 | 实测值 |
|---|---|
| 阶段 `exit_criteria` 有机器映射的 | `21` 条标准里 **0** 条（`criteria_gates` 与 `manual_only_criteria` 均为 0） |
| 不在任何清单、且 CI 不可达的门禁 | **83** 个（其中全仓无人引用 **62** 个） |
| 有 `INCOMPLETE` 语义的门禁脚本 | **0** 个（`SKIP` 语义 25 个） |
| `results.jsonl` 式聚合器（缺项/重复即判不通过） | **0** 个 |
| 校验 `claims/` 声明规范的脚本 | **0** 个（`claims/` 有 `CLAIM_PROVENANCE_SPEC.md` + `INDEX.yaml`，11 个文件） |
| 已实现却**无清单、无 CI 执行点**的「解药」门禁 | **5 道**：`check_orphan_sources.sh`（已登记 18 个未参与编译的 `.rs`）、`check_ssot_duplicate.py`、`check_a04_evidence_reproducible.sh`、`check_test_count_monotonic.sh`、`check_test_inventory.sh` |
| 红色门禁能否拦住合并 | **不能**：`develop/v4.1.0` 与 `main` 的 `enable_status_check=false`、`status_check_contexts=null`、`required_approvals=0` |

「报告都好看、实际一测一用就一堆问题」不是态度问题，是这四层缺口叠加的必然结果：

> **清单不可核对** ⇒ 任何「N/N 全通过」无意义 →
> **判据不可证伪** ⇒ 「绿」分不清「没问题」还是「门禁坏了」→
> **三态不诚实** ⇒ SKIP/没跑 也能记成通过 →
> **红了也不拦** ⇒ 全套门禁退化为建议。

因此本方案**先立设施（§4），再按 P0→P3 整改（§5）**，并且刻意接在仓库已有的正确件上（§2 末），不另起一套。

---

## 1. 实测基线（2026-11-04）

### 1.1 门禁面

| 度量 | 值 | 说明 |
|---|---|---|
| `scripts/gate/` 文件总数 | `209` | 其中 `check_*` = `168` |
| `STAGE_CONFIG.yaml` 的 `gate_definitions` | `16` 条 | G1…G16 |
| `stages.<S>.required_gates` | DRAFT 2 / ALPHA 10 / BETA 19 / RC 26 / GA `35` | 合计 92 项（含 cargo 命令），去重后脚本 `34` 个 |
| `.gitea/workflows` 直接引用的门禁脚本 | `41` | 传递可达（门禁调门禁）= `100` |
| 孤儿门禁（不在清单 & CI 不可达） | **`83`** | 附录 B |
| 其中全仓无任何引用（死门禁） | **`62`** | 附录 A（2026-11-05 修探针引用口径后刷新，见 §1.4） |
| 只在 `.github/workflows` 出现的门禁 | **`4`** | 附录 B-2 |
| 含负向夹具（selftest / 反例 / 反向变异）的门禁 | **`23` / `168`** | 即 `145` 道无法证明「能变红」 |
| 含 `\|\| true` / `\|\| :` 吞错 | `56` 个（其中 ≥3 处 `24` 个） | — |
| 静态上没有任何非零退出路径 | `4` 个 | 「永不失败」 |
| 含 `INCOMPLETE` 语义 | `0` | 三态缺一态 |
| `results.jsonl` 聚合契约 | `0` | 缺项/重复/退出码丢失无人核对 |

### 1.2 声明与证据面

| 度量 | 值 |
|---|---|
| `docs/releases/v4.1.0/*.md` | `71` 份 |
| 其中 `PASS` / `全绿` / `✅` 命中 | **`1132`** 处 |
| `docs/releases/v4.1.0/evidence/` 文件 | `206` |
| `audit/v4.1.0/` 的 run 目录 / `manifest.yaml` / finding yaml | `8` / `7` / 8 |
| `docs/governance/lessons/` | `5` 篇 |
| 顶层 `.md` | `45` 个 |
| `.github/workflows` / `.gitea/workflows` | `13` 个 yml / `8` 个 yml（另含 `ci-test.md` 冒充 workflow） |

### 1.3 合并拦截（Gitea API 只读实测，未写入任何配置）

| 分支 | protected | enable_status_check | status_check_contexts | required_approvals |
|---|---|---|---|---|
| `develop/v4.1.0` | true | **false** | **null** | **0** |
| `main` | true | **false** | **null** | **0** |
| `develop/v4.0.0`（对照） | true | false | null | 1 |

### 1.4 2026-11-05 刷新（修探针引用口径后）

| 度量 | 本文旧值 | 归档探针 `20261104-govprobe-001` | 本树重跑（run `20261105-triage-001`） |
|---|---|---|---|
| 全仓无人引用的门禁 | 37 | 36 | **62** |
| `docs/releases/v4.1.0/*.md` | 68 | 70 | **71** |
| `PASS`/`全绿`/`✅` 命中 | 1098 | 1123 | **1132** |

三件事要分开说：

1. 旧值（37 / 68 / 1098）**指不回任何已归档 run**，按 §2.4 属于"文档声称"，已按归档 run 更正。
2. 文档数与声明命中会随新增文档移动，所以每次刷新都必须钉一个 run；本行的数由
   `20261105-triage-001` 的 `gov-metrics` 步骤固定。
3. 死门禁从 36 涨到 62 **不是新增债务**，是探针把散文提及当成调用（见 R7）。

---

## 2. 七条机制性根因

### R1 分母失控：四份「门禁清单」互不核对

`gate_definitions`（16 条，人写的编号 G1…G16）、`stages.<S>.required_gates`（GA `35` 项）、`scripts/gate/` 里的 `168` 个脚本、`.gitea/workflows` 的 `41` 个直接调用点 —— 四者之间没有任何元门禁核对一致性。

两个成熟仓库的原始事故记录就是这一条：BustubX-EDU `tools/gates.py` 文件头记的是 2026-10-10～10-11 连续三轮漏报第 9 道门禁，根因是「同一个 `ci_gate.sh` 里同时存在 7、8、9 三种分母，收尾还写了一句自然语言的 `✅ 门禁通过（7/7 + C7 回归 + 发布文档一致性 + 锁序）`」，于是任何「数一下 `[N/7]` 出现几次」的动作必然出错。BidSystem 的对策是**分母恒定 11 且 Agent 不得改清单**（`gate_count_c11` 核对）。

sqlrustgo 现在的处境更糟：连一份「唯一清单」都不存在，`83` 个脚本处于清单之外。

### R2 判据不可证伪：`145` 道门禁没有反例

门禁红了，第一问必须是「它真的抓到了问题，还是它自己坏了」。判据是**把问题条件去掉，门禁必须变绿**。做不到，这道门禁就可能永远绿着，也可能永远红着 —— 两者都携带零信息。

最沉重的一页来自 BustubX-EDU 的 `tools/audit_mainpath.py`（一个**用来防止自欺**的审计脚本）：

- 打分语句完全不读 probe 的匹配结果 —— 把源码目录指向不存在的路径，16 个环节零证据，分数照打 75.625，输出「76% 已达标」；
- 跑测试的正则只匹配 `test result: ok.`，`test result: FAILED.` 被静默跳过，且从不检查 `subprocess` 返回码，`main()` 无条件 `return 0` —— **脚本彻底失效时它输出全绿并返回成功**；
- 分母里混入「永久不做」项并按 0 分计入平均，把决策伪装成欠债，理论满分只剩 87.5%。

这条对 sqlrustgo 有直接警示：**照搬工具而不补夹具与反向变异，等于把病一起搬过来。**

sqlrustgo 侧的对应事实：`168` 道门禁里只有 `23` 道带 selftest/反例；`56` 道用 `|| true` 吞错；`4` 道静态上根本没有失败路径（`check_architecture_freeze.sh`、`check_coverage_single_implementation.sh`、`check_r2_invariants.sh` 等）。

### R3 标准↔门禁 未闭合：`21` 条阶段标准，0 条落到机器上

`STAGE_CONFIG.yaml` 各阶段有 `exit_criteria`（DRAFT 3 / ALPHA 4 / BETA 5 / RC 4 / GA 5），但 `criteria_gates` 与 `manual_only_criteria` 全是 0 —— 也就是**没有任何一条标准被登记为「由哪道门禁覆盖」或「只靠人读、原因是什么」**。

同一形状在 BustubX-EDU v1.1.0 已造成三起真实假达成（`tools/check_ssot_coverage.py` 的立档理由）：

- RC 的「TPC-H 全部 21 个查询 PASS」长期挂着，实际引擎 **0/21** 条可执行；
- RC 的「SOAK 24 小时 PASS」挂着，24 小时长跑**从未执行**（门禁只做「脚本齐备」就绪性检查）；
- GA 的「性能回归 <10% PASS」—— **判据根本不存在**（脚本从不读基线、从不算百分比）。

sqlrustgo 的 G7 至今同形：`gate_definitions.G7` 的名字写着 `24h Soak (compressed-time, real 24h pending)`，而 `check_p13_soak_test.sh` 的 11 项判据里**没有一项执行过 soak**：文件存在、`cargo check --test soak_test`、≥10 个单测、三个 driver「是合法 Python / 可 import」。同理，AGENTS §1-4 的五状态里 `Verified` 只被 `check_agent_harness.sh` 核对「`IMPLEMENTATION_TRUTH.md` 这个文件存在」，没有任何机器核对**某个模块真的处于哪个状态**。

### R4 红了也拦不住：状态检查关闭、审批为 0

§1.3 的实测意味着：任何 PR 只要没有 CI 之外的约束就能合进 `develop/v4.1.0`。`41` 个 CI 调用点、92 项清单、`168` 道脚本，对合并动作**没有强制力**。BustubX-EDU 的 gov-009 教训正是「IAIA 的第三维：门禁红了到底能不能拦住合并」，其 `check_merge_blocking.py` 在拿不到 API 时**明确记 SKIP**（HTTP 401），而不是当作通过。

### R5 幽灵执行点：`13` 个 GitHub workflow 永不运行

4 道门禁（附录 B-2）**只**在 `.github/workflows` 里出现，而项目 AGENTS §8-11 明令禁止推 GitHub 与使用 `gh` CLI ⇒ 这 `13` 个 workflow 是死配置。`.gitea/workflows/ci-test.md` 是 markdown 冒充 workflow，`soak_168h.yml` / `soak_probe.yml` / `z6g4-168h-soak.yml` 三个同义 soak 配置并存。任何引用它们的「CI 已覆盖」都是空承诺。

### R6 声明与证据不绑定：`1132` 处 PASS 声明，0 个总闸

`71` 份 v4.1.0 文档里有 `1132` 处 `PASS`/`全绿`/`✅`。`claims/` 下有 `G-04_CLAIM_PROVENANCE_SPEC.md` 与 `INDEX.yaml`（11 个文件），但**没有任何脚本引用 `claims/`**；`results.jsonl` 式聚合契约 `0` 个。仓库已经有正确的零件（`check_evidence_binding.sh`、`audit/v4.1.0/*/manifest.yaml`、trusted-baseline 的 EvidenceManifest），缺的是「每条声明必须指向一个 run」的总闸。

### R7 度量工具自身不可证伪：引用识别把散文当调用

`scripts/gate/gate_executability.py` 的 `gate_calls()`（生产门禁用来算"门禁调门禁"可达性的函数）
扫描的是**未去注释的脚本正文**。于是「门禁 A 的注释里写了一句参见 B」= 「A 调用了 B」，
B 就从死门禁名单上被抹掉。本探针自己踩过：`gov_probe.py` 的 usage 文本里出现
`scripts/gate/check_audit_manifest.sh`，那道门禁就被判活了。

实测影响：死门禁 36 → 62（26 道只是被注释提过）。已修探针侧
（`gov_probe.strip_prose()` + `test_gov_probe.py` 的 P1/P2 夹具；反向变异 E=strip_prose 直通、
F=只去 docstring 均 rc=1）。生产侧**未修**：`gate_executability.py` 是清单内门禁的判据来源，
改它等于改门禁判据，按 AGENTS §8-7 需 owner 授权 ⇒ 已登记 Gitea ISSUE 与看板 `MX-1`，
并与 P0-1 的 `gate_registry.jsonl` 合并解决：注册表成立后，调用关系是**显式声明**的，
散文扫描不再是判据来源，这个洞就没有承载体。

### 必须同时记住的正向件（整改要接在这里）

| 已有件 | 价值 |
|---|---|
| `scripts/gate/gate_executability.py` + `check_gate_executability.sh`（#5450/#5461） | 已把「清单有没有人执行」变成 fail-closed 判据，实测当前 **92 项全 OK**；自带 4 组夹具自测。**这就是 registry 视图的现成地基** |
| `scripts/gate/check_correctness_tests_production_storage.sh`（#5199/#5400，IAIA-410 F6 案例 C） | 已经把「测试跑在非生产存储 ⇒ 绿也不是生产配置」钉成**按分支判定装配链**，并在注释里自证旧版 grep 拼写导致恒红=零信息 |
| `scripts/gate/check_orphan_sources.sh`（#4939） | **fail-closed 台账的正确形状**：`KNOWN_ORPHANS` 登记 18 个游离在构建图外的 `.rs`，注释明写「从名单里删条目不会让门禁变松，只有改代码才行」——孤儿回收台账就该是这个方向。**但它自己既不在清单也不在 CI 可达** |
| `tests/integration/gate/` 的 `ci_wiring_honesty_test.rs` / `gate_criterion_existence_test.rs` / `gate_dimension_honesty_test.rs` / `alpha_gate_blocker_test.rs` | 门禁诚实性已有 Rust 端判据，honesty 设施应**驱动**它们而不是重写 |
| #5581 覆盖率阈值/测量单一实现 + 元门禁、#5577/#5389 测试不得覆写已归档证据、#5466 证据可复现性、#5525 备份→恢复→验证闭环带三条假成功反例、trusted-baseline 三态判决 | 方向与 BidSystem/BustubX 一致，属于同一治理 DNA |
| `docs/releases/v4.1.0/IAIA_410_SSOT.md` 的 run/finding 状态机（`not_started`…`regression_verified`；`suspected`…`refuted`） | 比 BidSystem 与 BustubX 都更细，应当作为「声明↔证据」绑定的承载层 |

---

## 3. 三仓融合：移植 / 改造 / 不移植

| 机制 | BidSystem | BustubX-EDU | sqlrustgo 现状 | 结论 |
|---|---|---|---|---|
| 唯一门禁登记表（Gate 对象：id/name/command/skip_env） | `gates/gates.py`（分母恒 11） | `tools/gates.py`（43 道，id 一经发布不改名） | 无（4 份清单并存） | **移植**，并与既有 executability 矩阵合并成一个视图 |
| 结果聚合契约（`results.jsonl`；缺项/重复/退出码丢失 ⇒ INCOMPLETE） | `run_gates.py` + `gate_report.py` | 同 | 无 | **移植** |
| 分母元门禁 | `check_gate_count.py` + `gate_count_c11` | `check_gate_count.py` | 无 | **移植**（改为「清单↔registry↔脚本目录」三方核对，不锁死数字） |
| 门禁诚实性（声明 effect ↔ 实际行为；夹具回归） | `tools/gate_honesty.py` + `gates/tests/` 夹具 | `GATE_HONESTY_SPEC.md` + `check_gate_honesty_c14.sh` + `run_gate_meta_regressions.sh` | Rust 端 4 个 honesty 测试；`23`/`168` 有夹具 | **移植 + 对接**既有 Rust 测试 |
| 标准↔门禁 闭合（R1–R6 + `manual_only_criteria` 显式登记） | stage_consistency 部分 | `check_ssot_coverage.py`（六条规则，牙齿是 R5） | `criteria_gates`=0 | **移植**（对 `STAGE_CONFIG.yaml` 扩字段） |
| 合并拦截校验 | BRANCH-SYNC 文档 | `check_merge_blocking.py`（401 ⇒ 明确 SKIP） | 无，且实测保护未开 | **移植 + 先打开保护** |
| 能力声明核查（防「报告好看」） | `anti_fraud_audit.py` / `afp_adjudicate.py` / `rules/anti_fabrication.yaml` | `check_capability_claims.py`、`check_dist_freshness.py`、`check_orphan_tests.py`、`audit_mainpath.py`（含自白） | `check_evidence_binding.sh` 局部 | **移植 + 改造**（见 P1-2，用「使用面冒烟」而非静态 grep） |
| 证据台账 / 清单 | `evidence/.../self-findings.jsonl`（`self_find.py list/add/close`）、债务台账 `docs/governance/debt/INDEX.md` | `gen_evidence_inventory.py` + `EVIDENCE_INVENTORY.json` | `audit/*/manifest.yaml`、EvidenceManifest | **移植台账机制**，承载层用 IAIA-410 SSOT |
| 反向变异 / 变异测试 | AGENTS §2.5 纪律 | `cargo-mutants.toml`、`_coverage_reverse_mutation.sh`、`develop/probe-*-mutation` 分支 | 个别门禁自带夹具 | **移植为纪律 + 一个质量维度**（不进阶段清单数量） |
| 三态 PASS/FAIL/INCOMPLETE | AGENTS §2.3（硬规则） | gate_report 三态 | 仅 SKIP，`INCOMPLETE` 0 处 | **移植**（ADR-008 已有语义，补机器） |
| 文档盖章 / 新鲜度 / 悬空路径 | `doc_stamp.py`、`doc_freshness.py`、`doc_path_claim_check.py`（实测悬空 22→0）、`gen_doc_index.py` | `check_docs_links.py`、`check_release_docs.py` | `check_docs_links*.sh` | **移植**（注意「写盘口径==核对口径」，见 P2） |
| 知识锚 / lessons / patterns 闭环 | `KNOWLEDGE-ANCHOR.yaml` `next[]` | `KNOWLEDGE_LOOP.md` + `patterns/` + `playbook/` | lessons 仅 `5` 篇 | **轻移植** |
| 投标红线三门禁 | `bid_redline` / `bid_traceability` / `bid_consistency` | — | 无关 | **不移植**（无投标场景） |
| AFP 双席跨厂商签署 | `afp_adjudicate.py`（ChatGPT + DeepSeek 双通道） | `rules/ai_signoff_policy.yaml` | ADR-014 multi-AI（自我签发禁止） | **不移植双厂商硬要求**；只移植「AI 不得自我签发验收」的机器化 |
| 三实例 Gitea 同步（252/250/willy2） | `sync_check.py --no-net` | 多远端 | 5 远端 + 自有 sync 工具（另有会话未提交 WIP） | **不移植**，只沿用其「漂移检测」思路 |
| 分母恒 11 这个**数字** | — | — | — | **继承纪律，不继承数值** |

---

## 4. 设施方案

设施先于整改：没有能变红的机器，P0–P3 的任务写出来就只是文档。

### 4.1 目录与文件（新增，均不改既有 `scripts/gate/` 判据）

```
gate/
  registry.py           # 唯一清单：Gate(id,name,command,skip_env,owner,stage_min,effect,fixtures)
                        # id 一经发布不得改名（改名=让既往 results.jsonl/证据失效）
  run_gates.py          # 按 registry 执行 → results.jsonl（每 gate 一行）
  gate_report.py        # 分母取自 registry；缺项/重复/退出码丢失 ⇒ 整体 INCOMPLETE
  check_registry.py     # 元门禁：registry ↔ STAGE_CONFIG.required_gates ↔ scripts/gate/ 三方核对
  honesty.py            # gate_id → 夹具 + 反向变异记录；驱动既有 Rust gate_*_honesty_test
  ssot_coverage.py      # exit_criteria ↔ criteria_gates 闭合（R1–R6）
  merge_blocking.py     # Gitea 保护配置符合 SSOT；API 不可用 ⇒ 明确 SKIP
  capability_claims.py  # 使用面冒烟（真 CLI + 真客户端 + 真持久化 + 崩溃重启读回）
  evidence_inventory.py # claim → run_id → manifest.yaml → log hash
  gate_ledger/
    gate_catalog.jsonl  # 每个 check_* 三选一：LISTED / RETIRED(reason) / ARCHIVED(reason)
    fixtures.jsonl      # gate_id → 夹具路径 + 反向变异运行号
  tests/                # 设施自身的夹具（见 4.6 纪律）
audit/v4.1.0/runs/<UTC日>-<kind>-<seq>/   # 复用仓内既有证据 SSOT，不自造布局
  results.jsonl  logs/  EVIDENCE_INVENTORY.json  manifest.yaml
```

`scripts/gate/gate_executability.py` 保留为 **registry 的一个视图**（清单→执行点），不再独立充当「清单」。

### 4.2 数据契约

- **`results.jsonl`**：`{gate_id, exit, verdict, log, started_at, finished_at, agent, commit, runner}`。一行都不能少、一行都不能重。
- **判决规则（写死在 `gate_report.py`）**：
  - `verdict ∈ {PASS, FAIL, INCOMPLETE}`；
  - registry 有、results 没有 ⇒ **INCOMPLETE**（不是 FAIL，更不是 PASS）；
  - results 有、registry 没有 ⇒ **INCOMPLETE**（幽灵结果）；
  - 退出码丢失/超时 ⇒ **INCOMPLETE**；
  - 只有全 PASS 才整体 PASS。**SKIP 必须落 ledger 并写原因**（对齐 AGENTS §8-5 与 ADR-008）。
- **`gate_catalog.jsonl`**：`168` 个现存脚本必须逐个有归宿，未登记即红 —— 这是「孤儿回收」的机器承载，避免一次性清理后再长回来。
  台账必须 **fail-closed**：**删掉一条登记不能把红变成绿**（`check_orphan_sources.sh` 已是此形状，直接沿用）。反向变异判据：从 catalog 里删一行 ⇒ 元门禁必须立刻红。
- **`fixtures.jsonl`**：每道门禁一行；无夹具 = 显式登记为 `UNFALSIFIABLE(reason)` 并进债务台账，**不允许沉默**。

### 4.3 STAGE_CONFIG 扩展（向后兼容）

```yaml
stages:
  ALPHA:
    exit_criteria: [...]
    criteria_gates:          # 键与 exit_criteria 逐条同序（R2）
      - [gate_id, ...]       # 或 "manual: 哪一部分没被机器覆盖 + 原因"（R3）
    manual_only_criteria: [] # R5 的逃生口，会被打印在门禁输出里（R6 防登记不存在项）
    functional_gates: [...]  # R4：当前阶段及之后不得为空
```

上线策略：第一个迭代对缺字段只 WARN 并打印缺失清单，第二个迭代 FAIL —— 避免与主干每天数十次提交正面冲突。

### 4.4 合并拦截（与 CI 一次做对）

1. 新增 CI job `gate-aggregate`：跑 `run_gates.py` + `gate_report.py`，产出 Gitea commit status（context 名写进 `STAGE_CONFIG.yaml`，不散落在 workflow 里）；
2. `develop/v4.1.0`（先）与 `main`（后）打开 `enable_status_check` + `status_check_contexts=[gate-aggregate]` + `required_approvals ≥ 1`；**需 owner 授权**；
3. `merge_blocking.py` 把「保护配置与 SSOT 一致」变门禁；拿不到 API ⇒ 明确 SKIP 并说明，绝不静默通过。

分批建议：保护一开，从未执行过的 GA 清单会立刻变红。按阶段清单规模推进 —— 先 ALPHA（10 项），再 RC，最后 GA（`35` 项）。

### 4.5 与既有件的接口（避免重复造）

| 既有件 | 关系 |
|---|---|
| `gate_executability.py` | registry 视图；`check_gate_executability.sh` 改为读 registry |
| `tests/integration/gate/gate_*_honesty_test.rs` | `honesty.py` 调用它们；负向夹具由 shell/py 夹具提供 |
| `check_evidence_binding.sh` | 上游：它产出的绑定关系喂给 `evidence_inventory.py` |
| trusted-baseline 的 `EvidenceManifest` | 作为 `results.jsonl` 的 run 承载（同一 run_id 语义） |
| `debt-registry.yaml` | 债务台账承载；**先定型**（当前 11 个键但 `debts` 条目 0，结构已漂） |
| `docs/agent-harness/IMPLEMENTATION_TRUTH.md` 五状态 | 新增 `module_state.jsonl`（模块 → 状态 + 支撑 run_id），把「Verified」从口号变成必须有 run 的字段 |

### 4.6 夹具纪律（两条，缺一即视为该判据未成立）

1. **夹具不得触碰真实仓库**：只在临时目录构造假仓（BidSystem lesson 18 的真实教训：夹具往真仓里写文件，把仓库污染成证据）。
2. **反向变异必做**：新增/修改判据时，PR 里必须附「把问题条件去掉后门禁变绿」的运行记录（AGENTS §2.5 同义）。做不到 ⇒ 该门禁登记为 `UNFALSIFIABLE` 并进台账。

---

## 5. 整改任务队列

> 每项都给：做什么 / 验收判据（含反向变异）/ 风险。工作量按 4.1.0 主干当前节奏（近 14 天 135 次提交）估计。

### P0 —— 让「能否变红 / 能否拦住」先成立（≈1 周，4 个 PR）

| # | 做什么 | 验收判据 |
|---|---|---|
| P0-1 | `gate/registry.py` + `gate_catalog.jsonl` + `check_registry.py`；把 `168` 个 `check_*` 逐个登记（进清单 / 退役带理由 / 归档带理由） | 元门禁红=清单与目录不吻合；**反向变异：从 registry 删一条登记 ⇒ 必须红**；孤儿数 `83` → 0 |
| P0-2 | `run_gates.py` + `gate_report.py` + `results.jsonl` + 三态 | 三个夹具必须判 INCOMPLETE：缺一项、重复一项、退出码丢失；全 PASS 才绿 |
| P0-3 | Gitea 保护打开（owner 授权）+ `merge_blocking.py` | 把 `enable_status_check` 关掉 ⇒ 该门禁必须红；API 不通 ⇒ 输出必须是 SKIP 而非 PASS |
| P0-4 | 死门禁回收 `62` 个（附录 A） | `gate_catalog.jsonl` 里逐个有 reason；`62` → 0；**不与 P0-1 混在同一 PR** |
| P0-5 | 幽灵执行点处置：`13` 个 GitHub workflow + `ci-test.md` + 三个 168h-soak 配置归一 | executability 的扫描根集合在 registry 里显式声明；`.github` 若保留必须标注「不执行、仅存档」 |

### P1 —— 声明与测量绑定（直击「报告好看」，≈2–3 周，5 个 PR）

| # | 做什么 | 验收判据 |
|---|---|---|
| P1-1 | `ssot_coverage.py`：`21` 条 `exit_criteria` 全部映射到门禁或 `manual_only_criteria` | R1–R6 全过；**反向变异：故意漏掉一条映射 ⇒ 必须红**；R5 只对「当前阶段及之后」发牙齿 |
| P1-2 | `capability_claims.py` = **使用面冒烟**：真 CLI + 真 MySQL 客户端 + 真持久化落库 + 崩溃后重启读回 + 备份→恢复→逐值比对（复用 #5525 三条假成功反例）；并把 `check_correctness_tests_production_storage.sh` 的「按分支判定装配链」扩到**所有**声称验证存储语义的测试（AGENTS §1-5：`BufferPool/Page/BPlusTree` 是 Test-only，生产实现在 `crates/storage/src/file_storage.rs`） | 把生产存储默认换成 `MemoryStorage` ⇒ 必须红（该门禁注释已自证此判据可用）；只 Mock 的测试不得支撑任何「已验证」声明（AGENTS §8-8） |
| P1-3 | 负向夹具扩面：先覆盖 GA/RC/BETA 清单内的 `34` 道 + 名字含 SOAK/PERF/COMPAT/TPCH 的全部 | `fixtures.jsonl` 对清单内门禁 100% 覆盖；未覆盖者必须显式 `UNFALSIFIABLE(reason)` 并进台账，不允许沉默 |
| P1-4 | 每道门禁写 `effect`（能抓什么/抓不到什么），`honesty.py` 核对声明与夹具一致 | 声明与夹具矛盾（声称能抓 X 但没有 X 的夹具）⇒ 红；对接 Rust 端 4 个 honesty 测试 |
| P1-5 | `evidence_inventory.py` + 文档声明分级：`1132` 处声明按「必须绑定 run」/「允许叙述」两级；新增/修改声明必须带 `source_agent + source_run + evidence_hash`（AGENTS §9 已有要求，给机器） | 未绑定的新声明 ⇒ 红；`claims/` 与 `INDEX.yaml` 从 0 核查器变成有核查器 |

### P2 —— 文档事实面与仓库卫生（≈1–2 周）

| # | 做什么 | 要点 |
|---|---|---|
| P2-1 | 移植 `doc_stamp` / `doc_freshness` / `doc_path_claim_check` / `gen_doc_index`（BidSystem 实测悬空路径 22→0），接既有 `check_docs_links*.sh` | **口径教训**（BidSystem SF-08 家族）：生成物「写盘口径」必须等于「核对口径」。把墙钟时间放进 H1 标题或索引字段 ⇒ 逐字节比对必然永远红。sqlrustgo 门禁输出里的 `Date: $(date -u ...)` 同理，需从复现比对域剔除 |
| P2-2 | 目录策略机器化：顶层 `45` 个 `.md`、`bench/ benches/ benchmarks/`、`test/ tests/`、`reports/ audit/ verify/ claims/ evidence/` 归位 | `DIRECTORY_POLICY.md` / `FILE_GOVERNANCE.md` 已存在，缺的是执行点 |
| P2-3 | 版本口径一致性 | **实测自洽，勿当缺陷**：`Cargo.toml version = "4.1.0"`、`VERSION = v3.11.0`（注释明确是「最后 released GA tag」）、`STAGE.yaml current_stage: "ALPHA"` 三者当前无矛盾；只需一条挂靠既有 YAML 类门禁的核对防漂移 |

### P3 —— 闭环与自演进（持续）

- **债务台账定型**：`debt-registry.yaml` 现有 11 个键但 `debts` 条目 0 ⇒ 先定 schema，再挂 `check_cross_version_debt.sh` / `check_int_debt.sh` / `check_arch_sem_debt.sh`。
- **自发现台账**：移植 BidSystem `self_find.py`（子命令是 `list/add/close`，不是 `--add`）+ `self-findings.jsonl`，作为「门禁外自己发现的问题」的统一入口。
- **lessons 锚**：`5` 篇 → `KNOWLEDGE-ANCHOR.yaml` 的 `next[]`，让每轮整改必须消费上一轮的坑。
- **IAIA-410 闭环**：把 `audit/v4.1.0/` 的 `8` 个 run / `7` 个 manifest / 8 个 finding yaml 纳入 `EVIDENCE_INVENTORY`；`regression_verified` 必须有一个新的 run，`refuted` 必须有反证记录。
- **测试有效性维度**：引入 `cargo-mutants`（BustubX 已用）与 `_coverage_reverse_mutation.sh` 等价物，作为**质量报告维度**而非新增阶段门禁（避免分母再漂）。

---

## 6. 排期与 PR 切分

- **登记与回收分开**：P0-1（建台账）与 P0-4（回收 37 个）分两个 PR。合在一起，diff 里既有新增又有删除，reviewer 无法判断「被删的判据是不是本来该保留的」。
- **registry 只追加不重排**：4.1.0 主干每天数十提交，registry 是高冲突文件。规则：条目按 `gate.id` 追加，禁止重排/改名（改名等于让既往证据失效）。
- **新增设施脚本进 registry，不进阶段清单**：否则会重演分母漂移。阶段清单只承载「阶段语义」，registry 承载「清单真值」。
- **gate 清单类变更走独立 PR + owner 批准**（BidSystem §2.1 / §2.8 的纪律：不许用改文档来迁就现实）。
- 粗略节奏：P0 ≈ 1 周（4–5 PR）→ P1 ≈ 2–3 周（5 PR）→ P2 ≈ 1–2 周 → P3 持续。

---

## 7. 需要 owner 决定或授权的动作

1. **打开 `develop/v4.1.0` / `main` 的状态检查与审批**（`enable_status_check`、`status_check_contexts`、`required_approvals ≥ 1`）。后果：从未真正执行过的清单项会立刻变红 ⇒ 建议只对 ALPHA（10 项）先开。
2. **`.github/workflows` 的 `13` 个文件**：删除、归档还是保留标注？（项目禁推 GitHub，但文件仍在制造「CI 已覆盖」的错觉。）
3. **`62` 个全仓无人引用的门禁**：归档到 `scripts/gate/_archive/` 还是删除？两者都可，但必须留 reason。
4. **是否授权把四份清单收敛为两份**（registry + 阶段清单）。这是本方案的枢纽，也是唯一会动到「分母」的动作。
5. **凭据**：Gitea API 一律经 credential helper，不落盘、不打印、不进日志。本次分支保护探测为只读，未写入任何配置。

---

## 8. 明确不做

- 投标场景红线三门禁（`bid_redline` / `bid_traceability` / `bid_consistency`）与 `【待填写】` 纪律 —— 无投标产物；
- AFP 双席**跨厂商**签署硬要求（BidSystem 依赖 ChatGPT + DeepSeek 双通道）—— 保留 sqlrustgo 既有 ADR-014「禁止 Agent 自我签发验收」并机器化即可；
- BidSystem 的 252/250/willy2 三实例同步链 —— sqlrustgo 已有 5 远端工具，且主工作区另有会话的未提交 WIP（`scripts/check_repo_sync.sh` 等 5 个文件），**不动**；
- 「分母恒 11」这个具体数字 —— 继承纪律，不继承数值；
- 本文件自身的入库：需按 `DOC_CHECK_CORRECTION_RULES.md` 七步流程 + 复核报告，未获批准前保持未跟踪状态。

---

## 9. 剩余工作量与时间估算（2026-11-05 刷新）

> 口径先说清楚，否则"多少周"没有意义。
> **一个有效人日** = 完成一次**可复核闭环**：复现 → 修复 → 夹具（含反向变异）→ PR →
> 合并 → 看板/台账更新 → 证据归档。缺任何一环不计入完成，只计入"进行中"。
> SOAK 这类墙钟不能折算人力，单列。

### 9.1 实测吞吐（每条都可重放，不是拍的）

| 测量项 | 实测值 | 复现 |
|---|---|---|
| CI 真实终态（2026-08-08 → 11-04） | 1358 runs：**success 39（2.87%）**、failure 1109、cancelled 209 | `python3 scripts/gate/gov_probe.py ci` → run `20261105-cifact-001/logs/002-gov-ci.stdout` |
| CI 单次反馈时长 | median **853s（14 分钟）**、p90 **2606s（43 分钟）**、max 10790s | 同上（`dur_median_s` / `dur_p90_s`） |
| Actions jobs 分母 | 3495（此前文档出现过"3911"，两个口径都对不上 ⇒ 已作废） | 同上（`kind: ci_jobs`） |
| 提交吞吐（作者日） | 10-04..10-10：**579 提交/7 天**（日均 83，峰值 106）；10-21..10-25：235（日均 47）；**10-25 → 11-04 仅 2** | `git log --date=short --pretty=%ad develop/v4.1.0 \| sort \| uniq -c` |
| 专家分诊单条 | 37 条实测：body 截断 2600 ⇒ **中位 39s / 均值 51s**；短正文 6 条 4.2s/条 | `scripts/gate/expert_triage.py`（产物尚未入库，见 IL-1） |
| 席位标定 | chatgpt-web **4/4** @68.4s；flash-next-252 **3/4** @101.1s；deepseek `ok=false` | run `20261104-seats-002` |

三条推论：**(a)** 编码不是瓶颈——历史上有过日均 83 提交的吞吐；**(b)** 那个吞吐正是"报告好看、实测就坏"的来源
（同期 CI 成功率只有 2.87%）；**(c)** 整改期的真实限速是**验证墙钟**（一次 PR 的流水线 14–43 分钟）+ 单人单席位。

### 9.2 105 条待整改 issue 的分层（普查实测，不含看板 #5592 自身）

| 维度 | 分布 |
|---|---|
| 优先级 | P0 **47** / P1 **43** / P2 **2** / 无标签 **13** |
| 成本标签 | t0 **20** / t1 **52** / t2 **19** / 无标签 **14** |
| 能力面 | build 39、test 24、governance 15、review 7、analysis 3、docs 2、refactor 2、architecture 1、benchmark 1、无标签 12 |

按成本标签加权（t0=0.25、t1=0.7、t2=1.5、无标签=0.8 人日）：

| 分组 | 条数 | 加权人日 | 说明 |
|---|---|---|---|
| 工程面（build/test/refactor/architecture/benchmark/无标签） | 78 | **71.0** | 必须真跑编译与测试，受 §9.1 的验证墙钟限速 |
| 治理面（governance/review/analysis/docs） | 27 | **10.1** | 与 §5 的 P0/P1 设施项高度重叠，按"设施顺带清账"计入，不重复计费 |
| 合计 | 105 | **81.1** | 其中 P0 47 条 = 工程面 24.7 + 治理面 5.3 |

### 9.3 合计与区间

| 块 | 有效人日 | 备注 |
|---|---|---|
| 判定系统（P0-1/-2/-4/-5/-6/-7 + KR-2 + MX-1） | **13–19** | P0-1 要把 168 道 `check_*` 逐个登记；P0-4 要处置 62 道死门禁（修引用口径后从 36 涨到该值，+2~3 人日） |
| 声明与测量绑定（P1-1..P1-5） | **21–32** | P1-3 夹具扩面单块 9–15（清单内 34 道 + SOAK/PERF/COMPAT/TPCH 命名族） |
| 文档事实面与卫生（P2-1/-2/-3） | **4.5–6.5** | 含 §DR-1 的基线刷新 |
| 闭环与自演进（P3） | **5–8** | 债务台账、self-find、lessons 锚、IAIA 闭环 |
| 105 条 issue 清账 | **71.0**（工程面）+ 治理面随设施顺带 | 见 §9.2 |
| **合计** | **115–137** | |

换算成日历时间（单人连续投入）：

| 情形 | 吞吐假设 | 预计 |
|---|---|---|
| 乐观 | 多席位并发 + 批量 PR，日均 ≥1.8 有效人日 | **9 周** |
| **基准** | 单人，日均 1.0–1.3（CI 流水线与开发重叠、无 owner 等待） | **13–20 周** |
| 保守 | 判定系统返工一轮，或 SOAK 首次失败后回归 | **≥20 周** |

**GA 的硬门槛还包含**：`ING-1` 168 小时 SOAK = **7 个自然日墙钟**（可与开发并行，但 GA 前必须完整跑通一次，
且当前 `STAGE.yaml` 的 `known_gaps.V400_09_soak` 从未闭过）；三次阶段跃迁（ALPHA→BETA→RC→GA）各需 owner 批准。

> **勘误（对自己先执行 §2.4）**：本节前身口头给过"乐观 6 周 / 保守 11–13 周"。
> 那组数按**条数**外推，未按成本标签加权，也漏了 P1-3/P1-5 的夹具与声明绑定硬工。
> 以 §9.2 的加权分布为准，上表替换该口径。

### 9.4 会推翻上表的三件事（已登记，不是假设）

1. **ING-1**：168h SOAK 从未跑通。第一次真跑若红（大概率——`check_p13_soak_test.sh` 吞 3 处退出码），
   修复轮次不可预估，保守值直接生效。
2. **P0-3**：`develop/v4.1.0` 与 `main` 的 `enable_status_check=false`、`required_approvals=0`（§1.3）。
   在 owner 打开之前，"门禁会拦住"这句话本身不成立，任何估算都不含"被拦后返工"的成本。
3. **EX-5**：DeepSeek 席位需 owner 登录。两血统裁定（`adjudication_capable`）现在靠 gpt+qwen 成立；
   若 qwen 血统的 EX-6 掉分被降权，分诊退回单系 `advisory-only`，§5 的 IL-1/P1-1 判据要重做。

### 9.6 分诊切分：106 / 106 已分诊（grade：cross-checked 12 / advisory-only 94）

| 可自动化 | 条数 | 对推进的含义 |
|---|---|---|
| auto | 37 | 现有门禁与测试就能闭环 |
| auto-needs-infra | 32 | 要先有 §5 的设施，否则只能人肉跑一遍 |
| needs-human-decision | 29 | 下一步不在代码里，在 owner 的决定里 |
| manual-host | 7 | 需要宿主机权限（act_runner 注册、凭据轮换等） |
| infra-config | 1 | 配置项 |

69 / 106 = 65% 落在自动化通道，37 / 106 = 35% 必须先由人或基础设施动。
这解释了「47 个 P0」为什么不等于「47 个工程任务」：其中一批的下一步是配置与决策。
两个独立口径落在同一量级（`cap/build`+`cap/test` = 62 条 vs `auto`+`auto-needs-infra` = 69 条），
是分布没跑偏的信号，但**估算仍以 §9.2 的成本标签加权为准** —— 分诊是分诊建议级
（94 行 advisory-only），不是裁定。

### 9.5 "整治好了"的机器判据（避免又变成一份好看的报告）

| 判据 | 现在 | GA 时必须 |
|---|---|---|
| open issue | 106 | 0（含带理由的关闭/降级，逐条有说明） |
| 阶段标准落到机器（`stage_map`） | **0 / 21** | 21 / 21（`manual_only` 需 owner 签字） |
| 清单内门禁有负向夹具 | 23 / 168 | 清单内 34 道 100%，其余显式 `UNFALSIFIABLE(reason)` |
| 吞退出码的门禁 | 56 | 0（清单内）；清单外要么登记要么退役 |
| 孤儿 / 死门禁 | 83 / 62 | 0 / 0（进清单或带理由归档） |
| 幽灵执行点（`.github/workflows`） | 13 | 0 或标注"仅存档、不执行"且无声明引用 |
| 文档 PASS 类声明 | 1123 处命中 | 100% 绑定 `source_run` 或被降级为叙述 |
| CI 真实成功率 | 2.87%（39/1358） | ≥95%（连续 20 次 push 无红） |
| SOAK 168h | 从未跑通 | 一次完整 run，verify 退出码 0 |

**结论**：能整治好，但顺序不能反。**先修判定系统（P0/P1），再清 105 条 issue**——
否则以本项目历史 83 提交/日的吞吐，再给三个月仍然会产出一份"报告好看、实测就坏"的版本。
基准 13–20 周的前提是 owner 先把 P0-3 与 EX-5 解掉；这两项我无权也无能自行完成。

---

## 附录 A —— 全仓无人引用的 `62` 道门禁（2026-11-05 修引用口径后重测）

```
check_4019_sysbench_baseline.sh
check_4020_bulk_load_sf10.sh
check_4021_prometheus_metrics.sh
check_4022_slow_query_log.sh
check_5_principles_v310.sh
check_a04_evidence_reproducible.sh
check_alpha_v3.10.0.sh
check_arch2_no_bypass.sh
check_attack_surface.sh
check_audit_manifest.sh
check_backup_restore_closed_loop.sh
check_beta_e2e.sh
check_beta_v3.10.0.sh
check_bustubx_b_track_v312.sh
check_bustubx_edu_cli_smoke_v312.sh
check_cli_binary_names.py
check_coverage.sh
check_execution_semantics.sh
check_g11_qps.sh
check_g12_sysbench.sh
check_g15_perf_report.sh
check_g16_ddl_syntax.sh
check_g1_tpch_22_22.sh
check_g6_set_ops_syntax.sh
check_g_correctness_v390.sh
check_ga_only_v312.sh
check_gate_script_refs.sh
check_gmp_consumer.sh
check_historical_backlog_disposition.sh
check_main_freshness.sh
check_mainline.sh
check_no_log_tbl_json.sh
check_oracle_present.sh
check_orphan_sources.sh
check_p32_statistics.sh
check_p33_cost_optimizer.sh
check_p34_parallel_executor.sh
check_p35_simd.sh
check_plan_integrity_v310.sh
check_pr_dag.sh
check_prepared_stmt.sh
check_principles.sh
check_proof.sh
check_rc_gate_v3.10.0.sh
check_rc_v3.11.0.sh
check_release_binary.sh
check_sql_compat.sh
check_ssot_duplicate.py
check_stage_template.sh
check_storage_index_wal.sh
check_teaching_corpus_v312.sh
check_test_count_monotonic.sh
check_test_inventory.sh
check_tpch_sf10.sh
check_v312_01_blocker_closed.sh
check_v312_dml_integrity.sh
check_v312_join_subquery_semantics.sh
check_v312_no_silent_success.sh
check_v312_parser_real_scripts.sh
check_v312_promotion_to_rc.sh
check_v312_type_function_semantics.sh
check_validation_chain.sh
```

口径：`dead` = 不在 `STAGE_CONFIG.yaml` 任何阶段清单、不在 CI（含门禁调门禁的传递可达）、
且**没有任何脚本在可执行位置**引用它。2026-11-04 曾列 `37` 条，2026-11-05 首次重测得 `36` 条，
两者都偏低：探针原先在扫描引用时**没有去掉注释与文档字符串**，脚本注释里一句「参见
`scripts/gate/xxx.sh`」就让那道门禁"活着"。修 `gov_probe.strip_prose()` 后实测 `62` 条
（新增 `26` 条），逐条 grep 复核过：这些门禁只出现在文档正文与脚本注释里。
复现：`python3 scripts/gate/gov_probe.py metrics`（`dead_list` 字段），归档见 run `20261105-triage-001`。

处置三选一:进阶段清单（若判据仍有意义）/ 归档（历史版本线，如 `*_v3.10.0`、`*_v312`）/ 删除（判据已被更严的门禁覆盖，需写明由谁覆盖）。

## 附录 B —— 清单外门禁的分类

### B-1 孤儿共 `83` 个（不在阶段清单 & CI 不可达）

- 全仓无任何引用：**`37`**（附录 A）
- 仅被其它脚本引用（可能是「门禁调门禁」但根不在 CI）：`46`
- 主要类别：历史版本线（`*_v3.10.0` / `*_v3.11.0` / `*_v312` / `*_v310` / `*_v390` / `*_v380`）、性能与基准（`check_g11_qps` / `check_g12_sysbench` / `check_tpch_sf10` / `check_4019_sysbench_baseline`）、长稳与 soak（`check_ga2_soak_artifact`）、可复现性（`check_a04_evidence_reproducible`，**已实现却是孤儿**，见 #5466）、死代码/孤儿源（`check_orphan_sources`）、SSOT 重复（`check_ssot_duplicate.py`）

> **本轮整改要用的解药，仓库里已写好五件，且五件全部无人执行**（实测：既不在 `required_gates`，也不在 CI 传递可达集合）：
> `check_orphan_sources.sh`（未参与编译的 `.rs` ⇒ 其 `#[test]` 永不运行；注释里已记 `tools/src/backup.rs` 七层类型漂移的真实案例）、`check_ssot_duplicate.py`（跨文档重复内容 ⇒ SSOT 漂移）、`check_a04_evidence_reproducible.sh`（已入库证据能否复现，#5466）、`check_test_count_monotonic.sh`（测试数不得倒退）、`check_test_inventory.sh`（测试清单一致性）。
> P0-1 登记时这五道**优先接上执行点**，不做回收；其余历史版本线（`*_v3.10.0`、`*_v312` 等）才是回收对象。

### B-2 只在 `.github/workflows` 出现（`4` 个）

```
check_alpha_v380.sh
check_cross_version_debt.sh
check_no_log_tbl_json.sh
check_sql_corpus_gate.sh
```

## 附录 C —— 复现命令

```bash
# 基线（与 252 Gitea 一致性）
git -C /Users/liying/dev/sqlrustgo fetch gitea252 refs/heads/develop/v4.1.0
git -C /Users/liying/dev/sqlrustgo rev-list --left-right --count develop/v4.1.0...FETCH_HEAD   # 0 0

# 清单↔执行点矩阵（当前 92 项全 OK）
cd /Users/liying/dev/sqlrustgo/.worktrees/410 && python3 scripts/gate/gate_executability.py

# 孤儿 / 死门禁 / 幽灵执行点（§1.1、附录 A、B）
python3 scripts/gate/gov_probe.py metrics        # 孤儿/死门禁/幽灵执行点/清单计数（JSONL，stdout 即产物）
python3 scripts/gate/gov_probe.py honesty        # 每道 check_* 的可证伪性（反例夹具、吞退出码）
python3 scripts/gate/gov_probe.py auditmanifest  # 已提交 audit run 的证据校验退出码
python3 scripts/gate/gov_probe.py issues         # open issue 普查（type=issues）

# 归档与重放（看板引用的每个数字都指向一个 run）
python3 audit/v4.1.0/audit_run.py run    --run-id 20261104-govprobe-001
python3 audit/v4.1.0/audit_run.py verify --run-id 20261104-govprobe-001

# 阶段标准映射（§1.1 criteria_gates=0）
python3 - <<'PY'
import yaml; c=yaml.safe_load(open("docs/governance/STAGE_CONFIG.yaml"))
for s,v in c["stages"].items():
    print(s, len(v.get("required_gates") or []), len(v.get("exit_criteria") or []),
          len(v.get("criteria_gates") or {}), len(v.get("manual_only_criteria") or {}))
PY

# 分支保护与状态检查（只读）
T=$(printf "protocol=http\nhost=192.168.0.252:3000\n\n" | git credential fill | grep '^password=' | cut -d= -f2-)
curl -s --config - <<EOF | python3 -c "import json,sys;d=json.load(sys.stdin);print({k:d.get(k) for k in ('name','protected','enable_status_check','status_check_contexts','required_approvals')})"
url = http://192.168.0.252:3000/api/v1/repos/openclaw/sqlrustgo/branches/develop%2Fv4.1.0
header = Authorization: token $T
EOF
```

> **更新（2026-11-05）**：上述探针已正式化为 `scripts/gate/gov_probe.py`（PR #5593），
> 本文引用的基线数字已归档为可校验 run `audit/v4.1.0/runs/20261104-govprobe-001`。
> 原先写在 `/tmp/srg-probe/` 的临时版本不再是任何声明的出处。
> 实测口径提醒：`gov_probe.py metrics` 必须先调用 `build_matrix()` 再读
> `gate_executability.ALL_REQUIRED_SCRIPT_PATHS`，漏调会让孤儿数从 83 虚高到 95（见看板 Q10 / P0-7）。
