# V410-OWNER-RULING —— v4.1.0 重构放行裁决请求

> ## ⚠️ 本文件是**裁决请求**，不是已签的裁决
>
> 阶段跃迁属主专属（C1）。**实施者与控制者都不得代属主签字、不得填属主名字、不得写「已放行」。**
> 本文件由 Task 9 实施者生成，内容为：12 条判据的**实测**结果的引用、**已裁决**的豁免登记、
> **预留的豁免槽位**、**残余未裁决项**清单，以及一份**明确留空**的属主裁决位。
> 属主裁决位在 §4，**未填**。

- 生成：V410-READINESS-IMPLEMENTATION-PLAN / Task 9 Step 4
- 判据实测：`docs/refactor/v4.1.0-c/takeover/V410-READINESS-VERDICT.json`（本文件**引用**它，不复述数字）
- 基准树：BASE `fd849ae20541901d5e59f45cd3ba2c51093f5d5e` · **冻结时点**的 develop tip `8cbde5b6d33c876fa1197474c46d50142a9823bd`（注：该 tip 是**冻结记录里**的 develop tip；live develop tip 已前移，详见 §1 的注记）
- 承载分支：`base/v4.1.0-frozen` · 隔离 worktree：`/Volumes/workspace/dev/wt-v410-base`

---

### 0.1 本支建立了什么 / 未建立什么

**建立了**：唯一基线与冻结记录、123 个材料文件的前向绑定、11 份 ADR 的机器字段、
75 场景矩阵与它的机器可读校验链、12 条判据的**实测**判定（本文件只引用，不复述数字）。

**未建立**：放行本身（§4 的裁决位留空）、Oracle 副签（§3.1.1 的名单为空）、
判据 4 的映射（C01-15 的 `approved_adr_refs` 仍为 `[]`）、判据 8 的 7/7（实测 5/7 + 2/7）、
回归保护（§3.1.3：校验器与自测一个都没进 CI）。

**信任锚（M-5）**：`handoff/10_HANDOFF_MANIFEST.json` 按设计**不在自己的哈希范围内**
（仓内也没有产物绑它的 sha256），所以一次「改文件 + 重签」能骗过 `--check` ——
**真正的锚是 git 历史与冻结记录 `BASELINE_FREEZE.json` 的 `base_sha` / `tree_sha`**
（`fd849ae205…` / `72d378e8e5…`），不是 manifest 自己。
逐条残余清单见 [V410-RESIDUAL-CHECKLIST.md](V410-RESIDUAL-CHECKLIST.md)。

---

## 1. 判据汇总（12 条）

| # | 判据 | 判定 | 实测出处（`V410-READINESS-VERDICT.json`） |
|---|---|---|---|
| 1 | BASE 唯一 / develop tip 是其祖先 / 含 BASE 的分支已推送 | **PASS** | `criteria[0]` |
| 2 | 45da 的 123 个材料文件全在 BASE 树内 | **PASS** | `criteria[1]` |
| 3 | 前向/历史 SHA 分界 | **PASS** | `criteria[2]` |
| 4 | ADR 字段与 A2.2 映射规则 | **FAIL**（映射层；见 §1.1 与 §3.1 的 O1） | `criteria[3]` |
| 5 | manifest 零漂移 | **PASS** | `criteria[4]` |
| 6 | L1 有终态 + **Oracle 签署** + 双向差异表 | **FAIL** | `criteria[5]` |
| 7 | ALPHA 阶段清单有 CI 执行点 | **PASS** | `criteria[6]` |
| 8 | ALPHA 7 道负向夹具两层 RED | **FAIL** | `criteria[7]` |
| 9 | 门禁编目内部自洽 | **PASS** | `criteria[8]` |
| 10 | 外审包三套统计一致（71/71/71） | **PASS** | `criteria[9]` |
| 11 | dead 门禁口径唯一化 | **PASS** | `criteria[10]` |
| 12 | LOCAL_ONLY 路由 fail-closed | **WAIVED**（O9） | `criteria[11]` |

**合计：PASS 8 · FAIL 3（#4、#6、#8）· WAIVED 1（#12）**

> **计划正文验收段写的「11 条 PASS + 1 条 WAIVED」是 Task 7/10 之前的乐观预期，以实测为准。**
> 差额**来自三条判据**：**#4**（A2.2 R1 未满足：O1 的待决清单非空）、**#6**（Oracle 副签 0 条）与
> **#8**（夹具泳道未达 7/7）。R63 明确：不得为了让总数好看而改判、拆分或重定义判据。
>
> **#4 的判定依据**（修订第 1 轮按计划正文改判，详见 `V410-READINESS-VERDICT.json` 的 `criteria[3].observed`）：
> ① 计划正文 Step 4 的口径「O1 的待决清单**若非空则 A5 #4 不能判 PASS**」——清单**非空**
> （`ADAPTER-MAPPING-PENDING.md` 的 `| C01-15 | … |` 一行，其 `approved_adr_refs` 保持 `[]`）；
> ② 判据自身的措辞「**映射满足 A2.2 五条规则**」——A2.2 **R1**「每个 task 的 `approved_adr_refs` 至少 1 项」
> 被 C01-15 违反；③ 校验器**不担保 R1**（`verify_handoff.py:383-384` 在 `refs` 为空时直接 `return`），
> 且它自己的输出是反面证据（`"adr_approval_state": "PENDING"` 与 note「the owner's ADR decision is
> PENDING, not verified」）。⇒ `--check` 退出 0 **只说明可核方式里的断言成立，不等于判据 4 成立**。
> 属主若要放行这条，需要在 §2.2 的**槽位 C** 里自己豁免，或先裁决 O1。

