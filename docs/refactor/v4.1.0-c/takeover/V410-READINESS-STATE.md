# v4.1.0 重构就绪整备 —— 当前状态与续跑指引

> **本文件是本计划的记忆锚点。** 新会话（无论是重启后恢复，还是别的 agent 接手）
> **先读本文件**，再读同目录的 `V410-READINESS-LEDGER.md`（逐条裁决台账）。
> 设计论证在 `V410-REFACTOR-READINESS-DESIGN.md`，实施计划在
> `V410-READINESS-IMPLEMENTATION-PLAN.md`。
>
> 最后更新：2026-11-25

---

## 0. 一句话状态

**9 个计划任务已实施完成**（Task 1–8 + Task 10 通过独立评审；Task 9「放行判据汇总 + R2 外审包 +
属主放行裁决」**已实施完成** —— 修订第 1 轮处理 1 Important + 8 Minor，修订第 2 轮处理 3 Important + 8 Minor，
**最终整支评审修复波**处理 9 Important + 4 条必修 minor，**回归清扫**再处理 8 条 Minor ——
**评审与修订全部结束，没有待评审项**）。
**12 条准入判据的实测判定：PASS 8 · FAIL 3（#4、#6、#8）· WAIVED 1（#12）**（判定结果**未变**）。
**剩余待办全部是属主裁决**：判据 **4 / 6 / 8** 的处置 + **25 条待裁决项**
（逐条清单见 `V410-OWNER-RULING.md` §3）。

当前**不能**宣布放行，也**不能**宣布「整备失败」：**材料层全绿**（基线、SHA 绑定、ADR 机器字段、
manifest、门禁编目、外审包三套统计与 dead 口径都自洽），缺的是**证据层**（#8：夹具泳道未达 7/7）
与**签署层**（#6：Oracle 副签 0 条）。缺口已逐条量化并登记。

**最终整支评审修复波（2026-11-25）只做了「让陈述与事实对齐」**：判定结果、门禁阈值、门禁清单、
产品代码**一个都没动**。改掉的是三处**自己写过的假话**（判据 8 的实测树 SHA、Oracle 判定的 commit 绑定、
`observed` 里手写散文冒充命令输出），外加一条**让判据 6 具备「合法授权后才可能 PASS」的逻辑**。

**放行与否是属主的裁决**：`V410-OWNER-RULING.md` 是**裁决请求**（属主裁决位留空），
实施者与控制者都未代签。

---

## 1. 事实锚点（不要重新推导，直接采信）

| 项 | 值 |
|---|---|
| 承载分支 | `base/v4.1.0-frozen` |
| 隔离 worktree | `/Volumes/workspace/dev/wt-v410-base` |
| **BASE（内容基线提交）** | `fd849ae20541901d5e59f45cd3ba2c51093f5d5e` |
| BASE 的 tree | `72d378e8e58797eae18502ab474325015b8993d4` |
| BASE 的 parent（**冻结时点**的 develop tip） | `8cbde5b6d33c876fa1197474c46d50142a9823bd` |
| 冻结记录（唯一基线身份） | `docs/refactor/v4.1.0-c/takeover/BASELINE_FREEZE.json` |
| handoff manifest | `docs/refactor/v4.1.0-c/handoff/10_HANDOFF_MANIFEST.json` |

> **`base_sha` 一律填 BASE，不随提交前移。** 这是 R-BASE，Task 2 之后所有前向绑定都钉死在这里。

> **远端 tip 的当前值（2026-11-25 实测，会变）**：`origin/develop/v4.1.0` =
> `ea285c5fda77e7790e965ae482e3c1b015fabc10`（**已包含** `base/v4.1.0-frozen`，
> PR #5928 的 merge commit —— 冻结分支已被并入 develop）；
> `origin/base/v4.1.0-frozen` = **`ebcf8bc2997649e333067bbb8497c1dbcdf09db3`**
> （**远端** tip；本地分支已领先它 —— 本波修复波尚未推送）。develop 里**还没有**冻结分支上的
> **7 个**提交（在远端 tip `ebcf8bc299` 处实测：`d358289919`、`09ba360f6c`、
> `e2d98149d2`、`33f1c7ceb9`、`6c345db7cb`、`a2d0e3ecec`、`ebcf8bc299`）。
> ⚠️ **旧版本这里写的是「最后两个提交」和 tip = `e2d98149d2…` —— 那是过时的快照，已按实测重算。**
> ⚠️ **本修复波自身还会再加提交** ⇒ 合并前请以
> `git log --oneline ea285c5fda..base/v4.1.0-frozen` 的**实测输出**为准，不要按固定数字操作。
> 这两个值会随推送/合并变动；**BASE 不会变**。

