# SQLRustGo v4.1.0 重构准入整备 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 v4.1.0 从「材料互相矛盾、无权威基线」整备到「基线唯一、契约可核、验收有据、门禁会说真话」，产出属主可据以放行 C-ALPHA-02 的判据。

**Architecture:** 五段式依赖链 A1→A2→A3→A4→A5。A1 建立唯一基线并导入全部重构材料；A2 把属主已下的 ADR 裁决落到机器可核字段并重签 manifest；A3 把 75 场景矩阵从未定义状态修到可执行并产出绑定新基线的证据；A4 只补 ALPHA 阶段执行点与 7 道负向夹具（不碰其余 161 道门禁）；A5 汇总 12 条判据交属主放行。全程复用既有 `verify_handoff.py` 与 `tools/test_factory.py`，不新建调度器或账本。

**Tech Stack:** git / bash / Python 3（`json`、`hashlib`、`jsonschema`）/ GitHub Actions 兼容的 Gitea Actions（`.gitea/workflows/ci.yml`）/ Rust `cargo test`（仅通过既有测试工厂调用）

**Spec:** `docs/refactor/v4.1.0-c/takeover/V410-REFACTOR-READINESS-DESIGN.md`（rev 2，已并入 builder check 18 项复核）

---

## Global Constraints

以下每行都从 spec 逐字抄来。**每个任务都隐含包含本节。**

- **C1** 门禁清单变更、阶段跃迁、ADR 批准、SOAK 启动为属主专属，不得由执行者代行
- **C2** 不得新建第二套 Harness / 调度器 / 账本（`new_scheduler_or_runner: false`）
- **C3** `SQLRUSTGO_LOCAL_ONLY` 材料禁止云端 fallback，分类缺失即 BLOCKED
- **C4** 所有执行必须在隔离 worktree，不在权威工作区直接改
- **C5** Receipt 必须绑定完整 commit SHA，跨 SHA 证据不可继承
- **C6** ADR 批准指向固定 commit 的正文与 sha256，内容变更须重审
- **C7** Oracle 由 Owner/独立 Reviewer 签署，实施者不得自签
- **C8** ALPHA 阶段门禁不得在每次 push 全量执行（`ci.yml:1700-1704`）
- **R-ADR** 10/11 份 ADR 机器字段非空、档位正确，映射满足 A2.2 五条规则
- **R-BASE** `BASE`（**内容基线提交**：develop tip + 材料导入 + 冲突解决后的内容）唯一，develop tip 是其祖先，含 BASE 的分支 `base/v4.1.0-frozen` 已推送且其 tip 是 BASE 的后代。**后续所有任务的 `base_sha` 字段一律填 BASE，不随提交前移**
- **R-SHA** 前向绑定 `base_sha` 唯一且等于 `BASE`；历史绑定逐项未变
- **R-RED** 已知的 RED 不得塌陷为普通 FAIL；`KNOWN_FAIL` 必须保留 RED 五元组身份
- **R-ORACLE** 签署人 ≠ 该场景实现者
- **R-GATE** 未放宽任何门禁阈值
- **R-NEWCODE** 本计划全程不写数据库内核代码；DAG 内仍无核心编码包

**已裁决事项（spec §6.1）：** O6 Oracle 分层签署（先签 T1 试点，再批量签 T2/T3）；O7 ADR-410-011 并入 `adrs/` 但不进 `approved_adr_refs`；O8 发 R2 包、R1 不动；O9 跨仓库 LOCAL_ONLY 不纳入、显式豁免并记录。

---

## Review Focus

spec 隐含、但任何任务的测试都不会自动覆盖的失败模式。下列每条都已在对应任务中钉死测试。

1. **基线未推送时 receipt 不可验证** —— BASE 若只存在于本地 worktree，所有绑定它的 receipt 无法被 CI 或其它 agent 校验，「唯一基线」只是本机事实。（Task 1）
2. **条件批准 ADR 被引用** —— 006/011 前置冻结未满足，若进入 `approved_adr_refs`，等于记录了一次未生效的批准。（Task 3）
3. **RED 被塌陷为 FAIL** —— pilot 词汇 `FAILED_RED` 映射到 `KNOWN_FAIL` 时若丢五元组，17 项已知 RED 会静默降级。（Task 5）
4. **历史 SHA 被回填** —— 为满足「所有 base_sha 等于 BASE」而改写冻结的历史 receipt，等于伪造审计轨迹。（Task 2）
5. **dry-run 被当作执行** —— `check_stage.sh --dry-run` 只解析不执行；用它充当 ALPHA 执行点会得到一个永不红的假绿。（Task 8）

---

## 文件结构

| 文件 | 责任 | 任务 |
|---|---|---|
| `docs/refactor/v4.1.0-c/takeover/BASELINE_FREEZE.json` | 唯一基线的身份记录（sha/tree/parent/cherry-pick 台账） | T1 |
| `docs/releases/v4.1.0/refactoring/tools/verify_baseline.py` | 校验 BASE 身份与材料完整性 | T1 |
| `docs/releases/v4.1.0/refactoring/tools/verify_sha_scope.py` | 校验前向/历史 SHA 分界 | T2 |
| `docs/releases/v4.1.0/refactoring/tools/verify_handoff.py` | ADR 状态与 manifest 的唯一校验器 | T3, T4, T5 |
| `docs/releases/v4.1.0/refactoring/tools/requirements.txt` | Python 依赖钉版 | T3 |
| `docs/releases/v4.1.0/refactoring/tools/matrix_driver.py` | 矩阵 → test_factory 调用 → 回写状态 | T5 |
| `docs/releases/v4.1.0/refactoring/tools/verify_matrix.py` | 校验矩阵字段完整性与状态词汇合规 | T5 |
| `audit/v4.1.0/neg/` | ALPHA 7 道的负向夹具 | T8 |
| `docs/releases/v4.1.0/GATE_CATALOG.jsonl` | 门禁编目（168 条） | T8 |
| `.gitea/workflows/ci.yml` | ALPHA 阶段执行点 | T8 |
| `docs/refactor/v4.1.0-c/takeover/R2-RELEASE-PACKAGE.md` | R2 外审包生成说明与放行裁决记录 | T9 |

---

### Task 1: 建立唯一基线并导入全部重构材料

**Files:**
- Create: `docs/refactor/v4.1.0-c/takeover/BASELINE_FREEZE.json`
- Create: `docs/releases/v4.1.0/refactoring/tools/verify_baseline.py`
- Modify: `scripts/gate/check_g1_tpch_baseline.sh`（仅解决冲突，不改逻辑）

**Interfaces:**
- Consumes: `8cbde5b6d3`（develop tip）、`45da6968ae`、`b681b74ca3`、`cd8cefe623`、`9a61cf59c0`、`cfd4306b7e`
- Produces: `BASELINE_FREEZE.json`，含字段 `base_sha` / `tree_sha` / `parent` / `frozen_at` / `source_ref` / `cherry_picked[]` / `material_import{from,files}`。后续 Task 3 的 `verify_handoff.py` 从此文件读 `BASE`。

> **`BASE` 的准确定义（实施中确立，务必按此理解后续所有任务）**：`BASE` = **内容基线提交** = develop tip + 材料导入 + 冲突解决后的内容。
> 它是 develop tip 的**后代**、分支 tip 的**祖先**。
> **后续所有任务的 `base_sha` 字段一律填 `BASE`（内容基线），不是分支 tip，也不随每次提交前移**——
> 否则「冻结」失去意义，且 receipt 的 SHA 绑定会逐提交漂移。
> （提交无法包含自身 SHA，故 `base_sha` 只能是内容基线，不能是含 freeze 记录的那次提交。）

- [ ] **Step 1: 建隔离 worktree**

> **待裁决前置（spec O11）**：本任务默认把 BASE 落在 `base/v4.1.0-frozen` 分支。
> 若属主裁决「直接推进 `develop/v4.1.0`」，则本任务的 Step 1 改为
> `git worktree add /Volumes/workspace/dev/wt-v410-base -b develop/v4.1.0 8cbde5b6d3`，
> 且 Step 11 的推送目标改为 `origin develop/v4.1.0`。**默认走前者**（不改权威分支）。
>
> **待裁决前置（spec O12）**：Task 2 Step 5 会碰到 `14_HANDOFF_MANIFEST.json` 的
> `authority.upstream_c01_manifest_resign:false`。默认行为是**只改 `base_sha`、
> 不改 `adr_base_commit`、不重算 `files_digest`**，并在 Task 4 的重签中一并处理。
> 若属主另有裁决，Task 2 Step 5 需相应调整。

```bash
git worktree add /Volumes/workspace/dev/wt-v410-base -b base/v4.1.0-frozen 8cbde5b6d3
```
Expected: 创建成功，`git -C /Volumes/workspace/dev/wt-v410-base log -1 --format=%H` 输出 `8cbde5b6d3...`

- [ ] **Step 2: cherry-pick 材料导入提交**

```bash
cd /Volumes/workspace/dev/wt-v410-base && git cherry-pick 45da6968ae
```
Expected: **无冲突**，输出 `123 files changed`。若出现冲突则停止并回报——spec 记录为 clean。

- [ ] **Step 3: 导入 ADR-410-011 的运行时契约材料**

ADR-410-011 不在 `45da6968ae`（该提交不含 `runtime-contract/` 目录），而在 `b681b74ca3`
（分支 `codex/v410-runtime-5915`，**不是 develop 祖先**）。
按设计文档「011 绑定以其自身文件为准」，以**目录**为单位导入该运行时契约包（56 个文件），
**不**导入该提交的其余 178 个文件（PR #5915 的代码/脚本改动超出「材料导入」范围）：

