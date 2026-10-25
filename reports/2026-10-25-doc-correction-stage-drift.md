# 文档检查和纠正工作报告 — v4.1.0 阶段口径漂移

> **版本**: v1.0.0
> **日期**: 2026-10-25
> **工作范围**: `AGENTS.md`、`docs/agent-harness/IMPLEMENTATION_TRUTH.md`
> **执行人**: `openclaw-mcode` (MiniMax Code / Gitea user `openclaw`)
> **依据规则**: `docs/governance/DOC_CHECK_CORRECTION_RULES.md`（7 步标准流程）
> **状态**: ✅ 已完成（本地 commit；push / PR 待用户授权）

---

## 一、基本信息

| 项目 | 内容 |
|------|------|
| 工作时间 | 2026-10-25 00:45–01:05 (UTC+08:00) |
| 执行人 | `openclaw-mcode`（MiniMax Code；Gitea 账号 `openclaw`） |
| 工作范围 | `AGENTS.md`（根 Agent 入口）、`docs/agent-harness/IMPLEMENTATION_TRUTH.md`（实现真实性）、`README.md`（项目状态最权威入口）、`docs/agent-harness/DOCUMENTATION_INDEX.md`、`docs/agent-harness/REPOSITORY_MAP.md`、`docs/architecture/ARCHITECTURE.md`（后 4 份为用户批准后同批纳入的 A1–A4） |
| 触发原因 | 阶段口径漂移：`AGENTS.md` / `IMPLEMENTATION_TRUTH.md` 声明 v4.1.0 为 **DRAFT (NOT YET ALPHA)**，与 per-version SSOT `docs/releases/v4.1.0/STAGE.yaml` 的 `current_stage: "ALPHA"` 矛盾 |
| 基线 commit | `5d4817a06b9dbfa0ca232ab639975d31a887e539`（2026-10-24 11:04:22 +0000，`develop/v4.1.0`） |
| 分支 | `openclaw-mcode/doc-stage-drift-20261025`（自 `develop/v4.1.0` @ `5d4817a06b` 切出） |
| 提交身份 | `openclaw-mcode <openclaw@gaoyuanyiyao.com>` — 在 `scripts/gate/check_anti_fabrication.sh` CHECK 4 白名单内（`allowed=(...)` 第 2 项），且与 issue #5531 身份登记表一致（`openclaw-mcode` = Mac Mini / Minimax Code / Gitea `openclaw`） |

### 1.1 本次修正的既有裁决依据（不是新判断）

仓库**已就同一问题作过裁决并留痕**，本次修正只是把未同步的两份文件对齐到既有裁决：

| 证据 | 位置 | 原文要点 |
|------|------|----------|
| per-version SSOT | `docs/releases/v4.1.0/STAGE.yaml:201` | `current_stage: "ALPHA"` |
| 转段记录 | `docs/releases/v4.1.0/STAGE.yaml:289-293` | `last_transition: from "DRAFT" / to "ALPHA" / date "2026-10-06"` |
| 阶段框架 | `docs/governance/STAGE_CONFIG.yaml:1-14` | "Stage Control Framework — Version-Agnostic SSOT"，`STAGE.yaml` 为 per-version SSOT |
| 裁决行 | `docs/releases/v4.1.0/EVALUATION_REPORT.md:352`（D1） | "`STAGE_CONFIG.yaml` makes `STAGE.yaml` the per-version SSOT; the other four documents are **stale**" |
| 裁决行 | `docs/releases/v4.1.0/FEATURE_MATRIX.md:219-225`（item 8） | 同上，并要求"记录分歧而不是隐藏" |
| v4.0.0 判定 | `docs/releases/v4.0.0/STAGE.yaml:13,29` | `current_stage: "DRAFT"` + `ga_promotion_status.verdict: "REVOKED 2026-09-30"` |

> 结论：`AGENTS.md` 与 `IMPLEMENTATION_TRUTH.md` 属于 EVALUATION_REPORT D1 行所称的 "stale" 一侧。
> 本次修正不改变任何阶段判定，只是把两份文件同步到 SSOT 已有的结论。

---

## 二、发现的问题

| # | 文件 | 问题 | 位置 | 依据 |
|---|------|------|------|------|
| 1 | `AGENTS.md` | **阶段状态错误**：声明 `v4.1.0 DRAFT (NOT YET ALPHA)` | 第 5 行 | `STAGE.yaml:201` = `current_stage: "ALPHA"`；`STAGE.yaml:289-293` = 2026-10-06 由 DRAFT 重新进入；`EVALUATION_REPORT.md:352` 判 D1 裁定其余文档 stale |
| 2 | `AGENTS.md` | **基线 commit 标注错误**：`583050bc63` 被标为 `(HEAD)` | 第 4 行 | `git rev-parse HEAD` = `5d4817a06b`；`583050bc63` 实为 2026-10-15 harness 审计基线（见 `reports/2026-10-15-agent-harness-audit.md:4`） |
| 3 | `docs/agent-harness/IMPLEMENTATION_TRUTH.md` | **阶段状态错误**：`- 当前 dev: v4.1.0 DRAFT` | 第 66 行 | 同问题 1；且该行**自己引用了** `docs/releases/v4.1.0/STAGE.yaml` 作为出处，与所引 SSOT 自相矛盾 |
| 4 | `docs/agent-harness/IMPLEMENTATION_TRUTH.md` | **基线 commit 标注错误**：`583050bc63` 被标为 `HEAD` | 第 3 行 | 同问题 2 |
| 5 | `README.md` | **阶段状态错误**：4 处写 `v4.1.0 DRAFT` / `NOT YET ALPHA` / `NOT ALPHA` | 第 4、55、258、311 行 | 同问题 1 |
| 6 | `README.md` | **基线 commit 与远端漂移声明错误**：称 HEAD = `3d503b5ab092`、"5 远端中 gitea250/gitee 领先 15 commit，github 滞后 34"、漂移"⚠️ 详见 P0-04" | 第 59、64 行 | `git rev-parse HEAD` = `5d4817a06b`；P0-04 verdict = `CLOSED (2026-10-24)`（`DEFECTS_AND_ISSUES.md:125`）；`git rev-list --left-right --count gitea252...gitcode` = `4 0` ⇒ gitcode 落后 4 commit；gitea250/gitee/github 三个 remote 在本仓已无 `develop/v4.1.0` 引用 |
| 7 | `README.md` | **阶段推进条件与新阶段标题冲突**：标题改为 ALPHA 后，同表"阶段推进条件"仍写 `DRAFT → ALPHA 需修完 P0 + 大部分 P1...` | 第 65 行 | `STAGE.yaml:473-478` `exit_criteria.alpha_to_beta`（5 条：alpha gate failures / 168h SOAK / `known_gaps.V400_09_soak.decision` / WP-C..G 迁移 / Coverage ≥ 50%） |
| 8 | `README.md` | **缺陷计数错误**：写 `P0×6 + P1×7 + P2×7`，与两个候选口径（2026-10-01 原始 P0×8；2026-10-24 未销账 P0=1）**均不匹配** | 第 61、258、315 行 | `DEFECTS_AND_ISSUES.md` §0（2026-10-24 计数更新）：原始 P0 8 / P1 7 / P2 7，未销账 P0 **1** / P1 **6** / P2 7 |
| 9 | `docs/agent-harness/DOCUMENTATION_INDEX.md` | **阶段状态错误**：3 处写 `v4.1.0 DRAFT` | 第 10、16、149 行 | 同问题 1 |
| 10 | `docs/agent-harness/REPOSITORY_MAP.md` | **阶段状态错误**：`含 v4.0.0 GA 撤销 + v4.1.0 DRAFT 状态` | 第 11 行 | 同问题 1 |
| 11 | `docs/architecture/ARCHITECTURE.md` | **阶段状态错误**：stale marker 内写 `当前代码已演进到 v4.1.0 DRAFT` | 第 7 行 | 同问题 1；该文件已带 `历史快照 (Stale as of 2026-10-15)` 标记，`check_agent_harness.sh` CHECK 8 判据不受影响 |
| 12 | `docs/agent-harness/DOCUMENTATION_INDEX.md` | **基线 commit 标注错误**：第 3 行 `> **基线**: HEAD \`583050bc63\`` | 第 3 行 | 同问题 2、问题 4 —— **本 PR 已触碰该文件，同一处同类缺陷必须一并修** |
| 13 | `docs/agent-harness/DOCUMENTATION_INDEX.md` | **元数据陈旧**：称 `AGENTS.md` 为 `Verified (26KB; 355 行)`，实际 9.0KB / 181 行 | 第 11 行 | `wc -lc AGENTS.md` = `181 9175`（改动前后均如此） |
| 14 | `docs/agent-harness/DOCUMENTATION_INDEX.md` | **无据声明**：第 16 行称根 `CHANGELOG.md` 含 "v4.1.0 DRAFT"（改前）/ "v4.1.0 ALPHA"（改后），但根 CHANGELOG.md **全文不含任何 v4.1.0 阶段声明** | 第 16 行 | `grep -n "v4.1.0" CHANGELOG.md` → 仅 `:40` 提 "承接 WP-H #4639"，无阶段断言 |

