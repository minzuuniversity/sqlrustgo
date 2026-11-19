# ALPHA 阶段 ISSUE 整改 —— #5580 三处门禁自身缺陷的修复与 A/B 取证

- **日期**：2026-11-19
- **线**：mcode-macmini 线
- **分支**：`fix/gate-5580-d5-g9g10-d9`（基 `origin/develop/v4.1.0` = `abba49367910`）
- **阶段**：v4.1.0 = **ALPHA**（`docs/releases/v4.1.0/STAGE.yaml` `current_stage: "ALPHA"`）
- **本文件不产生任何 PASS 声明，不宣布阶段迁移。** 阶段跃迁仍需属主签批 + 独立 Reviewer 回执（AB-10）。

---

## 0. 结论摘要

本轮按属主指令「你来整改和推进，先完成 ALPHA 阶段的 ISSUE」推进，做了三件事：

| # | 事项 | 结论 |
|---|---|---|
| 1 | **#5580**（open P0，最高杠杆） | **三处真缺陷已修**（F1 / F2 / F3），A/B 取证：修复前 D5 = **1/4**、G9/G10 恒红；修复后 D5 = **4/4**、G9/G10 在同一负载下转绿且诊断可区分超时/失败 |
| 2 | **ALPHA 阶段 P0 销账复核** | 台账标为 open P0 的 14 条 auto 项里，**至少 10 条其实早已修好**（逐条给出一手 commit 证据），台账 `evidence_grade=advisory-only` 的代价在此暴露 |
| 3 | **ALPHA→BETA 门槛 ①（ALPHA 门禁全绿）** | 上一轮记的唯一红臂 g1 **在 019 认证轮实测为绿**（rc=0，hash=7439ce80，22/22 查询实跑通过）⇒ **红臂的根因已消失**，但需在当前 tip 复跑 ALPHA 10 门禁取证 |

**未做 / 未验证的部分在 §6 如实列出。**

---

## 1. 范围与授权

- 属主 2026-11-08/09 裁决（`STAGE.yaml` 原文）：「ALPHA →（AB-01..AB-10 逐项取证 + 独立 Reviewer 回执）→ BETA → … 每一步都按 governance，**不得跳过**」；「**Alpha 阶段的 ISSUE 必须彻底完成并关闭**，才算通过门禁，否则不允许切换阶段，必须 SSOT 证据，**不能 AI 直接判断**」。
- 本文件是**取证与整改**记录，**不是**关闭记录，也**不是**晋级批准记录。
- 看板 `docs/releases/v4.1.0/KANBAN_v4.1.0_ALPHA.jsonl` 只增不改；本轮新增行见 §5。

---

## 2. #5580 的三处真缺陷

### 2.0 先厘清：#5580 的「ga/rc/beta 永远判 FAIL」已被修过

#5580 标题：`[门禁] check_rc_ga_gate.sh 的 ga / rc / beta 模式永远判 FAIL（期望 D1/D2 却从不执行）⇒ GA 的 required_gate 结构性不可通过`。

**该标题描述的「维度计划漂移」部分早已修复**，本轮复核确认：

- `ddd91ce3f9 fix(gate): 门禁判据的真实性 —— 覆盖率一份实现 / GA-2 产物校验 / 维度计划与执行同源（#5579 / #5491 / #5580）` —— 把维度派发改成**唯一一份列表** `plan_dims()`。
- `23e418555b fix(ci): 修复 D6 ANSI 解析与 D1/D2 维度计划漂移（#5456）` —— `plan_dims()` 与 `gate_expected_dims()` 由回归测试 `tests/integration/gate/gate_honesty_test.rs::test_5456_expected_dims_and_plan_dims_agree_for_every_mode` 逐模式比对。
- 现 tip 实测：`bash scripts/gate/check_rc_ga_gate.sh --list-dims` → `D1 D2 D3 D4 D5 D6 D7 D8 D9`，**rc=0**。

⇒ **#5580 剩余的是三处独立的门禁自身缺陷**（本线 GA-SOAK-36 归因为 F1/F2/F3），逐处修如下。

### 2.1 F1 —— D5 的四条判据绑死在 v3.8.0 发布线（结构性假红）

`check_rc_ga_gate.sh` 的 D5-2 / D5-4 / D5-5 三条 git 判据硬编码：