```bash
cd /Volumes/workspace/dev/wt-v410-base
git checkout b681b74ca3 -- docs/releases/v4.1.0/refactoring/runtime-contract/20261120-pr5915/
git status --porcelain | grep -c '^A '
```
Expected: 输出 `62`（全部为新增文件，路径均在 `runtime-contract/20261120-pr5915/` 下）

**副作用确认**：`evidence/inputs/*.rs` 与 `tools/*.py` 都在 `docs/` 下——
cargo 不编译 `docs/` 下的 `.rs`，CI 的 python 步骤也不扫 `docs/`，故无编译或门禁影响。

- [ ] **Step 4: 确认材料已进树**

```bash
git -C /Volumes/workspace/dev/wt-v410-base ls-tree -r --name-only HEAD -- docs/refactor/v4.1.0-c/handoff/ | wc -l
git -C /Volumes/workspace/dev/wt-v410-base ls-tree -r --name-only HEAD -- docs/releases/v4.1.0/refactoring/REFACTOR_TASK_DAG.json
```
Expected: 第一条 > 0；第二条输出 `docs/releases/v4.1.0/refactoring/REFACTOR_TASK_DAG.json`

- [ ] **Step 5: cherry-pick TPC-H 修复（唯一真 delta）**

```bash
git -C /Volumes/workspace/dev/wt-v410-base cherry-pick cd8cefe623
```
Expected: **CONFLICT**，仅 `scripts/gate/check_g1_tpch_baseline.sh`（hunk 在 177–189 行，共 12 行）

- [ ] **Step 6: 解决冲突**

三方合并 `scripts/gate/check_g1_tpch_baseline.sh`：
- **保留 develop 侧**该 hunk 的全部既有逻辑行
- **加入 HEAD 侧**的 TPC-H 双解析链同目录修正（`cd8cefe623` 引入的路径归一）
- 删除全部 `<<<<<<<` / `=======` / `>>>>>>>` 标记

```bash
git -C /Volumes/workspace/dev/wt-v410-base diff --check || true
grep -rn '<<<<<<<\|>>>>>>>' /Volumes/workspace/dev/wt-v410-base/scripts/gate/check_g1_tpch_baseline.sh
```
Expected: `grep` 无输出

- [ ] **Step 7: 继续 cherry-pick**

```bash
git -C /Volumes/workspace/dev/wt-v410-base cherry-pick --continue
```
Expected: `cd8cefe623` 提交成功

- [ ] **Step 8: cherry-pick 两个已为空提交并记录**

```bash
git -C /Volumes/workspace/dev/wt-v410-base cherry-pick 9a61cf59c0   # 预期 "previous cherry-pick is now empty"
git -C /Volumes/workspace/dev/wt-v410-base cherry-pick --skip
git -C /Volumes/workspace/dev/wt-v410-base cherry-pick cfd4306b7e   # 预期同样为空
git -C /Volumes/workspace/dev/wt-v410-base cherry-pick --skip
```
Expected: 两条被 `--skip`；工作树干净（`git status --porcelain` 无输出）

- [ ] **Step 9: 写基线冻结记录**

在 `/Volumes/workspace/dev/wt-v410-base/docs/refactor/v4.1.0-c/takeover/BASELINE_FREEZE.json` 写入：

```json
{
  "base_sha": "<HEAD 的完整 40 位 SHA>",
  "tree_sha": "<git rev-parse HEAD^{tree} 的输出>",
  "parent": "8cbde5b6d33c876fa1197474c46d50142a9823bd",
  "frozen_at": "<ISO8601 带时区>",
  "source_ref": "base/v4.1.0-frozen",
  "cherry_picked": [
    {"sha": "45da6968ae2baaade6a31206eeee5854a8514ee2", "state": "applied"},
    {"sha": "b681b74ca3c0373cb2b7c44ff2cb032cf1a1c540", "state": "path-import",
     "path": "docs/releases/v4.1.0/refactoring/runtime-contract/20261120-pr5915/",
     "files": 56},
    {"sha": "cd8cefe623e148964fd0c4ce0ccfcc56eff762df", "state": "applied",
     "conflict": "scripts/gate/check_g1_tpch_baseline.sh"},
    {"sha": "9a61cf59c0756ecfba2987cc02a2b5994eff7d5b", "state": "already-present"},
    {"sha": "cfd4306b7e6d6c8a7130d1fed7769ba834ed6de8", "state": "already-present"}
  ],
  "material_import": {"from": "45da6968ae2baaade6a31206eeee5854a8514ee2", "files": 123}
}
```

- [ ] **Step 10: 写并运行基线校验器**

创建 `docs/releases/v4.1.0/refactoring/tools/verify_baseline.py`，实现 4 项断言（失败即非零退出）：

| 断言 | 判定 |
|---|---|
| B1 | `git merge-base --is-ancestor 8cbde5b6d3 HEAD` 成立（develop tip 是 BASE 的祖先） |
| B2 | `BASELINE_FREEZE.json` 的 `base_sha` == 实际 HEAD，`tree_sha` == 实际 tree |
| B3 | `45da6968ae` 引入的 123 个文件 + `b681b74ca3` 的 `runtime-contract/20261120-pr5915/` 目录 56 个文件，全部存在于 HEAD 树内（`git ls-tree -r --name-only <rev> -- <path>` 与 HEAD 求差集为空） |
| B4 | `cherry_picked` 5 条齐全（applied 2 + path-import 1 + already-present 2），且两条 `already-present` 对应的 diff 在 develop 上已存在（`git diff origin/develop/v4.1.0..HEAD -- <该提交改动文件>` 对这两条为空）；`path-import` 条目的 path 目录文件数为 62 |

```bash
cd /Volumes/workspace/dev/wt-v410-base && python3 docs/releases/v4.1.0/refactoring/tools/verify_baseline.py
```
Expected: 输出 4/4 PASS，退出码 0

- [ ] **Step 11: 提交并推送基线**

```bash
cd /Volumes/workspace/dev/wt-v410-base
git add docs/refactor/v4.1.0-c/takeover/BASELINE_FREEZE.json \
        docs/releases/v4.1.0/refactoring/tools/verify_baseline.py \
        docs/releases/v4.1.0/refactoring/runtime-contract/
git commit -m "chore(v4.1.0): 冻结唯一实施基线并导入重构材料"
git push -u origin base/v4.1.0-frozen
```
Expected: 推送成功

- [ ] **Step 12: 确认基线可被他人检出（Review Focus #1）**

```bash
git ls-remote origin refs/heads/base/v4.1.0-frozen
```
Expected: 输出 1 行，其 SHA 是 `BASELINE_FREEZE.json` 的 `base_sha` 的**后代**（`base_sha` 本身已随 freeze 记录提交推送到该分支）。**若无输出则本任务不算完成**——未推送的基线不构成权威基线。

---

### Task 2: 前向/历史 SHA 重指

**Files:**
- Modify: `docs/releases/v4.1.0/refactoring/REFACTOR_TASK_DAG.json`（18 处 `tasks[].base_sha`）
- Modify: `docs/releases/v4.1.0/refactoring/TEST_COVERAGE_MATRIX.json`（75 处 `cases[].base_sha`）
- Modify: `docs/refactor/v4.1.0-c/handoff/03_TASK_MANIFEST.jsonl`（18 行 `base_commit`）
- Modify: `docs/releases/v4.1.0/refactoring/test-factory-design/14_HANDOFF_MANIFEST.json`（`base_sha` / `adr_base_commit`）
- Create: `docs/releases/v4.1.0/refactoring/tools/verify_sha_scope.py`

**Interfaces:**
- Consumes: `BASELINE_FREEZE.json` 的 `base_sha`（Task 1）
- Produces: 校验器 `verify_sha_scope.py`，退出码 0 表示前向绑定唯一且历史绑定未变。Task 3 的 `verify_handoff.py` 依赖前向绑定已统一。

- [ ] **Step 1: 写分界校验器**

创建 `verify_sha_scope.py`，从 `BASELINE_FREEZE.json` 读 `BASE`，实现三组断言：

| 断言 | 判定 |
|---|---|
| S1（前向） | `REFACTOR_TASK_DAG.json` 全部 18 个 `tasks[].base_sha` == BASE |
| S2（前向） | `TEST_COVERAGE_MATRIX.json` 全部 75 个 `cases[].base_sha` == BASE |
| S3（前向） | `03_TASK_MANIFEST.jsonl` 全部 18 行 `base_commit` == BASE，且与 DAG 同序 task_id 一一对应 |
| S4（前向） | `14_HANDOFF_MANIFEST.json` 的 `base_sha` == BASE |
| S5（历史） | `audit/v4.1.0/commands_pin_*.yaml` 与 `audit/v4.1.0/runs/*/manifest.yaml` 的 `frozen_commit` 全部保持原值（与 `git show 45da6968ae:<path>` 逐字相同） |
| S6（历史） | pilot `evidence/rerun-summary-20261120.json` 的 `base_sha` 仍为 `e9ff21fca3aa945f6fe543ce8f91dfc19e6aaed0` |

先在未改动的树上跑一次，确认 S5/S6 此刻成立而 S1–S4 失败：

```bash
cd /Volumes/workspace/dev/wt-v410-base && python3 docs/releases/v4.1.0/refactoring/tools/verify_sha_scope.py
```
Expected: 报 S1–S4 FAIL、S5–S6 PASS

- [ ] **Step 2: 重指 DAG 的 18 处**