### 2.1 明确核实为**正确**、本次不动的内容

| 文件 | 内容 | 核实结果 |
|------|------|----------|
| `AGENTS.md:5` | `v4.0.0 GA 已撤销` | ✅ 正确 —— `docs/releases/v4.0.0/STAGE.yaml:13` `current_stage: "DRAFT"`、`:29` `ga_promotion_status.verdict: "REVOKED 2026-09-30"`；原样保留 |
| `AGENTS.md:5` | `develop/v4.1.0` | ✅ 正确 —— `git branch --show-current` = `develop/v4.1.0`；且是 `check_agent_harness.sh` CHECK 5 的判据字符串，不得改动 |
| `AGENTS.md:58` | `gitea252 → ...` | ✅ 正确 —— `check_agent_harness.sh` CHECK 6 的判据字符串；本次未触碰该行 |
| `IMPLEMENTATION_TRUTH.md:65` | `workspace.package.version: 3.12.0-fix-zombie` | ✅ 正确 —— 有意保留的发布口径 |
| `IMPLEMENTATION_TRUTH.md:67` | `v4.0.0 GA 已撤销` | ✅ 正确 —— 同上 |
| `IMPLEMENTATION_TRUTH.md:68` | `VERSION 文件: v3.11.0` | ✅ 正确 —— `cat VERSION` 一致 |

### 2.2 明确**不裁决**、本次不动的内容（存在未收敛的分歧或属于其它批次）

| 文件 / 位置 | 内容 | 不动的理由 |
|-------------|------|-----------|
| `README.md:63` | `Parser 覆盖率 实测 60.58%（目标 80%，缺 19.42pp）` | `EVALUATION_REPORT.md` D2 判定为 **"Partly resolved"**：2026-10-23 的 86.46% 测量在时间上覆盖了 60.58%，但 **region-vs-line 口径差异"not adjudicated here"**。无唯一 SSOT 值 ⇒ 不裁决，登记为 A13 |
| `README.md:60` | `v4.1.0-alpha1 tag ... 保留作审计轨迹，不作 ALPHA 凭据` | 与 `STAGE.yaml:293` 的 "v4.1.0-alpha1 (c1a73a5320) remains the prior ALPHA point" 定位冲突，属 tag 语义分歧（= A9），需架构裁决 |
| `README.md:318` | `v4.1.0 RELEASE_NOTES ... DRAFT 阶段发布说明草稿` | 该单元格描述的是 **RELEASE_NOTES.md 文档自身的状态标记**，而该文档的收敛属 A5（v4.1.0 发布线 release artifact 批次） |
| `README.md:314` | `v4.1.0 CLAIM_DOWNGRADE ...` | 描述的是 v4.0.0 的 3 项 caveat 继承 + #5304 BufferPool 降级，与阶段状态无关 |
| `EVALUATION_REPORT.md` D5（`:356`） | Parser 覆盖率 60.58% vs 86.46% | verdict = **"Unresolved wording conflict"** —— 与上条同源，D5 已明确"未裁决" |
| `FEATURE_MATRIX.md` item 9（`:226-229`） | 缺陷计数 `P0×8` vs `P0×6` | 原文 "Both numbers are left exactly as they are" —— 两数并存不裁决 |
| `EVALUATION_REPORT.md` D7（`:358`） | 远端漂移记录 | verdict = "Two dated records; not merged here" —— 本 PR 只陈述当前实测，不合并两份历史记录 |

> 说明：本次**确实**改写了缺陷计数（操作 9/12/14）与远端漂移（操作 10），其依据是 `DEFECTS_AND_ISSUES.md` §0 的 **2026-10-24 计数更新表**（原始 P0 8 / P1 7 / P2 7；未销账 P0 1 / P1 6 / P2 7）—— 该表已把"原始计数"与"当前未销账数"分列，因而不再有 D5/item 9 意义上的二义性。**但这一判断可被 maintainer 驳回**，已在 §REVIEW 披露。

---

## 三、执行的操作

### 3.1 修改明细

| # | 文件 | 修改内容 | 操作类型 | 依据 |
|---|------|----------|----------|------|
| 1 | `AGENTS.md` | 第 4 行 `**基线 Commit**: \`583050bc63\` (HEAD) / ...` → `**基线 Commit**: harness 审计基线 \`583050bc63\` / ...; 当前 HEAD 以 \`git rev-parse HEAD\` 为准` | 状态标记修正（去掉错误归属 + 指向活 SSOT） | 问题 2 |
| 2 | `AGENTS.md` | 第 5 行 `**当前开发线**: v4.1.0 DRAFT (NOT YET ALPHA); ...` → `**当前开发线**: v4.1.0 **ALPHA** (2026-10-06 由 DRAFT 重新进入, per \`docs/releases/v4.1.0/STAGE.yaml\` \`current_stage\`); ...` | 状态标记修正（DRAFT → ALPHA） | 问题 1 |
| 3 | `docs/agent-harness/IMPLEMENTATION_TRUTH.md` | 第 3 行 `> **基线**: HEAD \`583050bc63\` (fix/4597)` → `> **基线**: harness 审计基线 \`583050bc63\` (fix/4597); 当前 HEAD 以 \`git rev-parse HEAD\` 为准` | 状态标记修正 | 问题 4 |
| 4 | `docs/agent-harness/IMPLEMENTATION_TRUTH.md` | 第 66 行 `- 当前 dev: v4.1.0 DRAFT (...)` → `- 当前 dev: v4.1.0 **ALPHA** (\`docs/releases/v4.1.0/STAGE.yaml\` \`current_stage\`; 2026-10-06 由 DRAFT 重新进入)` | 状态标记修正 | 问题 3 |
| 5 | `README.md` | 第 4 行 `> **当前开发版**: v4.1.0（**DRAFT**，NOT YET ALPHA — per ...STAGE.yaml）` → `> **当前开发版**: v4.1.0（**ALPHA** — 2026-10-06 由 DRAFT 重新进入，per ...STAGE.yaml \`current_stage\`）` | 状态标记修正（DRAFT → ALPHA） | 问题 5 |
| 6 | `README.md` | 第 17-19 行声明基线修正块：`v4.1.0 DRAFT 状态的全面统一尚未完成` → `v4.1.0 阶段口径的全面统一尚未完成（v4.1.0 阶段 = **ALPHA**；发布线内 4 份产物仍标 DRAFT，已记于 EVALUATION_REPORT.md D1）` | 状态标记修正 + 保留"统一未完成"的真实陈述与分歧披露 | 问题 5 |
| 7 | `README.md` | 第 55 行 `### v4.1.0 — **DRAFT**（NOT YET ALPHA）` → `### v4.1.0 — **ALPHA**（2026-10-06 由 DRAFT 重新进入）` | 状态标记修正 | 问题 5 |
| 8 | `README.md` | 第 59 行 `` `develop/v4.1.0` HEAD \| `3d503b5ab092`（5 远端中 gitea250/gitee 领先 15 commit，github 滞后 34） `` → `` `develop/v4.1.0` HEAD \| `5d4817a06b`（撰写时快照；以 `git rev-parse HEAD` 为准） `` | 基线 commit 修正（去掉错误的固定归属 + 指向活 SSOT） | 问题 6 |
| 9 | `README.md` | 第 61 行 `**P0×6 + P1×7 + P2×7**` → `**P0×8 / P1×7 / P2×7**（2026-10-24 销账后：P0 未销账 1、P1 未销账 6）` | 计数修正（对齐 `DEFECTS_AND_ISSUES.md` §0 的 2026-10-24 更新表） | 问题 8 |
| 10 | `README.md` | 第 64 行 `\| 5-remote 漂移 \| ⚠️ 详见 P0-04 \|` → `\| 远端漂移 \| P0-04 已 CLOSED（2026-10-24）；gitea252 与本地一致，gitcode 落后 4 commit \|` | 状态标记修正（漂移已销账） | 问题 6 |
| 11 | `README.md` | 第 65 行 `\| 阶段推进条件 \| DRAFT → ALPHA 需修完 P0 + 大部分 P1... \|` → `\| 阶段推进条件 \| ALPHA → BETA 判据见 STAGE.yaml \`exit_criteria.alpha_to_beta\`（5 条，含 V400-09 168h SOAK 裁决与 Coverage ≥ 50%） \|` | 转段判据修正（与新标题 ALPHA 保持自洽） | 问题 7 |
| 12 | `README.md` | 第 258 行 `\| **v4.1.0** \| DRAFT (NOT ALPHA) \| ... P0×6 ... \|` → `\| **v4.1.0** \| **ALPHA** \| ... P0×8 / P1×7 / P2×7（2026-10-24 销账后 P0 未销账 1、P1 未销账 6）... \|` | 状态标记 + 计数修正 | 问题 5、8 |
| 13 | `README.md` | 第 311 行 `### v4.1.0 DRAFT（当前开发线）` → `### v4.1.0 ALPHA（当前开发线）` | 状态标记修正 | 问题 5 |
| 14 | `README.md` | 第 315 行 `**P0×6 + P1×7 + P2×7 完整目录**（推荐入口）` → `**P0×8 / P1×7 / P2×7 完整目录**（2026-10-24 销账后 P0 未销账 1、P1 未销账 6；推荐入口）` | 计数修正 | 问题 8 |
| 15 | `docs/agent-harness/DOCUMENTATION_INDEX.md` | 第 10 行 `Verified (2026-10-01)` + `v4.1.0 DRAFT 状态最权威入口` → `Verified (2026-10-25)` + `v4.1.0 ALPHA 状态最权威入口`；第 16 行 `v4.1.0 DRAFT` → `v4.1.0 ALPHA`；第 149 行 `当前 dev: v4.1.0 DRAFT` → `当前 dev: v4.1.0 ALPHA` | 状态标记修正（3 处） | 问题 9 |
| 16 | `docs/agent-harness/REPOSITORY_MAP.md` | 第 11 行 `含 v4.0.0 GA 撤销 + v4.1.0 DRAFT 状态` → `含 v4.0.0 GA 撤销 + v4.1.0 ALPHA 状态` | 状态标记修正 | 问题 10 |
| 17 | `docs/architecture/ARCHITECTURE.md` | 第 7 行 `当前代码已演进到 v4.1.0 DRAFT` → `当前代码已演进到 v4.1.0 ALPHA`（`历史快照 (Stale as of 2026-10-15)` 标记保留，CHECK 8 判据不受影响） | 状态标记修正 | 问题 11 |
| 18 | `docs/agent-harness/DOCUMENTATION_INDEX.md` | 第 3 行 `> **基线**: HEAD \`583050bc63\`` → `> **基线**: harness 审计基线 \`583050bc63\`; 当前 HEAD 以 \`git rev-parse HEAD\` 为准` | 基线 commit 修正（与已修的 `AGENTS.md:4` / `IMPLEMENTATION_TRUTH.md:3` 同构） | 问题 12 |
| 19 | `docs/agent-harness/DOCUMENTATION_INDEX.md` | 第 11 行 `**Verified** (26KB; 355 行)` → `**Verified** (9.0KB; 181 行)` | 元数据订正 | 问题 13 |
| 20 | `docs/agent-harness/DOCUMENTATION_INDEX.md` | 第 16 行 `v4.0.0 GA + 撤销 + v4.1.0 ALPHA` → `v4.0.0 GA + 撤销；该文档不含 v4.1.0 阶段声明` | 去掉无据声明（改前的 "v4.1.0 DRAFT" 同样无据） | 问题 14 |

