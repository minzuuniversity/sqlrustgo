# V410 基线冻结声明 —— v4.1.0 内容基线（BASE）冻结完成

> ## 本文件的性质
>
> 本文件**宣告**一件事：**v4.1.0 的内容基线（BASE）已经冻结完成**。
>
> 本文件**不宣告**重构准入的任何结论 —— 既不宣告准入成立，也不宣告准入被拒。
> 12 条判据里有 **2 条**是 FAIL（#4 映射层 / #6 签署层），准入处于**分层收口态**。
> 判据 #8（证据层）在本轮由 FAIL 转 PASS。
> 属主裁决位（`docs/refactor/v4.1.0-c/takeover/V410-OWNER-RULING.md` §4）**仍然是空的**。
>
> **AFP Type A/B/D 一律不触。** 本文件不涉及任何阶段跃迁、任何签署、任何放行批次的授予。

---

## 证据头（AFP 五字段）

| 字段 | 值 | 出处 |
|---|---|---|
| `source_agent` | `dsh-b`（takeover 线 Task 9 实施者） | `docs/refactor/v4.1.0-c/takeover/V410-READINESS-VERDICT.json` → `generated_by` |
| `source_run` | `20261126-criterion8-negfix-001`（判据 #8 的两层夹具泳道，RED=7/7）；判据表由 `gen_verdict.py` 在 `docs/refactor/v4.1.0-c/takeover/` 内复算 | `V410-READINESS-VERDICT.json` → `criteria[7].observed` |
| `timestamp` | 2026-11-26（本声明签发日）；BASE 冻结时点 `2026-11-23T11:42:14.812504+00:00` | `docs/refactor/v4.1.0-c/takeover/BASELINE_FREEZE.json` → `frozen_at` |
| `evidence_hash` | 判据表 sha256 `b4044400a8a24e7932f3f38410fbd47556458e65e6b72fd881633d8ef6b93a36` | 本文件签发时对 `V410-READINESS-VERDICT.json` 实算（判据 #8 转 PASS 后的版本） |
| `conflict_resolution` | 见 §2 的分层归因表：3 条 FAIL 按**映射层 / 签署层 / 证据层**归因，**不合并、不取交集、不折中**；每条 FAIL 的补齐责任人单独写明。**#8 已由本轮修复并转 PASS** ⇒ 剩下 2 条（#4 映射层 / #6 签署层） | `V410-OWNER-RULING.md` §1.1 |

**BASE 身份（冻结记录的权威副本）**：

| 项 | 值 |
|---|---|
| `base_sha` | `fd849ae20541901d5e59f45cd3ba2c51093f5d5e` |
| `tree_sha` | `72d378e8e58797eae18502ab474325015b8993d4` |
| `source_ref` | `base/v4.1.0-frozen`（已推送远端） |
| `parent`（冻结时点的 develop tip） | `8cbde5b6d33c876fa1197474c46d50142a9823bd` |

出处：`docs/refactor/v4.1.0-c/takeover/BASELINE_FREEZE.json`。

---

## 1. 内容基线冻结完成

**结论：BASE 唯一、完整、已推远端、已成为 develop 的祖先。** 逐条判据与出处如下 ——
每条结论都能在 `V410-READINESS-VERDICT.json` 的 `criteria[]` 里按索引复核。