> ### ⚠️ 判据 1 的一条必须让属主知道的观察（不改变判定，但要写明）
>
> 判据 1 按**计划正文 Step 1 的命令**判 **PASS**（`verify_baseline.py --strict` 4/4 +
> 远端 `base/v4.1.0-frozen` 的 tip 是 BASE 的后代，成立）。
> 但**设计稿 §A5 #1 的另一条可核方式**
> （`git merge-base --is-ancestor origin/develop/v4.1.0 BASE`）在本次实测中**不成立**：
> 远端 `develop/v4.1.0` 的 tip 已是 `9f582882bb15bfa049dd4313fd7e9cf2326a3ce7`，
> 它是 BASE 的**后代**而非祖先。原因是 `develop/v4.1.0` **已被两次合并**了 `base/v4.1.0-frozen`：
> PR **#5928**（merge commit `ea285c5fda`，第一批）与 PR **#5929**（merge commit `9f582882bb`，
> 整备剩余全部提交）。
>
> 这不是 BASE 的缺陷，而是**冻结分支已被并入 develop** —— O11（BASE 是否推进 develop）
> 事实上已被行动回答。判据 1 的三项实质条件仍全部成立：BASE 唯一 ✓；
> **冻结时点的 develop tip** `8cbde5b6d3` 是 BASE 的祖先 ✓（`BASELINE_FREEZE.json` 记录，
> `verify_baseline.py` 的 B1 用的就是它）；含 BASE 的分支已推送 ✓。
>
> **合并状态（2026-11-25 实测；本段旧数字已过时）**：
> (a) 远端 `base/v4.1.0-frozen` 与本地 HEAD 现**同为** `f768c1c8b9e3bc32a1b7934b943ad13cbccd9c50`
> （它仍是 BASE 的后代 ✓）。
> (b) **冻结分支已全部并入 develop** —— `git log --oneline origin/develop/v4.1.0..base/v4.1.0-frozen`
> 实测 **0 条**（PR #5929 已合并，merge commit `9f582882bb15`）。
> (c) ⇒ **「缺 N 个提交、需先合并」的旧指引已作废**；裁决文件、台账、生成器与全部修订**已在 develop 上**。
> 后续若继续在 `base/v4.1.0-frozen` 上提交，需要**新开 PR** 才会进 develop —— 那条路已验证可走。
>
> ⚠️ **仍请以这条命令的实测输出为准，不要按任何固定数字操作**：
>
> ```bash
> git log --oneline origin/develop/v4.1.0..base/v4.1.0-frozen
> git ls-remote origin refs/heads/develop/v4.1.0 refs/heads/base/v4.1.0-frozen
> ```

### 1.1 三条 FAIL 的分层归因（R66）

| 判据 | 缺在哪一层 | 具体缺口 | 补齐它需要谁 |
|---|---|---|---|
| **#4** | **映射层**（既不是材料层、也不是证据层、也不是签署层） | ADR 的**机器字段**（11 份的档位、`Approvals` 字段、`adr_catalogue`）**齐了**，`verify_handoff.py --check` 的 **10 条**变异臂**全被拒**（条数从 `--check` 的实际输出里数出来，不是抄的）；缺的是**映射**：A2.2 **R1**「每个 task 的 `approved_adr_refs` 至少 1 项」被 **C01-15** 违反（`ADAPTER-MAPPING-PENDING.md` 的待决清单非空）。该包与 11 份 ADR 的「目标不变量」**无交集**（它的唯一不变量 INV-17 是测试工厂的认证口径不变量，不是架构不变量），所以「填一个无关 ADR 去凑数」被 A2.2 R2 明令禁止 —— **清单里白纸黑字写着明知违反 R1 而保留** | **属主裁决 O1**（spec §6.2，与 O10「A2.2 五条规则是否采纳」同源）：二选一 —— (a) 裁定 C01-15 与任何 ADR 都无需绑定（把它归为「测试工厂治理包」，不在 ADR 覆盖范围内），则本行从清单移除、`approved_adr_refs` 保持 `[]` 并在放行记录里写明该口径；或 (b) 裁定它应绑定某份 ADR，则按 A2.2 R2 给出实际关联依据，执行者据此回填并重签 manifest。**属主不裁决，执行者不得自拟映射**（A2.2 R5 / O10） |
| **#6** | **签署层** | 材料齐了（44/44 有终态：42 PASS + 2 NOT_RUN；双向差异表 `L1-DIFF-fd849ae20541.md` 表头逐字符匹配），但**Oracle 副签 0 条** —— 44 条的 `assertion_review_verdict` 是 38 条 `ASSERTION_UNREVIEWABLE` + 6 条 `JUDGED_PENDING_COUNTERSIGNATURE`，`assertion_review_performed_by` 是**实施者本人**（`task7-executor/minimax-m3.1-flash`），按 R-ORACLE「签署人 ≠ 该场景实现者」不构成签署。**且判据 6 的判定逻辑本波已重写（整支评审 I-2）**：旧逻辑只查一个字段、**从不看是谁签的** ⇒ 属主一旦指认 Reviewer，仅凭一个字段值就能把 #6 翻成 PASS。新逻辑见 §3.1 的「属主授权的 Reviewer 名单」 | **属主授权的独立 Reviewer**（Codex 或 DSH Reviewer 池）；实施者不得自签（C7 / R-ORACLE）。属主把 Reviewer 填进 §3.1 的名单表后，判据 6 才可能转 PASS |
| **#8** | **证据层** | 夹具在**结构上**无法成立：ALPHA 7 道门禁的负向夹具泳道未达 7/7。缺口不是「断言被放松」，而是（a）覆盖率门禁 `check_coverage_v312.sh` 的坏产物注入**在完整树上就已红**（Task 1–7 导入的 RED 特征测试 `crates/storage/tests/tf_t4_*.rs` 按设计失败 ⇒ 红不可归因于夹具注入）；（b）**机器速度**：fx15 的 composite 门禁在 fx17 动过 Rust 源码后要重建 release，1–2 小时量级，fx15 的 7200s 预算本轮不够用 | 证据层：要么把覆盖率门禁的红**归因**拆开（让夹具注入的红与基线自带的红可区分），要么在**冷 target** 下给足预算；见 §3.3 的 180 分钟项 |

> **材料层全绿**：#1/#2/#3/#5/#9/#10/#11 全部 PASS —— 基线、SHA 绑定、ADR **机器字段**、manifest、
> 门禁编目、外审包三套统计与 dead 口径都自洽。**这次整备的成果没有缺口；缺的是「映射要属主拍板」、
> 「有人签」与「证据能归因」。**

### 1.2 判据 8 的实测口径与时间敏感性（R63 要求的写法）

- 本次实跑（同一棵树、同一命令，2026-11-25）：**RED = 5/7 + INCONCLUSIVE = 2/7**，**退出码 3**；
  实跑的树是 **`09ba360f6cfbf0be2aa4cf7d2064c4b8b50f7958`**；
