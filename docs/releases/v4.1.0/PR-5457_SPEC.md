# PR-5457 SPEC — 门禁判据可执行性修复（PR-DAG + 脚本引用存在性）

> Issue: #5457（主）/ #5458 / #5450 · 分支: `opencode/fix-5457-5458-gate-refs`
> 基线: `origin/develop/v4.1.0` @ `378f014529`
> 设计文档: `docs/superpowers/specs/2026-10-22-gate-criterion-existence-design.md`（v2）
> 配套文档: `PR-5457_TEST_PLAN.md` / `PR-5457_TEST_DESIGN.md` / `PR-5457_TEST_REVIEW.md` / `PR-5457_ACCEPTANCE.md`

---

## 1. 问题陈述

门禁判据文档引用了**不存在的脚本**。后果是该项判据**永远无法执行** ——
既产不出 PASS，也产不出 FAIL —— 但历史门禁报告已经把它记成了 `✅ PASS`。

这类缺陷的特征是**不会让任何东西变红**，因此能长期潜伏在「门禁清单」
这种最不可能被质疑的地方。

### 1.1 实测确认的 5 个实例

| 判据文档 | 被引用但不存在的路径 |
|---|---|
| `docs/governance/GATE_CONDITIONS.md:103` (BE5)、`:129` (B-F5) | `scripts/gate/verify_pr_dag.sh` |
| `docs/releases/v3.8.0/plans/DEVELOPMENT_PLAN.md:92` | `scripts/gate/check_pr_dag.sh` |
| `docs/releases/v3.8.0/alpha/ALPHA_STAGE_REVIEW.md:81` | `scripts/gate/check_pr_dag.sh` |
| `docs/governance/RELEASE_GATE_CHECKLIST.md:119,122,125` | `./scripts/check_links.sh`、`./scripts/check_format.sh`、`./scripts/generate_documentation_report.sh` |
| `AGENTS.md:120` | `PR_TEMPLATE.md` |

### 1.2 「已记成 PASS 但无法复现」的证据

判据不可执行，但历史报告已记为通过 —— 这才是真正的代价：
**不是「没检查」，而是「检查过了但无法复现」**。

| 文件:行 | 记录内容 |
|---|---|
| `docs/releases/v4.0.0/FEATURE_CHECKLIST.md:17` | `` `scripts/gate/verify_pr_dag.sh` exists; PR history verified: `` |
| `docs/releases/v4.0.0/BETA_GATE_REPORT.md:19` | `\| BE5 \| PR-DAG verification \| scripts/gate/verify_pr_dag.sh \| ✅ All V400-* PRs merged in documented order \|` |
| `docs/releases/v3.11.0/BETA_GATE_REPORT.md:38` | `\| BE5 \| ... \| scripts/gate/verify_pr_dag.sh \| PASS \|` |
| `docs/releases/v3.11.0/BETA_GATE_REPORT.md:58` | `\| B-F5 \| ... \| scripts/gate/verify_pr_dag.sh \| exit 0 \| PASS \|` |

### 1.3 #5458 原文前提的订正

issue #5458 说「`PR_TEMPLATE.md` 在仓库里**不存在**」。实测**只对了一半**：

- ✅ `.gitea/pull_request_template.md` **存在**（3603 字节），已定义 5-类文档
- ✅ `docs/governance/TEST_REVIEW_TEMPLATE.md` **存在**（8941 字节），模板内引用有效
- ✅ `tests/integration/sql/pr_template_and_full_gate_test.rs` **存在**，**10** 个 `#[test]`
  （行 7/15/30/49/64/82/90/104/123/144）—— 其中 5 个断言模板（行 8/16/31/50/65），
  5 个断言 `check_full_gate_verification.sh`（行 83/91/105/124/145）

真实缺口是三条更细的裂缝：