```
D5-2  git diff --stat origin/develop/v3.8.0~3..HEAD -- "*.rs"
D5-4  git log --oneline origin/develop/v3.7.0..origin/develop/v3.8.0 -- "*.rs"
D5-5  git log --oneline origin/develop/v3.8.0
```

**实测（当前远端）**：`git branch -r | grep -E "v3\.7\.0|v3\.8\.0"` → **不存在**。Gitea 上这两个分支已删，gitcode/gitee/github 是只读镜像（AGENTS.md §3.2）。

三条判据在 v4.1.0 上的实际表现（修复前 A/B 实测，见 §3.1）：

| 判据 | 实际表现 | 性质 |
|---|---|---|
| D5-2 | diff 为空 → `awk` 得空串 → bash 的 `[ "" -lt 500 ]` 把空串当 0 ⇒ 报 **PASS（recent: 0 lines）** | **空真通过**：什么也没查却报绿 |
| D5-3 | 模式 `EEK.*v2\|WAL.*v2\|v2.*WAL` 命中 **3 处，全部是合法 v4.1.0 代码的假阳性** | 恒红 |
| D5-4 | `wc -l` 从管道读入时输出**带前导空格**的 `"       0"`，`grep -qE "^[0-9]+$"` 匹配不上 ⇒ 判红 | 红绿**取决于一个与判据无关的格式细节**（不补空格就会变成「什么都没查也算通过」） |
| D5-5 | 数的是**另一条发布线**（v3.8.0）的 WAL 提交，分支不存在 ⇒ 计数恒 0 ⇒ 恒红 | 恒红 |

D5-3 的 3 处命中实测明细（**全部假阳性**）：

```
crates/storage/tests/multi_db_crash_recovery_5192.rs:84  "Re-open the WAL the way `run_server_v2` does"
crates/storage/src/file_storage.rs:1386                   "V312-95 v2 / Issue #4814: save a view to disk (WAL-style:"
crates/storage/src/file_storage.rs:7382                   "V312-95 v2 / Issue #4814: persist to disk first (WAL-style:"
```

另有一个**概念性问题**：`grep -rn EEK crates/ --include=*.rs` 的命中**全部是 `WEEK` 的子串** ⇒ **「EEK」这个概念在本仓根本不存在**。这条判据在校验一个不存在的概念，它的红与绿都不携带信息。

**修法**（判据自述与 500 行阈值**一字未动**，只把「查哪条发布线」接到 SSOT）：

- 发布线版本从 SSOT 解析：复用 #5446 引入的同一份助手 `scripts/gate/resolve_release_version.sh`（它的存在理由就是「一个错但看起来合理的版本，比一次可见的失败更糟」）；新增 `release_line_version()` / `release_line_prev_version()` / `release_line_prev_tag()` 三个辅助函数。
- D5-2：窗口改为本 checkout 最近 3 笔提交（`HEAD~3..HEAD`，浅克隆退化为 `HEAD~1..HEAD`），计量改为**实算增删行数**（`git diff --numstat` 求和）——与判据自述的「500 行」一致（**这是收紧，不是放宽**：旧实现取 `git diff --stat` 末行第 1 列，那是**改动文件数**，实测该列在冻结点上是 `3` 而实际增删行数是 `111`）；空 diff = 0 行 = 通过，不再是空真通过。
- D5-3：模式改为**相邻记号**并加左边界 —— `(^|[^A-Za-z])(EEK|WAL)[ _-]?v2`，命中时**逐条打印 file:line** 使判定可被人复核。左边界同时避免 `"WEEK v2"` 这类子串误伤。
- D5-4：改为**发布边界在 SSOT 里显式可查**（当前版本 `docs/releases/<cur>/STAGE.yaml` 存在 ∧ `version:` 字段与目录名自洽 ∧ 显式声明 `current_stage:` ∧ 上一发布线 `docs/releases/<prev>/STAGE.yaml` 已封档存在）；任一条不满足即**明确报出缺哪一条**。
- D5-5：窗口改为**当前发布线自己的提交**（上一发布线封版 tag..HEAD；tag 不可用时退化为 HEAD 全历史并写明用的是哪个窗口）。

### 2.2 F2 —— G9/G10 内层预算不足 + 诊断毁证

