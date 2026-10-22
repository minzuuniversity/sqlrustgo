# 门禁判据可执行性修复设计（#5457 / #5458）

> **Author**: opencode-z440
> **Date**: 2026-10-22
> **Base**: `origin/develop/v4.1.0` @ `378f014529`
> **Worktree**: `.worktrees/opencode-z440-20261022`
> **Branch**: `opencode/fix-5457-5458-gate-refs`
> **Status**: v2 — 经 spec review 修订（见 §0 修订记录）

---

## 0. 修订记录（v1 → v2）

v1 spec 经独立 spec-review subagent 审查，判定 **NEEDS-REVISION**。经复核，
以下指控**全部成立**，v2 已全部采纳：

| # | 评审指控 | 复核证据 | v2 的处置 |
|---|---|---|---|
| D1 | BE5/B-F5 的行号是 82/108 | 实际 **103 / 129**（v1 抄了历史引用里的旧行号，未核对活文件）| 改为 103/129，并新增「行号必须在实现时对 HEAD 复核」规则 |
| D2 | **目标文档搞错了** | BE5/B-F5 指的是 `docs/releases/v3.8.0/plans/DEVELOPMENT_PLAN.md` §2「PR DAG（依赖执行图）」；`E2E_PR_DAG_MAPPING.md` 属于 **B-F8**，且已有可用验证器 `check_beta_e2e.sh` | **组件一重定向**到 `DEVELOPMENT_PLAN.md` §2 + §2.1 状态表；E2E 映射表移出范围 |
| D3 | 标识文法与文件不匹配 | `PR-2873` 在仓库里 0 命中；`#[0-9]+` 唯一命中是 **Issue #2603** 而非 PR | 改用 `PR-[0-9]{3,4}[A-Z]?`；**绝不**在 `#` 表示 Issue 的文档里套裸 `#[0-9]+` |
| D4 | merge 顺序比对有 3 处倒置，**永远不可能 PASS** | 实测 PR-2806 比 PR-2697 晚合并 2.8 天；文档行序是**功能序 F-01..F-23** | **删除顺序比对**。改为校验「id 存在」+「状态列与 git 现实一致」 |
| D5 | DAG id 不是 Gitea PR 号 | `PR-830A` = Gitea **#2656**；且仓库两处散文映射与 git **互相矛盾** | 用 **merge commit 标题**里同时出现的 `(Gitea #N)` 作为权威映射（git 历史自带，无歧义） |
| D6 | 存在性脚本的正则**看不到** #5450 的实例 | #5450 的 3 个缺失脚本在 `./scripts/` 下，**不在** `scripts/gate/` 下 → v1 的正则 0 命中 | 正则放宽到 `(\./)?scripts/[...]\.sh`；扫描范围重新推导 |
| D7 | 6 文档范围只覆盖 ~90 个实例中的 1 个；3 个 scope 是 no-op | 实测 `RELEASE_GATE_CHECKLIST.md` / `AI_COLLABORATION.md` / `RELEASE_LIFECYCLE.md` 各 0 命中 | 范围改为**实际有命中的 SSOT 判据文档** + committed baseline 记录已知历史债（照抄仓库既有 `doc_pointer_paths` 模式） |
| D8 | 新 Rust 测试跑不起来 + FAIL `no_orphan_test_file` | `tests/integration/gate/` 下每个文件都需在 `Cargo.toml` 显式 `[[test]]` 注册 | **v1 实现中已踩中并修复**（已加 `[[test]]` 块）|
| D9 | 「11 个测试」实为 10 个；且该文件一半测的是 D9 脚本不是模板 | 实测 10 个 `#[test]`（行 7/15/30/49/64/82/90/104/123/144）| 订正为「10 个：5 个模板断言 + 5 个 D9 脚本断言」|
| D10 | exit 2 约定与 house style helper 契约 | `GATE_CONDITIONS.md:258` 确为「0=PASS,1=FAIL」；house style 是 `ERRORS`/`log_*` | 明确写出 0/1/2 约定 + 采纳 `check_anti_fabrication.sh` 的 `ERRORS`/`log_*` 风格 |
| D11 | 「改 2 行」低估 5 倍 | `v3.8.0` 在模板里出现 **10 次**（行 4/21/22/23/24/25/49/77/79/87），分属 4 种语义 | 只改 21–25 的 `PR-XXX_*` 族（5 行），其余 5 处按语义分类明确排除 |
| D12 | 自豁免机制多余且自我拆台 | v1 既不改 `GATE_CONDITIONS.md`，新脚本就不被引用 → **根本不需要豁免** | 删除自豁免机制 |
| D13 | 还有第三个实例 `scripts/gate/check_pr_dag.sh` | `DEVELOPMENT_PLAN.md:92` 与 `ALPHA_STAGE_REVIEW.md:81` 都推荐这个名字 | **两个名字都覆盖**：`check_pr_dag.sh` 作为 `verify_pr_dag.sh` 的 alias |
| D14 | v1 缺「❌PASS 记录」的证据引用 | `v4.0.0/FEATURE_CHECKLIST.md:17`、`v4.0.0/BETA_GATE_REPORT.md:19`、`v3.11.0/BETA_GATE_REPORT.md:38,58` | 补入 §1.1 证据表 |
| R1 | 新 gate 可能被 drift machinery 吞掉 | `check_drift_not_pass.sh:45` 扫 `-eq 2` 模式 | §5 增加 V14：跑 `check_drift_not_pass.sh` |
| R2 | 新测试一旦被 gate 引用即成 gate test，禁止 `#[ignore]` | ADR-008 Policy 2 / P16 | 测试文件**不含** `#[ignore]`，并加 V15 验证 |
| R4 | P11 要求 header 含 Purpose + P 引用 | `check_gate_self_verification.sh:74-115`（WARN 级） | header 采纳该格式 |
| R5 | 禁止 `\|\| true` 后读 `$?` | P14 + `gate_honesty_test.rs:579-629` | 新脚本不使用该模式，并加 V14 验证 |
| R6 | `check_docs_pointer_paths.sh` 的 baseline 已把 `verify_pr_dag.sh` 记为 `planned_artifact_never_committed` | 实测 baseline 命中 | 文件落地后该 baseline 条目会变为 resolved；§5 增加该 gate |
| R7 | 新 test target 需过 `check_test_target_lint_budget.sh` | baseline `total: 0, by_lint: {}` | §5 增加 V16 |
| R8 | 无调用方的 gate 会立刻腐烂 | 仓库自己的教训（`check_stage.sh:20-23` 记录 #5440） | §5 明确标注 CI 接线待办 issue，不假装已接线 |