| 判据 | 结论 | `criteria[]` 索引 | 实测出处（`observed` 内的可核事实） |
|---|---|---|---|
| **#1** BASE 唯一、develop tip 是其祖先、含 BASE 的分支已推送 | **PASS** | `criteria[0]` | `[B1] PASS develop tip 8cbde5b6d3 is an ancestor of HEAD`；`[B2] PASS base_sha fd849ae205, tree_sha 72d378e8e5, source_ref base/v4.1.0-frozen and frozen_at all match the repository`；`[B4] PASS ledger complete`；`git ls-remote` 与 `git merge-base` 的实测输出 |
| **#2** `45da6968ae` 的 123 个材料文件全在 BASE 树内 | **PASS** | `criteria[1]` | `[B3] PASS 123 material file(s) + 56 runtime-contract file(s) imported over develop, all still present in HEAD` |
| **#3** 前向/历史 SHA 分界（前向绑定唯一等于 BASE；历史绑定逐项未变） | **PASS** | `criteria[2]` | `verify_sha_scope.py` 的实测输出（`exit=` 后的逐条断言） |
| **#5** manifest 零漂移 | **PASS** | `criteria[4]` | 冻结记录集 `audit/v4.1.0/runs/*/manifest.yaml` 逐字节比对结果 |
| **#7** ALPHA 阶段清单有 CI 执行点 | **PASS** | `criteria[6]` | `criteria[6].observed` 的四项实测布尔 |
| **#8** ALPHA 7 道负向夹具两层 RED | **PASS**（本轮转 PASS） | `criteria[7]` | `RED=7/7 INCONCLUSIVE=0/7`，退出码 0，零 `exit_code=124`；归档 run `audit/v4.1.0/runs/20261126-criterion8-negfix-001/`；树身份为**观测值**（泳道日志自报 `tree_sha=5324d2348c… dirty_files=0`） |
| **#9** 门禁编目内部自洽 | **PASS** | `criteria[8]` | `scripts/gate/gate_registry.py check` 的实测输出：`check_n=168`、`catalog_rows=168`、`fails=0` |
| **#10** 外审包三套统计一致（71/71/71） | **PASS** | `criteria[9]` | 三套统计的 SSOT 行 + `PASS #10` / `FAIL #10` 逐条 |
| **#11** dead 门禁口径唯一化 | **PASS** | `criteria[10]` | dead 口径的实测计数 |

**BASE 的合入路径**：BASE 经 PR **#5928**（`Merge pull request 'Base/v4.1.0 frozen'`）合入 `develop/v4.1.0`；
其后 #5929 / #5930 / #5931 / #5932 均为 `base/v4.1.0-frozen` → `develop/v4.1.0` 的合并。
`develop/v4.1.0` 的 tip 在本声明签发时为 `a604ed9ab3`（PR #5932）。

**冻结的是「内容」，不是「准入」。** 冻结记录（`BASELINE_FREEZE.json`）里登记的是 123 份材料文件
+ 56 份 runtime-contract 文件的导入、11 份 ADR 的机器字段、以及冻结记录集的一次**登记式**扩容
（`historical_set_additions`，登记 `20261124-task8-alpha-negfix-001` 的 manifest）。冻结不改变任何
门禁的判定口径，也不构成任何放行。

---

## 2. 重构准入状态：2 条 FAIL 的分层归因

**12 条判据的当前分布：PASS 9 · FAIL 2（#4 / #6）· WAIVED 1（#12）。**
出处：`V410-READINESS-VERDICT.json` → `summary` → `{"total":12,"pass":9,"fail":2,"waived":1,"fail_criteria":[4,6]}`。

**判据 #8（证据层）在本轮由 FAIL 转 PASS** —— ALPHA 7 道负向夹具两层 RED 达 **7/7**
（`RED=7/7 INCONCLUSIVE=0/7`，退出码 0，零 `exit_code=124`），归档 run
`audit/v4.1.0/runs/20261126-criterion8-negfix-001/`。
它的树身份现在是**观测值**（泳道日志自报 `tree_sha=5324d2348c… dirty_files=0`），
不再是事后重构的推定值。

下表**照抄** `V410-OWNER-RULING.md` §1.1 的归因结论，**不改判**（#8 一行保留，作为本轮
修复前的原始归因记录）：