---

## 2. 已完成的任务与各自的证据锚点

| 任务 | 做了什么 | 证据锚点 | 状态 |
|---|---|---|---|
| **Task 1** | 唯一基线 + 123 个材料文件导入 + 冻结记录 | `BASELINE_FREEZE.json`、`tools/verify_baseline.py` | ✅ 评审通过 |
| **Task 2** | 前向/历史 SHA 分界重指 | `tools/verify_sha_scope.py`（S1–S6） | ✅ 评审通过 |
| **Task 3** | `verify_handoff.py` 断言闭包（ADR 档位 / manifest / 变异臂） | `tools/verify_handoff.py`、`tools/test_verify_handoff.py` | ✅ 评审通过 |
| **Task 4** | 11 份 ADR 的机器字段落地（档位记入正文） | `takeover/dsh-b/ADR-DECISION-RECORD.md` | ✅ 评审通过 |
| **Task 5** | 75 场景矩阵修复 + 矩阵驱动层 + 状态词汇 | `TEST_COVERAGE_MATRIX.json`、`tools/matrix_driver.py`、`tools/verify_matrix.py` | ✅ 评审通过 |
| **Task 6** | L1 切片实跑（44 条）+ 双向差异表 + C5 绑定 | `refactoring/evidence/L1-RERUN-*.json` / `L1-DIFF-*.md` / `L1-BINDING-*.json` | ✅ 评审通过 |
| **Task 7** | Oracle 逐条判定（**判定完成，签署 BLOCKED**） | `takeover/ORACLE-REVIEW-LOG-T1.md` | ✅ 实质交付通过 |
| **Task 8** | ALPHA 阶段执行点 + 7 道负向夹具 | `.gitea/workflows/ci-alpha-stage.yml`、`audit/v4.1.0/neg/fx*.sh` | ✅ 修订 2 后通过 |
| **Task 10** | 逐用例重做 invariant 映射 + 修复 38 条不可判定断言 | `TRANSACTION_INVARIANTS.md:30`、`ORACLE-REVIEW-LOG-T1.md:505-511` | ✅ 评审通过 |
| **Task 9** | 12 条判据汇总 + R2 外审包 + 属主放行裁决（**裁决请求**，属主未签） | `takeover/V410-READINESS-VERDICT.json`、`takeover/R2-RELEASE-PACKAGE.md`、`takeover/V410-OWNER-RULING.md`、`takeover/verify_r2_package.py`（判据 10/11 的可复算锚点）；R2 包 `…/DSH-EXTERNAL-REVIEW-001/sqlrustgo-v4.1.0-r2/`（zip sha256 `ea3611eee5da3ab0…`） | ✅ 已实施完成，**评审与修订全部结束**（修订第 1 轮 1 Important + 8 Minor；修订第 2 轮 11 findings 全部 ADDRESSED；**最终整支评审**返回 0 Critical / 9 Important / 11 Minor，**修复波**经 re-review 判定 11 条 finding 全部 ADDRESSED；**回归清扫** 8 条全部 ADDRESSED —— **无待评审项**）；判定 **PASS 8 / FAIL 3 / WAIVED 1**；剩余待办 = **属主裁决**（判据 4 / 6 / 8 + 25 条待裁决项） |

---

## 3. Task 9 的结论（2026-11-25）

Task 9 产出**四件**东西（均在 `docs/refactor/v4.1.0-c/takeover/`；另有第五件是**生成器**，见下）：

1. `V410-READINESS-VERDICT.json` —— 12 条判据逐条的 `id` / `criterion` / `command` /
   `observed` / `verdict`（每条 `command` 都**真的跑过**，`observed` 是真实输出摘要）；
2. `R2-RELEASE-PACKAGE.md` —— R2 外审包的生成说明 + R1 逐字节未变的快照与复核；
3. `V410-OWNER-RULING.md` —— **裁决请求**（不是已签的裁决）：判据引用、已裁决的豁免（只有判据 12）、
   判据 **4 / 6 / 8** 的**预留豁免槽位（留空，A / B / C）**、残余未裁决项、**留空的属主裁决位**、以及标明
   「这是建议，不是裁决」的控制者建议；