- **⚠ 这个树身份是「推定/重构」，不是从日志里观测到的**（2026-11-25 整支评审 I-1 修正）：
  - **泳道日志本身不含树身份** —— `run.log` 里没有任何 40 位十六进制串，只有
    `verify_neg_fixtures: scratch tree = /Volumes/workspace/dev/wt-v410-base` 与逐夹具判决；
  - 推定依据（两个互相独立、均可复算的事实）：
    ① `git reflog --date=iso`：HEAD 从 **2026-11-25 09:16:07**（`09ba360f6c` 的提交时刻）
    到 **2026-11-25 12:08:52**（`e2d98149d2` 的提交时刻）一直是 `09ba360f6c`；
    ② 泳道日志的 mtime 是 **2026-11-25 12:07:40** —— **早于** `e2d98149d2` 作为提交存在的那一刻；
  - 泳道以**仓库自身**作 scratch 树，且 `verify_neg_fixtures.sh` 在脏树上**拒绝启动**
    （「The fixtures restore with `git checkout -- . && git clean -qfd`」）⇒ 全程 HEAD = `09ba360f6c`；
  - **先前写的 `e2d98149d264…` 是错的**：那是泳道跑完之后才被创建的那个提交，
    被当成了「写裁决文件那一刻的 tip」，而不是泳道实际跑过的那棵树 ——
    这正是 **C5（证据必须绑定完整 commit SHA，跨 SHA 证据不可继承）** 的缺陷类，
    而且出现在**属主要签的那份**文件里。
- **结论未受影响（这条很重要）**：`git diff --name-status 09ba360f6c HEAD` 实测 **12 个**改动路径，
  其中 **8 个**在 `docs/refactor/v4.1.0-c/takeover/` 下（裁决文件、台账、生成器、校验器），
  **4 个在 takeover/ 之外**：
  `docs/refactor/v4.1.0-c/handoff/01_ARCHITECTURE_BASELINE.json`、
  `docs/refactor/v4.1.0-c/handoff/10_HANDOFF_MANIFEST.json`、
  `docs/releases/v4.1.0/refactoring/TEST_COVERAGE_MATRIX.json`、
  `docs/releases/v4.1.0/refactoring/evidence/SOURCE_BINDINGS.json`。
  这 4 个的改动是**重签产物与矩阵字段改动**，**不是**泳道的输入：
  - `handoff/01_ARCHITECTURE_BASELINE.json`：只改了 `timestamp` 与 `source_bindings.sha256`；
  - `handoff/10_HANDOFF_MANIFEST.json`：只改了 `timestamp` 与三个被重签文件的 sha256
    （矩阵换了内容 ⇒ 它的哈希变了 ⇒ manifest 里记的哈希跟着重签）；
  - `evidence/SOURCE_BINDINGS.json`：只改了 `timestamp`；
  - `refactoring/TEST_COVERAGE_MATRIX.json`：改的是 44 条用例的**签署绑定字段**
    （`assertion_reviewed_at` 拆分并改名为 `assertion_judged_at`（同值）+ 新增
    `assertion_signed_at`（= `null`，副签没发生）、`assertion_reviewed_commit` 由 BASE 改指
    `2923612f51…`）—— **判定结果与签名均未改变**，见 `ORACLE-REVIEW-LOG-T1.md` §11.1 / §11.2。
  ⇒ **门禁脚本、负向夹具、门禁编目、夹具台账、产品代码一个都没动**
  ⇒ **RED = 5/7 这个测量与判据 8 = FAIL 这个判定不受影响**。
  （生成器现在**自己算**这件事：`gen_verdict.py` 会跑 `git diff --name-only <实测树> HEAD`
  并按「泳道实际读取的路径集合」分类，结论写在裁决 JSON 的 `criteria[7].note` 里。）
- 上一次（**不是**冻结提交那一棵树）记的是 **6/7 + 1/7**，跑在 commit
  **`b556199c86`**（来源 `task-8-report.md:1130`）——**那个 SHA 要写清楚**，因为它与 BASE
  不是同一个 commit，**跨 SHA 的证据不可继承**（C5）；
- **两次都不是 7/7**；
- 两次的差异来自**机器速度**（fx17 动过 Rust 源码后 composite 的 release 重建要 1–2 小时，
  fx15 的 7200s 预算这次没够上），**不是断言被放松**；
- ⇒ **判据 8 的结论对机器速度敏感**。在冷 target 上复跑可能得到不同的分布；无论哪种结果，
  只要不是 7/7，判据 8 就是 FAIL（INCONCLUSIVE 不得四舍五入成 RED，超时/信号也不得算 RED —— R50）。

---

## 2. 豁免记录

### 2.1 已裁决的豁免（**目前唯一一条**）

| 字段 | 值 |
|---|---|
| `waiver_id` | `WAIVER-V410-A5-12-LOCALONLY-ROUTE-20261123` |
| `waived_criterion` | **12** |
| `reason` | 跨仓库 LOCAL_ONLY 不纳入本次准入：`SQLRUSTGO_LOCAL_ONLY` 在 sqlrustgo 仓库**零命中**，它存在于 DSH 仓库（`deepseek-schedule/docs/test-factory/LOCAL_MODEL_ROUTING_POLICY.md`）；DSH 路由器的 LOCAL_ONLY 是**策略级**标志（`scheduler/router.py:277` / `:290` 的 `if local_only and str(rid).startswith("cloud"): continue`），**不存在 per-task 的数据分类维度**（`scheduler/` 内 `data_classification` / `DATA_CLASS` 零命中）⇒「缺分类即拒绝」在本次准入范围内**未成立** |
| `scope` | 本次准入范围（**不含**跨仓库 LOCAL_ONLY 路由）；仅覆盖 v4.1.0 C-ALPHA-02 的启动裁决 |
| `compensating_control` | 调度侧**显式指定 LOCAL_ONLY 策略**；分类缺失即 BLOCKED（C3）；本次准入期间的任何跨仓库派发仍须由调度侧显式指定 LOCAL_ONLY 策略 |
| `expires_at` | **失效条件（不写编造日期）**：当 DSH 仓库引入 per-task 数据分类维度、或 `scheduler/blockers.py` 的 `DATA_BLOCKED` / `SECURITY_BLOCKED` 开始由分类字段填充时，本豁免**自动失效**，须重新裁决；或最迟在 v4.1.0 进入 BETA 阶段时复核 |
| `approved_by` | **待属主确认（确认位见 §4 的「O9 顺带确认」栏，尚未签）** —— ⚠️ 见下方说明 |