用脚本改写 `REFACTOR_TASK_DAG.json`：`json.load` → 遍历 `tasks` → 置 `t["base_sha"] = BASE` → `json.dump(indent=2, ensure_ascii=False)`。
保持键序不变（用 `object_pairs_hook=OrderedDict`）。

- [ ] **Step 3: 重指矩阵的 75 处**

同上，改 `TEST_COVERAGE_MATRIX.json` 的 `cases[].base_sha`。**不得改动其它任何字段**（本任务只管 SHA）。

- [ ] **Step 4: 重指 task manifest 的 18 行**

逐行替换 `03_TASK_MANIFEST.jsonl` 的 `base_commit` 字段值，保持行序与 JSON 结构。

- [ ] **Step 5: 重指第二份 manifest**

改 `14_HANDOFF_MANIFEST.json` 的 `base_sha` 为 BASE。
`adr_base_commit` **保持 `45da6968ae` 不变**——它是 ADR 原文的提交身份（spec A1.4「ADR 正文绑定」行）。
`files_digest` 本任务不重算（依赖 Task 4 重签）。

- [ ] **Step 6: 运行分界校验器**

```bash
cd /Volumes/workspace/dev/wt-v410-base && python3 docs/releases/v4.1.0/refactoring/tools/verify_sha_scope.py
```
Expected: S1–S6 全 PASS，退出码 0。**若 S5/S6 失败，说明历史绑定被误改——回滚重做（Review Focus #4）。**

- [ ] **Step 7: 提交**

```bash
cd /Volumes/workspace/dev/wt-v410-base
git add docs/releases/v4.1.0/refactoring/REFACTOR_TASK_DAG.json \
        docs/releases/v4.1.0/refactoring/TEST_COVERAGE_MATRIX.json \
        docs/refactor/v4.1.0-c/handoff/03_TASK_MANIFEST.jsonl \
        docs/releases/v4.1.0/refactoring/test-factory-design/14_HANDOFF_MANIFEST.json \
        docs/releases/v4.1.0/refactoring/tools/verify_sha_scope.py
git commit -m "chore(v4.1.0): 前向绑定重指新基线，历史绑定保持原值"
```

- [ ] **Step 8: 重签 manifest（Task 2 改了 finalize 范围内的文件）**

本任务修改了 `REFACTOR_TASK_DAG.json`、`TEST_COVERAGE_MATRIX.json`、
`03_TASK_MANIFEST.jsonl`、`14_HANDOFF_MANIFEST.json`——全部在 `finalize()` 的哈希范围内，
故必须重签，否则 Task 3 的 `--check` 会因 manifest 漂移失败：

```bash
cd /Volumes/workspace/dev/wt-v410-base
python3 docs/releases/v4.1.0/refactoring/tools/verify_handoff.py --finalize
python3 docs/releases/v4.1.0/refactoring/tools/verify_handoff.py --check
```
Expected: `--finalize` 退出 0；`--check` **此时可能因 ADR 断言（`approved_adr_refs==[]`）
失败——这是预期的**，该断言由 Task 3 放宽。若失败信息只涉及 ADR 断言，属正常；
若涉及 manifest 哈希不一致，则说明重签未生效，须重跑 `--finalize`。

---

### Task 3: 打通 `verify_handoff.py` 的断言闭包

**Files:**
- Modify: `docs/releases/v4.1.0/refactoring/tools/verify_handoff.py`（9 处：行 16/60/65/89/90/116/122/123/132-133）
- Create: `docs/releases/v4.1.0/refactoring/tools/requirements.txt`
- Create: `docs/refactor/v4.1.0-c/takeover/dsh-b/`（**从主工作区导入**：36 个 DSH-B 已交付治理记录 —— ADR 裁决账本、RED 身份台账等，Task 4 与 Task 6 依赖它们）
- Test: `docs/releases/v4.1.0/refactoring/tools/verify_handoff.py` 自身的 8+1 反向变异臂

**Interfaces:**
- Consumes: `BASELINE_FREEZE.json`（Task 1）、重指后的 DAG 与 manifest（Task 2）
- Produces: `verify_handoff.py --check` 在 Task 4 之后返回 0。Task 4、5 依赖本任务放宽后的断言。

- [ ] **Step 1: 记录改动前的失败基线**

```bash
cd /Volumes/workspace/dev/wt-v410-base && python3 docs/releases/v4.1.0/refactoring/tools/verify_handoff.py --check
```
Expected: 非零退出。记录具体断言行号作为对照。

- [ ] **Step 2: 声明依赖**

创建 `docs/releases/v4.1.0/refactoring/tools/requirements.txt`：

```
jsonschema==4.23.0
```

```bash
python3 -m pip install -r docs/releases/v4.1.0/refactoring/tools/requirements.txt
python3 -c "import jsonschema; print(jsonschema.__version__)"
```
Expected: 输出 `4.23.0`。**若属主裁决 O13 改为「不引入依赖」，则跳过本步，改用仓库内既有校验器替代 JSON Schema 校验，并在 `verify_handoff.py` 中删除 `from jsonschema import …`。**

- [ ] **Step 3: 参数化基线**

改行 16：`BASE = "e9ff21fca3aa945f6fe543ce8f91dfc19e6aaed0"` → 从 `BASELINE_FREEZE.json` 的 `base_sha` 读取。
同步处理行 65（base_sha 断言）、行 90（`"tree"` 硬编码值改为读 `BASELINE_FREEZE.json` 的 `tree_sha`）、行 116（`assert sha(HARA MANIFEST.json)=="4649fccc…"` 改为读取实际值比对，不再硬编码）。

- [ ] **Step 4: 放宽 ADR 断言**

| 行 | 改为 |
|---|---|
| 60 | `assert t["approved_adr_refs"] and set(t["approved_adr_refs"]) <= REAL_ADR_IDS` |
| 123 | `assert d["approval_refs"] and set(d["approval_refs"]) <= REAL_ADR_IDS` |

`REAL_ADR_IDS` = `adrs/` 目录下实际存在的 ADR id 集合（ADR-410-001..011，Task 4 后为 11）。

- [ ] **Step 5: 改 ADR 生成器**

行 89：`"status":"PROPOSED","approval_refs":[]` → `status` 取 ADR 正文第 6 行解析出的档位、`approval_refs` 取其 `Approvals` 字段解析出的引用列表。
行 90：`"adr_approval":"PENDING"` → 改为按实际档位聚合出的状态。

- [ ] **Step 6: 改 ADR 数量断言**

行 122：`assert len(baseline["adrs"])==10` → `==11`（O7 裁决：011 并入编目）。

- [ ] **Step 7: 新增「档位越权」反向变异**

在现有 8 个变异臂之后新增第 9 个：构造一份 DAG 副本，把某 task 的 `approved_adr_refs` 指向档位为「仅批准方向/实施另审」或「条件批准」的 ADR（003/004/007/008/009/010/006/011 中任一），且该 task 的 `authorized_for_execution` 为 true —— 断言校验器**必须拒绝**它。

```bash
cd /Volumes/workspace/dev/wt-v410-base && python3 docs/releases/v4.1.0/refactoring/tools/verify_handoff.py --check
```
Expected: 8 个原变异 + 1 个新变异全部被拒（校验器自身报告「all mutations rejected」）

- [ ] **Step 8: 导入 `takeover/dsh-b/` 治理记录**

`dsh-b/` 是 DSH-B 的**已交付治理记录**（ADR 裁决账本、RED 身份台账、缺陷验收包等），
但它当前**只存在于主工作区的未跟踪文件里**，基线树内没有——
Task 4 需要 `dsh-b/ADR-DECISION-RECORD.md` 的 11 份 ADR 档位，
Task 6 需要 `dsh-b/RED-LEDGER.md` + `RED-IDENTITY-RECONCILIATION.md` 的 17 项 RED 五元组。

```bash
cd /Volumes/workspace/dev/wt-v410-base
mkdir -p docs/refactor/v4.1.0-c/takeover/dsh-b
cp -R /Volumes/workspace/dev/sqlrustgo/docs/refactor/v4.1.0-c/takeover/dsh-b/. \
      docs/refactor/v4.1.0-c/takeover/dsh-b/
find docs/refactor/v4.1.0-c/takeover/dsh-b -type f | wc -l
git add -f docs/refactor/v4.1.0-c/takeover/dsh-b/
```
Expected: 文件数 = 36（含 `artifacts/` 与 `c01-prep/` 子目录）；`git add -f` 成功
（`.gitignore` 有宽 `*.json` 规则，需 `-f`；加完后后续修改可正常 stage）

**只导入 `dsh-b/`**：`takeover/` 顶层是活跃工作区（设计文档、实施计划），不纳入基线。

- [ ] **Step 9: 收窄 `finalize()` 的哈希范围**

把 `finalize()` 的输入集合从「整个 `docs/refactor/v4.1.0-c/`」改为
「`docs/refactor/v4.1.0-c/handoff/` + `docs/refactor/v4.1.0-c/takeover/dsh-b/` + `docs/releases/v4.1.0/refactoring/`」，
**排除 `docs/refactor/v4.1.0-c/takeover/` 顶层**（活跃工作区，非交付物）。
同步更新行 132-133 的工作树前缀断言，允许 `takeover/` 顶层的改动而不允许其它前缀。

这样 `dsh-b/` 下的裁决账本与台账被 manifest 的逐文件 sha256 覆盖——内容被改动会在 `--check` 时暴露。

- [ ] **Step 10: 验证 takeover/ 写入不再导致漂移**