4. `verify_r2_package.py` —— 判据 10 / 11 的**可复算锚点**（修订第 1 轮按控制者裁定 M-8 追加）；
   它**只校验、不生成**，需要用 `--r2-dir` / `R2_DIR` 指向 R2 包（找不到包时打印人话并退出码 2）。
   **已同步为三处同一份**（本仓 `takeover/` + DSH 的 `r2-build/` + DSH 的 `sqlrustgo-v4.1.0-r2/`，
   sha256 前缀 `e8155879faaefd6e`；DSH 侧提交 `08742fc`，该仓无 remote 故**未 push**）
   ⇒ 先前登记的**两份校验器字节漂移（N-10）已关闭**。

第五件（**生成器**，控制者裁定 R69）：`gen_verdict.py` —— 12 条判据的**重跑器**。
它原来只活在 git-ignored 的 `.superpowers/sdd/` 里，于是它里面的硬编码字符串（旧的
「差额来自两条判据」、旧的变异臂数）改了树、没改源，下一次重跑又会把陈旧字符串写回裁决文件。
现在它**已入仓**，裁决 JSON **可从「生成它的那棵树」复现**（⚠️ 不是「逐项一致」的承诺：
判据 1 记的远端 tip、判据 8 的树身份都随时间变 —— 后者还是**推定/重构**的，见 §3 的 #8 行与
`V410-OWNER-RULING.md` §1.2）：

```bash
cd /Volumes/workspace/dev/wt-v410-base
git status --porcelain          # 必须为空（判据 3 的校验器拒绝脏树；生成器也会拒绝）
python3 docs/refactor/v4.1.0-c/takeover/gen_verdict.py   # 重出 V410-READINESS-VERDICT.json
```

生成器里的数字**全部从实际输出数出来**：顶部 note 由 `criteria` 派生（不可能再与 `summary` 矛盾）、
变异臂条数由 `--check` 的输出 `len()` 得出、判据 8 的退出码由脚本的退出码语义推出。

**判定汇总：PASS 8 · FAIL 3 · WAIVED 1。** 三条 FAIL 的分层归因：

| 判据 | 层 | 缺口 |
|---|---|---|
| **#4** | **映射层** | A2.2 **R1**（「每个 task 的 `approved_adr_refs` 至少 1 项」）被 **C01-15** 违反 —— `ADAPTER-MAPPING-PENDING.md` 的待决清单**非空**。ADR 的**机器字段**齐了、**10 条**变异臂全被拒（条数从 `--check` 的实际输出里数出来），但 `--check` **不担保 R1**（`verify_handoff.py:383-384` 空列表直接 return），它自己的输出是 `"adr_approval_state": "PENDING"` ⇒ 判据 4 = FAIL。**需属主裁决 O1**（补 ADR 引用 vs 裁定无需绑定） |
| **#6** | **签署层** | 44 条有终态 ✓、双向差异表 ✓，但 **Oracle 副签 0 条**（38 条 `ASSERTION_UNREVIEWABLE` + 6 条 `JUDGED_PENDING_COUNTERSIGNATURE`；`assertion_review_performed_by` 是实施者本人）⇒ 需属主指认 1 名独立 Reviewer（R-T7-1） |
| **#8** | **证据层** | 实跑（2026-11-25，树 = **`09ba360f6cfbf0be2aa4cf7d2064c4b8b50f7958`**）**RED=5/7 + INCONCLUSIVE=2/7（退出码 3）**；上一次记的是 **6/7 + 1/7**，跑在 commit **`b556199c86`**（**不是**冻结提交 `fd849ae205`，跨 SHA 的证据不可继承，C5）；两次都不是 7/7。fx15 因 composite release 重建超 7200s 预算（exit 124）⇒ INCONCLUSIVE；fx18 的覆盖率门禁**在完整树上就已红** ⇒ 注入的红不可归因 ⇒ INCONCLUSIVE。**⚠ 这个树身份是「推定/重构」**：泳道日志本身**不含**树身份（`run.log` 里没有任何 40 位十六进制串），SHA 由 `git reflog` + 日志 mtime（2026-11-25 12:07:40，早于 `e2d98149d2` 的提交时刻 12:08:52）推定 ⇒ 泳道全程 HEAD = `09ba360f6c`。**结论不受影响**：`git diff --name-status 09ba360f6c HEAD` 里**没有**门禁脚本 / 负向夹具 / 门禁编目 / 产品代码的改动 ⇒ RED=5/7 与 FAIL 判定照旧。详见 `V410-OWNER-RULING.md` §1.2 |