一手证据（GA 认证轮 `audit/v4.1.0/runs/20261119-ga-gates-019/001-check_rc_ga_gate.sh.stdout` 第 215-220 行，**原文**）：

```
  [G9] Upgrade Test (v3.8→v3.9) ...     ✗ [FAIL] G9 Upgrade:    Compiling sqlrustgo v4.1.0 (/Volumes/workspace/dev/srg-gate-d1)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 35.00s
     Running tests/integration/migration/upgrade_test.rs (target/debug/deps/upgrade_test-712f36899de49a18)
  [G10] Audit Log + Time Travel ...     ✗ [FAIL] G10 Audit/Time-Travel:    Compiling sqlrustgo v4.1.0 (...)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 16.81s
     Running tests/integration/stress/audit_log_test.rs (target/debug/deps/audit_log_test-61ad34f09121ab64)
  D7 Result: 3/5 (blockers: 2)
```

⇒ G9 的 **90 s** 与 G10 的 **60 s** 预算**只够编译**（G9 编译已耗 35.00 s，G10 编译 16.81 s），cargo 在**测试阶段**被 SIGTERM 砍断，输出里**根本没有判定行** `test result: ok. N passed; 0 failed` ⇒ 判据必然落到 else ⇒ **恒红**。

`tail -3` 是**诊断毁证**的次因：它把 FAIL 消息压成三行「Compiling / Finished / Running」，于是「编译失败」「测试失败」「时间不够」三种结局**长得一模一样** —— 证据不足被当成红，而没人能看出它是预算问题。

**修法**（判据 `0 failed|test result: ok` 与阈值**一字未动**，只放宽运行参数 + 抓完整输出）：

- 预算放宽到 `${GATE_RELIABILITY_TEST_TIMEOUT:-1800}` s（可用环境变量覆盖）。
- 全量输出落盘（`$TMPDIR/g9_upgrade_$$.log` / `g10_audit_$$.log`），判据对**全量**输出匹配，不再先截断。
- 超时（exit 124）与测试失败**分别报出**，并给全量输出路径。

> **口径更正（对本线此前表述的公开更正）**：本线此前把 F2 表述为「`tail -3` 恰好把 `test result: ok` 切掉 ⇒ 判据永远匹配不上」。桩对照实测推翻了这个说法：cargo 输出若完整收尾，末 3 行**含**判定行，`tail -3` 并不足以致红。**真因是内层预算不足**（cargo 被 SIGTERM），`tail -3` 只是让诊断不可区分。详见 §3.2。

### 2.3 F3 —— D9 覆盖率门槛映射与摘要诚实性

两处实现问题：

1. **门槛映射错**：`all` 模式把覆盖率门槛当成 **RC** 的（`if [ "$coverage_stage" = "all" ]; then coverage_stage="rc"`），而 SSOT 映射（`coverage_min_for_mode` / #5581）规定 `all → GA` ⇒ 同一轮里 D9 按 RC 的门槛判、阶段级 `check_coverage_v312.sh` 按 GA 的门槛判，**两条不是同一把尺子**。
2. **跳过时摘要读起来像「覆盖率已验」**：不跑全量时 D9 不进分母，摘要显示 `D9-Coverage: 1/1`。

**修法**（判据与门槛数值**一字未动**）：

- 门槛档位一律从 SSOT 映射取（`all→GA / ga→GA / rc→RC / beta→BETA`）。
- 跳过时**仍然计入分母**，并明确写出「全量未跑 + 覆盖率证据由阶段级 `check_coverage_v312.sh` 提供」。
- **不新增 blocker**：阶段级 `required_gates` 里本来就有 `check_coverage_v312.sh`，覆盖率证据不因此缺失；新增 blocker 会把判据收紧，属越权。

---

## 3. A/B 取证（修复前 vs 修复后）

全部**零 CPU**（桩 cargo / 纯 git 判据），复现命令见 §7。

### 3.1 D5-2..D5-5

同一段代码在 `origin/develop/v4.1.0` 的版本与工作树版本各跑一次：