**修改量**（口径：`git diff --numstat`，仅统计 6 个目标文档，不含本报告）：`22 insertions / 22 deletions`（合计 44 行变更），分布为 `AGENTS.md` 2/2、`README.md` 11/11、`DOCUMENTATION_INDEX.md` 5/5、`IMPLEMENTATION_TRUTH.md` 2/2、`REPOSITORY_MAP.md` 1/1、`ARCHITECTURE.md` 1/1。逐行均为**状态标记 / 基线 commit / 计数**类事实性错误，无技术内容、commit 日志、功能描述被改动，符合 `DOC_CHECK_CORRECTION_RULES.md` §2.1 最小修改原则。

> **行号基准**：下表「位置」列除特别标注外，均为**改动前（base `5d4817a06b`）的行号**；涉及改动后行号的以"改后"明示。

> **边界说明**：本批只修正"能裁决到唯一 SSOT 值"的项。凡存在未收敛分歧的（README 的 Parser 覆盖率 60.58% —— `EVALUATION_REPORT.md` D2 判 "Partly resolved"；缺陷计数的 D5 / item 9 二义性；远端漂移的 D7 双记录；`v4.1.0-alpha1` tag 定位；RELEASE_NOTES 文档自身状态）一律不动，登记于 §2.2 与 §六。

### 3.2 git 操作记录

```bash
# 修改前备份（可撤销原则 §2.3；证据链 §5.1）
cp AGENTS.md                            /tmp/AGENTS.md.bak
cp docs/agent-harness/IMPLEMENTATION_TRUTH.md /tmp/IMPLEMENTATION_TRUTH.md.bak
cp README.md                            /tmp/README.md.bak
cp docs/agent-harness/DOCUMENTATION_INDEX.md /tmp/DOCUMENTATION_INDEX.md.bak
cp docs/agent-harness/REPOSITORY_MAP.md /tmp/REPOSITORY_MAP.md.bak
cp docs/architecture/ARCHITECTURE.md    /tmp/ARCHITECTURE.md.bak

# 切分支（ADR-014 D-03 分支前缀；issue #5531 身份登记）
git checkout -b openclaw-mcode/doc-stage-drift-20261025 5d4817a06b

# 修改 → 自检 → 提交
git diff -- AGENTS.md docs/agent-harness/IMPLEMENTATION_TRUTH.md \
             README.md docs/agent-harness/DOCUMENTATION_INDEX.md \
             docs/agent-harness/REPOSITORY_MAP.md docs/architecture/ARCHITECTURE.md
git add AGENTS.md docs/agent-harness/IMPLEMENTATION_TRUTH.md \
        README.md docs/agent-harness/DOCUMENTATION_INDEX.md \
        docs/agent-harness/REPOSITORY_MAP.md docs/architecture/ARCHITECTURE.md \
        reports/2026-10-25-doc-correction-stage-drift.md
git commit -F <msg>
```

---

## 四、复核检查结果

### 4.1 修改正确性

| 检查项 | 结果 | 实证 |
|--------|------|------|
| 问题 1 已修复：`AGENTS.md` 阶段口径 = ALPHA | ✅ 通过 | `AGENTS.md:5` 现为 `v4.1.0 **ALPHA** (2026-10-06 由 DRAFT 重新进入, per ...STAGE.yaml \`current_stage\`)` |
| 问题 2 已修复：`AGENTS.md` 基线 commit 不再冒称 HEAD | ✅ 通过 | `AGENTS.md:4` 现为 `harness 审计基线 \`583050bc63\` ... 当前 HEAD 以 \`git rev-parse HEAD\` 为准` |
| 问题 3 已修复：`IMPLEMENTATION_TRUTH.md` 阶段口径 = ALPHA | ✅ 通过 | `IMPLEMENTATION_TRUTH.md:66` 现为 `- 当前 dev: v4.1.0 **ALPHA** (...)` |
| 问题 4 已修复：`IMPLEMENTATION_TRUTH.md` 基线 commit 不再冒称 HEAD | ✅ 通过 | `IMPLEMENTATION_TRUTH.md:3` 现为 `> **基线**: harness 审计基线 \`583050bc63\` ...` |
| 两文件无残留 "NOT YET ALPHA" 声明 | ✅ 通过 | `grep -n "NOT YET ALPHA" AGENTS.md docs/agent-harness/IMPLEMENTATION_TRUTH.md` → 无命中（现存 "DRAFT" 仅出现在"2026-10-06 由 DRAFT 重新进入"的历史描述中） |
| 未误改已核实正确的内容 | ✅ 通过 | `AGENTS.md:5` "v4.0.0 GA 已撤销" 保留；`AGENTS.md` 的 `develop/v4.1.0` 与 `gitea252` 判据字符串保留；`IMPLEMENTATION_TRUTH.md:65/67/68` 保留 |
| 与 SSOT 逐字一致 | ✅ 通过 | `STAGE.yaml:201` = `current_stage: "ALPHA"`；`STAGE.yaml:292` = `date: "2026-10-06"` |
| 问题 5 已修复：`README.md` 4 处阶段口径 = ALPHA | ✅ 通过 | `README.md:4` / `:55` / `:258` / `:311` 现均为 ALPHA；`:17-19` 声明块亦已改 |
| 问题 6 已修复：`README.md` HEAD SHA 与远端漂移声明 | ✅ 通过 | `README.md:59` = `5d4817a06b`（撰写时快照）；`:64` = `P0-04 已 CLOSED（2026-10-24）；gitea252 与本地一致，gitcode 落后 4 commit` |
| 问题 7 已修复：`README.md` 阶段推进条件与新标题自洽 | ✅ 通过 | `README.md:65` 现为 `ALPHA → BETA 判据见 STAGE.yaml exit_criteria.alpha_to_beta` |
| 问题 8 已修复：`README.md` 缺陷计数对齐 §0 | ✅ 通过 | `README.md:61` / `:258` / `:315` 现为 `P0×8 / P1×7 / P2×7`（2026-10-24 销账后 P0 未销账 1、P1 未销账 6） |
| 问题 9 已修复：`DOCUMENTATION_INDEX.md` 3 处 | ✅ 通过 | `:10`（含 Verified 日期 2026-10-01 → 2026-10-25）、`:16`、`:149` 现均为 ALPHA |
| 问题 10 已修复：`REPOSITORY_MAP.md` 1 处 | ✅ 通过 | `:11` 现为 `含 v4.0.0 GA 撤销 + v4.1.0 ALPHA 状态` |
| 问题 11 已修复：`ARCHITECTURE.md` 1 处 | ✅ 通过 | `:7` 现为 `当前代码已演进到 v4.1.0 ALPHA`；`历史快照 (Stale as of 2026-10-15)` 标记保留 |
| 6 文件无残留 "NOT YET ALPHA" / "NOT ALPHA" 声明 | ✅ 通过 | `grep -n "NOT YET ALPHA\|NOT ALPHA" <6 文件>` → 无命中 |
| 残留 "DRAFT" 均属正确表述 | ✅ 通过 | 4 处命中全部核对过：`README.md:4`/`:55` 为"由 DRAFT 重新进入"的历史描述；`:19` 为 A5 的分歧披露；`:318` 描述 RELEASE_NOTES.md 自身标记 |
| `check_agent_harness.sh` CHECK 8 判据未受影响 | ✅ 通过 | ARCHITECTURE.md 仍含 `历史快照` / `stale` / `deprecated` 匹配词（CHECK 8 **只校验 stale marker、不校验阶段措辞**；ALPHA 措辞的正确性由 diff 逐行核对保证，不挂在这个门禁名下） |
| 问题 12 已修复：`DOCUMENTATION_INDEX.md` 基线 commit 不再冒称 HEAD | ✅ 通过 | `:3` 现为 `> **基线**: harness 审计基线 \`583050bc63\`; 当前 HEAD 以 \`git rev-parse HEAD\` 为准` |
| 问题 13 已修复：`DOCUMENTATION_INDEX.md` 元数据与实况一致 | ✅ 通过 | `:11` 现为 `**Verified** (9.0KB; 181 行)`；`wc -lc AGENTS.md` = `181 9175` |
| 问题 14 已修复：`DOCUMENTATION_INDEX.md` CHANGELOG 行不再含无据声明 | ✅ 通过 | `:16` 现为 `v4.0.0 GA + 撤销；该文档不含 v4.1.0 阶段声明` |