R2 外审包在 `…/DSH-EXTERNAL-REVIEW-001/sqlrustgo-v4.1.0-r2/`（**兄弟目录副本**；R1 的 `package/`
21 个文件在生成前后**逐字节未变**）；构建链在同目录的 `r2-build/`（`build_r2.sh` 是唯一入口，幂等）。
`owner/approvals/` **空** —— 签署是属主的行为。

**⚠️ 判据 1 的一条观察（不改变判定）**：本任务运行期间 `develop/v4.1.0` 被合并了
`base/v4.1.0-frozen`（PR #5928，merge commit `ea285c5fda`），所以远端 develop 的 tip 现在是
BASE 的**后代**而非祖先（设计稿 §A5 #1 的字面检查因此不再成立）；冻结时点的 develop tip
`8cbde5b6d3` 仍是 BASE 的祖先 ✓。远端 `base/v4.1.0-frozen` 的 tip 现为
**`ebcf8bc2997649e333067bbb8497c1dbcdf09db3`**（**远端** tip；本地分支已领先它 —— 本波修复波尚未推送）；
develop 里**还没有**冻结分支上的**7 个**提交（在远端 tip `ebcf8bc299` 处实测）——
⚠️ 旧版本这里写的是「最后两个提交」，**已按实测重算**。

---

## 4. 已定且**不可被后续会话重新推翻**的结论

> 下面每一条都经过独立评审，并且有可复算的证据。**续跑时不要重新论证它们**；
> 要改必须带新证据。

- **R47**：`GATE_CATALOG.jsonl` 的 `neg_fixture` 原义（门禁**自带** selftest）不变，
  「有外部夹具」是**另一个**字段 `external_fixture`；基准是台账
  `docs/releases/v4.1.0/NEG_FIXTURES_v4.1.0.jsonl` + `audit/v4.1.0/neg/fx*.sh` 实物。
- **R48**：7 道 ALPHA 门禁里**只有 1 道**（`check_anti_ignore_gate.sh`）原本就有夹具；
  新造的是 `fx15`–`fx20`（+ 表征夹具 `fx21`/`fx22`）。
- **R49**：每道夹具的缺陷规格来自台账的 `target` 字段，不得自创。
- **R50**：**超时（124）/信号不算「门禁变红」**，只能报 INCONCLUSIVE；宁可 6/7 也不凑 7/7。
- **R51**：CI 执行点用 `check_stage.sh`（两态）；三态 runner 的**执行**接线属 kanban IL-7。
  代价：ALPHA 执行面**丢失 INCOMPLETE 语义** ⇒ Task 9 必须登记为残余项。
- **R52**：ALPHA 执行点**不由 push 触发**，落在独立 workflow
  `.gitea/workflows/ci-alpha-stage.yml`（`workflow_dispatch` + 独立 concurrency 组 +
  `cancel-in-progress: false`）。**不能放进 `ci.yml`**：它的 concurrency 组带
  `cancel-in-progress: true`，任何非 push 触发器都会取消别人正在跑的 run。
- **R53**：ALPHA 执行点**只允许有一处**。
- **R54**：两层 RED 必须来自**同一次** `run_gates.py execute` 的**同一份** `results.jsonl`；
  汇总层必须是 `gate_report.py` 的**独立重解析**；目标门禁必须在汇总里被标为 FAIL（因果闭合）。
- **R55**：新夹具落 `audit/v4.1.0/neg/fxNN-<gate>-<缺陷类型>.sh`，并**必须**在台账登记。
- **R56**：本计划**不新增门禁、不改 `STAGE_CONFIG.yaml`、不改 168 分母**。
- **R57**：证据 run 的 manifest 必须叫 `manifest.yaml`；新增的 run 路径在冻结记录里
  **显式登记**（`historical_set_additions`），S5 的期望集 = 冻结集 ∪ 登记增补集。
- **R58**：机器可读的声明（台账行、`effect.catches`）**只允许写夹具真正证明的性质**；
  证明反面时写成 **known hole**。
- **R59**：台账是**生成物**——`audit/v4.1.0/gen_neg_ledger.py` 必须能逐字节复现已提交的台账。
- **R60**：端到端跑过一次 ALPHA（27m06s / 退出码 1 / 13 PASS 4 FAIL），**不得用 dry-run 冒充**。
- **R61**：夹具泳道的第四种期望值 `inconclusive` **被接受**（它无法冒充红）。
- **R40–R42**：Oracle 签署人必须是**属主授权的独立 Reviewer**；实施者不得自签；
  不可独立判定的断言**不得**为凑数而签署。