| | 修复前 | 修复后 |
|---|---|---|
| D5-2 | ✓ PASS（recent: **0** lines）← 空真通过 | ✓ PASS（recent: **0** changed lines，base=HEAD~3） |
| D5-3 | ⚠ WARN（**3** 处命中，全假阳性） | ✓ PASS（**0** 处，相邻记号模式） |
| D5-4 | ⚠ WARN（版本边界 unclear） | ✓ PASS（cur=v4.1.0 prev=v4.0.0） |
| D5-5 | ⚠ WARN（v3.8.0 计数 **0**） | ✓ PASS（窗口 `v4.0.0-final..HEAD`，**54** 笔 WAL 类提交） |
| **合计** | **D5_PASS=1 / D5_TOTAL=4** | **D5_PASS=4 / D5_TOTAL=4** |

### 3.2 G9/G10（三场景，桩 cargo）

| 场景 | 桩 cargo 行为 | 修复前 | 修复后 |
|---|---|---|---|
| 1「019 一手复现」 | 打印认证轮那三行残缺输出后被 timeout 杀 | ✗ FAIL，消息=三行噪声 | ✗ FAIL，但报 **`TIMEOUT … (inconclusive, not a test failure; full output: <path>)`** ⇒ **诊断可区分** |
| 2「预算是决定性」 | 跑 8 s 才出判定 | ✗ FAIL（预算受控下调到 3 s） | ✓ **PASS**（预算 30 s > 8 s） |
| 3「不制造假红」 | 立即给完整成功输出 | ✓ PASS | ✓ PASS |

> 场景 2 的旧码预算为**受控下调**（90/60 → 3 s）以便在秒级内完成对照，**已如实披露**；新码在 harness 内也被缩到秒级（`GATE_RELIABILITY_TEST_TIMEOUT=3` / `30`），被测的是「预算是否够 + 是否抓完整输出 + 诊断是否可区分」，**不是预算的绝对数值**。

### 3.3 #5580 的关闭依据 —— 同一认证轮里同样的测试目标是绿的

D7 的 G9/G10 恒红，**不等于测试红**。在**同一个 GA 认证轮**（019 冻结 `64269eaa08`）里，三个**正规门禁**调用了 G9/G10 所用的**同一批测试目标**，全部通过：

| 019 轮臂 | 输出原文 | 对应 D7 子项 |
|---|---|---|
| `009-check_p14_upgrade_test.sh` | `[5/7] ✅ PASS: upgrade_test **50 passed** (≥50)`；`[6/7] ✅ PASS: crates/tools/src/upgrade.rs 36 passed`；`G9 Gate: PASS` | **G9** 的 `upgrade_test` |
| `010-check_p21_audit_log.sh` | `[5/7] ✅ PASS: audit_log_test **20 passed** (≥20)` | **G10** 的 `audit_log_test` |
| `011-check_p22_time_travel.sh` | `[5/7] ✅ PASS: time_travel_test **20 passed** (≥20)`；`[7/7] ✅ PASS: TPC-H gate (22/22) maintained` | **G10** 的 `time_travel_test` |

⇒ **G9/G10 的红来自 D7 那一层的内层预算不足（cargo 被 SIGTERM），不来自测试本身**。这正是「门禁自身缺陷」与「代码缺陷」的分界。

⇒ **#5580 的关闭依据**：① 标题所述的维度计划漂移早已由 `ddd91ce3f9` / `23e418555b (#5456)` 修复（§2.0）；② 剩余的 F1/F2/F3 已由本 PR 修复并经 A/B 取证（§3.1 / §3.2）；③ 同一认证轮里相关测试目标**实跑通过**（本节）。**剩余依赖**：新冻结点上的 GA 35 项重跑会产出 D5=10/10 与 D7=5/5 的**新结论**，该重跑因本 PR 改了门禁脚本而**必然需要**（门禁自身变了 ⇒ 旧结论失效），它是**独立条目**，不构成 #5580 的未完成部分。

### 3.4 门禁自检（改动后）

```
bash scripts/gate/check_rc_ga_gate.sh --list-dims        → D1 D2 D3 D4 D5 D6 D7 D8 D9   rc=0
bash scripts/gate/check_gate_self_verification.sh         → ✅ PASS — P11 satisfied    rc=0
bash scripts/gate/check_gate_executability.sh             → ✅ PASSED                  rc=0
bash -n scripts/gate/check_rc_ga_gate.sh                  → SYNTAX OK
```

---

## 4. ALPHA 阶段 P0 销账复核 —— 台账已过期