```bash
cd /Volumes/workspace/dev/wt-v410-base
echo "probe" >> docs/refactor/v4.1.0-c/takeover/.drift-probe
python3 docs/releases/v4.1.0/refactoring/tools/verify_handoff.py --finalize
python3 docs/releases/v4.1.0/refactoring/tools/verify_handoff.py --check
rm docs/refactor/v4.1.0-c/takeover/.drift-probe
```
Expected: 两次命令均退出 0——即 `takeover/` 下的写入不进入 manifest 哈希范围

- [ ] **Step 11: 提交**

```bash
cd /Volumes/workspace/dev/wt-v410-base
git add docs/releases/v4.1.0/refactoring/tools/verify_handoff.py \
        docs/releases/v4.1.0/refactoring/tools/requirements.txt
git commit -m "fix(v4.1.0): 打通 verify_handoff 断言闭包并收窄 manifest 哈希范围"
```

---

### Task 4: ADR 批准落地并重签 manifest

**Files:**
- Modify: `docs/releases/v4.1.0/refactoring/adrs/ADR-410-001.md` … `ADR-410-010.md`（各第 6 行 `Status:` / `Approvals:`）
- Move: `docs/releases/v4.1.0/refactoring/runtime-contract/20261120-pr5915/ADR-410-011-UNIFIED_RUNTIME_CONTRACT.md` → `docs/releases/v4.1.0/refactoring/adrs/ADR-410-011.md`
- Modify: `docs/releases/v4.1.0/refactoring/ADR_INDEX.md`（状态列 + 第 7 行说明 + 第 3-4 行 `evidence_hash`）
- Modify: `docs/releases/v4.1.0/refactoring/REFACTOR_TASK_DAG.json`（18 处 `approved_adr_refs`）

**Interfaces:**
- Consumes: `verify_handoff.py` 的放宽后断言（Task 3）、`ADR-DECISION-RECORD.md:35-47` 的裁决档位
- Produces: 11 份 ADR 的机器字段与裁决账本逐条一致；`approved_adr_refs` 满足 A2.2 五条规则；重签后的 `10_HANDOFF_MANIFEST.json`

- [ ] **Step 1: 读出裁决档位**

从 `docs/refactor/v4.1.0-c/takeover/dsh-b/ADR-DECISION-RECORD.md:35-47` 逐条抄出 11 份 ADR 的档位：

| 档位 | ADR |
|---|---|
| 无条件批准 | 001、002、005 |
| 条件批准（前置未满足） | 006、011 |
| 仅批准方向 / 实施另审 | 003、004、007、008、009、010 |

- [ ] **Step 2: 更新 10 份 ADR 的状态行**

对 `ADR-410-001..010.md` 各第 6 行，把
`Status: PROPOSED / OWNER_REVIEW_REQUIRED。Approvals: []。…`
改为形如
`Status: APPROVED / APPROVED_DIRECTION / CONDITIONALLY_APPROVED。Approvals: [<被本 ADR 批准/依赖的 ADR id 列表>]。Approval: <裁决引用>，绑定 sha256:<本文件新 sha256>。…`
（`Status` 取值按档位映射：`无条件批准→APPROVED`，`仅批准方向/实施另审→APPROVED_DIRECTION`，`条件批准→CONDITIONALLY_APPROVED`）

> **`Approvals` 字段的语义必须是「ADR id 列表」**（R22）。`verify_handoff.py` 从 ADR 正文的
> `Approvals` 解析出 `approval_refs` 并断言其 ⊆ 真实 ADR id 集合 —— 所以该字段里不能放
> `OWNER-DECISION-C-ROUTE`、`ADR-DECISION-RECORD.md#<id>` 这类非 ADR token。
> 裁决引用（路由、日期、账本锚点）放**另一个字段** `Approval:`，该字段不被解析、不被断言。

- [ ] **Step 3: 并入 ADR-410-011**

```bash
cd /Volumes/workspace/dev/wt-v410-base
git mv docs/releases/v4.1.0/refactoring/runtime-contract/20261120-pr5915/ADR-410-011-UNIFIED_RUNTIME_CONTRACT.md \
       docs/releases/v4.1.0/refactoring/adrs/ADR-410-011.md
```
其第 6 行状态改为 `Status: CONDITIONALLY_APPROVED（7 项补充验收契约冻结前不授权实现）`，`Approvals` 填裁决引用。
**不得**把 011 写入任何 task 的 `approved_adr_refs`（O7 裁决 + A2.2 规则 4）。

- [ ] **Step 4: 同步 ADR_INDEX.md**

状态列 11 行按 Step 1 的档位填写；第 7 行说明改为「11 份 ADR：001-010 + 011（条件批准，前置未满足）；011 已并入编目但不进入 `approved_adr_refs`」；第 3-4 行 `evidence_hash` 改为实际值。

- [ ] **Step 5: 填充 `approved_adr_refs`（18 处）**

依 `ADR_INDEX.md` 的依赖图 + 每个 task 的 `invariant_ids`/`target_files`，为 18 个 task 填入关联 ADR。**必须满足 A2.2 五条规则**：

| 规则 | 判定 |
|---|---|
| R1 | 每个 task 至少 1 项 |
| R2 | 引用项必须与该 task 的 `invariant_ids`/`target_files` 实际关联 |
| R3 | **不得**引用 003/004/007/008/009/010（仅批准方向）于 `authorized_for_execution=true` 的 task |
| R4 | **不得**引用 006/011（条件批准，前置未满足）于任何 task |
| R5 | 只能引用 `adrs/` 中真实存在的 id |

**若映射无法满足 R1–R5**（例如某 task 与任何已批准 ADR 都无关联），则该 task 的 `approved_adr_refs` 保持 `[]`，
并把该 task_id 记入 `docs/refactor/v4.1.0-c/takeover/ADAPTER-MAPPING-PENDING.md`——
**不得**为凑数填入无关 ADR。该清单将在 Task 9 的放行裁决记录中作为残余未裁决项呈交属主（spec O1/O10）。
> A2.2 的五条规则本身已在设计文档中固化并经属主审阅，执行者**按规则填映射**，
> 无需就规则本身再行请示。属主只在放行裁决时对残余项拍板。

- [ ] **Step 6: 重签 manifest**

本任务修改了 `adrs/`、`ADR_INDEX.md`、`REFACTOR_TASK_DAG.json`——全部在 `finalize()` 范围内，
故**必须先重签再校验**：

```bash
cd /Volumes/workspace/dev/wt-v410-base && python3 docs/releases/v4.1.0/refactoring/tools/verify_handoff.py --finalize
```
Expected: 退出 0；`10_HANDOFF_MANIFEST.json` 与 `01_ARCHITECTURE_BASELINE.json` 被重写，后者 11 条 ADR 的 `status` 非 `PROPOSED`

- [ ] **Step 7: 校验映射规则与全链**

```bash
cd /Volumes/workspace/dev/wt-v410-base && python3 docs/releases/v4.1.0/refactoring/tools/verify_handoff.py --check
```
Expected: 退出 0；8+1 变异全部被拒（含 Task 3 Step 7 新增的「档位越权」变异臂 —— Review Focus #2）；
`assert len(cases)==75` 成立；Markdown 链接校验通过

- [ ] **Step 8: 提交**

```bash
cd /Volumes/workspace/dev/wt-v410-base
git add docs/releases/v4.1.0/refactoring/adrs/ docs/releases/v4.1.0/refactoring/ADR_INDEX.md \
        docs/releases/v4.1.0/refactoring/REFACTOR_TASK_DAG.json \
        docs/releases/v4.1.0/refactoring/tools/ \
        docs/refactor/v4.1.0-c/handoff/
git commit -m "docs(v4.1.0): ADR 批准落地并重签 handoff manifest"
```

---

### Task 5: 修复 75 场景矩阵并建立矩阵驱动适配层

**Files:**
- Modify: `docs/releases/v4.1.0/refactoring/TEST_COVERAGE_MATRIX.json`（75 条的 `invariant_ids` / `invariant_mapping_status` / `case_id`）
- Create: `docs/releases/v4.1.0/refactoring/tools/matrix_driver.py`
- Create: `docs/releases/v4.1.0/refactoring/tools/verify_matrix.py`
- Modify: `docs/releases/v4.1.0/refactoring/tools/requirements.txt`

**Interfaces:**
- Consumes: `TRANSACTION_INVARIANTS.md`、`STORAGE_CAPABILITY_CONTRACT.md` 的不变量 id；`tools/test_factory.py`（在 `/Volumes/workspace/dev/srg-test-factory-pilot/`，**须先迁入基线**）；`05_TEST_FACTORY_CONTRACT.json` 的 7 类词汇
- Produces: `matrix_driver.py` 的 `run_slice(repo: Path, matrix: Path, cases: list[str], out_dir: Path) -> list[dict]`，每条返回含 `case_id` / `execution_status` / `actual` / `artifact_sha256` / `red_identity` 的字典；`verify_matrix.py` 退出码 0 表示矩阵合规

**CLI 与库函数的关系（Task 6 会用到）**：`matrix_driver.py` 同时提供命令行入口，
但**命令行入口只是 `run_slice()` 的薄包装**，不重复实现任何逻辑。
CLI 的 `--matrix` 传 `matrix` 参数，`--slice T1,T2,T3` 解析为 `cases` 列表（元素为 `matrix_index`），
`--out` 传 `out_dir`，`--repo` 缺省为当前工作树根。

- [ ] **Step 1: 迁入 test_factory 与其依赖测试源**