**v2 的核心变化**：组件一从「校验 E2E 映射表」重定向为「校验
`DEVELOPMENT_PLAN.md` §2 的 PR DAG 依赖链 + §2.1 状态表与 git 现实一致」。

---

## 1. 问题陈述

仓库里存在一类**"规范指向缺失实体"**缺陷：门禁判据文档引用了一个
不存在的脚本，于是该项判据**永远无法执行**——既产不出 PASS，也产不出 FAIL，
但历史门禁报告里已经把它记成了 `✅ PASS`。

这类缺陷的特征是**不会让任何东西变红**，因此能长期潜伏在
"门禁清单"这种最不可能被质疑的地方。

### 1.1 已实测确认的实例

| Issue | 判据文档 | 被引用但不存在的路径 | 实测命令 |
|---|---|---|---|
| #5457 | `docs/governance/GATE_CONDITIONS.md:103` (BE5)、`:129` (B-F5) | `scripts/gate/verify_pr_dag.sh` | `ls -l scripts/gate/verify_pr_dag.sh` → `No such file or directory` |
| #5457 | `docs/releases/v3.8.0/plans/DEVELOPMENT_PLAN.md:92` | `scripts/gate/check_pr_dag.sh` | 同上 |
| #5457 | `docs/releases/v3.8.0/alpha/ALPHA_STAGE_REVIEW.md:81` | `scripts/gate/check_pr_dag.sh` | 同上 |
| #5450 | `docs/governance/RELEASE_GATE_CHECKLIST.md:119,122,125` | `./scripts/check_links.sh`、`.检查` `./scripts/check_format.sh`、`./scripts/generate_documentation_report.sh`（3 个）| `ls` 逐个 → 不存在 |
| #5458 | `AGENTS.md:120` | `PR_TEMPLATE.md`（文件名与实际路径不匹配）| `ls .github/PULL_REQUEST_TEMPLATE.md .github/PR_TEMPLATE.md docs/governance/PR_TEMPLATE.md` → 三个都不存在 |

### 1.2 「❌PASS 记录」的证据（#5457 的核心危害）

判据不可执行，但历史报告已记为通过：

| 文件:行 | 记录内容 |
|---|---|
| `docs/releases/v4.0.0/FEATURE_CHECKLIST.md:17` | `` `scripts/gate/verify_pr_dag.sh` exists; PR history verified: `` |
| `docs/releases/v4.0.0/BETA_GATE_REPORT.md:19` | `| BE5 | PR-DAG verification | scripts/gate/verify_pr_dag.sh | ✅ All V400-* PRs merged in documented order |` |
| `docs/releases/v3.11.0/BETA_GATE_REPORT.md:38` | `| BE5 | PR-DAG 图表存在且与实际提交一致 | scripts/gate/verify_pr_dag.sh | PASS |` |
| `docs/releases/v3.11.0/BETA_GATE_REPORT.md:58` | `| B-F5 | PR-DAG 与实际一致 | scripts/gate/verify_pr_dag.sh | exit 0 | PASS |` |