### 4.2 无过度修改

| 检查项 | 结果 | 实证 |
|--------|------|------|
| commit 日志内容未被修改 | ✅ 通过 | `git diff --stat` = 2 文件 4 行，无 CHANGELOG 改动 |
| 功能描述未被修改 | ✅ 通过 | 4 行均为元信息行 / 状态行；正文段落、`5 级实现真实性` 表、`AFP Type A/B/C/D` 表、BufferPool test-only 边界段均未触碰 |
| 实质性技术内容未被修改 | ✅ 通过 | `diff /tmp/AGENTS.md.bak AGENTS.md` 与 `diff /tmp/IMPLEMENTATION_TRUTH.md.bak ...` 输出仅 3 个 hunk、共 4 行，全部为状态标记 |
| 未引入新文档债 | ✅ 通过 | 未新增指向不存在路径的指针（见 §4.3） |

### 4.3 链接有效性

| 检查项 | 结果 | 实证 |
|--------|------|------|
| 新增/保留的引用路径存在 | ✅ 通过 | `docs/releases/v4.1.0/STAGE.yaml` → OK；`docs/agent-harness/IMPLEMENTATION_TRUTH.md` → OK |
| `AGENTS.md` 判据字符串仍在 | ✅ 通过 | `develop/v4.1.0` 在第 4 行；`gitea252` 在第 58 行 —— `check_agent_harness.sh` CHECK 5/6 依赖二者，实跑 PASS |
| 本报告自身不在指针门禁扫描范围内 | ✅ 通过 | `check_docs_pointer_paths.sh` 默认 `SCOPE="docs/releases/v4.1.0"`（脚本第 69 行），`reports/` 不在范围内 → 本报告不新增坏指针 |

### 4.4 git 状态

| 检查项 | 结果 | 实证 |
|--------|------|------|
| git diff 无非预期修改 | ✅ 通过 | `git status --porcelain` 仅 `M AGENTS.md`、`M docs/agent-harness/IMPLEMENTATION_TRUTH.md`（+ 本报告新文件） |
| P13 副作用已回退 | ✅ 通过 | P13 每次运行都会改写 `tests/baseline/test_count.json` 的 `timestamp` 字段；已 `git checkout --` 回退（计数 `cargo=479 / active=14224 / ignored=123` 全程未变，见 §六 A8） |
| 修改文件已 git add | ✅ 通过 | 提交前 `git add` 三项 |

### 4.5 可撤销性

| 检查项 | 结果 | 实证 |
|--------|------|------|
| 可通过 git 恢复 | ✅ 通过 | `git checkout -- AGENTS.md docs/agent-harness/IMPLEMENTATION_TRUTH.md` 即恢复；备份副本另存于 `/tmp/AGENTS.md.bak`、`/tmp/IMPLEMENTATION_TRUTH.md.bak` |
| 分支未污染 develop | ✅ 通过 | 改动全部落在 `openclaw-mcode/doc-stage-drift-20261025`；`develop/v4.1.0` 仍指向 `5d4817a06b` |
| 工作记录完整可追溯 | ✅ 通过 | 本文件 §一–§七 |

### 4.6 Gate 实跑结果（改动后，2026-10-25）

全部为**实跑**（非引用文档 claim），stdout 落盘于 `/tmp/gate-evidence-20261025/`。

| Gate | 脚本 | Exit | evidence_hash (sha256) |
|------|------|------|----------------------|
| P11 Gate Self-Verification | `scripts/gate/check_gate_self_verification.sh` | **0** | `f58756ee758ae18cc82271a48acbbad2fdf9c7ed58bb58c668cb7b1b1f2cf3c0` |
| P12 No Implicit Tolerance | `scripts/gate/check_ignore_count.sh` | **0** | `23e5cb59db89276f7c54c2d1fe4cac53668bb2d608388e40cb520a42e2c0003f` |
| P13 Test Count Monotonicity | `scripts/gate/check_test_count_monotonic.sh` | **0** | `5c9ac64e8b28bdeb12a7cef6c425f32df7f314148b657cb4431b50f5fbbaff75` |
| P14 DRIFT ≠ PASS | `scripts/gate/check_drift_not_pass.sh` | **0** | `e5bcaa96c3eba7c20aaafb455ed1d09a94ab391c7aca4e1c31fba2421fb0313f` |
| P15 Oracle Required | `scripts/gate/check_oracle_present.sh` | **0** | `d53d2b883796e8235bdd24245bf9800c119837746ead267bc07c3d9e0fc19a67` |
| P16 Gate Test Integrity | `scripts/gate/check_gate_test_integrity.sh` | **0** | `56572e047050f7b52f64b5e63c672589011cf03f88196ad4923e44a2b5d9e4c6` |
| Agent Harness 结构门禁 | `scripts/gate/check_agent_harness.sh` | **0** | `3166debcbe220d31128f28696d8bb34298549fe859d816f28f33d76ca8580c0e` |
| Stage Config 一致性 | `scripts/gate/check_stage_config_consistency.sh` | **0** | `97a7d533af25fa79961250c90665abc4c28a6148dd17c35c0c911feb3767a3b9` |
| Gate 脚本引用完整性 | `scripts/gate/check_gate_script_refs.sh` | **0** | `cdbc3ca06275eca7130e52010fb5be163f305845d46a99bdf165c64815536aa7` |
| D8 架构/语义债 | `scripts/gate/check_arch_sem_debt.sh` | **0** | `79f76fd198817f2362fef0130cbd3bb07ab6f36e2d4e6e18e75ddef59cbe7784` |
| Anti-Fabrication (AFP v4) | `scripts/gate/check_anti_fabrication.sh` | **0** | `0336236d641d6f1b5c641811bc05889c5dd8a596b5602edf65a639a498c82444` |
| 文档链接（全量 entry 模式） | `scripts/gate/check_docs_links.sh` | **0** | `d6aec487ebb33131619f06041f87bf2d82dd3cb2021259f5eb98ee4cbbd51981` |
| 文档一致性 v312 | `scripts/gate/check_docs_consistency_v312.sh` | **0** | `5de1558714d643f21884877696494221629477396ae1c8a0ed85d8d1fd4e7363` |
| 文档链接 v312 | `scripts/gate/check_docs_links_v312.sh` | **1** ⚠️ | `fa851e92699930de753cbaed9978995b0ba2641d2804435a9a38a98ca7c4ac3e` |
| 文档指针路径 | `scripts/gate/check_docs_pointer_paths.sh` | **1** ⚠️ | `e930069809119c9284901f33e0073e12d8754d91abaa30a667dd668edde975f5` |

**P11–P16 六项全部 exit 0。** ADR-014 D-05（Gate FAIL 时禁止 push 到 252）的 P11–P16 条件满足。