| 判据 | 缺在哪一层 | 具体缺口（§1.1 原文要点） | 补齐它需要谁 |
|---|---|---|---|
| **#4** | **映射层**（既不是材料层、也不是证据层、也不是签署层） | ADR 的**机器字段**（11 份的档位、`Approvals` 字段、`adr_catalogue`）**齐了**，`verify_handoff.py --check` 的 **10 条**变异臂**全被拒**；缺的是**映射**：A2.2 **R1**「每个 task 的 `approved_adr_refs` 至少 1 项」被 **C01-15** 违反（`ADAPTER-MAPPING-PENDING.md` 的待决清单非空）。该包与 11 份 ADR 的「目标不变量」**无交集**（唯一不变量 INV-17 是测试工厂的认证口径不变量，不是架构不变量），所以「填一个无关 ADR 去凑数」被 A2.2 R2 明令禁止 | **属主裁决 O1**（spec §6.2，与 O10 同源）：二选一 —— (a) 裁定 C01-15 与任何 ADR 都无需绑定，则本行从清单移除、`approved_adr_refs` 保持 `[]` 并在放行记录里写明该口径；或 (b) 裁定它应绑定某份 ADR，则按 A2.2 R2 给出实际关联依据，执行者据此回填并重签 manifest。**属主不裁决，执行者不得自拟映射**（A2.2 R5 / O10） |
| **#6** | **签署层** | 材料齐了（44/44 有终态：42 PASS + 2 NOT_RUN；双向差异表 `L1-DIFF-fd849ae20541.md` 表头逐字符匹配），但**Oracle 副签 0 条** —— 44 条的 `assertion_review_verdict` 是 38 条 `ASSERTION_UNREVIEWABLE` + 6 条 `JUDGED_PENDING_COUNTERSIGNATURE`，`assertion_review_performed_by` 是**实施者本人**（`task7-executor/minimax-m3.1-flash`），按 R-ORACLE「签署人 ≠ 该场景实现者」不构成签署 | **属主授权的独立 Reviewer**（Codex 或 DSH Reviewer 池）；实施者不得自签（C7 / R-ORACLE）。属主把 Reviewer 填进 §3.1.1 的名单表后，判据 6 才可能转 PASS |
| **#8** | **证据层** | 夹具在**结构上**无法成立：ALPHA 7 道门禁的负向夹具泳道未达 7/7。缺口不是「断言被放松」，而是（a）覆盖率门禁 `check_coverage_v312.sh` 的坏产物注入**在完整树上就已红**（Task 1–7 导入的 RED 特征测试 `crates/storage/tests/tf_t4_*.rs` 按设计失败 ⇒ 红不可归因于夹具注入）；（b）**机器速度**：fx15 的 composite 门禁在 fx17 动过 Rust 源码后要重建 release，1–2 小时量级，fx15 的 7200s 预算本轮不够用 | 证据层：要么把覆盖率门禁的红**归因**拆开（让夹具注入的红与基线自带的红可区分），要么在**冷 target** 下给足预算 |

> **材料层全绿**：#1/#2/#3/#5/#7/#9/#10/#11 全部 PASS —— 基线、SHA 绑定、ADR **机器字段**、manifest、
> 门禁编目、外审包三套统计与 dead 口径都自洽。**这次整备的成果没有缺口。**
> **证据能归因**这一条已在本轮补上（#8 转 PASS）；**还缺**的是「映射要属主拍板」与「有人签」。

**判据 12（WAIVED）**：唯一一条已登记的豁免
`WAIVER-V410-A5-12-LOCALONLY-ROUTE-20261123`。它的 `approved_by` 字段目前是
「**待属主确认（确认位见 §4 的「O9 顺带确认」栏，尚未签）**」—— 豁免本身已裁决，
但**那条记录还没有属主的背书**。见 §5。

---

## 3. 分层放行的层序与解锁条件

**层序是有依赖的，不能颠倒。** 每一层只解锁下一层的一小步；每一层的产物都是下一层的输入。