⇒ **没有任何可复现的方式能重新验证这些 ✅**。这正是"判据不可执行"的
真正代价：不是"没检查"，而是"检查过了但无法复现"。

### 1.3 #5458 原文前提的订正

issue #5458 的正文说「`PR_TEMPLATE.md` 在仓库里**不存在**」。实测发现这个
陈述**只对了一半**：

- ✅ `.gitea/pull_request_template.md` **存在**（3603 字节），且**已定义
  5-类文档**：SPEC / TEST_PLAN / TEST_DESIGN / REVIEW / ACCEPTANCE
- ✅ `docs/governance/TEST_REVIEW_TEMPLATE.md` **存在**（8941 字节），
  模板内对它的引用路径有效
- ✅ `tests/integration/sql/pr_template_and_full_gate_test.rs` **已存在**，
  **10** 个 `#[test]`（行 7/15/30/49/64/82/90/104/123/144）—— 其中
  **5 个**断言模板（行 8/16/31/50/65），**5 个**断言
  `scripts/gate/check_full_gate_verification.sh`（行 83/91/105/124/145）

因此 #5458 的**真实**缺口是三条更细的裂缝：

| # | 真实缺口 | 实测依据 |
|---|---|---|
| a | `AGENTS.md` 引用 `PR_TEMPLATE.md` 这个**文件名**，与实际路径 `.gitea/pull_request_template.md` 不匹配 | `AGENTS.md:120` |
| b | 模板内 `docs/releases/v3.8.0/PR-XXX_*.md` 硬编码旧发布线 | 模板行 21–25（`v3.8.0` 共出现 10 次，另 5 处语义不同，见 §3.4）|
| c | 「5 类文档是否齐备」**无机器判据** | 现有测试只读模板文件本身（行 9–80），不读**实际 PR 正文** |

---

## 2. 目标与非目标

### 2.1 目标

1. **让 BE5 / B-F5 可执行**：实现 `scripts/gate/verify_pr_dag.sh`，
   使 `GATE_CONDITIONS.md` 的这两行判据能被真实执行并产出 PASS/FAIL。
2. **根治同型缺陷**：新增一个**存在性自检**脚本，扫描所有门禁判据文档
   引用的 `scripts/gate/*.sh`，验证其存在。一次覆盖 #5450 / #5457 /
   #5458 三个同型实例，并且**对未来的同类缺陷自动生效**。
3. **订正 #5458 的三处真实裂缝**（a/b/c）。
4. **让我的提交身份能通过门禁**：CHECK 4 白名单加入
   `opencode-z440@gaoyuanyiyao.com`。

### 2.2 非目标

- ❌ 不改任何 Gate 阈值、分支保护配置、CI workflow
- ❌ 不修改历史门禁报告里的既有 `✅ PASS` 记录（那是历史事实，
  追加勘误说明即可；改写历史报告超出本 issue 范围）
- ❌ 不合并自己的 PR（PERMISSION_MATRIX §3：AI Developer 合并 PR = ❌）
- ❌ 不触碰 `.gitea/workflows/ci.yml`、`crates/mysql-server/src/lib.rs`、
  `src/execution_engine.rs`（他人 agent 高频改动，冲突面大）

---

## 3. 设计

### 3.1 组件一：`scripts/gate/verify_pr_dag.sh`（+ `check_pr_dag.sh` alias）

**职责**：校验 **PR-DAG 依赖图**与仓库实际的提交历史一致。

> **v2 重定向**（评审 D2）：v1 误把 `E2E_PR_DAG_MAPPING.md` 当作目标。
> 那是 **B-F8** 的产物，且已有可用验证器 `check_beta_e2e.sh`。
> BE5 / B-F5 真正指向的是下面这份文档。

#### 3.1.1 目标文档的真实结构（实测，2026-10-22）

`docs/releases/v3.8.0/plans/DEVELOPMENT_PLAN.md` §2「PR DAG（依赖执行图）」：

```
## 2. PR DAG（依赖执行图）

按依赖顺序排列，10 个 PR 必须依次合并：

PR-800  COM_QUERY AST Routing (L0)           ← 入口重构（先做）
   ↓
PR-810  ExecutionEngine → Router            ← 解耦层（依赖800）
   ↓
...
PR-900  ExecutionEngine 拆分清理             ← 收尾重构（依赖890）

### 2.1 PR DAG 实际执行状态

| PR | 规划阶段 | 实际状态 | 说明 |
|----|----------|----------|------|
| PR-800 (COM_QUERY AST Routing) | Phase 0 | ⚠️ 部分落地 | TransactionalFacade DEFERRED (#2603) |
| PR-810 (ExecutionEngine → Router) | Phase 0 | ❌ 未合并 | 无 Issue 追踪 |
| PR-830 (WAL + WriteBuffer) | Phase 1 | ✅ 已合并 | PR-830A~E + PR-830F |
| PR-840 (DML Transaction Interception) | Phase 1 | ❌ 未合并 | 无 Issue 追踪 |
...
```

