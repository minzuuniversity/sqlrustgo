# PR-5933 TEST_REVIEW — 独立审核记录

> Issue: #5592 / #5598 · 分支: `fix/v410-criterion8-evidence-layer-20261126`
> 基线: `origin/develop/v4.1.0` @ `a604ed9ab3`
> 配套文档: `PR-5933_SPEC.md` / `PR-5933_TEST_PLAN.md` / `PR-5933_TEST_DESIGN.md` / `PR-5933_ACCEPTANCE.md`

---

## 1. 本审核的性质与边界

> **本文件是实施者侧的自我审核记录，不是独立 Reviewer 的回执。**
>
> 判据 6 的签署层要求 `assertion_reviewed_by` **≠** `assertion_review_performed_by`
> （R-ORACLE），且必须落在**属主授权的 Reviewer 名单**里。本 PR 的 `assertion_review_performed_by`
> 是实施者本人，**因此本文件不构成、也不试图构成**判据 6 意义上的签署。
>
> **本 PR 不涉及 L1 切片的 44 条 Oracle 副签**（那是 REF-C6-1，属主域）。

本审核覆盖的是**本 PR 自己的改动**：门禁脚本、负向夹具、泳道编排、判据生成器、台账与编目、
看板、声明。

---

## 2. 审核清单与结论

### 2.1 门禁行为变更的显式性（R10）

| 检查 | 结论 |
|---|---|
| 变更是否在三处显式列出？ | ✅ 声明 §5.4、PR body §「门禁行为变更（属主级，须签字）」、看板 §3 Q16 / §6 |
| 是否说明了「从什么变成什么」？ | ✅ 「`measured 7 of 8 crates`（FAIL）」→「`average coverage: 87.72% (threshold: 85%) — measured 8/8 crates`（rc=0）」 |
| 是否说清了「不是什么」？ | ✅ 不改 168 分母、不新增 `check_*`、不改 `COVERAGE_MIN_*`、`coverage_verdict` 一字未改 |
| 是否说清了「其它原因不可测仍 FAIL」？ | ✅ 声明 §5.4、SPEC §4、TEST_DESIGN §2.5 |

### 2.2 有没有借改门禁掩盖产品缺陷（R7）

| 检查 | 结论 | 证据 |
|---|---|---|
| RED 特征测试的**源码**动了吗？ | ✅ **零改动** | `git diff HEAD --stat -- crates/storage/tests/` 为空 |
| RED 特征测试还红吗？ | ✅ **仍然红** | `cargo test -p sqlrustgo-storage --test tf_t4_stub_silent_success --test tf_t4_ai_candidate_v1` ⇒ `0 passed; 4 failed` |
| 有没有加 `#[ignore]`？ | ✅ **没有** | 门禁测试**断言**这两个文件里不含 `#[ignore]` |
| 断言改了吗？ | ✅ **没有** | `git diff` 里 `crates/storage/tests/` 无任何改动 |
| 阈值档改了吗？ | ✅ **没有** | `git diff` 不含 `STAGE_CONFIG.yaml`；`coverage_verdict` 不含在 diff 里 |
| `coverage_verdict` 改了吗？ | ✅ **一字未改** | `git diff scripts/gate/common.sh` 的 diff 段里不含该函数 |
| 换了个 crate 测吗？ | ✅ 没有 | L1_8 的 8 个 crate 一个不少，全部实测出数字 |

### 2.3 排除面是否可审计（R2）

| 检查 | 结论 | 证据 |
|---|---|---|
| 排除范围限定吗？ | ✅ **2 文件 / 8 函数** | `COVERAGE_RED_CHARACTERISATION_TEST_FILES` 只有 2 条；提取出 8 个函数名 |
| 排除在报告里可见吗？ | ✅ **每次覆盖率报告都打印** | `coverage_print_excluded_tests()` 在 `coverage_measure_set()` 开头调用；实测输出含三个分组与 23 个函数名 |
| 排除有门禁测试钉死吗？ | ✅ | `alpha_gate_blocker_test.rs` 扩展已有断言（6 个文件 + 3 条新断言） |
| 排除是具名分组吗？ | ✅ | 独立数组 `COVERAGE_RED_CHARACTERISATION_TEST_FILES`，报告里单独一行 `red-characterisation` |
| 会不会悄悄缩小？ | ✅ | 提取是**运行时**的（新加一条 RED 特征测试自动被覆盖）⇒ 列表不需手改 ⇒ 不会因为「忘了加」而缩小 |