| 层 | 对象 | 解锁条件 | 当前状态 |
|---|---|---|---|
| **层 0** | 判据 **#8**（证据层） | ALPHA 7 道负向夹具两层 RED 达 **7/7** | ✅ **本轮达成** —— `RED=7/7 INCONCLUSIVE=0/7`，退出码 0，零 `exit_code=124`；归档 run `audit/v4.1.0/runs/20261126-criterion8-negfix-001/` |
| **层 0'** | 判据 **#4**（映射层） | **属主裁决 O1**：裁定 C01-15 的映射口径（A2.2 R5 / O10） | **未解锁** —— 属主域，执行者不得自拟 |
| **层 1** | 判据 **#6**（签署层）· T1 30 条**逐条**副签 | 属主把独立 Reviewer 填进 `V410-OWNER-RULING.md` §3.1.1 的名单表（该表是判据 6 的机器输入；表里没有序号行 = 名单为空 = 判据 6 判 FAIL） | Reviewer 已获属主授权为独立 Reviewer 通道；**名单表仍待填写** |
| **层 2** | 判据 **#6**（签署层）· T2/T3 14 条**批量**副签 | 层 1 完成（同一位 Reviewer 的副签资格已验证） | 未解锁 |
| **层 3** | **核心重构包** | **属主裁决 O5**（放行批次） | 未解锁 —— 属主域 |

**层 1 与层 2 的顺序是「先 30 条逐条、再 14 条批量」**：先在小批量上验证 Reviewer 的副签资格
（判据 6 的条件 (6)「每条被判 PASS 的用例都有 `assertion_reviewed_by`，且该值 ≠
`assertion_review_performed_by`，且落在属主授权名单里」），再把资格扩大到批量。

**层 0 与层 0' 互不依赖**：#8 是执行者域（证据层，可由执行者通过修复归因与预算解决），
#4 是属主域（映射层，执行者不得自拟）。因此 #8 可以先于 #4 完成 —— 但**两条都完成之前，
准入不能宣告**。

---

## 4. 本声明**不**宣告的事

以下每一条都是**显式否定**。本文件的任何读者都不得从本文件推出相反的结论。

1. **不宣告重构准入的任何结论。** `V410-OWNER-RULING.md` §4 的裁决位（放行 / 不放行、签署人、
   签署日期、放行批次、附加条件）**全部是空的**，本文件**没有**代填，也**没有**代签。
2. **不宣告 `current_stage` 变更。** `docs/releases/v4.1.0/STAGE.yaml` 的 `current_stage` 保持
   **`"ALPHA"`**，**本轮不改该文件**。C-ALPHA-02 是 ALPHA **之内**的重构阶段，不是阶段跃迁。
3. **不宣告任何门禁阈值被放宽。** 本轮**不改** `docs/governance/STAGE_CONFIG.yaml` 的
   `COVERAGE_MIN_*`（ALPHA 50 / BETA 75 / RC 80 / GA 85），**不改** 168 分母，**不新增** `check_*`。
4. **不宣告「材料层全绿 ⇒ 准入成立」。** 材料层全绿只说明**这次整备的成果没有缺口**；
   **还缺**的是映射（#4）与签署（#6）—— 两者各自独立地挡在准入前面
   （证据（#8）已于本轮补齐，但它补的是**可归因性**，不是准入本身）。
5. **不宣告判据 12 的豁免已有属主背书。** 见 §5 第 2 条。
6. **不宣告「双血统咨询可得」。** 见 §5 第 1 条。

---

## 5. 残余风险

### 5.1 deepseek-web 通道不可用 ⇒ 双血统不可得

**实测（2026-11-26，本会话）**：

| 通道 | 探活结果 | 结论 |
|---|---|---|
| `chatGPT-web`（`http://127.0.0.1:9656/v1`） | `available: true`，模型 `web-session-chat` / `web-session-thinking` | **可用** |
| `deepseek-web`（`http://127.0.0.1:9655/v1`） | `health: ok`，但 `/v1/models` 返回 **HTTP 401 Unauthorized**，`available: false` | **不可用** |

`health: ok` 与 `available: false` 同时出现，是因为健康端点只证明**服务进程**活着，不证明
**模型调用**可用 —— 这正是 Q6 记录的同一类缺口。