> ### ⚠️ 判据 12 的豁免目前只有一份「自称 PROPOSED」的设计稿背书（2026-11-25 整支评审 I-8）
>
> 本记录原先写「属主裁决 O9（2026-11-23，见设计稿 §6.1『已裁决』表 O9 行）」。
> **复核的事实**：
> - 那份设计稿 `docs/refactor/v4.1.0-c/takeover/V410-REFACTOR-READINESS-DESIGN.md` 的**头部**写着
>   `design_status: **PROPOSED — 待属主批准后转为实施计划**`、`prepared_by: DSH-B 主导（本会话）`；
> - 在整个 `takeover/` 目录里**搜不到任何独立于执行方的、属主签署的 O9 记录**。
>
> ⇒ 全支**唯一一处**「记录了属主决定了某事」的地方，是由执行方自己写的**未批准提案**背书的。
> **实施者与控制者不得伪造一个属主签署，也不得把 O9 从「已裁决」降级**（那是属主的裁决）——
> 所以本波**只做两件事**：
> ① 把 `approved_by` 字段改成指向**§4 旁边新增的、留空的 O9 确认位**（不再指向那份自称 PROPOSED 的文档）；
> ② 在 §4 的裁决位**旁边**放一行「请属主顺带确认 O9（一行）」。
> **在属主填那一行之前，本条豁免的 `approved_by` 是空的。**

**豁免后果（不得静默跳过，设计稿 §A5 #12 明列）**：

1. 「缺分类即拒绝」在本次准入范围内**未成立**；
2. DSH 路由器的云端 fallback 拦截仅依赖**策略级标志 + 代码路径**，不依赖 per-task 数据分类，
   因此**单个任务的分类缺失不会导致自动 BLOCKED**；
3. 本次准入期间的任何跨仓库派发，仍须由调度侧显式指定 LOCAL_ONLY 策略；
4. 缺口转为独立待办，移交 DSH 仓库治理，**不阻断 C-ALPHA-02**。

### 2.2 预留的豁免槽位（**留空，由属主填**）

判据 **#4**、**#6** 与 **#8** **不豁免**。若属主决定豁免其中任何一条，须**由属主自己**填写下列字段
（实施者与控制者不得代填）：

#### 预留槽位 A —— 判据 6

| 字段 | 值 |
|---|---|
| `waiver_id` | __________________________________________ |
| `waived_criterion` | 6 |
| `reason` | __________________________________________ |
| `scope` | __________________________________________ |
| `compensating_control` | __________________________________________ |
| `expires_at` | __________________________________________ |
| `approved_by` | __________________________________________ |

#### 预留槽位 B —— 判据 8

| 字段 | 值 |
|---|---|
| `waiver_id` | __________________________________________ |
| `waived_criterion` | 8 |
| `reason` | __________________________________________ |
| `scope` | __________________________________________ |
| `compensating_control` | __________________________________________ |
| `expires_at` | __________________________________________ |
| `approved_by` | __________________________________________ |

#### 预留槽位 C —— 判据 4（**控制者修订第 1 轮扩授权**：原本只预授权 #6 与 #8）

> 判据 4 的 FAIL 来自 **A2.2 R1 未满足**（C01-15 的 `approved_adr_refs` 为 `[]`），
> 而**不是**机器字段缺失。属主有两条路：**(i)** 先裁决 O1（补 ADR 引用 or 裁定无需绑定），
> 判据 4 随后可复评为 PASS；**(ii)** 显式豁免本条并承担后果。**实施者与控制者都不得代填下列字段。**

| 字段 | 值 |
|---|---|
| `waiver_id` | __________________________________________ |
| `waived_criterion` | 4 |
| `reason` | __________________________________________ |
| `scope` | __________________________________________ |
| `compensating_control` | __________________________________________ |
| `expires_at` | __________________________________________ |
| `approved_by` | __________________________________________ |

> **不填也合法**：留空即表示「不豁免」。若属主豁免，请注明这是**基于本次实测**的判断，
> 并写明 §1.2 的时间敏感性（#8）与 §1.1 的 #4 行（A2.2 R1 未满足的具体位置）。

---

## 3. 残余未裁决项

### 3.1 设计稿 §6.2 的 13 项（逐条标注是否影响放行）

| # | 事项 | 是否影响放行 | 说明 |
|---|---|---|---|
| **O1** | 档位「仅批准方向/实施另审」（003/004/007/008/009/010）是否足够支撑 `approved_adr_refs` 填充 | **影响（= 判据 4 FAIL 的直接原因）** | `ADAPTER-MAPPING-PENDING.md` 的待决清单**非空**（C01-15，其 `approved_adr_refs` 保持 `[]`）。计划正文 Step 4 的口径是「**若非空则 A5 #4 不能判 PASS**」⇒ 判据 4 判 **FAIL**（修订第 1 轮已改判，依据见 §1 的 #4 判定依据段与 `V410-READINESS-VERDICT.json` 的 `criteria[3].observed`）。关闭它需属主二选一：裁定 C01-15 与任何 ADR 都无需绑定，或给出 A2.2 R2 要求的实际关联依据（**属主不裁决，执行者不得自拟映射** —— A2.2 R5 / O10）。若属主选择豁免本条，请在 **§2.2 的槽位 C** 填写 waiver 字段 |
| **O2** | 75 场景中哪些标记 `required_for_certification=true` | **影响（但不是 #6 的决定因素）** | 实测：75/75 全为 `false`（属主尚未标定）。计划正文说「O2 未裁决时 #6 以『L1 44 条有终态 + Oracle 签署 + 差异表』为满足」—— #6 已经因为 **Oracle 副签 0 条** 而 FAIL，与 O2 无关；但 O2 仍决定**阶段出口口径**（哪些场景算「认证必需」），须在放行批次里定 |
| O3 | 是否授权 83 道 dead/orphan 门禁的批量处置 | **不影响** | 明确不在 A4 内；本轮未处置任何门禁 |
| O4 | 「GPT-6 独立评审报告」指哪一份文件 | **影响** | 放行记录要引用它；仓库里有多份 Codex 产物，身份须属主指认 |
| **O5** | A5 通过后放行哪一批核心重构包 | **影响** | 18 包 DAG 内**无任何核心编码包**（全部 `preparation_test_or_analysis`）；放行批次须属主指定 |
| O10 | ADR→task 映射规则是否采纳 A2.2 的 5 条，或另定 | **影响** | A2.2 R1 与 O1 直接相关；#4 的口径取决于它 |
| O11 | BASE 是否直接推进 `develop/v4.1.0`，还是只落在 `base/v4.1.0-frozen` | **不影响** | BASE 已落在 `base/v4.1.0-frozen` 且该分支已推送；推进 develop 的节奏是另一件事 |
| O12 | `14_HANDOFF_MANIFEST.json` 的 `authority.upstream_c01_manifest_resign:false` 如何处置 | **不影响** | 本轮未触及；manifest 零漂移已由 #5 证明 |
| O13 | 是否允许为 `verify_handoff.py` 增加 `jsonschema` 依赖，或改用仓库内既有校验器 | **不影响（但已按现状绕过）** | 仓库内已有 `refactoring/tools/requirements.txt`（钉 `jsonschema==4.23.0`）。本机默认 python3 未装 ⇒ Task 9 用**临时 venv**（`/tmp/v410-task9-venv`，**不入仓**）跑 `--check`，结果 6/6、11 变异臂全拒。若属主要求把依赖固化进 CI，那是另一项裁决 |
| O14 | `TEST_EXECUTION_CONTRACT_DECISION.md` 的 RED 执行方案（A/B/C） | **影响** | 决定已知 RED 的执行与记账方式；本轮未裁决 |
| O15 | 矩阵驱动适配层是否算「新建 runner」 | **不影响** | `matrix_driver.py` 已实现并通过独立评审（Task 5/10） |
| **O16** | 「测试已消失」的分类（7 类契约无此类） | **影响** | 场景数未定；`NOT_RUN` 与「已消失」当前无法区分。实测：75 场景中 `execution_status` = PASS 42 / NOT_RUN 33；L1 切片的 2 条 `NOT_RUN` 是否属「已消失」未定 |
| O17 | ALPHA 执行面用 `check_stage.sh`（两态）还是 `run_gates.py`（三态） | **影响** | R51 已定：CI 执行点用两态 `check_stage.sh` ⇒ **CI 执行面丢失 INCOMPLETE 语义**；三态只在负向夹具泳道里用。已登记为残余 |