把 `/Volumes/workspace/dev/srg-test-factory-pilot/tools/test_factory.py` 复制到
`docs/releases/v4.1.0/refactoring/tools/test_factory.py`，
把 `/Volumes/workspace/dev/srg-test-factory-pilot/tests/integration/test_factory/tf_t1_*.rs`、`tf_t4_*.rs`
复制到基线的 `tests/integration/test_factory/`。
**注意**：pilot 的 `Cargo.toml` 处于未提交修改态——只复制 `.rs` 文件，**不复制 `Cargo.toml`**。

- [ ] **Step 2: 写矩阵校验器**

创建 `verify_matrix.py`，实现：

| 断言 | 判定 |
|---|---|
| M1 | 75 条 `cases[]` 的 `invariant_ids` 全部非空 |
| M2 | 75 条的 `invariant_mapping_status` 全部 ≠ `UNREVIEWED` |
| M3 | `case_id` 在 75 条内唯一（`PILOT-T6-001`/`PILOT-T6-002` 各重复一次） |
| M4 | 每个 `invariant_ids` 元素都能在 `TRANSACTION_INVARIANTS.md` 或 `STORAGE_CAPABILITY_CONTRACT.md` 中找到 |
| M5 | `execution_status` ∈ 7 类契约词汇 ∪ pilot 词汇（映射前允许 pilot 词汇） |
| M6 | 每条 `KNOWN_FAIL` 具备完整 RED 五元组：`red_identity{test_fn, package_or_target, source_sha256, rc, attribution}` |

```bash
cd /Volumes/workspace/dev/wt-v410-base && python3 docs/releases/v4.1.0/refactoring/tools/verify_matrix.py
```
Expected: M1–M3、M6 FAIL（矩阵尚未修复）；M4、M5 PASS 或给出具体不匹配项

- [ ] **Step 3: 填 75 条 invariant 映射**

为每条 case 填 `invariant_ids`：从该 case 的 `source.declared_invariant`（如「undo log 语义：回滚后状态 == 事务前」）映射到 `TRANSACTION_INVARIANTS.md` / `STORAGE_CAPABILITY_CONTRACT.md` 中的实际不变量 id。
同时把 `invariant_mapping_status` 由 `UNREVIEWED` 改为 `MAPPED`。

**`required_for_certification` 本任务不动**（保持 75/75 `false`）：该字段是阶段出口口径，
属主专属（O2），执行者代填等于自签。在 Task 6 完成后向属主提请标定。

- [ ] **Step 4: 唯一化 case_id**

重复的 `PILOT-T6-001`（×2）、`PILOT-T6-002`（×2）改为唯一 id（如 `PILOT-T6-001`、`PILOT-T6-001B`、`PILOT-T6-002`、`PILOT-T6-002B`），并在 `case-mapping` 记录中同步。
**不得**改动 `case_id` 之外的语义字段。

- [ ] **Step 5: 写七态映射表**

在矩阵 JSON 顶层新增 `status_vocabulary_mapping`：

```json
{"PASS":"PASS","COVERED_PASS":"PASS","FAILED_RED":"KNOWN_FAIL","COVERED_RED":"KNOWN_FAIL",
 "FAIL":"FAIL","NOT_RUN":"NOT_RUN","INFRA_ERROR":"INFRA_ERROR",
 "UNSUPPORTED":"UNSUPPORTED","DIALECT_DIFFERENCE":"DIALECT_DIFFERENCE"}
```

`FAILED_RED`/`COVERED_RED` → `KNOWN_FAIL` 的映射**必须同时保留 `red_identity` 五元组**（Review Focus #3）。

- [ ] **Step 6: 写矩阵驱动适配层**

创建 `matrix_driver.py`：

```python
def run_slice(repo: Path, matrix: Path, cases: list[str], out_dir: Path) -> list[dict]:
    """按 cases 指定的 matrix_index 集合执行，返回每条的
    {case_id, execution_status, actual, artifact_sha256, red_identity}。
    execution_status 取 7 类契约词汇；FAILED_RED/COVERED_RED 映射为 KNOWN_FAIL
    并回填 red_identity 五元组。"""
```

行为约定：
- 一次调用 = 一个 package + 一个 target + N 个 `--filter`，由 `matrix_driver.py` 从 case 列表聚合
- 调用 `/Volumes/workspace/dev/wt-v410-base/docs/releases/v4.1.0/refactoring/tools/test_factory.py run --repo <repo> --suite <suite> --out <out_dir>`
- 3 个非 cargo 场景（`PILOT-T6-001` kill-9、`PILOT-T6-002` checkpoint-torn-write、`PILOT-T5-101` odkU-rollback-noop）走各自 bespoke 探针；探针未迁入前，这 3 条记 `NOT_RUN` 并在 `actual` 写明原因
- `test_factory.py` 返回零测试时，`execution_status` 必须是 `NOT_RUN`（其既有反例修复），**不得**是 `PASS`

- [ ] **Step 7: 写适配层单元测试**

创建 `docs/releases/v4.1.0/refactoring/tools/test_matrix_driver.py`，至少覆盖 4 个用例：

| 用例 | 断言 |
|---|---|
| D1 | pilot 词汇 `FAILED_RED` 输入 → 输出 `execution_status == "KNOWN_FAIL"` |
| D2 | 同上 → 输出 `red_identity` 五元组齐全且非空 |
| D3 | `COVERED_PASS` 输入 → 输出 `PASS` 且带覆盖来源标注 |
| D4 | test_factory 报告零测试 → 输出 `NOT_RUN` 且 `execution_status != "PASS"` |

```bash
cd /Volumes/workspace/dev/wt-v410-base && python3 docs/releases/v4.1.0/refactoring/tools/test_matrix_driver.py
```
Expected: 4/4 PASS

- [ ] **Step 8: 运行矩阵校验器**

```bash
cd /Volumes/workspace/dev/wt-v410-base && python3 docs/releases/v4.1.0/refactoring/tools/verify_matrix.py
```
Expected: M1–M6 全 PASS，退出码 0

- [ ] **Step 9: 提交**

```bash
cd /Volumes/workspace/dev/wt-v410-base
git add docs/releases/v4.1.0/refactoring/TEST_COVERAGE_MATRIX.json \
        docs/releases/v4.1.0/refactoring/tools/ \
        tests/integration/test_factory/
git commit -m "feat(v4.1.0): 矩阵驱动适配层与 75 场景不变量映射"
```

- [ ] **Step 10: 重签 manifest**

本任务向 `docs/releases/v4.1.0/refactoring/tools/` 迁入了 `test_factory.py` 与新增了 3 个工具脚本，
该目录在 `finalize()` 哈希范围内，故须重签：

```bash
cd /Volumes/workspace/dev/wt-v410-base
python3 docs/releases/v4.1.0/refactoring/tools/verify_handoff.py --finalize
python3 docs/releases/v4.1.0/refactoring/tools/verify_handoff.py --check
```
Expected: 两条命令均退出 0。**注意**：`tests/integration/test_factory/` 不在 finalize 范围内，
故其中的 `.rs` 文件不进入 manifest——这是设计使然（`finalize()` 只哈希两个 docs 前缀）。

---

### Task 6: 执行 L1 风险切片并产出双向差异表

**Files:**
- Create: `docs/releases/v4.1.0/refactoring/evidence/L1-RERUN-<BASE前12位>.json`（逐条结果）
- Create: `docs/releases/v4.1.0/refactoring/evidence/L1-DIFF-<BASE前12位>.md`（差异表）
- Modify: `docs/releases/v4.1.0/refactoring/TEST_COVERAGE_MATRIX.json`（`execution_status` / `actual` / `artifact_sha256` / `red_identity`）

**Interfaces:**
- Consumes: `matrix_driver.run_slice()`（Task 5）、`pilot/evidence/rerun-summary-20261120.json` 与 `case-mapping-20261120.json`、`takeover/dsh-b/RED-LEDGER.md` + `RED-IDENTITY-RECONCILIATION.md`
- Produces: 逐条结果 JSON + 差异表 Markdown。Task 7 的签署依据、Task 9 的 A5 #6 判据依赖这两份产物。

- [ ] **Step 1: 确定 L1 切片**

L1 = feature ∈ {T1, T2, T3} 的 44 条（T1:30 T2:6 T3:8）。

```bash
cd /Volumes/workspace/dev/wt-v410-base && python3 -c "
import json
cs=json.load(open('docs/releases/v4.1.0/refactoring/TEST_COVERAGE_MATRIX.json'))['cases']
l1=[c for c in cs if c['feature'] in ('T1','T2','T3')]
print(len(l1)); print([c['case_id'] for c in l1][:5])
"
```
Expected: `44`

- [ ] **Step 2: 在 BASE 上重取测试清单**

矩阵中 ~66 条引用 `cargo test --list @ e9ff21fc`（242 提交前的快照）。在 BASE 上重新取：

```bash
cd /Volumes/workspace/dev/wt-v410-base && cargo test --list --all-features --lib > /tmp/list-base.txt 2>/dev/null; wc -l /tmp/list-base.txt
```
逐条比对矩阵 `source.test_name` 是否仍存在于新清单。**不存在的记 `NOT_RUN`，
并在 `actual` 写明「测试在 BASE 上不存在（TEST_VANISHED）」**——
7 类契约无此类，**不得擅自新增类别**；该情形在 Task 9 的放行裁决记录中作为残余项呈交属主（spec O16）。

- [ ] **Step 3: 执行 L1 切片**