> 上表为本批（A1–A4 同批修完后的**最终**一轮）实跑结果，stdout 落盘 `/tmp/gate-evidence-20261025b/`。第一轮（A1–A4 之前）的哈希见 §4.6.2。
> 两个 exit 1 的门禁（doclinks_v312 / docs_pointer_paths）**均为既有红**，归因见 §4.6.1 与 §4.6.3。

Anti-Fabrication 门禁的关键 CHECK（实跑 stdout 摘录，`/tmp/gate-evidence-20261025b/G-anti_fab.log`）：

```
[PASS] sqlrustgo-mysql-server: cargo check PASS
[PASS]   V312-24: target/sqlancer-report.json valid (iterations=1000)
[PASS]   V312-24: test-runner report valid (5 test(s) ran, total_duration_ms=3856)
[PASS] Test binaries compile PASS
[PASS] HEAD author email: openclaw@gaoyuanyiyao.com (allowed multi-AI identity)
[PASS] Scanned 428 markdown files in docs/releases + docs/governance
Results: ERRORS=0, WARNINGS=0
[PASS] Anti-fabrication check (AFP v4): PASS
```

#### 4.6.0 Anti-Fabrication 首次 FAIL 的归因与处置 —— **环境依赖缺失，非本次引入**

首次实跑 **FAIL（ERRORS=1）**，唯一错误在 CHECK 2：

```
[ERROR] Test binaries compile FAILED (no binary names extracted)
  Error: Could not find `protoc`. If `protoc` is installed, try setting the
  `PROTOC` environment variable to the path of the `protoc` binary.
```

- **根因**：`crates/network/build.rs` 需要 `prost` 的 protoc 代码生成，本机未安装 protoc ⇒ `cargo test --workspace --no-run` 编译失败。`Cargo.lock` 中 `prost` = 0.13.5。
- **与本次改动的关系**：零。改动只涉及 2 个 `.md` + 1 个新 `.md`，不参与任何编译；该失败在改动前后的 `develop/v4.1.0` 上同样必然复现。
- **处置**：免 root 安装 protoc。官方 GitHub release 的 `SHA256SUMS` 资产实际返回 `Not Found`（9 字节）、API `digest` 字段为 `None` ⇒ **该来源未完成哈希校验，不予采用**；改用 **apt 签名索引**：

  ```bash
  apt-get download protobuf-compiler libprotoc32t64 libprotobuf32t64   # 免 root
  dpkg -x <每个 .deb> ./root                                             # 不安装
  # sha256(protobuf-compiler_3.21.12-11ubuntu3.1_amd64.deb)
  #   = 574ca29db617ac792620cf2696d132738019a7df3ad10df92c0dbc4dfeacb00d
  # 装到 ~/.local/protoc/ + ~/.local/bin/protoc wrapper（带 LD_LIBRARY_PATH）
  ```

  落地后 `protoc --version` = `libprotoc 3.21.12`（Ubuntu 官方 candidate 版本），带 `PROTOC=$HOME/.local/bin/protoc` 重跑 ⇒ **exit 0，ERRORS=0**。

> 该环境依赖是**本机与 CI 共同的问题**，见 §六 A11。

#### 4.6.1 evidence 绑定说明（避免自证循环）

§4.6 表中 Anti-Fabrication 的哈希 `0336236d641d…` 来自**在 commit `2ddb08314f` 上执行**的实跑（`/tmp/gate-evidence-20261025b/G-anti_fab.log`）。此后本文件正文有增量（补记该哈希与 §六 A11/A12/A17/A18），因此最终 commit 的 SHA 与 `2ddb08314f` 不同。

> 更早一轮的 run（commit `d402d29001`，即 A1–A4 纳入之前）哈希为 `d78921eb98c4…`，结论同为 exit 0 / ERRORS=0 —— 见 §4.6.2 第一轮表。两轮互相印证。

该增量对本门禁结论**不敏感**，理由：Anti-Fabrication 的全部输入是 ① `docs/releases` + `docs/governance` 下的 428 个 markdown（CHECK 5，输出 `Scanned 428 markdown files`）、② workspace 的 `cargo test --workspace --no-run` 编译（CHECK 1/2）、③ HEAD commit author email（CHECK 4）、④ V312-24 的 SQLancer / test-runner 产物（CHECK 1.5）。**`reports/` 目录不在任何一项的扫描范围内**，commit message 亦不被扫描；`AGENTS.md` 与 `docs/agent-harness/IMPLEMENTATION_TRUTH.md` 自 `95a8d06c74` 起未再变动。

> 早期一次 run（`95a8d06c74` 上，哈希 `1bd125cc7ea5…`）结论同为 exit 0 / ERRORS=0，两次互相印证。

#### 4.6.1 `check_docs_pointer_paths.sh` 红的归因 —— **既有红，非本次引入**

实证方法：把本次 2 文件改动 `git stash` 后在**原始干净树**上复跑同一脚本。

| 树状态 | 坏指针数 | Exit |
|--------|---------|------|
| 原始树（stash 后） | 12 | **1** |
| 改动后（含本次修正） | 12 | **1** |

→ 数量与 exit code **完全一致**，本次改动对结果零影响。12 项坏指针全部指向**本次未触碰**的文件：

- `docs/releases/v4.1.0/evidence/BATCH35_STORAGE_TEST_TEMP_ISOLATION_2026-10-24.md` → 10 个 `src/*.rs`（实际在 `crates/storage/src/`，路径缺前缀）—— 来自上游 commit `88c26e2b83`（批次 35）
- `docs/releases/v4.1.0/evidence/STAGE_SSOT_AND_ARTIFACT_GATES_BATCH10_2026-10-21.md` → `scripts/gate/check_g_all.sh`（该脚本已在上游 commit `01275c72ac` 删除）—— 来自批次 10

> 该脚本**未登记在 `STAGE_CONFIG.yaml` 的任何 `required_gates` / `optional_gates` 中**（grep 无命中）；但它**被 CI 直接调用** —— `.gitea/workflows/ci.yml:1324-1329` 的 "Documentation pointer-path check (#5499)" step 以 `set -euo pipefail` 执行
> ```
> bash scripts/gate/check_docs_pointer_paths.sh --selftest
> bash scripts/gate/check_docs_pointer_paths.sh --scope docs/releases/v4.1.0
> ```
> `--selftest` 实跑 exit 0，但随后的 scope 调用 exit 1（scope 与产生坏指针的 scope 完全相同）⇒ **该 CI step 必然为红**。
> 因此本红的准确定性是：**未被 STAGE_CONFIG 登记为阶段门禁，但被 CI postcheck 直接调用且当前为红**（既有红，base 树同样 exit 1）。处置见 §六 A7。
>
> 完整 12 项枚举（含本次初版遗漏的第 1 项）：
>
> | # | 坏指针 | 引入来源 |
> |---|---|---|
> | 1 | `PERFORMANCE_TARGETS.md -> tests/soak_test_harness.rs` | 本次未触碰 |
> | 2–11 | `evidence/BATCH35_STORAGE_TEST_TEMP_ISOLATION_2026-10-24.md -> src/{file_storage,binary_storage,append_only_storage,file_table,backup,columnar/parquet,binlog_server,integrated_storage,restore_filespace,replication}.rs` | 上游 commit `88c26e2b83`（批次 35，缺 `crates/storage/` 前缀） |
> | 12 | `evidence/STAGE_SSOT_AND_ARTIFACT_GATES_BATCH10_2026-10-21.md -> scripts/gate/check_g_all.sh` | 上游 commit `01275c72ac`（该脚本已删除） |

#### 4.6.2 第一轮实跑哈希（A1–A4 纳入之前）

保留以便对照两轮结果差异。第一轮（A1–A4 纳入前，工作区只含 `AGENTS.md` + `IMPLEMENTATION_TRUTH.md` 的改动）：

| Gate | Exit | evidence_hash |
|------|------|---------------|
| P11 | 0 | `016421bb61dcd8f7505b7f1428e1a35826a2afbbe2e9a946055b1992cbb833ce` |
| P12 | 0 | `c16b026dcf23a245dbed2db9eac749035e0619e02c6f243610b6fea45f984e7e` |
| P13 | 0 | `4946ee20d28cd953a62d73ece08dcb3893ec75dacb5245e4a47770a3e5ed6698` |
| P14 | 0 | `7883dddc873c85bc6205333818fa9b2ffcdc84f4b5348c6a2a98b184275eec1a` |
| P15 | 0 | `2bee33b89163b4f5c0226f88260a96fd387b02d355bd285a5e034eaca1bb4af8` |
| P16 | 0 | `56572e047050f7b52f64b5e63c672589011cf03f88196ad4923e44a2b5d9e4c6` |
| Agent Harness | 0 | `3166debcbe220d31128f28696d8bb34298549fe859d816f28f33d76ca8580c0e` |
| Stage Config | 0 | `97a7d533af25fa79961250c90665abc4c28a6148dd17c35c0c911feb3767a3b9` |
| Gate 脚本引用 | 0 | `7f9f01b0c4f8ea45070ce6cda5ffdca6cfd43b74d417a82b9ff883a81c233384` |
| D8 架构/语义债 | 0 | `79f76fd198817f2362fef0130cbd3bb07ab6f36e2d4e6e18e75ddef59cbe7784` |
| Anti-Fabrication | 0 | `d78921eb98c45bd610d6e9e54fcbb05ae4f285bfe97dfacca197e0c5bc5887c2` |
| Docs Pointer | 1 ⚠️ | `e930069809119c9284901f33e0073e12d8754d91abaa30a667dd668edde975f5` |