- **R43–R46**（Task 10）：`invariant_ids` 必须**逐用例**从**测试源码**派生；
  宁可留 13 条零覆盖，也不保留无法复核的覆盖。

---

## 5. 必须由**属主**裁决的事项（Task 9 已逐条汇总进 `V410-OWNER-RULING.md` §3）

| # | 事项 | 阻塞什么 |
|---|---|---|
| **O1** | ADR 档位「仅批准方向/实施另审」是否足够支撑 `approved_adr_refs` 填充；`ADAPTER-MAPPING-PENDING.md` 的待决清单**非空**（C01-15） | **判据 4 = FAIL 的直接原因**（映射层）；属主二选一：补 ADR 引用 or 裁定无需绑定 |
| O3/O4/O5/O10/O11/O12/O14/O15/O17 | spec §6.2 的其余待裁决项 | 各自对应判据（**是否影响放行**已在 `V410-OWNER-RULING.md` §3.1 逐条标注） |
| **O2** | 75 场景的 `required_for_certification` 未标定（实测 75/75 为 `false`） | 阶段出口口径（不决定 #6 —— #6 已因 Oracle 副签缺位而 FAIL） |
| **O16** | 「测试已消失」的分类（7 类契约无此类） | 场景数未定 |
| **N1** | 在基线里注册 `[[test]] tf_t1_undo_invariants`，vs 把 `PILOT-T1-101/102` 记为不可认证 | L1 切片能否作验收凭据 |
| **N2** | `assembly=UNVERIFIED` 的场景可否记 PASS —— 两种读法并列（DSH-B 字面读法下「42 PASS」不可作验收凭据，而该切片不含任何 RED） | L1 切片的证据效力 |
| **N3** | ledger #10/#11 的 `source_sha256` 滞后一次改写 | 台账自洽 |
| **N4** | ledger 实为 18 行而其正文自称 17 | 台账自洽 |
| **R-T7-1** | Oracle 副签人身份 —— 目前**无可用 Reviewer** ⇒ Oracle 步骤 **BLOCKED** | **判据 6 = FAIL 的直接原因** |
| **R-T7-2** | 第 2 段签署是在错误指示下进行的（3 条已撤回） | 判据 6 |
| **R-T7-3** | `ORACLE-REVIEW-LOG-T1.md` 在 manifest 哈希范围之外 | 判据 5/6 |
| **GAP-13** | 两条 R3 测试所需能力在冻结基线上不存在 | 判据 6 |
| **GAP-14** | 3 条用例的原映射为标签派生，已撤下 | 判据 6 |
| — | **13 条不变量零覆盖**（INV-01/02/04/07/14 之外的） | 「可开始重构」的实质差距 |
| — | 3 条断言仍不可独立判定（T1-009 / T3-002 / T3-003） | 判据 6 |
| — | **判据 8 的归因口径与冷 target 预算**（覆盖率门禁在基线树上就红；composite 的 release 重建要 1–2 小时） | **判据 8 = FAIL 的两个子原因** |

**R60 的冷 target 数据缺口**：端到端 ALPHA 在**热 target** 下 23m58s 跑完，但冷 target
（CI 的真实情形）下 composite 的 release 构建要重跑，90 分钟预算两次撞墙，
**180 分钟（workflow 的 `timeout-minutes`）才跑完**。冷 cache 下 180 分钟仍可能紧张 ——
**已写进裁决记录**（`V410-OWNER-RULING.md` §3.3）。

---

## 6. 续跑指令（新会话照此接手）

1. `cd /Volumes/workspace/dev/wt-v410-base`，确认分支 `base/v4.1.0-frozen` 与 tip。
2. 读本文件 + `V410-READINESS-LEDGER.md`；再读 `V410-OWNER-RULING.md`（裁决请求）。
3. **9 个任务已实施完成**（Task 9 修订第 1 轮已处理 1 Important + 8 Minor；修订第 2 轮已处理 3 Important + 8 Minor；
   **最终整支评审**返回 0 Critical / 9 Important / 11 Minor，**修复波已实施完成**（re-review 判定 11 条 finding
   全部 ADDRESSED），**回归清扫已实施完成**（8 条全部 ADDRESSED），见
   `.superpowers/sdd/V410-READINESS-IMPLEMENTATION-PLAN/task-9-report.md` 的「修订 1 / 修订 2 /
   最终整支评审 · 修复波 / 回归清扫」小节）—— **评审已全部结束，没有待评审项**。
   实施侧的剩余动作**只有属主裁决**：在 `V410-OWNER-RULING.md` §4 的裁决位填
   「放行 C-ALPHA-02 / 不放行」（若豁免 #4 / #6 / #8，在 §2.2 的槽位 A / B / C 填 waiver 字段），
   并逐条处置 **25 条待裁决项**（清单在 `V410-OWNER-RULING.md` §3）。
