# 迭代 5 · 批次 10 —— GA 判据唯一化 + 执行者接线（#5437 / #5438 / #5439 / #5440 / #5441）

> **日期**: 2026-10-21
> **分支**: `fix/stage-ssot-and-artifact-gates-batch10`
> **基线**: `develop/v4.1.0` @ `5fbc6b498113`（批次 9 的 merge commit）
> **范围**: 治理层（SSOT / 驱动 / 门禁 / 文档指针）。**未触碰任何 Rust 代码**
> —— `git diff --stat -- src/ crates/ tests/` 为空（仅 `tests/baseline/test_count.json`
> 的时间戳被门禁刷新，随后已还原）。

---

## 0. 本批次做了什么（一句话）

**把「GA 有多少门禁、缺哪些产物、日志存在哪」这三件事，从「三份文档各说各话」
变成「一个来源 + 一套机器检查」，并且让那个来源第一次真的被执行。**

---

## 1. 问题与修法对照

| Issue | 症状（实测） | 本批次的修法 | 可判定的证据 |
|---|---|---|---|
| #5437 | 同一个「GA 有多少门禁」有 **4 个项数、3 套互斥语义**（SSOT 自称 20 项 / 自己又写 "G1-G16"=16 / GATE_CONDITIONS 的 G1~G6=6 / CONVERGENCE_TRACKER 的 G1-G16 表=16） | G 编号**降级为纯别名**（SSOT 明确声明「判据以 `required_gates` 脚本路径为准」）；`GATE_CONDITIONS.md` 的 GA 小节编号 `G1~G6` → **`GA1~GA6`** 并加判据唯一性横幅；`CONVERGENCE_TRACKER.md` 的历史 ✅ 表**加横幅不改写** | `check_stage_config_consistency.sh`（C1–C6）+ `check_stage.sh --stage GA --dry-run` 列出真实 20 项 |
| #5438 | G 编号 → 脚本**大面积错位**：G7→全量聚合器、G8→漂移检测器、G9→跨版本债务、G10→反捏造、G5→架构语义债、**G2/G3→INT 债务台账** | 逐条改回**脚本自述的 G 编号**（脚本头里写明「… G7 gate」的那一个）；并新增 C3 规则**让机器检查这件事** | 一致性门禁 C3：脚本头必须自称该 G 编号 |
| #5439 | v4.1.0 缺 **4/5** 份 GA `required_files` + **4/6** 份 GA `doc_artifacts`；`doc_artifacts` **从来没有被任何代码读过**；3/5 份 GA 必需文件因路径上粘了 `(state=GA)` 而**不可解析** | ① 注解从路径剥离（状态期望移入 `required_file_expectations`）；② 新门禁 `check_release_artifacts.sh` 按**当前阶段**判定，并**披露**后续阶段缺口；③ 补齐 ALPHA 唯一真缺件 `FEATURE_MATRIX.md` | `check_release_artifacts.sh` 的 MISS 列表（GA 6 项缺件逐条列出） |
| #5440 | `check_stage.sh` 是 SSOT 的唯一执行器，**CI 从未调用过**（`grep` 只在注释里命中） | 接入 CI：配置一致性门禁 + SSOT 驱动 dry-run + 产物门禁 | `.gitea/workflows/ci.yml` 新增 3 个 step |
| #5441 | 治理要求 GA 日志存 `docs/releases/v{VER}/logs/`，而 `.gitignore` **恰好排除该目录** ⇒ 证据链设计上无法入库 | `.gitignore` 放行 `gate_*.log`（决策记录入库），`logs/gates/` 原始落盘仍不入库；`check_no_log_tbl_json.sh` 白名单同步 | `git check-ignore` 探针（M7/M8/M9） |

---

## 2. 「一个判据只能有一份实现」—— 共享规则库

新文件 **`scripts/gate/stage_config_common.sh`**（336 行）持有框架里**全部**路径解析规则：

| 规则 | 内容 | 由来 |
|---|---|---|
| R1 | 门禁路径解析；`{VER}` 的去点 / 连字符 / 下划线三种拼法 | `check_alpha_v410.sh` vs `check_alpha_v3.12.0.sh` 命名不一致 |
| R2 | 必需文件路径**去 `(...)` 注解** | `(state=GA)` 是状态期望，不是路径 |
| R3 | `doc_artifacts` **相对发布目录**解析 | 否则每个 doc_artifact 都被判 MISS |
| R4 | G 编号**名实相符**：脚本路径须含该 G `name` 的实义词；实义词由 `name` 自动派生（不是第二份手维护映射表） | 「Soak → 聚合器」这类错位要能被机器检出 |
| — | `stage_artifact_status()`：**唯一**的产物存在性判定实现 | 驱动与产物门禁都调它，两边不可能给出不同结论 |