```bash
cd /Volumes/workspace/dev/wt-v410-base
python3 docs/releases/v4.1.0/refactoring/tools/matrix_driver.py \
  --matrix docs/releases/v4.1.0/refactoring/TEST_COVERAGE_MATRIX.json \
  --slice T1,T2,T3 \
  --out docs/releases/v4.1.0/refactoring/evidence/
```
Expected: 生成 `L1-RERUN-<BASE前12位>.json`，含 44 条逐条结果

- [ ] **Step 4: 回写矩阵**

把 44 条的 `execution_status` / `actual` / `artifact_sha256` 写回矩阵；`KNOWN_FAIL` 的补 `red_identity` 五元组。

- [ ] **Step 5: 生成与 pilot 的差异表**

对比 `L1-RERUN-<BASE前12位>.json` 与 `/Volumes/workspace/dev/srg-test-factory-pilot/evidence/rerun-summary-20261120.json`（base `e9ff21fca3`，75 案例 70 PASS / 5 red_confirmed）+ `case-mapping-20261120.json`。
每条差异须分类：`新增 RED` / `RED 消失` / `状态降级` / `PASS→FAIL` / `NOT_RUN`（含原因）。
**任何 `PASS→FAIL` 必须是新发现真缺陷并单独登记，不得静默归入已知 RED。**

- [ ] **Step 6: 生成与 RED-LEDGER 的差异表**

对比 `takeover/dsh-b/RED-LEDGER.md` 的 **17 项 RED 五元组**与本次结果。
每项在本次结果中的对应状态须可定位（`KNOWN_FAIL` 且五元组匹配）。
**RED 消失须解释原因**（已修复 / 断言失效 / 未覆盖）——不得静默少一项（Review Focus #3）。

- [ ] **Step 7: 合并为单一差异表**

把 Step 5、6 合并进 `L1-DIFF-<BASE前12位>.md`，表头固定为：
`| matrix_index | case_id | feature | pilot状态 | ledger状态 | BASE状态 | 差异类型 | 说明 |`

- [ ] **Step 8: 校验**

```bash
cd /Volumes/workspace/dev/wt-v410-base && python3 docs/releases/v4.1.0/refactoring/tools/verify_matrix.py
```
Expected: 退出 0。**L1 的 44 条全部有终态**；未执行的 T4/T5/T6 共 31 条为 `NOT_RUN` 且 `actual` 含原因。

- [ ] **Step 9: 提交**

```bash
cd /Volumes/workspace/dev/wt-v410-base
git add docs/releases/v4.1.0/refactoring/TEST_COVERAGE_MATRIX.json docs/releases/v4.1.0/refactoring/evidence/
git commit -m "test(v4.1.0): L1 风险切片复跑并产出双向差异表"
```

---

### Task 7: Oracle 签署（两段）

**Files:**
- Modify: `docs/releases/v4.1.0/refactoring/TEST_COVERAGE_MATRIX.json`（每条 `assertion_reviewed_by` / `assertion_reviewed_at` / `assertion_source_sha256`）
- Create: `docs/refactor/v4.1.0-c/takeover/ORACLE-REVIEW-LOG-T1.md`（第 1 段记录）

**Interfaces:**
- Consumes: Task 6 的 L1 结果 + 断言文本 + 测试源
- Produces: 每条 `PASS` 具备签署人/时间/源 sha256，且签署人 ≠ 实现者（O6）

- [ ] **Step 1: 第 1 段 —— 独立 Reviewer 逐条签 T1（30 条）**

签署人：**属主授权的独立 Reviewer**（Codex 或 DSH Reviewer 池），**不得**是该场景的实现者，
**且不得**是执行本次复跑的 agent。签署人标识须具名并写入 `assertion_reviewed_by`。

> **属主不介入签署环节**（属主已明确仅在最终放行裁决时介入）。
> 若独立 Reviewer 不可得，本任务**不得**由实施者自签——那会使 R-ORACLE 失效。
> 此时应把该任务标记为 BLOCKED 并在 Task 9 的放行裁决记录中列为残余项。

对 T1 的 30 条，逐条：
- 读断言文本与测试源（**不读执行日志**）
- 判定断言是否可独立验证
- 填 `assertion_reviewed_by` = Reviewer 标识、`assertion_reviewed_at` = ISO8601、`assertion_source_sha256` = 测试源文件 sha256
- 在 `ORACLE-REVIEW-LOG-T1.md` 记录：`case_id` / 签署人 / 判定 / 备注

**若发现某条断言在不读实现的前提下无法独立判定**（如「回滚后状态 == 事务前」未指明比较哪些字段），
该条**不得**标记 `PASS`，须记为 `ASSERTION_UNREVIEWABLE` 并列入待修断言清单——此时 T1 的 `PASS` 不计入阶段出口。

- [ ] **Step 2: 校验第 1 段**

```bash
cd /Volumes/workspace/dev/wt-v410-base && python3 -c "
import json,collections
cs=json.load(open('docs/releases/v4.1.0/refactoring/TEST_COVERAGE_MATRIX.json'))['cases']
t1=[c for c in cs if c['feature']=='T1']
bad=[c['case_id'] for c in t1 if c.get('execution_status')=='PASS' and not (c.get('assertion_reviewed_by') and c.get('assertion_reviewed_at') and c.get('assertion_source_sha256'))]
print('T1 未签署的 PASS:',bad)
"
```
Expected: 输出 `[]`（空列表）

- [ ] **Step 3: 第 2 段 —— 独立 Reviewer 批量签 T2/T3（14 条）**

**前提**：第 1 段流程验证通过（Step 1 无 `ASSERTION_UNREVIEWABLE`）。
由属主授权的独立 Reviewer（Codex 或 DSH Reviewer 池）按同规则签 T2 + T3 共 14 条。
签署人须 ≠ 该场景实现者（O6 / R-ORACLE），且不得是执行复跑的 agent。

- [ ] **Step 4: 校验第 2 段并确认签署人独立**

```bash
cd /Volumes/workspace/dev/wt-v410-base && python3 -c "
import json
cs=json.load(open('docs/releases/v4.1.0/refactoring/TEST_COVERAGE_MATRIX.json'))['cases']
l1=[c for c in cs if c['feature'] in ('T1','T2','T3')]
print('L1 有终态:',sum(1 for c in l1 if c.get('execution_status')),'/',len(l1))
print('签署人集合:',sorted({c.get('assertion_reviewed_by') for c in l1 if c.get('execution_status')=='PASS'}))
"
```
Expected: `L1 有终态: 44 / 44`；签署人集合中不含该批场景的实现者标识

- [ ] **Step 5: 提交**

```bash
cd /Volumes/workspace/dev/wt-v410-base
git add docs/releases/v4.1.0/refactoring/TEST_COVERAGE_MATRIX.json docs/refactor/v4.1.0-c/takeover/ORACLE-REVIEW-LOG-T1.md
git commit -m "test(v4.1.0): T1/T2/T3 Oracle 签署落盘"
```

---

### Task 8: ALPHA 阶段执行点与 7 道负向夹具

**Files:**
- Modify: `.gitea/workflows/ci.yml`（`stage-gates` job 内新增 ALPHA 执行点）
- Modify: `audit/v4.1.0/neg/`（新增/复用 7 道 ALPHA 门禁的坏产物夹具）
- Modify: `docs/releases/v4.1.0/GATE_CATALOG.jsonl`（`neg_fixture` 与 `effect` 声明对齐）
- Modify: `scripts/gate/gate_registry.py`（C2/C3/C5 判据与派生规则）

**Interfaces:**
- Consumes: `STAGE_CONFIG.yaml:88-121` 的 ALPHA 清单（7 个 `check_*.sh` + 3 条 cargo）、`audit/v4.1.0/neg/` 已有 fx01…fx14
- Produces: ALPHA 执行点（CI 内可指认）、7 道门禁的负向夹具证据、`GATE_CATALOG.jsonl` 内部自洽

- [ ] **Step 1: 盘点 ALPHA 7 道与已有夹具**

```bash
cd /Volumes/workspace/dev/wt-v410-base && python3 -c "
import json
rows=[json.loads(l) for l in open('docs/releases/v4.1.0/GATE_CATALOG.jsonl') if l.strip()]
al=[r for r in rows if 'ALPHA' in (r.get('stages') or [])]
for r in al: print(r['gate'], '| swallow=',r['swallow_hits'], '| neg_fixture=',r['neg_fixture'], '| effect.catches=',(r.get('effect') or {}).get('catches'))
"
ls /Volumes/workspace/dev/wt-v410-base/audit/v4.1.0/neg/
```
Expected: 7 行 ALPHA 门禁记录；`neg/` 下 14 个 fxNN

- [ ] **Step 2: 对齐编目的 `neg_fixture` 与 `effect`**

当前 7 条记录 `neg_fixture=false` 但 `effect.catches` 非空且 reason 写明「由夹具 fxNN-… 证明（kind=mutation-verified）」。
以**夹具实物**为准：改 `gov_probe.NEG_FIXTURE_RE` 的派生规则使其能识别这些夹具，
并使 `gate_registry.py check` 的 C2/C3/C5 判据从「派生值非空」改为「派生值与人工 `effect` 声明一致」。

```bash
cd /Volumes/workspace/dev/wt-v410-base && python3 scripts/gate/gate_registry.py check
```
Expected: 退出 0；`GATE_CATALOG.jsonl` 内 `neg_fixture` 与 `effect` 声明一致

- [ ] **Step 3: 为 7 道门禁各备一个坏产物**