### 2.4 超时会不会被当成红（R4）

| 检查 | 结论 | 证据 |
|---|---|---|
| `fx_is_red` 排除 124 吗？ | ✅ | `neg_two_layer.sh` 的 `fx_is_red`：`[ "$rc" -ne 124 ] || return 1` |
| 泳道汇总排除 124 吗？ | ✅ | `verify_neg_fixtures.sh`：`[ "$g_rc" != "124" ]` |
| fx18 排除 INCOMPLETE/124 吗？ | ✅ | fx18 的 G4 护栏（`:205-210`）⇒ `exit 4` |
| 判据表只在 red==7 时判 PASS 吗？ | ✅ | `gen_verdict.py`：`"verdict": "PASS" if red == 7 else "FAIL"` |
| 预热失败会不会被当成「跑过了」？ | ✅ | `exit 4`（setup 失败），绝不静默跳过、绝不判绿 |

### 2.5 树身份是不是真的被观测到（R5 / C5）

| 检查 | 结论 | 证据 |
|---|---|---|
| 泳道自报身份了吗？ | ✅ | 日志首部：`verify_neg_fixtures: tree_sha=… dirty_files=0` |
| `dirty_files` 带上了吗？ | ✅ | 同一行；`TREE != REPO` 时脏检查被跳过，只有 SHA 不足以标识被测树 |
| 判据表从日志**读**身份了吗？ | ✅ | `read_lane_identity()` 的正则 + 三种定性（观测值 / 部分观测值 / 推定值） |
| `observed` 里逐字引用了吗？ | ✅ | `id_line = lane_id["line"]`，原文进 `observed` |
| 旧路径还在吗？ | ✅ | 无 `tree_sha=` 行 ⇒ 退回 `--neg-run-tree` 并标「推定值」—— 不假装 |

### 2.6 分母有没有漂（R8）

| 检查 | 结论 | 证据 |
|---|---|---|
| 新增 `check_*` 了吗？ | ✅ **没有** | `git diff` 不含 `scripts/gate/check_*.sh` 的新增 |
| `STAGE_CONFIG.yaml` 动了吗？ | ✅ **没有** | `git diff` 不含该文件 |
| 分母仍为 168 吗？ | ✅ | `gate_registry.py check` ⇒ `check_n=168`、`catalog_rows=168`、`fails=0` |

### 2.7 声明有没有越过证据（R9）

| 检查 | 结论 | 证据 |
|---|---|---|
| 出现了「已放行」「准入通过」「C-ALPHA-02 已启动」吗？ | ✅ **没有** | `grep` 无输出 |
| 证据头五字段齐吗？ | ✅ | `source_agent` / `source_run` / `timestamp` / `evidence_hash` / `conflict_resolution` |
| 每条 PASS 结论有出处吗？ | ✅ | 逐条指向 `criteria[]` 索引 + `BASELINE_FREEZE.json` + `git log --merges` |
| 3 条 FAIL 的归因改判了吗？ | ✅ **没改** | 照抄 `V410-OWNER-RULING.md` §1.1 |
| `current_stage` 动了吗？ | ✅ **没有** | `git diff` 不含 `STAGE.yaml` |
| 代签了吗？ | ✅ **没有** | 裁决位与 O9 确认位**保持为空**；声明 §7 明说「本文件不设签署位」 |
| 声明里写的通道状态是实测的吗？ | ✅ | 本会话探活：`chatGPT-web available: true`（2 个模型）/ `deepseek-web HTTP 401 Unauthorized, available: false` |

### 2.8 预热会不会变成隐藏开关（R6）

| 检查 | 结论 | 证据 |
|---|---|---|
| 有 env 开关吗？ | ✅ **没有** | 脚本里无任何 `SKIP` / `DISABLE` / `IF` 分支能跳过预热 |
| 独立日志吗？ | ✅ | `$LOG_DIR/warmup.log` |
| 失败即 abort 吗？ | ✅ | `exit 4` |

### 2.9 看板合规