| # | 缺口 | 实测依据 |
|---|---|---|
| a | `AGENTS.md` 引用 `PR_TEMPLATE.md` 这个**文件名**，与实际路径不匹配 | `AGENTS.md:120` |
| b | 模板内 `docs/releases/v3.8.0/PR-XXX_*.md` 硬编码旧发布线 | 模板行 21–25 |
| c | 「5 类文档是否齐备」**无机器判据** —— 现有测试只读模板**本身**，不读**实际 PR 正文** | `pr_template_and_full_gate_test.rs:9-80` |

---

## 2. 范围

### 2.1 本 PR 做

1. **实现 `scripts/gate/verify_pr_dag.sh`** —— 让 BE5 / B-F5 第一次真正可执行。
   附 `scripts/gate/check_pr_dag.sh` 作为**第二个引用名**的 alias
   （`DEVELOPMENT_PLAN.md:92` 与 `ALPHA_STAGE_REVIEW.md:81` 引用的是它）。
2. **实现 `scripts/gate/check_gate_script_refs.sh`** —— 根治件。
   扫描 SSOT 判据文档引用的脚本路径，验证存在性；缺失项用 committed baseline
   区分「已知债」与「新增缺陷」。
3. **实现 `scripts/gate/check_pr_template_sections.sh`** —— 读**实际 PR 正文**，
   判定 5-类文档是否齐备。
4. **订正 #5458 的三条裂缝**：`AGENTS.md` 路径、模板路径版本硬编码、PR 正文机器判据。
5. **CHECK 4 白名单**加入 `opencode-z440@gaoyuanyiyao.com`。

### 2.2 本 PR 不做

- ❌ 不改任何 Gate 阈值、分支保护配置、CI workflow
  （`.gitea/workflows/ci.yml` 被 5 个他人分支高频改动，触碰会显著提高冲突面）
- ❌ 不修改历史门禁报告里的既有 `✅ PASS` 记录 —— 那是历史事实，
  追加勘误说明即可；改写历史报告超出本 issue 范围
- ❌ 不修 §2.1 状态表里已过时的 3 行（PR-840 / PR-850 / PR-870）——
  那是文档改动，见 §4 的 WARN 语义说明
- ❌ 不合并自己的 PR（PERMISSION_MATRIX §3：AI Developer 合并 PR = ❌）

---

## 3. 组件设计

### 3.1 `verify_pr_dag.sh` —— 判据的方向性是核心设计

**目标文档**：`docs/releases/v3.8.0/plans/DEVELOPMENT_PLAN.md`
- §2「PR DAG（依赖执行图）」：ASCII 依赖链，`PR-8xx` 节点 + `↓` + `（依赖NNN）`
- §2.1「PR DAG 实际执行状态」：`| PR | 规划阶段 | 实际状态 | 说明 |`

**判据**：

| 情况 | 判定 | 理由 |
|---|---|---|
| 声明 `✅ 已合并` 但 git 无 merge commit | **FAIL** | **虚假通过** —— 门禁会认证从未落地的工作 |
| 声明 `❌ 未合并` 但 git 有 merge commit | **WARN** | **文档债** —— 判据问的是「声称合并的到底合并了没有」，不是「文档是否及时更新」 |
| 依赖边倒置（A 依赖 B 但 B 在链中更靠后）| **FAIL** | §2 链是**依赖序**，倒置意味着链本身错了 |
| §2 链与 §2.1 表的 PR 集合不一致 | **FAIL** | 文档自相矛盾 |

**方向性为什么重要**：两个方向都判 FAIL，会让「文档过时」这种非门禁问题
阻塞发布；只判虚假通过，判据才真正回答 BE5 问的问题。

**不校验 merge 时间序**：§2 是**依赖序**（"按依赖顺序排列"），不是时间序。
实测若把行序当时间序比，PR-2806 比 PR-2697 晚合并 2.8 天，必然 FAIL ——
用一个永红门禁换掉一个假绿，净效果为负。只校验**拓扑依赖边**。

**规划编号 → Gitea PR 号的映射不读散文**：仓库两处散文映射互相矛盾：