#### 3.1.1 属主授权的独立 Reviewer 名单（**判据 6 的机器输入**）

> **这张表是判据 6 的机器输入。** `gen_verdict.py` 只读下面两个标记之间的内容，
> 且**只认「第一列是序号」的行**；表里没有序号行 = **名单为空 = 判据 6 判 FAIL**。
> **实施者与控制者不得代填**（C7 / R-ORACLE）—— 这是属主专属的动作。

<!-- OWNER_APPROVED_REVIEWERS:BEGIN -->

| # | Reviewer 标识（= 副签时写入矩阵 `assertion_reviewed_by` 的值） | 授权范围 | 授权日期 |
|---|---|---|---|
| （**当前为空** —— 属主尚未指认任何独立 Reviewer） | | | |

<!-- OWNER_APPROVED_REVIEWERS:END -->

**判据 6 的新判定逻辑（2026-11-25 整支评审 I-2 重写）** ——
PASS 需**同时**满足以下**六条** —— 机器门槛是 `all(cond6.values())`，覆盖**六个**键
（**其中四条是实质条件，另两条是防空判护栏**）。下面六条与生成器在 `observed` 里逐条打印的
六个键**一一对应**（编号 (1)…(6) 即 `cond6` 的插入序；**键名用生成器原文**）：

1. `c6_verify_matrix_exit_0` —— `verify_matrix.py` 退出码 = 0；
2. `c6_diff_table_exists_and_header_matches` —— 双向差异表存在**且表头逐字符匹配**；
3. `c6_all_l1_have_terminal_status` —— L1 切片**全部**用例有终态；
4. `c6_owner_approved_reviewer_list_present` —— 属主授权的 Reviewer 名单**必须存在且可读**
   （**防空判护栏**）；
5. `c6_at_least_one_case_judged_pass` —— **至少有一条被判 PASS 的用例**
   （`assertion_review_verdict` 存在且不是 `ASSERTION_UNREVIEWABLE`）——（**防空判护栏**）；
6. `c6_every_judged_pass_case_qualified_signer` —— **每条被判 PASS 的用例**
   都有 `assertion_reviewed_by`，且该值 **≠** `assertion_review_performed_by`（R-ORACLE），
   **且**落在上面那张**属主授权名单**里。

**实质条件 = (1)(2)(3)(6)**；**(4)(5) 是防空判护栏** —— 缺了它们，「零条被判 PASS」会让 (6)
**空判为真**（`all()` 对空集合取真），判据 6 会被错误地判成 PASS。
判据 6 FAIL 时，`observed` 会把**六个键全部打印**并列出为假的键，可直接读出是哪一条不成立。

- **名单为空 ⇒ 一律判 FAIL**，并在裁决 JSON 的 `criteria[5].observed` 里写明
  「**没有属主授权的 Reviewer 名单**」；
- **已判 PASS 但签署人不合格 ⇒ 判 FAIL**，并在 `observed` 里写明
  「有 N 条声称已判通过，但签署人不合格」并逐条列出不合格原因；
- **属主合法授权 + Reviewer 逐条副签之后，判据 6 会自然变成 PASS** ——
  逻辑**没有**为「让 #6 保持 FAIL」而写死。

**为什么必须这样改（旧逻辑的缺陷）**：旧实现只数「`assertion_review_verdict` 非空」这一个字段，
**从不检查签署人是谁**。矩阵里当时只有 `assertion_review_performed_by`
（= 实施者本人 `task7-executor/minimax-m3.1-flash`），**根本没有 `assertion_reviewed_by` 字段**。
后果是：**属主指认 Reviewer 这个动作本身，会仅凭一个字段值把 #6 翻成 PASS。**
这正是 Task 7 评审第 1 轮点名留给 Task 9 的义务（台账 `:174`）。

#### 3.1.2 R19 覆盖损失的披露（Task 1 裁决 R19 要求的补记）

> **R19 的原文**（台账 `:80`）：「接受『B3 不再对 179 个导入路径做内容比对』这一覆盖损失 ——
> **此项须在 Task 9 的放行裁决记录中写明**」。本节就是那份补记。

| 项 | 值 |
|---|---|
| **放弃了什么** | `docs/releases/v4.1.0/refactoring/tools/verify_baseline.py` 的 **B3** 在 **HEAD 侧**不再比对导入材料的**内容**（Task 2 的 R17 把它从「blob 相同」改成「**路径存在性**」）。冻结记录冻结的是**身份**，不是**内容** |
| **实测范围** | 导入材料 = **123** 个文件（`git diff <冻结时点的 develop tip> BASE` 去掉 runtime-contract 目录的 56 个）；B3 的 HEAD 侧现在只查这 123 个路径**还在不在** |
| **靠什么顶上** | handoff manifest 的**逐文件 sha256** + `verify_handoff.py --check` 的全量复算。B3 的**源侧**内容比对仍在（`compare_blobs(MATERIAL_COMMIT, base, …)`）——丢的只是「HEAD 侧」那一半 |
| **实测覆盖率** | **123 个里有 122 个在 manifest 的哈希范围内**（实测：逐路径求交）；**第 123 个是 manifest 自身**（`docs/refactor/v4.1.0-c/handoff/10_HANDOFF_MANIFEST.json`），按**设计**被排除（manifest 不哈希自己） |
| **残余风险** | `verify_handoff.py --check` **复算全部 314 个哈希**（实测 `manifest_bound_files` = 314，退出 0）⇒ 122/122 的内容完整性被覆盖，残余风险小。**但**：这 122 个文件的完整性由**一个** manifest 承担，manifest 自身的完整性靠 `manifest_self_hash` 与外部锚点 |