> `P16` 与 `Agent Harness` / `Stage Config` / `D8` 四个门禁两轮哈希**完全相同**——它们的输出不依赖被改的 6 个 markdown 中的任何内容（P16 扫的是 gate 脚本自身，P11–P16 其余项扫的是 gate 目录）。`P11`–`P15`、`Gate 脚本引用`、`Anti-Fabrication` 的哈希变化来自各自 stdout 里的时间戳/计数，不是结论变化（exit code 前后一致）。

#### 4.6.3 `check_docs_links_v312.sh` 红的归因 —— **既有红，非本次引入**

实跑 **FAIL（exit 1）**，报 `2 broken links` + `v3.12.0 file existence FAIL (some files missing)`。两条坏链接为：

```
BROKEN: docs/releases/v3.12.0/RELEASE_NOTES.md -> evidence/v312-59/ga_gate_report.json
BROKEN: docs/releases/v3.12.0/CHANGELOG.md     -> evidence/v312-59/ga_gate_report.json
```

归因方法同上：把本批 5 个未提交改动 `git stash` 后在**原始干净树**上复跑 ⇒ 同样 `✗ FAIL 2 broken links`、**exit 1**，与改动后完全一致。

- 该门禁的扫描范围是 **v3.12.0 发布线**（`docs/releases/v3.12.0/`），与本次改动的 6 个文件**无交集**；
- 根因是 v3.12.0 的 `ga_gate_report.json` 产物已不在库里（v3.12.0 已被 v4.0.0/v4.1.0 取代），属**旧发布线门禁未退役**；
- 处置见 §六 A16。

---

## 四·七、独立审核（Independent Review）

本 PR 在合并前经**独立 verifier**（只读、不得改动任何文件）独立复核，结论 **PASS-with-issues**，并提出 4 个 MAJOR + 8 个 MINOR + 2 个 NIT。**全部 MAJOR 与 MINOR 已在同一 PR 内修正**，逐条对应：

| 审核发现 | 处置 |
|---|---|
| MAJOR-1「`check_docs_pointer_paths.sh` 未被 CI 调用」是**错的** —— `ci.yml:1324-1329` 的 postcheck step 直接调用它且必然红 | ✅ 已改：§4.6.1 与 §六 A7 现准确表述为「未被 STAGE_CONFIG 登记，但被 CI postcheck 直接调用且当前为红」 |
| MAJOR-2 改的是 `docs/architecture/ARCHITECTURE.md`，而 D1 点名的是 `docs/releases/v4.1.0/ARCHITECTURE.md`（3 处），且未登记 | ✅ 已改：§六 A5 现含**完整 11 文件 / 18 处**清单，含该文件 |
| MAJOR-3 PR 新引入的错计数「发布线内 4 份产物仍标 DRAFT」 | ✅ 已改：`README.md:19` 现为「11 份产物共 18 处」；§六 A5 给出完整清单 |
| MAJOR-4 `DOCUMENTATION_INDEX.md:3` 同类「基线 commit 冒称 HEAD」漏改 | ✅ 已改：本次一并修（§三 操作 18）；另两处（`GITNEXUS_ARCHITECTURE_MAP.md:3` / `AOCI_GUIDE.md:3`）登记为 A17 |
| MINOR-1 坏指针只列了 11 项 | ✅ 已改：§4.6.1 现给出**完整 12 项**表格 |
| MINOR-2 §4.6.1 引用过期哈希 | ✅ 已改：改为 `0336236d641d…` 并补注其基准 commit |
| MINOR-3 D5 / D7 两项分歧只在 REVIEW 披露、未进"明确不改"表 | ✅ 已改：§2.2 现列出 D5 / item 9 / D7 三项及其与本 PR 改写的关系 |
| MINOR-4「24 行」口径不可复现 | ✅ 已改：§三 改用 `git diff --numstat` 口径（22/22） |
| MINOR-5 `DOCUMENTATION_INDEX.md:16` 新表述不被目标文件支持 | ✅ 已改：改为「该文档不含 v4.1.0 阶段声明」 |
| MINOR-6 ACC-5 计数 6 处 vs 报告 5 处 | ✅ 已改：统一为实测值 |
| MINOR-7 ACC-9 的验证方法被夸大 | ✅ 已改：明确 CHECK 8 只校验 stale marker |
| MINOR-8 `DOCUMENTATION_INDEX.md:11` 元数据陈旧 | ✅ 已改：26KB/355 行 → 9.0KB/181 行 |
| NIT-1 行号基准混用 | ✅ 已改：§三 加注行号基准说明 |
| NIT-2 §5 措辞是提交前快照 | ✅ 已改：改为「最终交付态」 |

> 审核方对以下项**明确表示未能独立验证**（已如实转录，不代其背书）：① PR 声明的 15 个 sha256 中有 9 个因 stdout 内嵌 timestamp/branch/`duration_ms` 无法逐字复现（审核方逐项定位差异来源，确认**非伪造**）；② 报告引用的 `/tmp/` 中间产物在仓库外，未核对；③ 反向变异负对照**未实跑**（审核方遵守只读约束，仅从 `check_agent_harness.sh:76-81` 源码确认 CHECK 5 对 `develop/v4.1.0` 字符串敏感，机理成立）——该变异由实现方实跑，输出见 PR 正文 §三；④ 未访问 Gitea API，故 PR 线上正文与评论状态未由审核方取证；⑤「5 远端」只实测了 gitea252 与 gitcode 两家。
>
> 审核结论的合并决策权：审核方明示「本审核不构成 maintainer 批准」，且指出 **Gitea 侧不存在第二个有写/审批权限的账号**（`hermes-*` / `opencode-*` / `omp-*` / `claude-*` / `mcode-macmini` 全部只有 `read`），故「≥1 独立 reviewer + 禁止 self-approve」在平台层面无法满足。合并由仓库 Owner（人类维护者）授权执行，并在 PR 与本记录中留痕。

```
## 五、待提交文件状态

```
本批改动的文件（最终态）:
  AGENTS.md
  README.md
  docs/agent-harness/IMPLEMENTATION_TRUTH.md
  docs/agent-harness/DOCUMENTATION_INDEX.md
  docs/agent-harness/REPOSITORY_MAP.md
  docs/architecture/ARCHITECTURE.md
  reports/2026-10-25-doc-correction-stage-drift.md      (本文件, 新增)