三个可解析的结构：
1. **§2 依赖链**：ASCII 块，10 个 `PR-8xx` 节点 + `↓` 箭头 + `（依赖NNN）`
2. **§2.1 状态表**：`| PR | 规划阶段 | 实际状态 | 说明 |`，
   状态取值 `✅ 已合并` / `❌ 未合并` / `⚠️ 部分落地`

**§2.1 状态表已经过时** —— 实测（2026-10-22，base `origin/develop/v4.1.0`）：

| §2.1 声称 | git 实测 merge commit 数 | 结论 |
|---|---|---|
| PR-830 `✅ 已合并` | PR-830A~F 均有 | ✅ 一致 |
| PR-840 `❌ 未合并` | **16** | ❌ **矛盾** |
| PR-850 `❌ 未合并` | **8** | ❌ **矛盾** |
| PR-870 `❌ 未合并` | **8** | ❌ **矛盾** |
| PR-800/810/820/860/880/890/900 `❌ 未合并` | 0 | ✅ 一致 |

⇒ **这正是该 gate 该抓的缺陷**：状态列与 git 现实不符。
BE5 判据「PR-DAG 图表存在且与实际提交一致」中的「与实际提交一致」，
指的就是这个。

#### 3.1.2 判定逻辑

1. 定位 PR-DAG 文档（`--dag` 覆盖；默认
   `docs/releases/v3.8.0/plans/DEVELOPMENT_PLAN.md`，
   并在 `docs/releases/*/plans/DEVELOPMENT_PLAN.md` 中查找含
   `PR DAG` 小节者）
2. 解析 §2 依赖链 → 得到有序的 PR 节点列表 + 依赖边
3. 解析 §2.1 状态表 → 得到每个 PR 的**声明状态**
4. 对每个 PR，在 base ref 的 git 历史中判定**实际状态**：
   - **存在 merge commit** ⇒ 实际已合并
   - **不存在** ⇒ 实际未合并
5. **状态一致性判定**（核心）：
   - 声明 `✅ 已合并` 但 git 里没有 merge commit ⇒ **FAIL**（虚假通过）
   - 声明 `❌ 未合并` 但 git 里**有** merge commit ⇒ **WARN**（状态表过时）
     —— 注意方向：这是**文档过时**而非**门禁失败**，因为门禁判据
     （"图表与实际提交一致"）关心的是"已合并的 PR 是否真的合并了"，
     而非"文档是否及时更新"。**方向性很重要**：
     - 虚假通过（声明合并、实际没有）= 门禁失败 ⇒ FAIL
     - 状态过时（声明未合并、实际合并了）= 文档债 ⇒ WARN + 列出
6. **依赖链完整性判定**：
   - 链中每个 `（依赖NNN）` 指向的节点必须存在
   - 若 A 依赖 B，而 B 在链中出现**在 A 之后** ⇒ 依赖顺序倒置 ⇒ FAIL
     （这是**拓扑序**检查，与 merge 时间序无关 —— 评审 D4 的关键区分）
7. **输出**：每个 PR 的声明状态、实际状态、merge commit 数、判定

#### 3.1.3 PR 标识 → Gitea PR 号的权威映射（评审 D5）

`PR-830A` 这类**规划编号不是 Gitea PR 号**（`GET /pulls/830` → 404），
且仓库里两处散文映射**互相矛盾**：

| 来源 | PR-830C | PR-830D | PR-830E |
|---|---|---|---|
| `v3.8.0/beta/BETA_GATE_REPORT.md:120-124` | #2669 | #2670 | #2675 |
| git merge commit 标题实测 | **#2670** | **#2674** | **#2691** |

⇒ **散文映射不可信**。权威来源是 **git merge commit 标题本身**，它同时
包含规划编号和 Gitea 号：

```
753162046c Merge pull request 'fix(storage): PR-830F WAL lifecycle — ...' (#2756) from ...
56484fdb6f Merge pull request 'fix(storage): PR-830F WAL lifecycle — ...' (#2697) from ...
```

匹配式：`Merge pull request ... <规划编号> ... (#<Gitea号>)`。
一次 grep 同时得到两个 id，**无需维护任何映射表**。

#### 3.1.4 退出码

对齐 `GATE_CONDITIONS.md:258`「0 = PASS, 1 = FAIL」与 house style
（`check_anti_fabrication.sh` 的 `ERRORS`/`log_*` 计数）：

| exit | 含义 |
|---|---|
| 0 | PASS：无虚假通过、依赖链拓扑有序 |
| 1 | FAIL：存在虚假通过（声明合并实际未合并）、或依赖顺序倒置、或解析不到任何 PR |
| 2 | DRIFT / ENV_ERROR：PR-DAG 文档不存在、base ref 不可解析、或 §2/§2.1 之一缺失 |