#### 3.1.3 本支的校验器与自测**一个都没进 CI**（**待属主裁决 —— 接线属 C1**）

> **实施者不接线。** 接线是**门禁清单变更**，按 **C1 属主专属**。

| 项 | 值 |
|---|---|
| **实测事实** | 在全部 9 个 workflow（`.gitea/workflows/*.yml`）里检索 `test_verify*` / `test_matrix_driver` / `verify_baseline` / `verify_sha_scope` / `verify_handoff` / `verify_matrix` ⇒ **零命中**；ALPHA 阶段的 `required_gates`（`docs/governance/STAGE_CONFIG.yaml`）**也不含**它们 |
| **哪些东西没接线** | `test_verify_baseline.py`（**31** 断言 / 7 变异探针）、`test_verify_handoff.py`（**42** 断言）、`test_verify_sha_scope.py`（**43** 断言）、`test_matrix_driver.py`、`matrix_driver.py`；以及 `verify_baseline.py` / `verify_sha_scope.py` / `verify_handoff.py` / `verify_matrix.py` 四个校验器本身 |
| **台账上它还开着** | 台账 `:62`（Task 1 minor, deferred）：「`test_verify_baseline.py` 未接 CI（CI 用显式路径调 fixture，无发现式 glob）→ **并入 Task 8 范围**，届时由 owner 批准的门禁变更一并接线」。**Task 8 没有兑现它，也没有任何文档记着它还开着** —— 本节是它第一次被写进属主可见的文档 |
| **后果（属主必须知道）** | **12 条判据是一次性的人工测量**。2026-11-25 之后的任何改动可以让某条判据红掉，而**到处没有红** —— 没有回归保护 |
| **为什么那些自测重要** | 让这四个校验器可信的，正是那些变异探针自测：它们证明「校验器会 bite」（例如同一夹具跑在 pre-fix 校验器上会失败 16 / 24 条断言）。它们**从不自动执行** |
| **属主要裁什么** | (a) 把 4 个校验器 + 4 个自测接进某个 workflow（**需要属主指定 workflow 与触发条件**）；(b) 明确「判据是一次性人工测量，本支不提供回归保护」并接受它；(c) 别的口径。**实施者与控制者不得代为接线**（C1 / C2） |

#### 3.1.4 ADR 正文里的旧状态断言与新 `Status:` 并存（**只登记，不改正文**）

> **正文改动是 C6 下的属主重审动作 ⇒ 实施者不许改正文。** 本节只把矛盾列出来。

| 项 | 值 |
|---|---|
| **矛盾是什么** | ADR **正文**里**仍有旧的状态断言**，与 Task 4 落地的机器字段 `Status: APPROVED` **并存**。例：`docs/releases/v4.1.0/refactoring/adrs/ADR-410-001.md:33` 仍写「原则接受C不等于此ADR已批」（⚠️ 正文在 `docs/releases/v4.1.0/refactoring/adrs/` 下，**不在** `docs/refactor/v4.1.0-c/takeover/dsh-b/` —— 那里只有 `ADR-DECISION-RECORD.md`，是裁决记录，不是 ADR 正文） |
| **为什么属主今天会踩到** | ADR 是属主读裁决依据的入口；同一个文件里两个互相矛盾的说法，谁读到哪一句取决于读到哪一行 |
| **出处** | 台账 `:117`（Task 4 minor, deferred）：「ADR 正文深处仍有旧状态断言（如 `ADR-410-001.md:33`「原则接受C不等于此ADR已批」）与新的 `Status: APPROVED` 并存 —— 头部改了、正文没改，处理半吊子」 |
| **为什么本波不修** | 改正文 = 改 ADR 内容 ⇒ 触发 **C6**（ADR 批准指向固定 commit 的正文与 sha256，内容变更须重审）。**那是属主的重审动作** |
| **属主要裁什么** | (a) 授权按 Task 4 的 R31 口径（「纯元数据编辑不触发 C6；实质正文变更必须重审」）逐份判定哪些句子属实质条款，然后授权重审；(b) 裁定「旧断言视为历史陈述、不删」并在本文件登记；(c) 别的口径 |

#### 3.1.5 R2 校验器漂移**已关闭**；但 R2 构建链仍无异地副本（**待属主裁决 —— 要不要给 DSH 仓配 remote**）

> **漂移那一条已经关闭**（不再需要属主裁「以哪份为准」）；**异地副本那一条仍然开着**。
> 本节是这两条在属主可见文档里的落点（原先它们只登记在台账里）。

| 项 | 值 |
|---|---|
| **漂移（N-10）—— 已关闭** | **2026-11-25：控制者授权并执行了「同步 `verify_r2_package.py`」** —— 把本仓那份**复制**到 DSH 仓的 `r2-build/` 与 `sqlrustgo-v4.1.0-r2/` 两处 ⇒ **三处字节完全相同**（原先 DSH 那两份是 `75de8cce6773`）。**本文件不记该文件的 sha256**：校验器自己的头注释是它字节的一部分，哈希会随下一次编辑立即过期；**可复算的字节同一性校验入口**在 `R2-RELEASE-PACKAGE.md` §9.1（`diff` + `shasum` 三条路径，当场可验）。DSH 侧最新提交 `0964dda681a8`；**该仓无 remote ⇒ 未 push**。同步后两个家都自解析正确、实跑均 **23 PASS / 0 FAIL**；R2 的 ZIP **21** 个条目、检索 `verify_r2` **0 命中** ⇒ **R2 包哈希未变**。详见 `R2-RELEASE-PACKAGE.md` §9.1 |
| **仍然开着的那一条** | **R2 构建链无异地副本、DSH 仓无 remote。** 构建链（`r2-build/`，`build_r2.sh` 是唯一入口）在 `…/DSH-EXTERNAL-REVIEW-001/r2-build/`，而**该仓没有任何 git remote**（`git remote -v` 为空，只有 `main` 一个分支）⇒ 构建链**没有异地副本**，也**无处可推**。**校验器同步没有改变这一条**（同步的是**只读**的校验器，不是**可写**的构建链） |
| **属主要裁什么** | 是否给 DSH 仓**配 remote**（配到哪里、是否推到与本仓同源的远端）；不配 ⇒ R2 的构建链只有一份、且只存在于某台机器的工作区里（**可复现性缺口**）。**实施者与控制者不得代为配置远端** |
| **出处** | 台账「最终残余（交给属主）」：「两份 `verify_r2_package.py` 仍未同步（DSH 那份 `75de8cce…` vs 本仓 `e8155879…`，内容等价、字节不同）；R2 构建链无异地副本（DSH 仓无 remote）⇒ 需属主放行「同步这一个文件」并决定是否给 DSH 仓配 remote」—— **前半已关闭**（台账已由控制者同步并追加更正条目），后半**仍然开着** |