**驱动（`check_stage.sh`）与两个新门禁（`check_stage_config_consistency.sh` /
`check_release_artifacts.sh`）全部 source 这一个文件。**

---

## 3. 新增门禁 1：`scripts/gate/check_stage_config_consistency.sh`

检查 **SSOT 自己**是否自洽（不执行 SSOT 里的门禁）：

| 规则 | 判据 | 抓什么 |
|---|---|---|
| C1 | 每个 `required_gates`/`optional_gates` 条目（cargo 除外）必须能解析到存在的文件 | 悬空引用（旧的 `check_soak_gate.sh` / `check_p13_soak.sh`） |
| C2 | 每个 `gate_definitions` 的 `script`/`alternative`/`additional_scripts` 必须存在 | 别名指向不存在的脚本 |
| C3 | G 的**主**脚本**必须在自己的文件头声明该 G 编号** | #5438 的错位（脚本压根不自称是那个门禁） |
| C4 | G 的脚本路径须含其 `name` 的实义词（自动派生） | 「名实不符」 |
| C5 | `required_files`/`doc_artifacts` 条目必须是**纯路径**，不得带 `(...)` 注解 | #5439 的 `(state=GA)` |
| C6 | `required_file_expectations` 的键必须属于同一阶段的 `required_files` | 指向没人要求的文件的空期望 |
| D1 | **披露**（不判红）：G 别名的脚本没有被任何阶段的门禁清单执行 | 让审阅者看见哪些关注点当前无人执行（见 §6 新立 issue） |

当前实测：**FAIL 0 / DISCLOSURE 14 / RESULT PASS**。

## 4. 新增门禁 2：`scripts/gate/check_release_artifacts.sh`

- **判据来源**：`STAGE_CONFIG.yaml` 的 `required_files` + `doc_artifacts`（与驱动同一份）。
- **阻断**：当前阶段缺件 ⇒ FAIL。
- **披露**：后续阶段（BETA/RC/GA）的缺件**逐条列出但不阻断** —— 版本还没到那一阶段，
  但「离 GA 还差什么」必须是**能读到的事实**，而不是要人重新推导的结论。
- `--all-stages` / `--stage <S>` 供晋升时使用。

当前实测（v4.1.0，ALPHA）：

```
ALPHA（阻断）: 7/7 齐
BETA （披露）: 5 齐 / 2 缺  FEATURE_CHECKLIST.md
RC   （披露）: 5 齐 / 4 缺  RELEASE_GATE_CHECKLIST.md, PERFORMANCE_TARGETS.md,
                             EVIDENCE_STATUS.md, GA_GATE_REPORT.md
GA   （披露）: 5 齐 / 6 缺  GA_GATE_REPORT.md, GA_RELEASE_TIMELINE.md,
                             EVIDENCE_STATUS.md, POST_GA_PLAN.md
```

**GA 的 6 项缺件没有被伪造，也没有被"跳过即 PASS"。** 它们在本批次被**变成可见**，
这正是 #5439 要求的「缺失可检测」。

## 5. `docs/releases/v4.1.0/FEATURE_MATRIX.md`（补齐 ALPHA 唯一真缺件）

145 行，**逐格从既有文档派生**，不引入任何新断言：

- §1 变更矩阵：来源 = `VERSION_PLAN.md` §2（5 条 delta）+ `CHANGELOG.md` P0 修复轮（7 条 F-xx）
- §2 里程碑矩阵：来源 = `ROADMAP.md` §1；**M-6 / M-8 的口径冲突两个读法都列出**
  （ROADMAP 快照 2026-09-29 说 NOT STARTED；`STAGE.yaml.last_transition` 2026-10-06 说已清除/已合并）
- §3 **本文件不声称什么**（不是 GA、不是接近 GA、ALPHA→BETA 判据未满足、7 条 open_issues 未解决、
  不声称任何门禁通过、不声称任何覆盖率/吞吐数字）
- §4 产物齐备度表（引自门禁实测输出）

---

## 6. 判别力自证（变异测试，13/13）

脚本 `/tmp/b10_mutations.sh`（临时文件，已在交付前删除）。在**真实工作树**上注入缺陷，
逐条断言对应门禁变红，再逐条还原：