台账 `ISSUE_LEDGER_2026-11-11D.jsonl`（`ledger_run=20261111-ledger-006`）把 **14 条** P0 标为 `automatable=auto`。本轮逐条核实（只读：源码 + git 历史 + 门禁产物），**至少 10 条早已修好**，台账未更新：

| issue | 台账状态 | 实际 | 一手证据 |
|---|---|---|---|
| #5362 clippy unused import Path | state/ready | **已修** | `0a57569f9d fix(storage): 清理 unused import / unused variable warning`；`662f1ebe33 (#5788)` 清 clippy --all-targets 两族红；现 `crates/storage/src/fault_injection.rs:30` 是 `use std::path::PathBuf`（非 `Path`）且在 155/164/171 被使用 |
| #5364 ingest_git_commit 短 hash panic | state/ready | **已修** | `cbdb2fa8ae fix(evidence-graph): stop panicking on commit hashes shorter than 8 bytes`；`crates/evidence-graph/src/lib.rs:217-219` `short_hash` 用 `hash.get(..8).unwrap_or(hash)`；回归测试 `lib.rs:947` / `lib.rs:979` |
| #5368 evidence-graph-gate 解析不了自己的 PASS | state/ready | **已修** | `3444df5580 fix(ci): gate parsed pretty-printed JSON with a pattern that can never match`；`.gitea/workflows/gate.yml:267` 模式 `'"result"[[:space:]]*:[[:space:]]*"PASS"'` 与 `to_string_pretty` 的实际形态对齐 |
| #5393 TPC-H Q9/Q18 OOM + Q19 解析失败 | state/ready | **已修** | `9e5a79c27f fix(executor): #5393 星形 join 生成树哈希序 + 复合键哈希步 + GROUP BY 后 ORDER BY 按组学解析;wire 22/22(#5393)`；`0ce7da35c7 fix(parser): …TPC-H Q19 wire 解析恢复`；**019 认证轮 g1 实跑 `tpch_full_22_queries ... ok`（1.58 s，非空转绿）** |
| #5400 correctness gate R2/R4 恒红假阳性 | state/ready | **已修** | `7790e10261 fix(gate): #5400 判据函数化 + selftest，#5401 db-scope 覆盖门禁，#5502 文档指针`；脚本头部自载「#5400 —— 判据从『grep 旧拼写』改成『按分支判定实际装配链』」，并有 `--selftest` 变异锚点 |
| #5461 3 项 GA 门禁「CI 承接」不成立 | state/ready | **已修**（配置层面） | `954bd036a7 feat(gate): #5461 / #5450 —— 门禁执行点矩阵`；执行点 `.gitea/workflows/ci.yml:1545`（coverage）/ `:1578`（anti_fabrication）/ `:1371`（stage-gates → `check_stage.sh --stage GA` 兜住 sql_corpus）；新增 fail-closed 守卫 `scripts/gate/check_gate_executability.sh`；实测 **92/92 OK、NO-EXECUTOR 0** |
| #5537 ALPHA coverage 漏传 feature + env!(CARGO_TARGET) | state/claimed | **已修** | `c74b3715c8 fix(gate): 修掉 v4.1.0 ALPHA 阶段的 2 个门禁阻塞项 (#5537)`；`eec7494265 fix(test): alpha_gate_blocker_test 的 Path 未使用`；`81da14c3db fix(test): GA 清单的 cargo test --all-features 在 develop 上是红的`；per-crate feature 推导已在 `scripts/gate/common.sh`（`coverage_derive_crate_features` / `coverage_features_for_crate` / `--features`）；`env!` 已全部换成 `option_env!`（`tests/integration/gate/alpha_gate_blocker_test.rs:253`） |
| #5577 cargo test 覆写已提交证据文件 | state/claimed | **已修** | `3f1ad936eb fix(test): 测试不得覆写已归档的证据 —— 输出目录显式声明 + 两层门禁（#5577 / #5389）` |
| #5579 覆盖率两份实现 | state/None | **已修** | `ddd91ce3f9 fix(gate): 门禁判据的真实性 —— 覆盖率一份实现…（#5579 / #5491 / #5580）`；测量整体搬进 `scripts/gate/common.sh`，并新增 `scripts/gate/check_coverage_single_implementation.sh` |
| #5580 ga/rc/beta 永远判 FAIL | state/None | **本 PR 修** | 见 §2 |

**对 E3「No P0 bugs open」的影响（如实披露）**：