### 3.2 Task 6/7/8 的裁决缺口（来自 `V410-READINESS-STATE.md` §5）

| # | 事项 | 是否影响放行 | 说明 |
|---|---|---|---|
| N1 | 在基线里注册 `[[test]] tf_t1_undo_invariants`，vs 把 `PILOT-T1-101/102` 记为不可认证 | **影响** | 决定 L1 切片能否作验收凭据 |
| N2 | `assembly=UNVERIFIED` 的场景可否记 PASS | **影响** | DSH-B 字面读法下「42 PASS」不可作验收凭据，而该切片不含任何 RED |
| N3 | ledger #10/#11 的 `source_sha256` 滞后一次改写 | **影响（台账自洽）** | 材料层 |
| N4 | ledger 实为 18 行而其正文自称 17 | **影响（台账自洽）** | 材料层 |
| **R-T7-1** | Oracle 副签人身份 —— 目前**无可用 Reviewer** ⇒ Oracle 步骤 **BLOCKED** | **影响（= #6 FAIL 的直接原因）** | 签署层；需属主指认/授权独立 Reviewer |
| R-T7-2 | 第 2 段签署是在错误指示下进行的（3 条已撤回） | **影响** | 签署层 |
| R-T7-3 | `ORACLE-REVIEW-LOG-T1.md` 在 manifest 哈希范围之外 | 影响（弱） | 材料层；Oracle 日志本身不受 receipt 绑定 |
| GAP-13 | 两条 R3 测试所需能力在冻结基线上不存在 | **影响** | 能力缺口 |
| GAP-14 | 3 条用例的原映射为标签派生，已撤下 | **影响** | 映射缺口 |

### 3.3 不是判据、但是「可以开始重构」与「可以放行」之间的实质差距

| 项 | 实测 | 影响 |
|---|---|---|
| **13 条不变量零覆盖** | `verify_matrix.py` 的 M7：75 条里 63 条 `NO_INVARIANT_MATCH`，只声明了 5 个 invariant id（INV-01×5、INV-02×1、INV-04×2、INV-07×1、INV-14×4）⇒ 13 条不变量**零覆盖**（INV-01/02/04/07/14 之外） | **影响**（材料层，但覆盖不了架构不变量的语义面）。R43–R46 已定：宁可留 13 条零覆盖，也不保留无法复核的覆盖。放行时这 13 条**必须**随批次一起指定责任包 |
| **3 条断言仍不可独立判定** | T1-009 / T3-002 / T3-003（R-T7-2 撤回的 3 条） | **影响**（签署层）。它们不能被签署，也不该被凑数签署；属主须决定它们是「不认证」还是「补证据后认证」 |
| **冷 target 下 ALPHA 需 180 分钟** | 端到端 ALPHA 在**热 target** 下 23m58s 跑完；**冷 target**（CI 的真实情形）下 composite 的 release 构建要重跑，90 分钟预算两次撞墙，**180 分钟**（workflow 的 `timeout-minutes`）才跑完 | **影响**（工程/成本面）。`ci-alpha-stage.yml:89` 的 `timeout-minutes: 180` 就是这条的产物；冷 cache 下 180 分钟仍可能紧张。它同时是判据 8 在本轮不达 7/7 的**技术原因之一**（见 §1.2） |

---

## 4. 属主裁决位（**留空 —— 由属主填写**）

```
属主裁决：□ 放行 C-ALPHA-02　　□ 不放行

签署人：____________________

签署日期：____________________

放行批次（spec O5，具体放行哪些包）：____________________

若豁免判据 **4 / 6 / 8**，请在 §2.2 的对应槽位（**A = 判据 6、B = 判据 8、C = 判据 4**）填写 waiver 字段。

附加条件 / 限制：____________________
```

### 4.1 O9 顺带确认位（**留空 —— 一行**）

> **为什么要单列一行**：判据 12 的豁免是全支**唯一**一条「记录了属主决定了某事」的地方，
> 而它现在的背书是一份**自称 PROPOSED** 的设计稿（`V410-REFACTOR-READINESS-DESIGN.md`
> 头部：`design_status: PROPOSED — 待属主批准后转为实施计划`、`prepared_by: DSH-B 主导`）。
> 整个 `takeover/` 目录里**没有**任何独立于执行方的、属主签署的 O9 记录。
> **实施者与控制者既不伪造签署，也不把 O9 从「已裁决」降级**（那是属主的裁决）。
> **所以只请您顺带写一行。**

```
判据 12 豁免的 O9 确认：□ 确认（跨仓库 LOCAL_ONLY 不纳入本次准入，显式豁免）　　□ 不确认 / 另议

确认人：____________________

确认日期：____________________
```

> **这一行不填，判据 12 的 `approved_by` 就是空的**（§2.1 的表已如实写成「待属主确认」）。
> **它不影响判据计数** —— 判据 12 判 WAIVED 与否由属主的豁免本身决定，不由这一行决定；
> 这一行只决定**那条豁免记录有没有属主背书**。

> **实施者不代签。** 在属主填写本节之前，本项目的状态是：
> **材料层已自洽，证据层与签署层仍有缺口；不得对外宣称「已放行」。**

---

## 5. 控制者的建议（**这是建议，不是裁决**）

> 本节由控制者/实施者提供，供属主参考。**它没有裁决权，也没有被任何一方签署。**
> 属主的裁决写在 §4。

**建议：现在**不**放行 C-ALPHA-02**，先补齐三件事（#4 映射、#6 签署、#8 证据），再重新提交本表；或者，由属主显式豁免
#4、#6 与 #8（§2.2 的**三个**槽位：A = 判据 6、B = 判据 8、C = 判据 4）并承担其后果。

理由（按层）：