> **DRIFT ≠ PASS**（ADR-006 P14 / `check_drift_not_pass.sh`）：
> exit 2 **不得**被记作 PASS。
> ⚠️ 评审 R1：`check_drift_not_pass.sh:45` 扫 `-eq 2` 模式。
> 本脚本**不使用** `cmd || true` 后读 `$?` 的模式（R5），
> 且 V14 会实跑该 gate 证明这一点。

#### 3.1.5 防假绿（ADR-008 P-Claim）

- 输出必须包含**实际 PR 数**、**声明已合并数**、**实际已合并数**、
  **虚假通过数**、**状态过时数** 五个数字
- 若解析出的 PR 数为 0 ⇒ FAIL（不允许空文档通过）
- 若 §2.1 状态表缺失 ⇒ exit 2（不得静默跳过）
- 输出 `source_agent` / `source_run` / `timestamp`（ADR-014 5 evidence fields），
  可用环境变量覆盖

#### 3.1.6 别名：`scripts/gate/check_pr_dag.sh`（评审 D13）

`DEVELOPMENT_PLAN.md:92` 与 `ALPHA_STAGE_REVIEW.md:81` 推荐的名字是
`check_pr_dag.sh`，与 `GATE_CONDITIONS.md` 引用的 `verify_pr_dag.sh`
**是两个不同的名字，指向同一个 gate**。

→ 提供 `check_pr_dag.sh` 作为 `verify_pr_dag.sh` 的薄 alias（exec 调用），
两者都被存在性 gate 覆盖。

#### 3.1.7 测试

`tests/integration/gate/gate_criterion_existence_test.rs`（**v1 实现已用此名**），
覆盖：脚本存在 / 可执行 / alias 一致 / 退出码语义 / 对空 DAG 判 FAIL /
对坏 base 判 exit 2 / 对虚假通过判 FAIL / 依赖倒置判 FAIL。

**必须在 `Cargo.toml` 注册 `[[test]]` 块**（评审 D8），
否则 `cargo test --test <name>` 报 "no test target named"，
且 `test_target_manifest_honesty_test.rs::no_orphan_test_file` 会 FAIL。
**禁止 `#[ignore]`**（评审 R2 / ADR-008 Policy 2 —— 一旦被 gate 引用
即成 gate test）。

### 3.2 组件二：`scripts/gate/check_gate_script_refs.sh`（根治件）

**职责**：扫描门禁判据文档中引用的脚本路径，验证其存在。

#### 3.2.1 扫描范围（评审 D7 修正）

v1 的 6 文档范围经实测**覆盖不足**：3 个 scope 文档（`RELEASE_GATE_CHECKLIST.md`、
`AI_COLLABORATION.md`、`RELEASE_LIFECYCLE.md`）在 v1 的正则下**各 0 命中**
（no-op），而全仓 `docs/` + `AGENTS.md` + `evidence/` + `.gitea/` 实测有
**约 90 个**不同的缺失脚本路径。

v2 的范围策略：**只扫「SSOT 判据文档」**（活文档，其路径必须可执行），
历史报告不扫（它们记录的是历史事实，路径指向当时的脚本）。

| 类别 | 文档 | 处理 |
|---|---|---|
| **SSOT 判据**（必须全过） | `docs/governance/GATE_CONDITIONS.md`、`docs/governance/RELEASE_GATE_CHECKLIST.md`、`docs/governance/STAGE_CONFIG.yaml`、`docs/governance/GATE_CI_CD.md`、`docs/governance/STAGE_CONTROL_FRAMEWORK.md`、`docs/governance/INDEX.md`、`docs/governance/RC_TO_GA_GATE_CHECKLIST.md`、`AGENTS.md` | 缺失 ⇒ FAIL |
| **历史记录**（不扫） | `docs/releases/v3.x.0/**`（含所有 `*_GATE_REPORT.md`）、`evidence/**` | 不纳入 —— 记录的是历史事实 |

> **为什么不直接扫全部**（评审 D7）：全扫会引入约 90 个**历史**缺失引用，
> 门禁将**永远红** —— 用一个永红门禁换掉一个假绿，净效果为负。
> 仓库自己已有正确的范式：committed baseline + `stale`/`new` 差分模型
> （`scripts/gate/check_docs_pointer_paths.sh` + `tests/baseline/doc_pointer_paths_releases_v4.1.0.json`）。
> v2 照抄该范式：已知的存量缺失进 baseline，**新增**缺失判 FAIL。

#### 3.2.2 提取规则（评审 D6 修正）

v1 正则 `scripts/gate/[A-Za-z0-9_./-]+\.sh` **看不到 #5450 的实例** ——
那 3 个缺失脚本在 `./scripts/` 下而非 `scripts/gate/` 下，v1 正则 0 命中。