| # | 注入 | 期望 | 实测 |
|---|---|---|---|
| M1 | G7 改回 `check_full_gate_verification.sh`（#5438 原缺陷） | 一致性门禁红，C3 | ✅ |
| M2 | G9 改回 `check_cross_version_debt.sh` | 一致性门禁红，C3 | ✅ |
| M3 | GA `required_files` 重新粘上 `(state=GA)` | 一致性门禁红，C5 | ✅ |
| M4 | `optional_gates` 改成不存在的 `check_soak_gate.sh` | 一致性门禁红，C1 | ✅ |
| M5 | 删掉 `FEATURE_MATRIX.md` | 产物门禁红并点名 | ✅ |
| M6 | `current_stage` 改成 `GA` | 产物门禁跟到 GA 并报 GA 缺件 | ✅ |
| M7 | 撤掉 `.gitignore` 的放行规则 | GA 门禁日志重新不可入库（证明该规则是**承重**的） | ✅ |
| M8 | 干净树 | GA 门禁日志**可入库** | ✅ |
| M9 | 文件治理门禁白名单 vs `.gitignore` | 两者对 gate 日志**一致**（且只对 gate 日志一致） | ✅ |
| — | 还原后 | 两个门禁重新全绿 | ✅ |

两个新门禁另有 `--selftest`（用合成配置证明「干净样本 PASS / 违规样本 FAIL 且报对规则」）：
- 一致性门禁：**6/6 规则**（C1–C6）逐条可判别
- 产物门禁：完整集 PASS / 缺一件 FAIL 且点名

### 6.1 证据链的**端到端实证**（不是设计说明，是实跑）

在 commit `4d8c8deb10` 上**实跑**了驱动（取最轻的 DRAFT 阶段，
因为它只要求 2 个产物 + 2 个门禁，其中 `cargo build --all-features`
复用已热的 target 目录）：

```
$ bash scripts/gate/check_stage.sh --version v4.1.0 --stage DRAFT
--- Required files & doc artifacts ---
  [OK]  docs/releases/v4.1.0/VERSION_PLAN.md
  [OK]  docs/releases/v4.1.0/ARCHITECTURE.md
  [OK]  docs/releases/v4.1.0/VERSION_PLAN.md
  [OK]  docs/releases/v4.1.0/ARCHITECTURE.md
  [OK]  docs/releases/v4.1.0/ROADMAP.md
--- Required gates (2) ---
  [PASS] check_docs_links.sh
  [PASS] cargo build --all-features
--- Optional gates (1, informational) ---
  [INFO] check_arch_invariants.sh (optional)
==================================================================
  Stage DRAFT (v4.1.0): 7 PASS, 0 FAIL
  gate logs: docs/releases/v4.1.0/logs/gates/
  run log:   docs/releases/v4.1.0/logs/gate_draft_4d8c8deb10_20261021_142051Z.log
```

产出的**决策记录随本 PR 入库**：

```
docs/releases/v4.1.0/logs/gate_draft_4d8c8deb10_20261021_142051Z.log   ← 入库
docs/releases/v4.1.0/logs/gates/check_docs_links.sh.log                ← 不入库
docs/releases/v4.1.0/logs/gates/cargo_build_--all-features.log          ← 不入库
```

`git status --untracked-files=all` 只列出第一个文件 —— **入库边界与设计一致**。
在此之前，「GA 门禁跑过了没有」在仓库里**查不到**（#5441）。

---

## 7. 本批次**顺带抓出**的真实缺陷（不是本批次引入的）

### 7.1 批次 9 的证据文档自带 2 条坏指针

`check_docs_pointer_paths.sh` 在本批次运行时判红：

```
evidence/EXTERNAL_TOOL_AND_DOC_POINTERS_BATCH9_2026-10-21.md -> docs/governance/AGENT_PROMPT_TEMPLATE.md
evidence/EXTERNAL_TOOL_AND_DOC_POINTERS_BATCH9_2026-10-21.md -> docs/releases/v3.13.0/perf/BIN_LOAD_PERF.md
```

**根因是流程顺序**：批次 9 **先跑门禁、后写证据文档**，而证据文档里**引用了两个
已知不存在的路径**（前者只存在于他人分支，后者是它要记录的缺陷本身）。
行内代码在本仓库就是指针约定 ⇒ **记录坏指针的文档，自己变成了坏指针**。

修法：① 那两处引用改为普通散文（路径照写，旁边句子说明它不存在），并在文档开头
写明这条**写作约定**；② 在门禁头部写明「**引用已知不存在的路径时不要用行内代码**」
与「**证据文档写完之后要再跑一次门禁**」。

> 这是本批次最重要的流程教训：**门禁要在证据文档之后跑，不是之前。**

### 7.2 G 别名大面积**没有执行者**（已立案，不在本批次擅自扩张 GA 判据）

C3/C4 修好了「映射错位」，但进一步检查发现：**16 个 G 别名里有 14 个的脚本，
不被任何阶段的 `required_gates`/`optional_gates` 执行**：

```
G1 G2 G3 G5 G6 G8 G9 G10 G11 G12 G13 G14 G15 G16
```