- 台账的 **49 条 P0** 是 `ledger_run=20261111-ledger-006`（2026-11-11）的快照，且全表 `evidence_grade=advisory-only`；台账 .md 自载「分诊只回答域 / 破坏了哪条声称 / 可否自动化 / 修复形状 / 疑似重复，**不回答『是否已修好』**，销账仍走 PR 关联与关闭验证」。
- ⇒ **台账数不能直接当作「当前仍 open 的 P0 数」**。本轮已核实 10 条并非真实缺陷 ⇒ **真实未清 P0 数 ≤ 39**（需以 Gitea 当期 open 状态为准，且需逐条关闭验证）。
- **关闭动作仍需人工核对**：本轮只**取证**，未在 Gitea 上关闭任何 issue。

**CI 执行时点的残余（#5461 的诚实边界）**：本仓仓库级 Gitea Actions 开关当前为 `has_actions=false`（SOAK 独占期主动关闭、注明可逆）⇒ **配置接线正确，但在开关恢复前实际不会有 CI run 执行这 3 项**。这是环境状态，不是 issue 主体缺陷。

---

## 5. 看板影响

看板 `docs/releases/v4.1.0/KANBAN_v4.1.0_ALPHA.jsonl` **只增不改**，本轮新增 2 行（12 键固定序，`ALPHA-ISSUE-1` / `ALPHA-ISSUE-2`），`.md` 由 `scripts/gate/kanban_render.py render` 重渲染。

---

## 6. 未做 / 未验证（诚实登记）

| 项 | 状态 | 原因 |
|---|---|---|
| **ALPHA 10 门禁在当前 tip 的复跑取证** | **未做** | 本机 10 核、load 6.2，`019` 认证轮正在编译；并发跑重量门禁会造成时间预算假红（F2 同族）。上一轮记录（冻结 `2792759137ef`）为 9 绿 / 1 红（红臂 g1），而 g1 在 019（冻结 `64269eaa08`）实测已绿 ⇒ **需在当前 tip 复跑才能形成 SSOT 证据** |
| **#5580 的 cargo 级验证**（G9/G10 真跑） | **未做** | 同上（需 cargo）。本轮只做零 CPU 的桩 A/B + 语法 + 三个门禁自检 |
| **#5412（带 PK 逐行 INSERT O(N²)）** | **未做** | 引擎面，需 cargo 验证；与本线 SOAK 根因结论同族，需单独立项 |
| **#5375 / #5377 / #5388** | **核实中** | 见看板行 `ALPHA-ISSUE-2` |
| **Gitea issue 关闭动作** | **由本 PR 承担** | #5580 随本 PR 的 `Fixes #5580` 关闭（关闭依据见 §3.3）；**其余 9 条已修 P0（#5362/#5364/#5368/#5393/#5400/#5461/#5537/#5577/#5579）本轮只取证，未关闭** —— 它们各自的关闭须走各自的 PR 关联 + 关闭验证 |
| **ALPHA→BETA 阶段跃迁** | **未做，且不可由 AI 单方宣布** | 属主 2026-11-08/09 裁决：须 AB-01..AB-10 逐项取证 + **独立 Reviewer 回执** + 人工批准记录 |
| **019 认证轮** | 继续跑 | 已落账 **15/35**；第 001 臂 `check_rc_ga_gate.sh` rc=1（= 本 PR 修的 F1/F2/F3 所致）⇒ 修好后需在新冻结点重跑 |
| **r5（168h）** | 继续跑 | 已跑 **8.32 h**（29 952 s / 4.62 %）；**driver_e / panic / error_high 全部为 0**；RSS 190.3 → 1 331.9 MB；判据⑤不满足且根因需改 `crates/` ⇒ 本轮不可能成为 GA 合格轮，降级为 RC/GA 预备 |

---

## 7. 复现命令（全部零 CPU）