`audit/v4.1.0/neg/` 已有可复用者：`fx02-anti-ignore-registry-drift`、`fx08-alpha-entry-doc-missing`、`fx14-stage-version-uninferable`。
其余 4 道（`check_alpha_v410.sh` 复合、`check_arch3_no_bypass.sh`、`check_arch_invariants.sh`、`check_coverage_v312.sh`、`check_sql_corpus_gate.sh`）
各新造 1 个最小坏产物夹具，命名 `fxNN-<gate>-<缺陷类型>`。

每个夹具的形状固定为：一份能让该门禁判 FAIL 的最小输入 + 一份 `expected`（期望的非零退出码）。

- [ ] **Step 4: 写「两层 RED」验证脚本**

创建 `audit/v4.1.0/neg/verify_neg_fixtures.sh`，对 7 道门禁逐道：
1. 在隔离目录中注入坏产物
2. 运行该门禁脚本 → 断言退出码非零（**门禁层 RED**）
3. 把该门禁的输出喂给阶段汇总面 → 断言汇总结论非 PASS（**阶段汇总层 RED**）

```bash
cd /Volumes/workspace/dev/wt-v410-base && bash audit/v4.1.0/neg/verify_neg_fixtures.sh
```
Expected: 7/7 每道输出「gate=RED stage=RED」，脚本退出 0

**关键**：两层必须是**同一次执行的两级**（坏产物 → 门禁输出 → 汇总判定），
不是各自独立跑一次。

- [ ] **Step 5: 加 ALPHA 执行点到 CI**

在 `.gitea/workflows/ci.yml` 的 `stage-gates` job 内新增一条：

```yaml
      - name: Stage gates — ALPHA list executes for real (develop/v* carrier)
        if: startsWith(github.ref, 'refs/heads/develop/')
        run: bash scripts/gate/check_stage.sh --version v4.1.0 --stage ALPHA
```

**约束（C8）**：若 `develop/v4.1.0` 当前为已知红分支，此 step 会在每次 push 把 CI 变红。
按 O17 裁决确定使用两态的 `check_stage.sh` 还是三态的 `run_gates.py` + `gate_report.py`；
若采用后者，命令为 `python3 scripts/gate/run_gates.py execute --version v4.1.0 --stage ALPHA`。

**禁止**使用 `--dry-run` 作为执行点——dry-run 只解析不执行，会得到假绿（Review Focus #5）。

- [ ] **Step 6: 验证执行点确实执行**

```bash
cd /Volumes/workspace/dev/wt-v410-base
bash scripts/gate/check_stage.sh --version v4.1.0 --stage ALPHA   # 期望非零（已知红）
bash scripts/gate/check_stage.sh --version v4.1.0 --dry-run | grep -c "check_alpha_v410.sh"
```
Expected: 第一条非零退出（证明真的执行了）；第二条 > 0（证明清单被解析到）

- [ ] **Step 7: 提交**

```bash
cd /Volumes/workspace/dev/wt-v410-base
git add .gitea/workflows/ci.yml audit/v4.1.0/neg/ docs/releases/v4.1.0/GATE_CATALOG.jsonl scripts/gate/gate_registry.py scripts/gate/gov_probe.py
git commit -m "feat(gate): ALPHA 阶段执行点与 7 道负向夹具"
```

---

### Task 9: 放行判据汇总、R2 外审包与放行裁决记录

**Files:**
- Create: `docs/refactor/v4.1.0-c/takeover/V410-READINESS-VERDICT.json`（12 条判据的逐条判定）
- Create: `docs/refactor/v4.1.0-c/takeover/V410-OWNER-RULING.md`（放行裁决 + 豁免记录）
- Create: `docs/refactor/v4.1.0-c/takeover/R2-RELEASE-PACKAGE.md`（R2 包生成说明）
- Regenerate: R2 外审包（在 `…/DSH-EXTERNAL-REVIEW-001/` 下，R1 不动）

**Interfaces:**
- Consumes: Task 1–8 的全部产物与校验器
- Produces: 12 条判据的可核判定 + 属主放行/不放行的书面裁决

- [ ] **Step 1: 逐条跑判据校验**

按下列映射执行 12 条判据，产出 `V410-READINESS-VERDICT.json`（每条含 `id` / `criterion` / `command` / `observed` / `verdict`）：

| # | 判据 | 命令 |
|---|---|---|
| 1 | BASE（内容基线）唯一、develop tip 是其祖先、含 BASE 的分支已推送 | `python3 docs/releases/v4.1.0/refactoring/tools/verify_baseline.py`（断言 B1+B2+B4）+ `git ls-remote origin refs/heads/base/v4.1.0-frozen`（其 SHA 须是 `base_sha` 的后代） |
| 2 | 45da 的 123 个材料文件全在 BASE 树内 | 同上脚本的断言 **B3**（`git ls-tree -r --name-only 45da6968ae` 与 HEAD 求差集为空） |
| 3 | 前向/历史 SHA 分界 | `python3 docs/releases/v4.1.0/refactoring/tools/verify_sha_scope.py` |
| 4 | ADR 字段与映射规则 | `python3 docs/releases/v4.1.0/refactoring/tools/verify_handoff.py --check` |
| 5 | manifest 零漂移 | 同上（`--check` 含 manifest 全文件哈希复算） |
| 6 | L1 有终态 + Oracle 签署 + 双向差异表 | `python3 docs/releases/v4.1.0/refactoring/tools/verify_matrix.py` + 存在 `L1-DIFF-*.md` 且表头为 `\| matrix_index \| case_id \| feature \| pilot状态 \| ledger状态 \| BASE状态 \| 差异类型 \| 说明 \|` |
| 7 | ALPHA 阶段清单有 CI 执行点 | `.gitea/workflows/ci.yml` 中存在 `--stage ALPHA` 的 step，且其 `if:` 匹配 `refs/heads/develop/` |
| 8 | ALPHA 7 道负向夹具两层 RED | `bash audit/v4.1.0/neg/verify_neg_fixtures.sh` |
| 9 | 门禁编目内部自洽 | `python3 scripts/gate/gate_registry.py check` |
| 10 | 三套统计一致（71/71/71） | R2 包内校验（Step 3） |
| 11 | dead 口径唯一化 | R2 包内全文检索「61 道…全部有文档引用」 |
| 12 | LOCAL_ONLY 路由 fail-closed | **显式豁免**（O9） |

**判据 6 与 O2 的关系**：`required_for_certification` 的属主标定（O2）尚未完成。
若放行时 O2 仍未裁决，判据 6 以「L1 44 条有终态 + Oracle 签署 + 差异表」为满足，
并在 `V410-OWNER-RULING.md` 的「残余未裁决项」段标注 O2 未裁决及其对阶段出口口径的影响。

- [ ] **Step 2: 生成 R2 包说明**

`R2-RELEASE-PACKAGE.md` 须写明：R1 保持不动（O8）；R2 的 `source_commit` 重指到 BASE；
三套统计由 `EVIDENCE-INDEX.json` **单向生成**（禁止手抄）；MANIFEST/SHA256SUMS 的自排除规则显式声明。

- [ ] **Step 3: 生成 R2 包并校验**

用 `finalize_package.py` 与 `gen_export_approval.py`（在 `…/DSH-EXTERNAL-REVIEW-001/sqlrustgo-v4.1.0/` 下）生成 R2，**不触碰 R1 的 `package/` 目录**。

```bash
cd /Volumes/workspace/dev/workspaces/dsh-ops/projects/DSH-EXTERNAL-REVIEW-001/sqlrustgo-v4.1.0
python3 finalize_package.py
python3 gen_export_approval.py
```
校验：
- `EVIDENCE-INDEX.json` / `EVIDENCE-COVERAGE.md` / `01-EXECUTIVE-SUMMARY.md` 三者统计**均为 71 且等级分布均为 24/41/6**
- R2 全文无「61 道…全部有文档引用」
- `MANIFEST.json` 显式声明 MANIFEST/SHA256SUMS 的自排除
- **R1 的 `FREEZE.json` 与 `package/` 内容逐字节未变**

- [ ] **Step 4: 写放行裁决记录**

`V410-OWNER-RULING.md` 须含：

| 段 | 内容 |
|---|---|
| 判据汇总 | 12 条逐条 PASS/FAIL/WAIVED |
| **豁免记录** | `waiver_id` / `waived_criterion: 12` / `reason`（跨仓库 LOCAL_ONLY 不纳入本次准入）/ `scope` / `compensating_control`（调度侧显式指定 LOCAL_ONLY 策略）/ `expires_at` / `approved_by` |
| 残余未裁决项 | spec §6.2 的 13 项，标注各自是否影响放行。**至少须含**：O1（`ADAPTER-MAPPING-PENDING.md` 中的 task 列表，若非空则 A5 #4 不能判 PASS）、O2（`required_for_certification` 未标定）、O16（TEST_VANISHED 场景数） |
| 结论 | 属主裁决：**放行 C-ALPHA-02** 或 **不放行**，并说明放行批次（spec O5） |

**不得**在任一判据为 FAIL 时写「放行」。

- [ ] **Step 5: 提交**

```bash
cd /Volumes/workspace/dev/wt-v410-base
git add docs/refactor/v4.1.0-c/takeover/V410-READINESS-VERDICT.json \
        docs/refactor/v4.1.0-c/takeover/V410-OWNER-RULING.md \
        docs/refactor/v4.1.0-c/takeover/R2-RELEASE-PACKAGE.md
git commit -m "docs(v4.1.0): 准入判据汇总与放行裁决记录"
```

---

## 执行顺序与依赖