**影响**：层 1 / 层 2 的独立 Reviewer 副签如果要求**双血统**（两个互相独立的外部咨询通道各出
一份意见），那么在 deepseek-web 恢复之前**凑不齐双血统**。当前**不得**把单血统结果当作双血一致
（这是 GA-SOAK-44 验收条件 ① 的同一条纪律）。chatGPT-web 单通道**可用** ⇒ 层 1 具备「有人签」
的前提，但**双血统**这一额外要求需要 deepseek-web 恢复。

**责任人**：属主恢复 deepseek-web 的可用性（上游认证/登录）。

### 5.2 判据 12 豁免的 `approved_by` 仍为空

`WAIVER-V410-A5-12-LOCALONLY-ROUTE-20261123` 的 `approved_by` 是
「**待属主确认（确认位见 §4 的「O9 顺带确认」栏，尚未签）**」。
唯一需要属主写的一行在 `V410-OWNER-RULING.md` §4.1：

```
判据 12 豁免的 O9 确认：□ 确认（跨仓库 LOCAL_ONLY 不纳入本次准入，显式豁免）　　□ 不确认 / 另议
确认人：____________________
确认日期：____________________
```

**这一行不填，判据 12 的豁免记录就没有属主背书。** 它**不影响判据计数**（判据 12 判 WAIVED 与否
由属主的豁免本身决定），但它决定**那条豁免记录有没有属主背书**。实施者不代签。

### 5.3 13 条不变量零覆盖

外审包（判据 #10 的对象）里的 **13 条不变量目前零覆盖** —— 没有任何测试在它们身上留下证据。
这与判据 #10「三套统计一致（71/71/71）」**不冲突**：#10 测的是**统计口径自洽**，不是**覆盖充分性**。
零覆盖是**已知的、已登记的**缺口，不是新发现的缺陷。

**责任人**：随批次指定责任包（属主在 §4 的「放行批次」栏指定）。

### 5.4 覆盖率门禁在冻结基线上的判定口径变更

本轮（判据 #8 的证据层修复）改的是**覆盖率门禁在冻结基线上的判定**：从「不可测 ⇒ FAIL
（measured 7 of 8 crates）」变成「可测的真实数字」。这是**属主级门禁行为变更**，已在
本文件、PR body 与看板 §3/§6 三处显式列出。

**变更的边界**：只排除**测量**（`scripts/gate/common.sh` 的
`COVERAGE_RED_CHARACTERISATION_TEST_FILES`），**不改** `#[ignore]`、**不改**任何断言、
**不改**任何阈值档；`coverage_verdict` **一字未改**（其它任何原因导致 crate 不可测仍然 FAIL 并
点名该 crate）。RED 特征测试**仍然 RED**（#5913/#5914 仍挂在 RED-LEDGER 上）。

**责任人**：属主签字。

---

## 6. 可核性

```bash
# 判据表（12 条逐条）
python3 docs/refactor/v4.1.0-c/takeover/gen_verdict.py --check

# 门禁编目（分母必须仍是 168）
python3 scripts/gate/gate_registry.py check

# 诚实性（每个「声称能抓」都必须落在夹具证据上）
python3 audit/v4.1.0/honesty.py check

# 负向夹具台账
python3 audit/v4.1.0/probe_neg_fixtures.py check

# 冻结记录
cat docs/refactor/v4.1.0-c/takeover/BASELINE_FREEZE.json
```

**本文件自身的 sha256 随 PR 一并归档在 PR 的 Evidence 段。**

---

## 7. 签署

**本文件不设签署位。** 签署位在 `V410-OWNER-RULING.md` §4（放行裁决）与 §4.1（O9 顺带确认），
本文件**没有**代填、也**没有**代签其中任何一项。

| 角色 | 姓名 | 日期 | 状态 |
|---|---|---|---|
| 生成（执行者） | `dsh-b` / Task 9 实施者 | 2026-11-26 | 已签（生成即记录） |
| 裁决（属主） | — | — | **待签** —— `V410-OWNER-RULING.md` §4 / §4.1 |