| 检查 | 结论 | 证据 |
|---|---|---|
| 只增不改吗？ | ✅ | JSONL diff 只含 4 个**新增**行；既有 230 行逐字节不变 |
| id 唯一吗？ | ✅ | 234 == 234 去重后 |
| 12 键序固定吗？ | ✅ | 4 行都是 `id/task/acceptance/evidence/status/blocked_by/domain/priority/stage/board/version/origin` |
| 状态在五词表内吗？ | ✅ | 完成 / 进行中 / 阻塞 / 阻塞 |
| 有单侧合并标记吗？ | ✅ **没有** | `kanban_merge_policy.py` ⇒ `verdict: OK` |
| MD 是生成物吗？ | ✅ | `kanban_render.py check` ⇒ rc=0，234 行 |
| 手写段被生成器覆盖了吗？ | ✅ **没有** | 生成段（LEDGER:BEGIN/END 之间）在 diff 里无变化 |

---

## 3. 审核中发现的、已在本 PR 内修掉的问题

| # | 问题 | 处置 |
|---|---|---|
| 1 | 最初打算给 `alpha_gate_blocker_test.rs` **新增** `#[test]` 来钉排除范围 —— 那会改动 `tests/baseline/gate_test_baseline.json` 的测试计数（另一个门禁的基线） | 改为**扩展已有**测试的断言数组 ⇒ 测试计数不变（实测仍 8 passed） |
| 2 | 最初在声明里写了「不宣告『C-ALPHA-02 已放行』」等**含被禁字样**的否定句 —— 红线是「不得出现该字样」，不是「不得断言该结论」 | 改写为不含被禁字样的否定表达（§4 仍保留「显式否定」的结构与含义） |
| 3 | fx18 最初没有钉阈值档 —— 门禁的缺省是 `ga`，但「靠缺省」不是决定 | 显式 `export SQLRUSTGO_COVERAGE_STAGE=ga` + **断言输出含 `stage=ga`**；断言的是档位不是数字 |
| 4 | 台账 fx18 行最初想手改 JSONL —— 但台账是**生成物** | 改**生成器**（`gen_neg_ledger.py`：fx18 从 `PEND` 移到 `MUT8` + `EVIDENCE_RUN_OVERRIDE`），`--check` 逐字节比对 |
| 5 | fx18 的 `evidence_run` 最初想用共享的 `run8`（20261124）—— 那一轮的树上覆盖率门禁**本来就是红的**，红不可归因 ⇒ 那是**假引用** | 新增 `EVIDENCE_RUN_OVERRIDE`，fx18 指向 2026-11-26 那一轮 |
| 6 | 泳道首跑失败（`FATAL: the working tree has uncommitted changes`）—— 因为归档 run 目录**建在仓内**，`git status --porcelain` 看见它 | 泳道日志改放仓外（`/tmp`），跑完再归档进仓；树在跑泳道时保持干净（这本来就是泳道的前提） |

---

## 4. 本 PR **未**审核、也**不应**由实施者审的东西

| 项 | 为什么 |
|---|---|
| L1 的 44 条 Oracle 副签 | 实施者不得自签（C7 / R-ORACLE）；属 REF-C6-1 |
| C01-15 的 ADR 映射 | 执行者不得自拟映射（A2.2 R5 / O10）；属 REF-C4-1 |
| 判据 12 豁免的背书 | §4.1 的 O9 确认行是属主的；实施者不代签 |
| 放行批次（O5）与阶段跃迁 | 属主专属；本 PR 不改 `current_stage` |
| 门禁阈值的任何调整 | 本 PR 不改阈值 |

---

## 5. 审核结论

| 项 | 结论 |
|---|---|
| 本 PR 的改动是否自洽？ | ✅ 是 |
| 是否有「借改门禁掩盖缺陷」？ | ✅ 无（R7 的每一条都有实测证据） |
| 是否有「声明越过证据」？ | ✅ 无（R9 的每一条都有实测证据） |
| 门禁分母是否漂移？ | ✅ 无（168） |
| 阈值是否被放宽？ | ✅ 无 |
| 是否有属主必须签字的动作？ | ✅ **有**：覆盖率门禁在冻结基线上的判定变更（属主级门禁行为变更） |
| 本 PR 是否可合并？ | ✅ 可合并 —— 但**合并不等于准入**：判据 #4 与 #6 仍在属主域，且本 PR 的合并**不解除**那两条 |

**审核人**：实施者（`dsh-b` / Task 9）· 2026-11-26
**性质**：自我审核记录，**不构成**判据 6 意义上的独立签署