```
Task 1 (基线)  ──► Task 2 (SHA 重指) ──► Task 3 (断言闭包) ──► Task 4 (ADR+重签)
                      │                        │
                      └──► Task 5 (矩阵+driver)┘──► Task 6 (L1 复跑) ──► Task 7 (签署)
                                                        │
Task 8 (ALPHA 门禁) ─────────────────────────────────────┤
                                                        └──► Task 9 (判据+R2+放行)
```

- Task 8 与 Task 1–7 无依赖，可并行（但都在同一 worktree 内，须串行提交）
- Task 3 与 Task 5 无依赖，可并行
- **Task 9 必须在全部前置完成后执行**

## 验收

全部 9 个任务完成，且下列全部成立：

```bash
cd /Volumes/workspace/dev/wt-v410-base
python3 docs/releases/v4.1.0/refactoring/tools/verify_baseline.py --strict && \
python3 docs/releases/v4.1.0/refactoring/tools/verify_sha_scope.py   && \
python3 docs/releases/v4.1.0/refactoring/tools/verify_matrix.py      && \
python3 docs/releases/v4.1.0/refactoring/tools/verify_handoff.py --check && \
python3 scripts/gate/gate_registry.py check                           && \
bash audit/v4.1.0/neg/verify_neg_fixtures.sh
```
全部退出 0，且 `V410-READINESS-VERDICT.json` 中 11 条 PASS + 1 条 WAIVED(#12)。

---

### Task 10: 逐用例重做 invariant 映射并修复不可判定断言

**Files:**
- Modify: `docs/releases/v4.1.0/refactoring/TEST_COVERAGE_MATRIX.json`（75 条的 `invariant_ids` / `source.declared_invariant` / 新增 `invariant_mapping_basis`；不改 `red_identity` / `execution_status` / `artifact_sha256` / `assertion_*`）
- Modify: `docs/releases/v4.1.0/refactoring/tools/verify_matrix.py`（新增 M7：分桶映射检测；契约由 6/6 变 7/7）
- Modify: `crates/storage/src/adaptive_hash_index.rs`、`crates/storage/src/buffer_pool.rs`（**仅 R3 那 2 条的测试函数体**，见 Step 5）
- Modify: `docs/refactor/v4.1.0-c/takeover/ORACLE-REVIEW-LOG-T1.md`（记录映射重做后的可判定性预判）

**Interfaces:**
- Consumes: `ORACLE-REVIEW-LOG-T1.md` 的 §3（30 条 T1 判定）、§4.2（14 条 T2/T3 判定）、§5（38 条待修断言清单 + R1/R2/R3 分档 + 逐条修复建议）、§6（结构性根因）；`TRANSACTION_INVARIANTS.md` 的 18 个不变量；各 case 的 `source.test_name` 指向的测试源码
- Produces: 75 条**逐用例**的 `invariant_ids` + 每条的判定依据；38 条断言的修复；可判定性预判。M7 断言防止分桶映射回归

- [ ] **Step 1: 读修复清单**

先读 `ORACLE-REVIEW-LOG-T1.md` 的 §5（38 条待修断言清单，含 R1/R2/R3 分档：R1 32 条「改断言文本即可」、R2 4 条「需改映射 + 改断言文本」、R3 2 条「需改测试」）与 §6（结构性根因）。**这是本任务的权威输入，不要自行重新判定那 38 条。**

- [ ] **Step 2: 逐用例重建 75 条的 invariant 映射**

Task 5 的映射是**按 feature 分桶批量贴的**——`declared_invariant` 全 75 条只有 13 种取值、逐 feature 为 T1 3 / T2 1 / T3 1 / T4 3 / T5 2 / T6 3（L1 子集 5 种，T1 的 28/30 共用同一句），Task 5 按其规则「declared_invariant 相同的整组用同一组 id」贴 `invariant_ids`，**从未逐用例核对**。后果：T1 组里混进了 `test_classify_sql_transaction` 这类与「undo log 语义」无关的测试。

对**全部 75 条**（不只是 L1 的 44 条——T4/T5/T6 的 31 条同样是分桶贴的）逐条重建：

1. 读该 case 的 `source.test_name` 指向的**测试函数源码**；
2. 判定它实际断言了什么（不是它声称要断言什么）；
3. 在 `TRANSACTION_INVARIANTS.md` 的 18 个不变量里找出与之**真正对应**的 id —— 可以是 0 个、1 个或多个；
4. 把依据写进该 case 的新字段 `invariant_mapping_basis`（一句可被独立复核的话，指明测试函数断言了哪个不变量的哪一面）。

**若某个测试函数实际断言的内容不对应任何现有不变量**，`invariant_ids` 留空 `[]` 并把该 case 记入待新建的不变量提案清单（`docs/refactor/v4.1.0-c/takeover/INVARIANT-GAP-PROPOSALS.md`）——**不得**为了非空而贴一个不对应的 id。`verify_matrix.py` 的 M4 只要求「引用的 id 必须存在」，不要求非空；空映射会由新增的 M7 单独报告。

- [ ] **Step 3: 修复 38 条不可判定断言的文本**

按 §5 的逐条建议改 `source.declared_invariant`，使断言在**只读断言文本 + 测试源码**的条件下可独立判定：

- **R1（32 条）**：补全断言文本指明比较哪些字段 / 哪个状态 / 哪个边界，而不是「回滚后状态 == 事务前」这类未指明比较对象的表述。
- **R2（4 条）**：先按 Step 2 收窄 `invariant_ids`（只留该用例真正触及的），再修断言文本。
- **R3（2 条）**：见 Step 5。

**不得**为了让数字好看而放宽判定标准，也不得把断言改写成测试**恰好**做的事而不提及其余部分——若测试只覆盖了声明的一部分，断言文本必须如实写明覆盖范围。

- [ ] **Step 4: 记录可判定性预判**

对 38 条修复后的 case 逐条给出**可判定性预判**（可判定 / 仍不可判定 + 理由），写入 `ORACLE-REVIEW-LOG-T1.md` 的新章节。

**这不是 Oracle 签署** —— Oracle 步骤仍为 BLOCKED（副签人待属主指定，见 §9 R-T7-1）。预判只是让副签人拿到一份可判定的材料。

- [ ] **Step 5: 修 R3 那 2 条的测试**

`PILOT-T3-001`（`crates/storage/src/adaptive_hash_index.rs`）与 `PILOT-T3-006`（`crates/storage/src/buffer_pool.rs`）的测试与所声称的不变量矛盾（前者只断言索引项被移除，后者只断言计数器，且断言文本的「写回时机」一半完全未触及）。

允许修改这两个**测试函数体**，使其真正断言所声称的不变量。约束：
- **只改测试函数体**，不改被测逻辑、不改非测试代码；
- 若做不到（例如被测功能根本没有该能力），则改为把该 case 的 `invariant_ids` 留空并记入 `INVARIANT-GAP-PROPOSALS.md`；
- 这两处 diff 必须在报告里**单独逐行列出**，供属主审。

- [ ] **Step 6: 给 `verify_matrix.py` 加 M7（分桶映射检测）**

M7 判定：若两个 case 的 `source.declared_invariant` 相同但 `invariant_ids` 不同，说明映射是逐用例的（好）；若不同 `declared_invariant` 的 case 共享**完全相同**的 `invariant_ids` 集合且该集合非空，则要求该 case 有 `invariant_mapping_basis` 说明为何如此，否则判 FAIL。

同时，M4 的语料来源保持读登记表 id 列（Task 5 已改），M1–M6 语义不变。

**契约变更**：`verify_matrix.py` 从 6/6 变为 7/7。**所有引用「6/6」的地方必须同步更新**（`docs/releases/v4.1.0/refactoring/tools/test_matrix_driver.py` 的断言、Task 6 的 `L1-BINDING` / `L1-DIFF` 记录、DSH-B 的 `REQUIREMENT-TEST-COVERAGE-MATRIX.md`）。

- [ ] **Step 7: 校验与重签**

```bash
cd /Volumes/workspace/dev/wt-v410-base
python3 docs/releases/v4.1.0/refactoring/tools/verify_matrix.py          # 期望 7/7
python3 docs/releases/v4.1.0/refactoring/tools/test_matrix_driver.py      # 期望 0 fail
python3 docs/releases/v4.1.0/refactoring/tools/verify_handoff.py --finalize
python3 docs/releases/v4.1.0/refactoring/tools/verify_handoff.py --check   # 期望 0
python3 docs/releases/v4.1.0/refactoring/tools/verify_baseline.py          # 期望 4/4
python3 docs/releases/v4.1.0/refactoring/tools/verify_sha_scope.py         # 期望 6/6（干净树）
```

**注意**：`verify_matrix.py` 与矩阵都在 manifest 哈希范围内 → 必须重签。主机 python3 缺 `jsonschema`，用 `/tmp/vh-venv`。

- [ ] **Step 8: 提交**

```bash
cd /Volumes/workspace/dev/wt-v410-base
git add docs/releases/v4.1.0/refactoring/TEST_COVERAGE_MATRIX.json \
        docs/releases/v4.1.0/refactoring/tools/ \
        docs/refactor/v4.1.0-c/takeover/ORACLE-REVIEW-LOG-T1.md \
        docs/refactor/v4.1.0-c/takeover/INVARIANT-GAP-PROPOSALS.md \
        crates/storage/src/adaptive_hash_index.rs \
        crates/storage/src/buffer_pool.rs
git commit -m "fix(v4.1.0): 逐用例重建 invariant 映射并修复 38 条不可判定断言"
```

**不得**改动 `red_identity`、任何 `assertion_*` 字段、`execution_status`、`artifact_sha256` 或 T4/T5/T6 的 `execution_status`。