4. Task 9 的产物**已过完独立评审**（与前 8 个任务同一标准）—— 两轮修订 + 最终整支评审修复波 + 回归清扫
   全部 ADDRESSED，**没有待评审项**；修订记录在
   `.superpowers/sdd/V410-READINESS-IMPLEMENTATION-PLAN/task-9-report.md` 的「修订 1 / 修订 2 /
   最终整支评审 · 修复波 / 回归清扫」小节。
5. 若属主要求补齐 #6 / #8 后复评：复跑
   `bash audit/v4.1.0/neg/verify_neg_fixtures.sh`（判据 8）与
   `python3 docs/refactor/v4.1.0-c/takeover/gen_verdict.py`（12 条判据，
   一条命令重出 `V410-READINESS-VERDICT.json`），然后更新 `V410-OWNER-RULING.md` 的实测引用。
   > ⚠️ **必须用仓内路径**（`docs/refactor/v4.1.0-c/takeover/gen_verdict.py`）。
   > `.superpowers/sdd/V410-READINESS-IMPLEMENTATION-PLAN/gen_verdict.py` 是**过时的副本，不得使用** ——
   > 它是 git-ignored 的，改了树、没改源，跑它会把 R69 修掉的陈旧字符串重新写回裁决文件
   > （这正是 R69 要消灭的那类回归）。
6. R2 外审包的外发批准是**独立**于 C-ALPHA-02 的一步：属主可直接签
   `…/DSH-EXTERNAL-REVIEW-001/sqlrustgo-v4.1.0-r2/EXPORT-APPROVAL-REQUEST.md`
   （批准栏留空，`owner/approvals/` 空）。**实施者不得代签**（C7）。
7. 若中途再次 compact：**以本文件与 git log 为准**，不要凭记忆。

### 过程脚本（不在 git 里，随手可重建）

| 路径 | 作用 |
|---|---|
| `.superpowers/sdd/V410-READINESS-IMPLEMENTATION-PLAN/progress.md` | 原始 ledger（含全部裁决行与评审记录） |
| `.superpowers/sdd/V410-READINESS-IMPLEMENTATION-PLAN/task-*-brief.md` | 各任务的计划正文切片 |
| `.superpowers/sdd/V410-READINESS-IMPLEMENTATION-PLAN/task-*-report.md` | 各任务的实施者报告 + 修订记录 |
| `.superpowers/sdd/V410-READINESS-IMPLEMENTATION-PLAN/gen_verdict.py` | ⛔ **过时的副本，不得使用**。**用仓内那份**：`docs/refactor/v4.1.0-c/takeover/gen_verdict.py`（R69 已把它入仓） |
| `.superpowers/sdd/V410-READINESS-IMPLEMENTATION-PLAN/task-9-neg-logs/` | Task 9 的负向夹具泳道实测日志（run.log + 逐夹具日志） |
| `.superpowers/sdd/V410-READINESS-IMPLEMENTATION-PLAN/review-*.diff` | 评审包（**可删**，可由 git 重新生成） |

R2 的**构建链**不在本仓，而在 `…/DSH-EXTERNAL-REVIEW-001/r2-build/`（权威归属：DSH 仓，**当前无 remote**）；
本仓只保存**校验器** `docs/refactor/v4.1.0-c/takeover/verify_r2_package.py`（判据 10 / 11 的可复算锚点，
可用 `R2_DIR=` 或 `--r2-dir` 指向 R2 包）。

`.superpowers/sdd/` 是 git-ignored 的临时区，**不是记忆**；记忆在仓库里（本文件 + 台账副本）。
R2 的构建链不在这里，而在 **`…/DSH-EXTERNAL-REVIEW-001/r2-build/`**（`build_r2.sh` 是唯一入口，幂等）。

---

## 7. 记忆边界

- 本文件记录**结论**，不记录**过程流水**；过程在 ledger 与 task 报告里。
- 凡是需要重新跑一遍才能确认的数字（覆盖率、通过率、耗时），本文件只给**当时的实测值**
  与**当时树的身份**，不声称它们现在仍然成立。
- 任何与本文件矛盾的**新**结论，必须附带**可复算的证据**并更新本文件 —— 否则以本文件为准。