v2 正则：`(\./)?scripts/[A-Za-z0-9_./-]+\.sh`
（覆盖 `scripts/` 与 `scripts/gate/` 两个层级）

路径归一化：去掉前导 `./`，按仓库根解析。

#### 3.2.3 退出码

| exit | 含义 |
|---|---|
| 0 | PASS：全部被引用脚本均存在，或缺失项全部已在 baseline 登记为存量债 |
| 1 | FAIL：存在**未登记在 baseline** 的缺失脚本（逐条列出） |
| 2 | ENV_ERROR：判据文档本身缺失 |

#### 3.2.4 为什么需要它（这是本设计的核心洞见）

#5450 / #5457 / #5458 是**同一个根因**的三个实例 —— 「判据表里的脚本路径
从未被验证过存在」。逐个手工补脚本只能治标；本脚本让**第四个同类缺陷
在出现的瞬间就被发现**，成本是「一次文本提取 + 若干次 `test -f`」。

#### 3.2.5 自豁免机制：删除（评审 D12）

v1 设计了「本脚本与 `verify_pr_dag.sh` 自动豁免」的机制。v2 **删除**它：

- 本 PR 若不改任何判据文档，新脚本就不会被引用 ⇒ **根本不需要豁免**
- v1 的豁免是「合并后手工移除」的 TODO ⇒ 在一个专门抓「永久失效引用」
  的门禁里留一个永久豁免口，是自相矛盾的
- 若将来判据文档真的引用了新脚本，baseline 机制会自然处理

#### 3.2.6 Baseline（评审 D7 / R6 要求的模式）

新增 `tests/baseline/gate_script_refs_baseline.json`：

```json
{
  "created_at": "2026-10-22T…Z",
  "generator": "scripts/gate/check_gate_script_refs.sh",
  "known_missing": {
    "./scripts/check_links.sh": "…为什么它缺失、谁来修…",
    "./scripts/check_format.sh": "…",
    "./scripts/generate_documentation_report.sh": "…"
  },
  "status": "PASS"
}
```

判据：缺失项**在** baseline ⇒ WARN（存量债）；缺失项**不在** baseline ⇒ FAIL（新增缺陷）。
baseline 里每条都必须带原因说明 —— 一个没有理由的 baseline 条目是
"把问题藏起来"，与本 issue 的目的相反。

> **同名 baseline 已有先例**：`tests/baseline/doc_pointer_paths_releases_v4.1.0.json`
> 记录了 `scripts/gate/verify_pr_dag.sh` 为 `planned_artifact_never_committed`。
> 本 PR 落地后该条目会变为 resolved（`check_docs_pointer_paths.sh` 会提示
> `--update`），见 §5 V12。

### 3.3 组件三：#5458 的三处订正

**(a) `AGENTS.md:120` 路径订正** ✅（v1 实现已完成）

把 `PR body 必填 5 类文档 (PR_TEMPLATE.md)` 改为指向真实路径
`.gitea/pull_request_template.md`。

**(b) 模板路径去版本硬编码**（评审 D11 修正）

模板内 `v3.8.0` 共出现 **10 次**（行 4/21/22/23/24/25/49/77/79/87），
分属 **4 种语义**。只改其中 5 行（21–25 的 `PR-XXX_*` 族）：

| 行 | 内容 | 处理 |
|---|---|---|
| 21–25 | `docs/releases/v3.8.0/PR-XXX_*.md`（5 类文档路径） | **改** → `docs/releases/<VERSION>/PR-XXX_*.md` |
| 4 | `生效: v3.8.0 起所有 PR 必填` | 不改（版本声明）|
| 49 | `artifacts/gate/v3.8.0/...` | 不改（产物目录）|
| 77 | `artifacts/gate/v3.8.0/evidence.json` | 不改（产物目录）|
| 79 | `bash scripts/gate/audit_testing.sh v3.8.0 <stage> <out>` | 不改（CLI 参数）|
| 87 | `` 从 `develop/v3.8.0` 切出 `` | 不改（分支名）|

⚠️ **`<VERSION>` 是散文占位符，不是可解析路径**。仓库的
`check_docs_pointer_paths.sh` 对新出现的断裂指针会 FAIL，但其默认 scope
是 `docs/releases/v4.1.0`，`.gitea/` **不在** scope 内 ⇒ 该占位符
不会被该 gate 捕获。不得声称它被覆盖。

> 注意：本项改动触及 `.gitea/pull_request_template.md`（2 个他人分支触及）。
> 改动面是 2 行的路径字符串，风险可接受。

**(c) 「5 类文档齐备」的机器判据**

在模板的 5-类文档清单各项加上**固定 HTML 标记**：

```markdown
- [ ] **SPEC** — ... <!-- section:SPEC -->
- [ ] **TEST_PLAN** — ... <!-- section:TEST_PLAN -->
- [ ] **TEST_DESIGN** — ... <!-- section:TEST_DESIGN -->
- [ ] **REVIEW** — ... <!-- section:REVIEW -->
- [ ] **ACCEPTANCE** — ... <!-- section:ACCEPTANCE -->
```