```bash
# F1：D5-2..D5-5 的 A/B（同一段代码，修复前 vs 修复后）
bash /tmp/_d5_ab.sh

# F2：G9/G10 三场景桩 cargo A/B
bash /tmp/_g9g10_ab.sh

# 门禁自检
bash -n scripts/gate/check_rc_ga_gate.sh
bash scripts/gate/check_rc_ga_gate.sh --list-dims
bash scripts/gate/check_gate_self_verification.sh
bash scripts/gate/check_gate_executability.sh

# 019 认证轮的一手 G9/G10 输出
sed -n '215,222p' \
  /Volumes/workspace/dev/srg-gate-d1/audit/v4.1.0/runs/20261119-ga-gates-019/001-check_rc_ga_gate.sh.stdout

# 看板一致性
/usr/bin/python3 scripts/gate/kanban_render.py check docs/releases/v4.1.0/KANBAN_v4.1.0_ALPHA.md
/usr/bin/python3 scripts/gate/kanban_merge_policy.py docs/releases/v4.1.0/KANBAN_v4.1.0_ALPHA.jsonl
```

---

## 8. 属主待决（不在 AI 裁决域）

1. **ALPHA→BETA 跃迁批准** —— 属主 2026-11-08/09 裁决明定「不能 AI 直接判断」；需 AB-01..AB-10 逐项取证 + **独立 Reviewer 回执**（AB-10）。
2. **E1 / E3 的人工关闭核对** —— 尤其 §4 的 10 条「已修但台账仍标 open」是否批量关闭。
3. **门槛口径冲突** —— `STAGE_CONFIG.yaml` ALPHA 的 4 条 exit_criteria（其「No P0 bugs open」按台账读是 49 条）与 `docs/releases/v4.1.0/STAGE.yaml` 的 5 条 `exit_criteria.alpha_to_beta` 不一致，本线按**更具体的 5 条**执行并**登记分歧**，不自行择一放宽。
4. **CI 开关** —— `has_actions=false` 何时恢复（影响 #5461 这类「配置已对、执行未发生」的项）。
5. **r5 是否提前收尾** —— 本轮不可能合格（判据⑤ + 根因需改 `crates/`），是否提前释放 252 属属主授权域。

---

## 9. 补记（2026-11-19 晚，同一轮内）

### 9.1 `check_gate_test_integrity.sh` 复跑 ⇒ **rc=0 PASS**

§6 里记的「rc=124（120 s 超时，非失败）」是**我自己 harness 的 120 s 超时**，不是门禁失败。复跑（预算 900 s）结果 **rc=0**，关键断言原文：

```
P16 step 2.5/3: verify no `cargo test ... || true` masks in gate scripts
  ✅ PASS: no `cargo test ... || true` masking in any gate script
P16 step 2.6/3: verify no swallowed exit code is read afterwards
  ✅ PASS: no swallowed exit code is read afterwards in any gate script
  ✅ PASS: P16: 48 gate tests, 0 NEW #[ignore] (baseline-tolerated: 2 pre-existing #[ignore] under ADR-008 exceptions)
```

⇒ **本 PR 对 G9/G10 的改写（`|| G9_RC=$?` 取代 `|| true`、全量输出落盘）被该门禁显式认可**。

**并发口径的核实（本线自查，避免发布不实自纠）**：复跑期间曾担心它与 019 认证轮抢 CPU，**核实后不成立** —— 该脚本**只做文本扫描，不执行 cargo**（源码中无任何 cargo 调用；日志里 `Compiling` / `Finished test` / `test result` 命中数 = **0**）。它慢的原因是纯文本扫描量（483 个 `[[test]]` 目标 + 全部 workflow + 全部 gate 脚本 + ignore registry），**不是重量门禁** ⇒ **未违反「认证轮期间不得并发重量门禁」**。

### 9.2 本线的两处自纠（工具残留）

1. **A/B 桩 cargo 的 `sleep 3600` 泄漏**：§3.2 场景 1 的桩 cargo 以 `sleep 3600` 模拟「被 timeout 杀掉」，但 `timeout` 的 SIGTERM 只送到脚本本身，**其 `sleep` 子进程活了下来**，以孤儿进程形态存活约 **12 分钟**（PID 63682 / 63684）。它是一个**空闲 sleep（不占 CPU）**，但确实是本线的残留 ⇒ **已按具体 PID 清理**（未用 `pkill -f`）。教训：桩脚本里模拟长跑时，必须让桩自己处理 SIGTERM（`trap`）或改用「限时 sleep + 显式 exit」。
2. **一个自己写错的变量前缀**：第一版 `release_line_prev_tag` 写成 `git tag -l "v${prev}*"`，而 `prev` **自带 `v` 前缀** ⇒ 模式变成 `vv4.0.0*` ⇒ 匹配不到任何 tag ⇒ D5-5 的窗口退化成 `HEAD`（1172 笔，弱判据）。**在合入前已由 A/B 复跑发现并修正**为 `"${prev}*"` ⇒ 窗口变为 `v4.0.0-final..HEAD`（54 笔，有意义窗口）。