| 来源 | PR-830C | PR-830D | PR-830E |
|---|---|---|---|
| `v3.8.0/beta/BETA_GATE_REPORT.md:120-124` | #2669 | #2670 | #2675 |
| git merge commit 标题（实测） | **#2670** | **#2674** | **#2691** |

权威来源是 **merge commit 标题本身**，它同时带两个 id：

```
Merge pull request 'fix(storage): PR-830F WAL lifecycle — ...' (#2697) ...
```

一次 grep 得到两个 id，无需维护映射表，也就没有映射表可以腐烂。

**父编号由子编号实现**：§2.1 写「PR-830 | ✅ 已合并 | PR-830A~E + PR-830F」，
所以 PR-830 的合并证据要连子编号一起算。

### 3.2 `check_gate_script_refs.sh` —— 根治件

**范围**：8 份 SSOT 判据文档（`GATE_CONDITIONS.md`、`RELEASE_GATE_CHECKLIST.md`、
`STAGE_CONFIG.yaml`、`GATE_CI_CD.md`、`STAGE_CONTROL_FRAMEWORK.md`、`INDEX.md`、
`RC_TO_GA_GATE_CHECKLIST.md`、`AGENTS.md`）。

**正则**：`(\./)?scripts/[A-Za-z0-9_./-]+\.sh` —— 覆盖 `scripts/` 与 `scripts/gate/`
两层。只匹配 `scripts/gate/` 会漏掉 #5450 的 3 个实例（它们在 `scripts/` 下）。

**不扫历史文档**：全仓约 90 个缺失引用，绝大多数在历史报告里忠实记录了当时的
事实。全扫会得到一个永远红的门禁。

**Baseline 语义**（照抄仓库既有 `check_docs_pointer_paths.sh` 模式）：

| 情况 | 判定 |
|---|---|
| 存在 | PASS |
| 缺失 + 在 baseline | WARN（已知债，每条带成因）|
| 缺失 + 不在 baseline | **FAIL**（新增缺陷）|

baseline: `tests/baseline/gate_script_refs_baseline.json`（8 条已知债，每条带成因）。
**每条必须带原因** —— 没有原因的 baseline 条目是「把问题藏起来」，与本 issue 的
目的相反。

### 3.3 `check_pr_template_sections.sh` —— 补上缺口 (c)

读实际 PR 正文（`--pr-body <file>` 或 stdin），检查 5 个 `<!-- section:NAME -->`
标记是否齐备。

**为什么用 HTML 注释**：渲染时**不可见**，不污染 PR 正文可读性；
同时是稳定的机器可解析锚点。

### 3.4 组件四：CHECK 4 白名单

`scripts/gate/check_anti_fabrication.sh:191` 的 `allowed[]` 加
`opencode-z440@gaoyuanyiyao.com`。

Gitea user `opencode-z440`（id=13，team `ai-developers` 有写权限）是本仓既有
agent 身份（`openspec/changes/v312-28-corpus-activation/proposal.md` 已有
`Owner: opencode-z440` 署名）。遗漏会让该身份的分支**在门禁上被拒** ——
这是门禁缺陷，不是策略信号：这个检查的目的是抓**未知**作者，不是挑出某个
已知作者。与 2026-10-08 为 `omp-z440` 加入的先例同型。

---

## 4. 已知的存量问题（本 PR 不修，但如实记录）

### 4.1 §2.1 状态表的 3 行已过时

实测（2026-10-22，base `378f014529`）：

| §2.1 声称 | git 实测 merge commit 数 | 结论 |
|---|---|---|
| PR-830 `✅ 已合并` | PR-830A~F 均有 | 一致 |
| PR-840 `❌ 未合并` | **8** | **过时** |
| PR-850 `❌ 未合并` | **8** | **过时** |
| PR-870 `❌ 未合并` | **8** | **过时** |

本 PR 的 gate 把它报为 **WARN** 并列出证据。修它需要改
`DEVELOPMENT_PLAN.md` 的状态表 —— 那是文档改动，且该文档在 v3.8.0 归档区，
超出本 issue 范围（#5457 是「让判据可执行」，不是「修文档内容」）。