并新增一个检查脚本 `scripts/gate/check_pr_template_sections.sh`：
对给定 PR 正文（从 stdin 或 `--pr-body <file>` 读）检查 5 个标记是否齐全。

- exit 0：5 个标记齐全
- exit 1：缺任一标记（逐条列出缺哪个）
- exit 2：PR 正文为空 / 无法读取

**为什么用 HTML 注释而不是标题**：HTML 注释在 GitHub/Gitea 渲染时**不可见**，
不污染 PR 正文的可读性；同时是稳定的机器可解析锚点。

**CI 接线**：**不在本 PR 范围内**。`.gitea/workflows/ci.yml` 被 5 个他人
分支高频改动，本 PR 触碰会显著提高冲突概率。脚本先落地并可本地实跑，
CI 接线留给单独的 issue（可在 issue 正文中注明该待办）。

### 3.4 组件四：CHECK 4 白名单

在 `scripts/gate/check_anti_fabrication.sh` 的 `allowed[]` 数组加入
`opencode-z440@gaoyuanyiyao.com`，并按 2026-10-08 为 `omp-z440` 加入时
的**同型注释风格**说明理由：

- Gitea user `opencode-z440`（id=13）在本仓有写权限
- 该身份是本仓既有的 agent 身份之一（`openspec/changes/v312-28-corpus-activation/`
  等处已有 `Owner: opencode-z440` 署名）
- 遗漏会让该身份的分支**在门禁上被拒**，这是门禁缺陷，不是策略信号 ——
  这个检查的目的是抓**未知**作者，不是挑出某个已知作者

**冲突面**：该文件被 3 个他人分支触及，但本改动**只加 1 个数组元素**，
git 三方合并的行级冲突面极小。

---

## 4. 并行隔离方案（ADR-014 D-02 / D-03）

| 维度 | 本 Agent | 说明 |
|---|---|---|
| Agent id | `opencode-z440` | 252 Gitea user id=13 |
| Worktree | `.worktrees/opencode-z440-20261022` | 符合 D-02 `{agent_id}-{date}` |
| Branch | `opencode/fix-5457-5458-gate-refs` | 符合 D-03 `opencode/` 前缀 |
| Base | `origin/develop/v4.1.0` @ `378f014529` | 与远端主线一致 |
| 身份 email | `opencode-z440@gaoyuanyiyao.com` | 需本 PR 加入 CHECK 4 白名单 |

### 4.1 文件面冲突热区（实测：168 个远端 fix/docs 分支）

| 文件 | 被触及分支数 | 本 PR 是否改 |
|---|---|---|
| `scripts/gate/verify_pr_dag.sh` | 0 | 新增 |
| `scripts/gate/check_gate_script_refs.sh` | 0 | 新增 |
| `scripts/gate/check_pr_template_sections.sh` | 0 | 新增 |
| `tests/integration/gate/verify_pr_dag_gate_test.rs` | 0 | 新增 |
| `scripts/gate/check_anti_fabrication.sh` | 3 | 改 1 行 |
| `AGENTS.md` | 3 | 改 1 行 |
| `.gitea/pull_request_template.md` | 2 | 改 2 行 |
| `docs/governance/GATE_CONDITIONS.md` | 2 | 不改（脚本落地后判据自然可执行） |
| `.gitea/workflows/ci.yml` | 5 | **不改** |
| `crates/mysql-server/src/lib.rs` | 7 | **不改** |
| `src/execution_engine.rs` | 5 | **不改** |

**新增文件全部零重叠**，是本设计与他人并行工作的隔离基础。

---

## 5. 验证计划

每个交付物都必须**实跑**并留下 stdout + exit code（AFP §6.1，
不得以文档 claim 当 PASS，ADR-001 G-08）。