0. **映射层要属主拍板**（#4）。ADR 的**机器字段**齐了、10 条变异臂全被拒，但 A2.2 **R1**
   「每个 task 的 `approved_adr_refs` 至少 1 项」被 **C01-15** 违反（`ADAPTER-MAPPING-PENDING.md`
   的待决清单非空，且清单明写「明知违反 R1 而保留」）。这条**不需要写代码**，需要属主裁决 **O1**：
   (a) 裁定 C01-15 与任何 ADR 都无需绑定（则该行从清单移除、`approved_adr_refs` 保持 `[]`
   并在放行记录里写明该口径，判据 4 可复评为 PASS）；或 (b) 裁定它应绑定某份 ADR，
   则按 A2.2 R2 给出实际关联依据，执行者据此回填并重签 manifest。**属主不裁决，执行者不得自拟映射。**
1. **材料层不需要再修**（但**映射层有一处要属主拍板**，见第 0 条）。#1/#2/#3/#5/#9/#10/#11 全绿：基线唯一、
   SHA 分界清晰、ADR **机器字段**落地、manifest 零漂移、门禁编目自洽、外审包三套统计一致且 dead 口径唯一。
   再花一轮不会让放行更近。**#4 不在这张绿名单里** —— 它的 ADR 机器字段确实是绿的（10 条变异臂全被拒），
   缺的是**映射**（A2.2 R1 被 C01-15 违反），属主不裁决 O1 之前它一直是 FAIL。
2. **签署层是「有没有人签」的问题，不是「材料够不够」的问题**（#6）。44 条断言的技术判定都在，
   缺的是一个**属主授权的独立 Reviewer** 的副签。这件事不需要写代码，需要属主**指认一个人** ——
   指认之后**把它填进 §3.1.1 的名单表**（判据 6 的机器输入）。
   **⚠ 判据 6 的判定逻辑本波已重写（I-2）**：现在光「有一个字段非空」不够，
   签署人必须**在名单里**且**≠ 判定人**；名单为空 ⇒ 一律 FAIL。
   **这条设计是为了防止「属主指认 Reviewer 这个动作本身就把 #6 翻成 PASS」。**
3. **证据层是「红能不能归因」的问题**（#8）。两个子问题：
   （a）覆盖率门禁在**基线树上就已红**（RED 特征测试按设计失败）⇒ 夹具注入的红不可归因；
   （b）**机器速度**：composite 的 release 重建要 1–2 小时，7200s 预算不够 ⇒ 夹具泳道在冷 target 上
   天然跑不完 7/7。这两条都需要**设计上的处置**（归因拆分 / 预算与缓存策略），不是重跑一次能解决的。
4. **外审包已经可以发**：R2 已生成并自校验（23/23），R1 逐字节未变，`owner/approvals/` 空着等属主签。
   R2 的外发批准是**独立于** C-ALPHA-02 放行的一步 —— 属主可以直接签 R2 的外发申请。
5. **还有三件事属主必须知道，但它们不改变任何判据**：
   （a）**判据 12 的豁免目前只有一份自称 PROPOSED 的设计稿背书**（I-8）—— 请在 **§4.1 顺带写一行**；
   （b）**R19 的覆盖损失**（I-5）—— B3 不再在 HEAD 侧比对导入材料的内容，
   靠 manifest 的逐文件 sha256 顶上（实测 123 个里 **122** 个在哈希范围内，第 123 个是 manifest 自身），
   详见 **§3.1.2**；
   （c）**本支的校验器与自测一个都没进 CI**（I-6）—— 12 条判据因此是**一次性的人工测量**，
   没有回归保护；接线属 **C1 属主专属**，详见 **§3.1.3**。

**建议的下一步（顺序）**：

0. 属主裁决 **O1**（C01-15 的映射口径）⇒ 判据 4 复评（见上面第 0 条）；
1. 属主指认 **1 名独立 Reviewer**（Codex 或 DSH Reviewer 池），**填进 §3.1.1 的名单表**，
   授权其对 L1 的 44 条逐条副签（不可独立判定的 3 条按 R-T7-2 留空或明示不认证）；
2. 属主裁决 **O2**（哪些场景 `required_for_certification=true`）与 **O5**（放行批次），
   并对 **O1/O10** 给出 C01-15 的口径；
3. 决定 #8 的处置：改覆盖率门禁的**归因口径**（让夹具注入的红与基线自带的红可区分），
   并把冷 target 的预算提到能跑完 7/7 的水平（≥180 分钟，或把 composite 拆成可缓存的子步）；
4. 复跑 `verify_neg_fixtures.sh`，若达 7/7 ⇒ #8 转 PASS；
5. 复跑 Task 9 的判据汇总（`gen_verdict.py`，一条命令）→ 重出本表 → 属主在 §4 签字；
6. 顺手把 **§4.1 的 O9 确认**写一行（判据 12 的豁免背书）；
7. 就 **§3.1.3（校验器接 CI）** 与 **§3.1.4（ADR 正文旧断言）** 给出裁决口径。

---

## 6. 本文件的边界（自陈）

- 本文件**没有**代属主签字、**没有**填属主名字、**没有**写「已放行」；
- 12 条判据的**数字**不在本文件里，全部在 `V410-READINESS-VERDICT.json`（由 `gen_verdict.py`
  实跑命令后生成）；本文件只引用判定与归因；
- 豁免只有 #12 一条，且它的 `approved_by` 指向 **§4.1 的留空确认位** ——
  **不再**指向那份自称 PROPOSED 的设计稿（I-8）；**#4、#6、#8** 的豁免槽位**全部留空**（A / B / C）；
- 建议（§5）与裁决位（§4）**分开**，且已标明「这是建议，不是裁决」；
- 本次创建**四个**新文件（`V410-READINESS-VERDICT.json` / `R2-RELEASE-PACKAGE.md` /
  本文件 / `verify_r2_package.py` —— 判据 10 / 11 的可复算锚点，修订第 1 轮按控制者裁定 M-8 追加），
  修订第 2 轮按控制者裁定 R69 再加**生成器** `gen_verdict.py`（裁决文件从此**可从生成它的那棵树**复现 ——
  ⚠️ 不是「逐项一致」的承诺：判据 1 记的远端 tip、判据 8 的树身份都随时间变，
  见 §1 与 §1.2）；
  另按 R67 的唯一例外更新 `V410-READINESS-STATE.md`；**未改动** Task 1–8 的任何产物
  （**唯一例外**：最终整支评审修复波改了矩阵的 `assertion_reviewed_commit` / `assertion_judged_at` /
  `assertion_signed_at` 三个字段，见 `ORACLE-REVIEW-LOG-T1.md` §11 —— **判定与签名均未改变**）。