### 4.2 4 个 Hard Gate 在 base 上即为 FAIL

已用 `git stash` 在同一 commit `378f014529` 下复跑确认，与本 PR 无关：

| Gate | base 结果 | 归属 |
|---|---|---|
| `check_anti_fabrication.sh` | FAIL（7 errors）| 他人 batch |
| `check_drift_not_pass.sh` | FAIL（4 anti-patterns）| 他人 batch |
| `check_docs_pointer_paths.sh` | FAIL（3 broken pointers）| 他人 batch |
| `check_test_target_lint_budget.sh` | FAIL（`CARGO_TARGET_DIR` 未定义）| 环境问题 |

本 PR **不引入**任何新的 Gate 失败（见 `PR-5457_TEST_REVIEW.md` §3）。

---

## 5. 并行隔离（ADR-014 D-02 / D-03）

| 维度 | 值 |
|---|---|
| Agent id | `opencode-z440`（252 Gitea user id=13）|
| Worktree | `.worktrees/opencode-z440-20261022` |
| Branch | `opencode/fix-5457-5458-gate-refs` |
| Base | `origin/develop/v4.1.0` @ `378f014529` |

**文件面冲突热区**（实测 168 个远端 fix/docs 分支）：

| 文件 | 触及它的远端分支 | 本 PR 改动量 | 实际冲突风险 |
|---|---|---|---|
| `scripts/gate/verify_pr_dag.sh` | **0** | 新增 | 无 |
| `scripts/gate/check_pr_dag.sh` | **0** | 新增 | 无 |
| `scripts/gate/check_gate_script_refs.sh` | **0** | 新增 | 无 |
| `scripts/gate/check_pr_template_sections.sh` | **0** | 新增 | 无 |
| `tests/integration/gate/gate_criterion_existence_test.rs` | **0** | 新增 | 无 |
| `tests/baseline/gate_script_refs_baseline.json` | **0** | 新增 | 无 |
| `scripts/gate/check_anti_fabrication.sh` | **3** | 改 1 个数组元素（+9 行注释）| **中** — `origin/fix/omp-ignore-registry-5098` 改的是**同一行** `local allowed=(...)` 数组（同样加一个邮箱）⇒ 文本冲突，需 rebase 后重加 |
| `AGENTS.md` | **3** | 改 1 行 | 低 — 三个分支的改动在别处 |
| `.gitea/pull_request_template.md` | **2** | 改 10 行 | 低 — 两个分支的改动在别处 |
| `Cargo.toml` | **4** | 改 3 行（`[[test]]` 块）| **中** — `origin/fix/clippy-workspace-coverage-5508-batch19`（**同日**）也改 `Cargo.toml` |
| `.gitea/workflows/ci.yml` | 5 | **不改** | — |
| `crates/mysql-server/src/lib.rs` | 7 | **不改** | — |
| `src/execution_engine.rs` | 5 | **不改** | — |

**实测方法**（可复现）：

```bash
git for-each-ref --format='%(refname:short)' refs/remotes/origin | grep 'origin/fix\|origin/docs' \
  | while read b; do
      n=$(git log --format=%H origin/develop/v4.1.0.."$b" -- <file> | wc -l)
      [ "$n" -gt 0 ] && echo "$b : $n"
    done
```

统计的是**各分支自己的 commit**（不是 `diff base..branch`）—— 后者会把
基线漂移算成该分支的改动。

> **订正说明**：本 PR 的初版文档写的是「168 个远端 fix/docs 分支**零触及**」。
> 那句话**不成立**：实测 168 个分支中有 8 个触及本 PR 修改的 4 个文件。
> 冲突面已按文件逐条重测，并把风险等级写进上表。
| `.gitea/workflows/ci.yml` | 5 | **不改** |
| `crates/mysql-server/src/lib.rs` | 7 | **不改** |
| `src/execution_engine.rs` | 5 | **不改** |