| # | 验证项 | 命令 | 通过标准 |
|---|---|---|---|
| V1 | 新脚本可执行 | `test -x scripts/gate/verify_pr_dag.sh` | exit 0 |
| V2 | verify_pr_dag 正常运行 | `bash scripts/gate/verify_pr_dag.sh` | exit 0，stdout 含 5 个计数 |
| V3 | verify_pr_dag 对空 DAG 判 FAIL | 构造空 DAG 临时文件 | exit 1 |
| V4 | verify_pr_dag 对坏 base 判 DRIFT | `--base refs/heads/nonexistent` | exit 2 |
| V5 | 存在性自检能抓出缺失脚本 | 构造含假路径的临时判据文档 | exit 1 且列出该路径 |
| V6 | 存在性自检对真实仓库 | `bash scripts/gate/check_gate_script_refs.sh` | exit 0（或仅 baseline 内 WARN）|
| V7 | PR 模板章节判据 | 缺标记的正文 → exit 1；齐全 → exit 0 | 双向 |
| V8 | 新增 Rust 测试 | `cargo test --test gate_criterion_existence_test --all-features` | 全部 PASS，0 failed，**0 ignored** |
| V9 | 格式 | `cargo fmt --check --all` | exit 0 |
| V10 | 静态检查 | `cargo clippy --all-features -- -D warnings` | 0 warnings |
| V11 | CHECK 4 通过 | `bash scripts/gate/check_anti_fabrication.sh` | CHECK 4 PASS |
| V12 | 文档指针路径 | `bash scripts/gate/check_docs_pointer_paths.sh` | exit 0（`verify_pr_dag.sh` 的 baseline 条目会提示已 resolved）|
| V13 | 完整回归 | `cargo test --all-features --no-fail-fast` | 无新增失败 |
| **V14** | **P14 drift 不被吞**（评审 R1/R5）| `bash scripts/gate/check_drift_not_pass.sh` | 新脚本**不**出现在 findings 里 |
| **V15** | **P16 gate test 完整性**（评审 R2）| `bash scripts/gate/check_gate_test_integrity.sh` | exit 0，新测试文件无 `#[ignore]` |
| **V16** | **test target lint 预算**（评审 R7）| `bash scripts/gate/check_test_target_lint_budget.sh` | exit 0 |
| **V17** | **P11 gate 自检**（评审 R4）| `bash scripts/gate/check_gate_self_verification.sh` | 新脚本 header 被识别（PASS 或 WARN，不能因新脚本而 FAIL）|
| **V18** | **孤儿测试文件**（评审 D8）| `cargo test --test test_target_manifest_honesty_test --all-features` | `no_orphan_test_file` PASS |
| **V19** | **既有 gate 诚实性**（评审 R5）| `cargo test --test gate_honesty_test --all-features` | 无新增失败 |

> V13 / V14 / V16 若因**仓库既有失败**而红，**如实报告**并附基线对比
> （在 base commit `378f014529` 上跑同一命令对比），
> 不得把既有失败算作本 PR 引入，也不得把既有失败隐瞒为 PASS。
>
> **已知基线状态（2026-10-22 实测）**：
> - `check_anti_fabrication.sh` 在 base 上即 **FAIL（7 errors）**
>   （V312-24 artifacts + `fault_coordination` / `sqlrustgo-storage` 编译）——
>   与本 PR 无关，是他人 batch 引入的存量问题
> - `check_drift_not_pass.sh` 在 base 上即 **FAIL（4 个 anti-pattern）**
>   （`check_p21_audit_log.sh` / `check_p13_soak_test.sh` / `check_p35_simd.sh` ×2）——
>   同样与本 PR 无关；**关键是新脚本不在该列表里**

### 5.1 门禁阻塞说明（ADR-001 G-09 / ADR-014 D-05）

按 G-09 与 D-05，**任一 Hard Gate FAIL 时禁止 push 到 252**。

实测 base `378f014529` 上 AFP 与 P14 均为 FAIL。因此本 PR 落地后
**不能直接 push**，需先由「发现 Gate FAIL 的 Agent 主导修复」
（`AI_COLLABORATION.md` §5.5.4）。

本 PR 的立场：**如实报告，不隐瞒**。
- 本 PR 不引入任何新的 Gate 失败（V14 证明新脚本不在 P14 findings 里）
- 存量失败的根因在他人 batch，本 PR 不越界修复
- 由 User 决定：(a) 授权在已知存量 FAIL 下 push 并在 PR body 中显式声明，
  或 (b) 先由其它 Agent 修掉存量 FAIL 再合本 PR

---

## 6. 证据要求

每个 commit 的 message 与 PR body 必须携带 ADR-014 的 5 evidence fields：

```
source_agent: opencode-z440
source_run:   <run-id>
timestamp:    <ISO8601 UTC>
evidence_hash: sha256:<hash of the gate stdout>
conflict_resolution: <与谁冲突、如何解决>
```

引用他人 claim 时须注明 `source agent + timestamp + evidence_hash`，
引用链每多一跳置信度降一级（AFP §5.4 / ADR-014 D-06）。

---

## 7. 风险

| 风险 | 缓解 |
|---|---|
| 新脚本自身被存在性自检判为缺失 | 脚本内显式豁免清单 + 原因注释；PR 合并后移除豁免 |
| `check_anti_fabrication.sh` 合并冲突 | 只加 1 个数组元素；冲突时以「先合入 develop 的一方」为准（ADR-014 §5.5.4） |
| `.gitea/pull_request_template.md` 合并冲突 | 只改 2 行路径字符串 |
| 既有测试失败被误算为本 PR 引入 | 合并前在 base 上跑同命令建立基线 |
| 与他人同改 `AGENTS.md` | 只改 1 行；冲突时保留双方内容 |