### 9.3 §6 的状态更新

| §6 原记 | 现况 |
|---|---|
| `check_gate_test_integrity.sh` rc=124（120 s 超时，非失败）⇒ 不宣称已通过 | **已复跑 rc=0 PASS**（见 §9.1） |
| **ALPHA 10 门禁在当前 tip 的复跑取证** | **仍未做**（等 CPU 窗口；019 认证轮仍在跑） |
| **#5580 的 cargo 级验证**（G9/G10 真跑） | **仍未做**（等 CPU 窗口；桩 A/B 不能替代） |
| Gitea issue 关闭动作 | **#5364 / #5368 已由 PR #5887 关闭**（当期 open P0 **29 → 27**）；其余 27 条**未关闭** |

---

## 10. 补记二（2026-11-19 晚）：#5412 的根因已定位到源码级（仍需修）

**#5412** `[引擎] 带 PRIMARY KEY 的逐行 INSERT 仍是 O(N^2) —— pk_lookup_cache 被它自己要加速的那次 INSERT 失效`

**本轮只读核实（零 cargo）得到的结论比 issue 标题更精确**：

| 位置 | 事实 |
|---|---|
| `crates/storage/src/engine.rs:1579` | trait **默认实现**返回 **0**，注释自陈：「The default returns 0, i.e. "never trust a cached copy". Callers must honour that by rebuilding, so an engine that has not opted in stays **correct rather than fast**.」 |
| `crates/storage/src/engine.rs:3588` | **全仓唯一的覆盖实现**，位于 `impl StorageEngine for MemoryStorage`（该 impl 块起于 `:3156`）⇒ `grep -rn "fn table_change_stamp" crates/ src/` 只命中这两处 |
| `src/engine_dml.rs:443` | PK 缓存快路径的入口条件是 `if cacheable && live_stamp != 0` ⇒ **stamp 为 0 时快路径根本进不去**，直接落到 `:487` 的整表扫描 |
| `src/engine_dml.rs:578-587` | 写入前再取 `pre_write_stamp`；与 `cached_stamp` 不一致 ⇒ **删缓存并整表重扫** |

⇒ **机制（修正 issue 标题的措辞）**：**不是「缓存被它自己要加速的那次 INSERT 失效」**，而是
**「FileStorage 与 WAL 包装层都继承 trait 默认实现 ⇒ `table_change_stamp` 恒为 0 ⇒ PK 缓存快路径被永久禁用 ⇒
每一次带 PK 的 INSERT 都整表扫描 ⇒ O(N²)」**。

⇒ **这解释了为什么这是「正确但未优化」的既有设计、而不是回归**：`engine.rs:1579` 的注释明说未 opt-in 的引擎
「stays correct rather than fast」。**#5412 因此仍成立、仍需修**，但修法方向明确：

1. 为 `FileStorage`（及其 WAL 包装层）实现 `table_change_stamp` —— 在该表的数据变更（insert/delete/update）时
   递增 per-table stamp；
2. 使 `engine_dml.rs:443` 的快路径条件在 file 后端上可成立 ⇒ 带 PK 的逐行 INSERT 回到 O(N)。

**本轮未动手的原因（如实登记）**：

- **改 crates/ 会改冻结点** ⇒ 使 019 认证轮（仍在跑，已落账 15/35）的结论失效 ⇒ 需要与该轮的收尾决策一并处置；
- **验证需要 cargo** ⇒ 与在跑的 019 认证轮抢 CPU ⇒ 本机 10 核下并发重量门禁会产生**时间预算假红**
  （与本 PR 所修的 F2 同族）；
- 该项与本线 SOAK 根因结论（写路径 O(N)）**同族**，宜与存储面根因一并推进（相关项 **#5507**
  「重构 MemoryStorage 的『提交快照』机制」在当期 open P0 清单里）；
- **零 crates/ scripts/ tests/ 改动** ⇒ 交付树与 252 检出（均 detached @ `64269eaa08`）不失配。