即：**GA 阶段要求的 20 项里，压根没有 TPC-H 基线、savepoint、upgrade、
audit log、time travel、QPS、sysbench、perf report、compatibility 这些门禁。**

同时，`scripts/gate/check_g_all.sh`（唯一跑全套 G 门禁的编排器）**本身是孤儿**
—— 全仓无人引用它 —— 而且它内含**第二份 G→脚本映射**（G2→`check_int2_no_orphan.sh`、
G3→`check_int3_single_expr.sh`），与 SSOT 的映射**并存**。

本批次的处理：
- **不**擅自把 14 个脚本塞进 GA 的 `required_gates` —— 那会改变「GA 全绿」的含义，
  必须由人决定，且与「声明判据唯一化」同一批次里做是自相矛盾的；
- 改为 **D1 披露**（一致性门禁每次运行都列出这 14 条）；
- **新立 issue** 记录该缺口与 `check_g_all.sh` 的第二份映射。

---

## 8. 回归

| 检查 | 结果 |
|---|---|
| `check_stage_config_consistency.sh`（新） | PASS（FAIL 0 / DISCLOSURE 14） |
| `check_release_artifacts.sh`（新） | PASS（ALPHA 7/7 齐；GA 缺件已披露） |
| `check_stage.sh --version v4.1.0 --dry-run` | PASS（7 产物 + 9 门禁全部解析成功） |
| `check_stage.sh --version v4.1.0 --stage GA --dry-run` | 预期红：GA 缺 6 份产物（**如实红，不掩盖**）；20 项门禁全部解析成功 |
| `check_test_manifest_honesty.sh` | PASS |
| `check_test_count_monotonic.sh` | PASS |
| `check_anti_ignore_gate.sh` | PASS |
| `check_drift_not_pass.sh` | PASS |
| `check_path_casefold_conflict.sh` | PASS |
| `check_off_production_path_registry.sh` | PASS |
| `check_test_target_lint_budget.sh` | PASS |
| `check_no_log_tbl_json.sh` | PASS |
| `check_test_external_tool_paths.sh`（批次 9 的 C7） | PASS |
| `check_docs_pointer_paths.sh`（批次 9） | PASS（27 坏指针全部在已登记债务内；**新增 0**） |
| `check_docs_links.sh` | PASS |
| `cargo fmt --check --all` | PASS |
| `cargo clippy --all-features --all-targets -- -D warnings` | exit 0 |
| Rust 改动 | 无（`git diff --stat -- src/ crates/` 为空） |

---

## 9. 本批次**没有**做的事（明确的边界）

1. **没有伪造 GA 产物**。`GA_GATE_REPORT.md` / `GA_RELEASE_TIMELINE.md` /
   `EVIDENCE_STATUS.md` / `POST_GA_PLAN.md` 仍然缺失 —— 它们只能在门禁**真跑过**之后生成。
2. **没有让 GA 全量门禁进 CI**。CI 只跑**配置一致性**、**SSOT 解析**、**当前阶段产物**三件事；
   GA 的 20 项全量执行是**发布时在 release 分支上**做的事（#5436）。
   理由：把「GA 全量」塞进每次提交的 CI，会得到一个**永远红**的 CI，而永远红的 CI
   传递不出任何信息。
3. **没有改写历史证据条目**。`CONVERGENCE_TRACKER.md` 的 "G1-G16 ✅" 表、
   `STAGE.yaml` 的 2026-09-30 回滚记录，都**原样保留**，只加横幅。
4. **没有动 Rust 代码**，没有触碰主工作区 `/Volumes/workspace/dev/sqlrustgo`。

---

## 10. 引用

- `docs/governance/STAGE_CONFIG.yaml` — SSOT（G 编号现为别名；GA 判据 = `stages.GA.required_gates`，20 项）
- `scripts/gate/stage_config_common.sh` — 共享解析规则库（R1–R4 + 唯一产物判定）
- `scripts/gate/check_stage_config_consistency.sh` — SSOT 自洽门禁（C1–C6 + D1 披露）
- `scripts/gate/check_release_artifacts.sh` — 阶段感知产物门禁
- `scripts/gate/check_stage.sh` — 阶段驱动（唯一执行者；日志落 `docs/releases/<ver>/logs/`）
- `docs/governance/GATE_CONDITIONS.md` — GA 小节编号 `GA1~GA6` + 判据唯一性横幅 + 日志入库要求
- `docs/releases/v4.1.0/FEATURE_MATRIX.md` — ALPHA 阶段的变更矩阵（诚实派生）
- `tests/baseline/doc_pointer_paths_releases_v4.1.0.json` — 文档指针已知债务基线（27 条，本批次未新增）