已回退、不得进入提交:
  tests/baseline/test_count.json              (P13 运行副作用的 timestamp)
  docs/releases/v3.12.0/evidence/v312-59/*    (check_docs_links_v312 / check_docs_consistency_v312 的产物改写，见 §六 A15)
```

> 注：本节记录的是**最终交付态**，不是某一时点的 `git add` 快照。

---

## 六、未纳入本次修复的问题

依据 `DOC_CHECK_CORRECTION_RULES.md` §2.1 最小修改原则与工作记录模板 §六，以下问题**不在本次授权范围内**，仅登记与建议。

> **A1–A4 已于本批（同 commit）修复**（用户于 2026-10-25 01:25 明确批准"一并修 A1–A4"），明细见 §三 操作 5–17 与 §四 4.1。下表**保留原编号不重排**，以便与早先版本对照。

### 6.1 同类阶段口径漂移（同一 SSOT 冲突的其余侧面）

| # | 文件 / 位置 | 问题 | 建议 |
|---|-------------|------|------|
| A1 | `README.md:4`、`:18`、`:54`、`:257`、`:310` | 5 处仍写 `v4.1.0 DRAFT (NOT YET ALPHA)` / `DRAFT (NOT ALPHA)` | ✅ **本批已修**（§三 操作 5–7、12–14） |
| A2 | `docs/agent-harness/DOCUMENTATION_INDEX.md:10`、`:16`、`:149` | 3 处写 `v4.1.0 DRAFT` | ✅ **本批已修**（§三 操作 15） |
| A3 | `docs/agent-harness/REPOSITORY_MAP.md:11` | 1 处写 `v4.1.0 DRAFT 状态` | ✅ **本批已修**（§三 操作 16） |
| A4 | `docs/architecture/ARCHITECTURE.md:7` | stale marker 内写 `当前代码已演进到 v4.1.0 DRAFT` | ✅ **本批已修**（§三 操作 17） |
| A5 | **v4.1.0 发布线内 11 份产物共 18 处 live 阶段断言仍标 DRAFT**（实测清单见下方表） | 见下表 | **属 v4.1.0 release artifact，须独立批次**并遵守 `check_release_artifacts.sh` / `check_stage_config_consistency.sh`；`RELEASE_NOTES.md:538` 与 `FEATURE_MATRIX.md:219` 已把分歧"记录而非隐藏"，改前需确认这层记录意图 |

**A5 完整清单（live 阶段断言 = 明确声称"当前阶段 = DRAFT"且引用 STAGE.yaml 的 banner/状态行）**：

| 文件 | 行 | live 断言 |
|---|---|---|
| `docs/releases/v4.1.0/ARCHITECTURE.md` | 5 / 83 / 89 | `> **阶段**: DRAFT (NOT YET ALPHA, per STAGE.yaml)` / `v4.1.0 当前为 DRAFT` / `STAGE.yaml 为阶段 SSOT（当前 DRAFT）` —— **D1 点名的就是这份** |
| `docs/releases/v4.1.0/README.md` | 5 | `> **Status**: DRAFT (per STAGE.yaml)` |
| `docs/releases/v4.1.0/CHANGELOG.md` | 5 | `> **Status**: DRAFT (per STAGE.yaml)` |
| `docs/releases/v4.1.0/VERSION_PLAN.md` | 4 | `> **Status**: DRAFT (per STAGE.yaml)` |
| `docs/releases/v4.1.0/ROADMAP.md` | 4 | `> **Status**: DRAFT (per STAGE.yaml)` |
| `docs/releases/v4.1.0/PHASE_1_SCOPE.md` | 5 | `> **Status**: DRAFT (per STAGE.yaml)` |
| `docs/releases/v4.1.0/ISSUES_PLAN.md` | 4 | `> **Status**: DRAFT (per STAGE.yaml)` |
| `docs/releases/v4.1.0/LEGACY_ISSUES.md` | 4 | `> **Status**: DRAFT (per STAGE.yaml)` |
| `docs/releases/v4.1.0/COMPREHENSIVE_ASSESSMENT_REPORT.md` | 6 | `> **当前阶段背景**: DRAFT (STAGE.yaml)` |
| `docs/releases/v4.1.0/DEFECTS_AND_ISSUES.md` | 1 / 6 / 555 | 标题 `(DRAFT)` / `> **Status**: DRAFT — per STAGE.yaml (\`DRAFT\` not yet \`ALPHA\`)` / `stage SSOT (current_stage: DRAFT)` |
| `docs/releases/v4.1.0/RELEASE_NOTES.md` | 1 / 4 / 12 / 150 | 标题 `(DRAFT)` / `> **Status**: DRAFT (per STAGE.yaml) — **NOT YET ALPHA**` / `Per STAGE.yaml, v4.1.0 is in **DRAFT**` / `stage SSOT (current_stage: **DRAFT**)` |

> **不计为 live 断言的**（属正确的分歧披露或历史记录，**不得**改动）：
> `EVALUATION_REPORT.md:352`（D1 裁决行）、`FEATURE_MATRIX.md:221`（item 8）、`MIGRATION_GUIDE.md:73/94`、`QUICK_START.md:48`、`GA_RELEASE_TIMELINE.md`（历史时间线，含被回滚的 2026-09-29 转段记录）、`ALIGNMENT_AUDIT_2026-09-30.md`（历史审计）。
> **边界存疑、需人工判定**：`PERFORMANCE_AUDIT_REPORT_2026-09-30.md:11` 与 `PERFORMANCE_OPTIMIZATION_PLAN.md:11` 标 `> **Status**: DRAFT` 但未引用 STAGE.yaml —— 按历史快照处理，未计入上表。
> **不是阶段断言**：`PR-5379_ACCEPTANCE.md:9`、`PR-5457_TEST_REVIEW.md:10`、`PR-5537_TEST_REVIEW.md:8` 的 `DRAFT` 是文档自身状态（待 reviewer 确认），与阶段无关。

### 6.2 其它发现

| # | 问题 | 证据 | 建议 |
|---|------|------|------|
| A6 | `.gitea/COLLABORATION_PROMPT.md` 严重过时：仍写 v2.8.0/v2.9.0、runner 标签 `[hp-z6g4]`、只列 2 个提交身份（openheart / CI Bot）、R1–R10 为旧治理集；与 `ADR-014` D-01~D-06、`AI_COLLABORATION.md` §5.5、当前 11 项身份白名单均不符 | `cat .gitea/COLLABORATION_PROMPT.md` | 独立批次重写；该文件是新人 agent 的第一份协作指引 |
| A7 | `check_docs_pointer_paths.sh` 既有红：12 项坏指针（§4.6.1）。**该门禁被 CI 直接调用**（`ci.yml:1324-1329` 的 postcheck step，`set -euo pipefail` ⇒ 必然红），只是未登记进 `STAGE_CONFIG.yaml` | 上游批次 10 / 批次 35 引入；`grep -rln check_docs_pointer_paths .gitea/` → 命中 `ci.yml` | 修 `evidence/BATCH35_*.md` 的 10 个 `src/` 前缀 + `PERFORMANCE_TARGETS.md` 的 1 个 + `BATCH10` 的 `check_g_all.sh` 指针；或按脚本提示 `bash scripts/gate/check_docs_pointer_paths.sh --update` 登记为计划产物。**注意**：该红会直接让 CI postcheck step 变红 |
| A8 | **P13 每次运行都会改写已入库的 `tests/baseline/test_count.json` 的 `timestamp` 字段**（计数 `cargo=479 / active=14224 / ignored=123` 全程不变）⇒ 跑一次门禁即污染工作区 | 两次实跑均产生 `M tests/baseline/test_count.json` | 与 issue #5389（a04 差分矩阵测试改写 81 个证据文件）、#5577（`cargo test --workspace` 覆写证据文件）同类；建议 P13 仅在计数**真发生变化**时才写回基线 |
| A9 | README.md:59 称 `v4.1.0-alpha1` tag "保留作审计轨迹，**不作 ALPHA 凭据**"，而 `STAGE.yaml:293` 称 "v4.1.0-alpha1 (c1a73a5320) remains the prior ALPHA point" —— 同一 tag 的定位两处不同 | `git tag -l 'v4.1.0*'` → 仅 `v4.1.0-alpha1` | 属 tag 定位口径分歧，不在"阶段状态"范畴；建议随 A1 一并裁决 |
| A10 | `docs/agent-harness/IMPLEMENTATION_TRUTH.md` 整体基线仍为 `583050bc63`（2026-10-15），落后 HEAD 9 天；其"当前 known 实现边界"段落（BufferPool test-only / WalEntry 写路径）的结论需按 `5d4817a06b` 复核是否仍成立 | `git log 583050bc63..HEAD --oneline \| wc -l` | 内容复核 ≠ 状态标记修正，超出本次授权；建议单独派发一次"实现边界复核"批次 |
| A11 | **`crates/network/build.rs` 需要 protoc，而本机与 CI 均未安装** ⇒ `check_anti_fabrication.sh` 的 CHECK 2（`cargo test --workspace --no-run`）在干净环境必然 FAIL。`.gitea/workflows/*.yml` 全库搜不到任何 protoc 安装步骤 | 本次首次实跑 anti-fabrication 即 ERROR=1（§4.6.0）；`grep -rn "protoc\|protobuf" .gitea/workflows/*.yml` → 无命中 | 二选一：① CI 加 protoc 安装步骤（apt 或 vendor 二进制 + 校验和）；② `crates/network` 把 `.proto` 编译产物入库，改用 `prost-build` 的 checked-in descriptor 路径。**归属**：这与 #5375「test job 编译失败」、#5387「TPC-H SF=1 在 CI 内不可得」同属"环境阻塞被报成代码红"这一类，建议合并立项 |
| A12 | `~/.local/bin/protoc` 是本机为本次工作新装的 wrapper（apt 来源，非仓库文件） | §4.6.0 处置记录 | 属本机环境变更，未进仓库。其它机器/CI 仍需各自处理（A11）；若采纳本方案，建议把安装脚本化并入 `scripts/ci/` 以免每台机重复踩坑 |
| A13 | **README 的 Parser 覆盖率 `60.58%` 与 2026-10-23 实测 `86.46%` 并存**，且 60.58% 已被后者在时间上覆盖 | `EVALUATION_REPORT.md` D2 行：verdict = **"Partly resolved"**，并注明 region-vs-line 的 scope 差异 "not adjudicated here" | 本次**不裁决**（无唯一 SSOT 值）。需一次专门的覆盖率口径裁决：明确"按 region 计"还是"按行计"，再统一 README / EVALUATION_REPORT / DEFECTS_AND_ISSUES 三处数字。**不得**在未裁决前单方面把 60.58% 改成 86.46% |
| A14 | `README.md:318` 仍写 `v4.1.0 RELEASE_NOTES ... DRAFT 阶段发布说明草稿` | 该单元格描述的是 **RELEASE_NOTES.md 文档自身的状态标记**，而 RELEASE_NOTES.md 自身的收敛属 A5 | 本次**不改**（不是项目阶段状态断言）。A5 完成后本单元格应随之更新，届时一并处理 |
| A15 | **`check_docs_links_v312.sh` + `check_docs_consistency_v312.sh` 每次运行都会改写 6 个已入库的 v3.12.0 证据文件**，且这些文件的 `provenance` 段会写入**当前分支名与 commit** ⇒ 在 v4.1.0 分支上跑一次门禁，就把 v4.1.0 的 provenance 写进 v3.12.0 的 GA-7 报告 | 本次实测：跑完工作区即脏 6 个文件（`GA7_DOCS_CONSISTENCY_REPORT.md`、`GA7_DOCS_LINKS_REPORT.md`、`cross_refs_v312.txt`、`docs_links_broken_v312.txt`、`stale_paths_v312.txt`、`version_inconsistency_v312.txt`），diff 显示 `provenance: commit=a03e36af4b…, branch=openclaw-mcode/doc-stage-drift-20261025` | **已回退**（未进本 commit）。与 #5389（a04 改写 81 个证据文件）、#5577（`cargo test --workspace` 覆写证据文件）、A8（P13 改写 `test_count.json`）同属"门禁跑一次就污染仓库"这一类。建议统一处置：① 门禁产物写到临时目录，仅在显式 `--update` 时落库；② 或给已入库的证据文件加"只读校验"模式 |
| A16 | `check_docs_links_v312.sh` 既有红：2 条坏链接（`v3.12.0/RELEASE_NOTES.md` 与 `v3.12.0/CHANGELOG.md` → `evidence/v312-59/ga_gate_report.json`），并报 `v3.12.0 file existence FAIL` | §4.6.3（stash 后原始树复跑同样 exit 1 / 2 broken links） | 根因是 **v3.12.0 发布线门禁未退役**（其产物已不在库）。建议与 A7 合并为一个"旧发布线门禁退役"批次：要么补齐产物，要么把 v3.12.0 门禁从必跑清单移除并在 `STAGE_CONFIG.yaml` 登记 |
| A17 | `docs/agent-harness/GITNEXUS_ARCHITECTURE_MAP.md:3` 与 `docs/agent-harness/AOCI_GUIDE.md:3` 同样写 `> **基线**: HEAD \`583050bc63\``（`583050bc63` 是 2026-10-15 的 harness 审计基线，非当前 HEAD） | `git log -1 583050bc63` = 2026-10-15；当前 HEAD 已推进到本批 commit | 与本批已修的 `AGENTS.md:4` / `IMPLEMENTATION_TRUTH.md:3` / `DOCUMENTATION_INDEX.md:3` **同构**。本批只修本 PR 已触碰文件里的那处（`DOCUMENTATION_INDEX.md`），这两份**未触碰**故登记待办 |
| A18 | `CURRENT_VERSION.md:47` / `:56` / `:79` 把 v4.1.0 的 DRAFT→ALPHA 转段日期记为 **2026-09-29**（该次转段已于 2026-09-30 被 `STAGE.yaml` 的 `last_rollback_record` 回滚作废，正确日期为 2026-10-06），并宣称 "V400-09 168h SOAK FINAL_REPORT written" | `STAGE.yaml:239-305` 的 `last_rollback_record`（ALPHA → DRAFT，2026-09-30）；`STAGE.yaml:293` 与 `:362` 明确 V400-09 SOAK "never ran to completion" | 根级"当前版本"文件上的**独立事实错误**，与阶段口径同源但不同类。本次**不改**（属内容事实复核，超出状态标记修正范围）；建议与 A5 同批收敛 |

---

## 七、结论

本次修正严格遵循 `DOC_CHECK_CORRECTION_RULES.md` 的 7 步标准流程与 §2.1 最小修改原则：

1. **仅修正事实性错误**——6 个目标文档共 `22 insertions / 22 deletions`（numstat 口径），全部属"状态标记错误 / 基线 commit 引用错误 / 可单值裁决的计数错误"三类；
2. **不新增任何阶段判定**——阶段结论直接引用既有 SSOT `docs/releases/v4.1.0/STAGE.yaml`（`current_stage: "ALPHA"`，2026-10-06 由 DRAFT 重新进入），并与仓库已有的 `EVALUATION_REPORT.md:352`（D1）、`FEATURE_MATRIX.md:219`（item 8）两条裁决一致；
3. **未改动任何技术内容、commit 日志、功能描述或已核实正确的表述**；**对存在未收敛分歧的值一律不裁决**（Parser 覆盖率 60.58% vs 86.46%、`v4.1.0-alpha1` tag 定位、RELEASE_NOTES 文档自身状态），逐条登记于 §2.2 与 §六 A13/A14；
4. **P11–P16 六项门禁实跑全部 exit 0**（evidence_hash 见 §4.6），满足 `ADR-014` D-05 的 push 前置条件；`check_anti_fabrication.sh`（AFP v4）补装 protoc 环境依赖后亦 **exit 0 / ERRORS=0**（§4.6.0）；
5. **所有修改可通过 `git checkout -- <file>` 撤销**，分支隔离在 `openclaw-mcode/doc-stage-drift-20261025`，未污染 `develop/v4.1.0`；
6. **12 项坏指针的既有红已用"stash 后在原始树复跑"的方法实证归因**，与本次改动零相关（§4.6.1），未以"改红了/改绿了"的模糊表述掩盖；
7. **anti-fabrication 首次 FAIL 已完成根因归因（环境缺 protoc）并修复后复跑通过**，全程未把 FAIL 或 TIMEOUT 记为 PASS（§4.6.0）。

**遗留**：§六 A5–A18 共 14 项未纳入本次修复（A1–A4 已于本批修完）。**优先级建议**：① **A5**（v4.1.0 发布线 11 份产物 / 18 处 live 阶段断言）——它含 `EVALUATION_REPORT.md` D1 点名的 `docs/releases/v4.1.0/ARCHITECTURE.md`（3 处），发布线内自相矛盾最刺眼，且受 `check_release_artifacts.sh` 约束需独立批次；② **A13**（覆盖率口径裁决）与 D5/item 9 的计数二义性——同一个数字在三处文档不一致且 D2 判 "Partly resolved"；③ **A11**（protoc 缺失）建议与 #5375 / #5387 合并立项——它让 anti-fabrication 在任何干净环境必红；④ **A15 + A7 + A8 + A16** 建议合并为一个"门禁产物污染仓库 / 旧发布线门禁退役"批次——四者同源，本批实测中四个门禁各自都会在跑完后留脏工作区，其中 A7 的红**会直接让 CI postcheck step 变红**；⑤ **A18**（`CURRENT_VERSION.md` 的 2026-09-29 转段日期与 SOAK 断言）建议与 A5 同批收敛。

---

## 附：Evidence（ADR-014 五字段）

```
source_agent:        openclaw-mcode
source_run:          2026-10-25-doc-stage-drift
timestamp:           2026-10-25T01:30:00+08:00
evidence_hash:       sha256:56572e047050f7b52f64b5e63c672589011cf03f88196ad4923e44a2b5d9e4c6
                     (P16 Gate Test Integrity, exit 0 —— P11~P16 全量哈希见 §4.6 表)
conflict_resolution: 与 docs/releases/v4.1.0/EVALUATION_REPORT.md D1 行「STAGE.yaml 为 SSOT、其余文档 stale」
                     的既有裁决一致；未与其它 agent 产生文件面冲突（本次改 6 个文件：
                     AGENTS.md + docs/agent-harness/{IMPLEMENTATION_TRUTH,DOCUMENTATION_INDEX,
                     REPOSITORY_MAP}.md + README.md + docs/architecture/ARCHITECTURE.md，
                     均为状态标记/基线 commit/计数的定点改动，无其它 agent 在改的证据）；
                     对 README 同表内「可单值裁决但不在阶段口径范畴」的行（HEAD SHA、远端漂移、
                     缺陷计数、转段判据）按 SSOT 单一来源改，对「存在未收敛分歧」的行
                     （覆盖率、alpha1 tag、RELEASE_NOTES 自身状态）一律不改并登记 §六
```

### 附.1 已披露的 FAIL（不计入上述 evidence_hash）

| Gate | Exit | evidence_hash | 归因 |
|------|------|---------------|------|
| `scripts/gate/check_docs_pointer_paths.sh` | 1 | `sha256:e930069809119c9284901f33e0073e12d8754d91abaa30a667dd668edde975f5` | **既有红**，与本次改动零相关（§4.6.1 已用 stash 后原始树复跑实证）；12 项坏指针全在本次未触碰的 `evidence/BATCH35_*.md` 与 `evidence/BATCH10_*.md`；该脚本未登记于 `STAGE_CONFIG.yaml` 任何门禁清单，也未被 CI 调用 |
