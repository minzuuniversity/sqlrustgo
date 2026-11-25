# SDD ledger — plan: docs/refactor/v4.1.0-c/takeover/V410-READINESS-IMPLEMENTATION-PLAN.md

Plan: 9 tasks, 74 steps. Spec: `docs/refactor/v4.1.0-c/takeover/V410-REFACTOR-READINESS-DESIGN.md` (rev 2).
Worktree (all execution): `/Volumes/workspace/dev/wt-v410-base`, branch `base/v4.1.0-frozen`.
Workspace (ledger/briefs/reports): `/Volumes/workspace/dev/sqlrustgo/.superpowers/sdd/V410-READINESS-IMPLEMENTATION-PLAN/`.

## Preflight scan (run before Task 1)

### Shared file / interface pairs

| Tasks | Shares | Produces → consumes | Finding | Ruling |
|---|---|---|---|---|
| T1 → T2 | BASE sha | `BASELINE_FREEZE.json.base_sha` → `verify_sha_scope.py` 读它 | 无冲突 | — |
| T1 → T4 | ADR-410-011 文件 | T4 Step 3 `git mv` 该文件，但 T1 只导入 `45da6968ae`，**该提交不含 `runtime-contract/`** | **阻塞**：源路径在基线树内不存在 | **R1**：T1 增 Step 3，按目录导入 `b681b74ca3` 的 `runtime-contract/20261120-pr5915/`（62 文件），不导入该提交其余 178 文件。计划已改 |
| T2 → T4 | `REFACTOR_TASK_DAG.json` | T2 改 `base_sha`；T4 改 `approved_adr_refs` | 同文件不同字段，顺序执行 | 无冲突 |
| T3 → T4 | `verify_handoff.py` | T3 放宽断言 → T4 填数据 | 顺序正确 | 无冲突 |
| T2/T4/T5 → manifest | `finalize()` 哈希范围 | 三者都改了 `docs/releases/v4.1.0/refactoring/` 与 `docs/refactor/v4.1.0-c/handoff/` 下的文件 → manifest 漂移 | **顺序错**：T4 原为 `--check`(Step6) 后 `--finalize`(Step7)，必然先失败 | **R2**：T2 尾加 Step 8 重签；T4 调换 Step 6/7 为「先 finalize 后 check」；T5 尾加 Step 10 重签。计划已改 |
| T4 → T5 | 矩阵 | T5 不改 case 数量（`assert len(cases)==75` 仍成立） | 无冲突 | — |
| T5 → T6 | `run_slice()` | T5 Interfaces 定义签名；T6 用 CLI 调用 | 签名与 CLI 形态不一致（self-review 已发现） | 已修：Task 5 Interfaces 增补「CLI 是 `run_slice()` 的薄包装」 |
| T6 → T7 | L1 结果 | T6 产出结果 → T7 签署 | 顺序正确 | — |
| T8 → T9 | 夹具记录 / CI step | T8 产出 → T9 判据 7/8 | 顺序正确 | — |

### Per-task internal consistency

| Task | Finding | Ruling |
|---|---|---|
| T1 | Step 9 的 `cherry_picked` 中 `9a61cf59c0`/`cfd4306b7e` 为 10 位短 SHA，与另两条 40 位不一致 | **R3**：全部改为完整 40 位。计划已改 |
| T1 | Step 10 的 B4 断言称「cherry_picked 4 条齐全」 | **R4**：改为 5 条（applied 2 + path-import 1 + already-present 2）。计划已改 |
| T1 | Step 11 `git add` 未含 `runtime-contract/` 目录 | **R5**：已补。计划已改 |
| T1 | O11/O12 前置注记引用了旧的 Step 号 | **R6**：已更新为 Step 11。计划已改 |
| T3 | Step 2 有两分支（装 `jsonschema` vs 不装），O13 未裁决 | **R7**：默认装依赖（`requirements.txt` 已写明版本）；O13 裁决为「不引入」时走替代分支。实施者按默认走 |
| T4 | Step 3 改 ADR-011 的 `Status`，但该文件自述「未批准前不改写其状态」 | **R8**：属主裁决账本（2026-11-21）**晚于** ADR-011 撰写（2026-11-20），且账本明写「Codex 重签时应并入」→ 改写为条件批准是**执行裁决**，非违背。须保留其原始 `evidence_hash:57fe6017…` 与 `source_run` 标识 |
| T6 | Step 2 的 `cargo test --list --all-features --lib` 不覆盖矩阵引用的所有 package/target | **R9**：实施者按各 case 的实际 package/target 逐条 re-list，不限于 `--lib` |
| T8 | Step 5 的 CI `if:` 需匹配 ALPHA carrier | 无冲突（`develop/v4.1.0`，条件 `refs/heads/develop/`） |

### Scan conclusion

No task contradicts the Global Constraints. No task mandates a test that asserts nothing or
duplicates a logic block. 9 rulings recorded above; 7 were applied to the plan text directly,
2 (R7, R9) are implementation-time directives handed to the implementer.

## Rulings log

- **Ruling**: ADR-410-011 按目录从 `b681b74ca3` 导入，不 cherry-pick 整个提交 — 该提交含 PR #5915 的代码/脚本改动，超出「材料导入」范围；裁决账本要求「以自身文件为准」 — 代价：基线不含 PR #5915 的其它产物，但那些不在本次准入范围
- **Ruling**: manifest 重签统一放在「改动完成后、校验前」 — 校验先于重签必然失败 — 代价：无
- **Ruling (R10)**: `runtime-contract/20261120-pr5915/` 是 **56 个文件**，不是计划写的 62 — 实测 `git ls-tree -r --name-only b681b74ca3 -- <dir> | wc -l` = 56；implementer 把记录写 56 并由 B4 断言自证（写 62 会让断言变红） — 代价：无，计划与 spec 已改
- **Ruling (R11)**: `BASE` = **内容基线提交**（develop tip + 材料导入 + 冲突解决后的内容），是 develop tip 的**后代**、分支 tip 的**祖先**；后续所有任务的 `base_sha` 一律填 BASE，不随提交前移 — 提交无法包含自身 SHA，故 base_sha 不能是含 freeze 记录的那次提交；且若随提交前移，「冻结」与 receipt 的 SHA 绑定都会失去意义 — 代价：分支 tip 会前于 BASE，`git ls-remote` 返回的不是 BASE 本身，计划与 spec 的判据 1 已改
- **Ruling (R12)**: `cd8cefe623` 的 cherry-pick **净 diff 为空**（develop tip 已含等价内容，冲突解决后 `scripts/gate/check_g1_tpch_baseline.sh` 与 develop **字节相同**） — 实测 `git diff 00355e68be fd849ae205 --stat` 空，且 `git diff origin/develop/v4.1.0 HEAD -- scripts/gate/check_g1_tpch_baseline.sh` 空 — 代价：`cherry_picked` 中该条标 `applied`（冲突已解决）而非 `already-present`，其「净 delta 为空」的事实需由 B4 断言补证

## Task log

- Task 1: implemented by subagent `abeb1d3` (sonnet), status DONE_WITH_CONCERNS. Branch `base/v4.1.0-frozen` pushed to Gitea, tip `a870bbdfb9`.
  Commits: ca8b1684ba (cherry-pick 45da6968ae) / 00355e68be (runtime-contract 目录导入) / fd849ae205 (冲突解决，空 diff) / 7e09c22370 (freeze 记录 + verifier) / 952a749628 (verifier 单分支克隆兼容) / a870bbdfb9 (note 去重)
  `verify_baseline.py` → 4/4 PASS, exit 0 (RED: exit 1 on missing/placeholder record). 6 mutation probes caught. Fresh `--single-branch` clone → 4/4 PASS.
  Controller 已核实 3 条 concern：R10 成立、R11 成立、R12 成立（implementer 措辞「empty」指净 diff 为空，非 cherry-pick 操作报 empty；实测 cherry-pick 到 develop tip **报冲突**，解冲突后净 diff 为空、文件字节相同 → 实现正确）。
- Task 1: review round 1 → ❌ 4 Important (I1 账本 state 不诚实 + B4 applied 分支是 subject 字符串匹配 / I2 already-present 比较基准是会移动的 origin/develop / I3 path-import 计数与 presence 钉在 HEAD，Task 4 移 ADR-011 会引爆 / I4 单分支克隆下 6 项交叉核对被跳过却仍报 4/4 PASS) + 5 Minor
- Task 1: fix round 1/5 (9 findings addressed, 0 open — I1 ledger 改标 already-present + net_delta/conflict_resolved 字段, applied 分支改 blob-ID 内容比对; I2 改比 frozen parent; I3 计数钉 frozen base_sha、HEAD 侧改 presence-only; I4 跳过项单独计数 + --strict; M1 删 --check; M2 blob 级导入比对; M3 断言 source_ref/frozen_at; M4 弃用 assert、改 raise + 拒 -O; M5 .gitignore allowlist); commits a870bbdfb9..4038e927b5
  新增 `docs/releases/v4.1.0/refactoring/tools/test_verify_baseline.py`（31 断言 / 7 mutation probes，reviewer 已逐个追证「会 bite」）; 单分支克隆 normal exit 0 + 6 skips 上报, --strict exit 1
- Task 1: complete (commits 8cbde5b6d3..4038e927b5, re-review clean — 9/9 ADDRESSED, 0 Critical/Important; 分支 tip 4038e927b5 已推 Gitea)
- Task 1: minor (deferred): B2 新增 source_ref 可解析性硬依赖 —— 派生分支的单分支克隆会红（B2 严格化）
- Task 1: minor (deferred): 工作树漂移检查扩到 8 个路径（含 check_g1_tpch_baseline.sh / gate_honesty_test.rs 等 Task 5 可能合法编辑的文件）
- Task 1: minor (deferred): test_verify_baseline.py 未接 CI（CI 用显式路径调 fixture，无发现式 glob）→ **并入 Task 8 范围**，届时由 owner 批准的门禁变更一并接线
- Task 1: minor (deferred): `cherry_pick` 字段未绑定到源提交（只校验载体存在且为空）
- Task 1: minor (deferred): `have_commit` 用 cat-file -e 测的是对象存在而非可达性（`--shared` clone 的 alternate 会掩盖可达性问题）
- Task 1: minor (deferred): Task 1 的 brief 已过时（62 / applied 2 / origin/develop）——后续任务读记录与计划，不读该 brief
- Task 1: ruling (R13): `cd8cefe623` 的 state 改 `already-present` —— 计划写的「唯一真 delta」被事实推翻，账本必须诚实
- Task 1: ruling (R14): 计数/枚举断言一律钉 frozen `base_sha`；HEAD 侧改 presence/subset —— 否则 Task 4 移 ADR-011 会让冻结校验器变红
- Task 1: ruling (R15): B4 的 applied 分支改 blob-ID 内容比对，不再用 commit-message 字符串匹配
- Task 1: ruling (R16): 跳过项单独计数并进 verdict；加 `--strict` —— 校验器必须能区分「核过了」与「没核」
- Task 2: implemented by subagent `a99403e` (sonnet), status DONE_WITH_CONCERNS. `verify_sha_scope.py` RED=2/6 → GREEN=6/6, exit 0; 9 mutation probes; 142 历史文件字节不变（git blob + cmp）; pilot evidence sha256 未变
  Commits: 8424708a72 (主提交, 5 个 brief 指定路径) / df49c8abb6 + 18f336a307 (重签 manifest, 机械) / 81f96533e1 + d7cc3d7402 (self-review 清理)
- Task 2: concern C1 → **Task 1 fix round 2 的输入**：Task 1 的 `verify_baseline.py` B3 变红（7 文件）——它检查导入材料的 **blob** 是否仍在 HEAD，而 Task 2 按计划**原地改写**了其中 4 个文件（DAG / 矩阵 / task manifest / 14_HANDOFF_MANIFEST）的 `base_sha` 值，blob 必然变。计划的发布判据 2（路径集合差）仍成立（0 路径缺失）
- Task 2: ruling (R17): `verify_baseline.py` 的 B3 改为**路径存在性**断言（导入路径集合 ⊆ HEAD 树路径集合），**不再要求 blob 相同** —— freeze 记录的职责是冻结*身份*，不是冻结*内容*；内容级保证由下游各任务的校验器（`verify_sha_scope.py` / `verify_matrix.py` / `verify_handoff.py`）承担。这与 Task 1 的 R14（HEAD 侧改 presence-only）同向
- Task 2: concern C3 → **ruling (R18)**: DAG 的第 19 处 `artifact_context.source_code_base_sha` = `e9ff21fca3` 归类为**来源/provenance 绑定**，**保持原值不改** —— 它记录外部 test-factory 包是在哪个源码基线上产生的，与本次实施基线无关，改它等于伪造来源记录。`verify_sha_scope.py` 应新增一条断言把它固定住
- Task 2: concern C2（预期）: `verify_handoff.py --check` 仅在 line 65 `assert t["base_sha"]==BASE` 失败（line 16 硬编码旧 BASE）—— 无 manifest 漂移（232 绑定文件、0 漂移）。Task 3 处理
- Task 2: concern C4: ① S5 字面「与 45da6968ae 字节相同」在未修改树上不成立（6 个 manifest 晚于它，1 个后来被修正但未动 `frozen_commit`）→ implementer 改用双参照（bytes vs frozen baseline, bindings vs 45da6968ae），**接受该调整**；② 主机 python 缺 `jsonschema`，`--check` 用了 `/tmp` 临时 venv —— Task 3 必须落地依赖声明（O13）
- Task 1: fix round 2/5 (R17 applied — B3 改判路径在场而非 blob 相同; verdict 改为 `N/4 PASS, M/4 PARTIAL -- N skipped` 形态; 删除导入路径仍 fail-closed、搬走不判红; 提交 10a2bfc968 + 36d474167e)
- Task 1: re-review round 2 → **Approved**（I1–I4/M1–M5 全部 ADDRESSED，R17 ADDRESSED，0 Critical；reviewer 用 8 组 revert/变异实验证明探针会 bite）
- Task 1: complete (commits 8cbde5b6d3..36d474167e, 2 fix rounds, review clean)
- Task 1: ruling (R19): 接受「B3 不再对 179 个导入路径做内容比对」这一覆盖损失 —— **导入材料的内容完整性由 `10_HANDOFF_MANIFEST.json` 的逐文件 sha256 + `verify_handoff.py --check` 复算保证**（当前 233 绑定文件、0 漂移），B3 的职责是身份与在场而非内容。此项须在 Task 9 的放行裁决记录中写明
- Task 1: minor (deferred): 实现者报告 §3.4 夸大 RELOCATABLE 名单的强制力 —— 未登记的**纯搬移**（内容不变）实际放行，只有「搬移+改写」才判红；属报告措辞，非代码缺陷
- Task 1: minor (deferred): rule ②（`by_blob`，按 blob 在 HEAD 任意位置找到即算在场）无专属探针 —— 去掉它当前所有探针仍绿
- Task 1: minor (deferred): FAIL 的检查行不带 skip 计数（verdict 行仍带总数）—— 观感问题
- Task 2: fix round 1/5 (4 Important + 3 Minor addressed: S6 改钉仓库内 tracked 副本 / S5 历史集改从冻结基线派生并加计数下限 / 加 R18 pinning 断言 / 加工作树洁净断言 / S5 binding 层加 `compared > 0` / 新增 `tools/test_verify_sha_scope.py` 43 断言 / 合并重复常量; 提交 68a5ee9ad5 7fffe97afd 02844c349e dc18cf3eed da00a05ede a1b5db310a + 3 次 manifest 重签)
  关键证据：同一夹具跑在 pre-fix 校验器（18f336a307）上**失败 16 条断言** —— 证明 4 个洞是实的
  `--finalize` exit 0; `--check` exit 1 于 `verify_handoff.py:65`（硬编码 BASE，Task 3 范围）; manifest 233 绑定文件 / 0 漂移; 142 历史文件未变
- Task 2: concern (implementer): S5 的历史集已**冻结** —— 将来新增 audit run manifest（Task 5/8）会判红，须在冻结记录或门禁中登记
- Task 2: concern (implementer): 门禁现拒绝在脏工作树上运行（故意的 fail-closed），故无法在编辑该工具本身时运行
- 教训: **不并行 dispatch 两个会写同一 worktree 的 agent** —— 本轮曾并行 Task 2 reviewer 与 Task 1 fix implementer，reviewer 观察到并发编辑导致其「工作树 clean」陈述失效（被审 commit 本身干净，manifest 绑的是 HEAD blob，无实质损害）。后续一律串行。
- Task 3: implemented by subagent `aeefa67` (sonnet). 导入 `takeover/dsh-b/`（**68 文件，不是计划写的 36** —— 主工作区的 dsh-b/ 在计划成文后长大了）；`verify_handoff.py` 9 处断言闭包；`finalize()` 哈希范围收窄为 `handoff/` + `takeover/dsh-b/` + `refactoring/`（takeover 顶层排除）
- Task 3: review round 1 → ❌ 6 Important（**1 tier 检测是子串匹配 → fail-open**：`NOT APPROVED`/`UNAPPROVED` 被判为 APPROVED；**2 `Approvals` 从未被解析**（正则锚行首，实际在行中）→ 目录 `approval_refs` 全空、断言空转、R22 未实施；**3 第 9 个变异臂测的不是它命名的规则**（撞 PROPOSED 分支而非档位越权分支），且 R3 分支无提交内测试；**4 HARA digest 锚点被换成自指比较**（`--finalize` 可重写）→ 协调篡改可过；**5 `BASELINE_FREEZE.json` 被移出 manifest 覆盖集**，而它是 base identity 的唯一来源 → 篡改 freeze + DAG + 重签 = 绿灯；**6 `{10,11}` 使 O7 永久不被强制**）+ 8 Minor
- Task 3: fix round 1/5 (6 Important + 8 Minor addressed; 提交 81282fce8f / 555ca013b6 / 7df0136b1e / 1851c911c2)
  新增提交内夹具 `tools/test_verify_handoff.py`（42 断言）；同一夹具跑在 pre-fix 校验器上**失败 24 条** —— 探针确实 bite
- Task 3: ruling (R23): `BASELINE_FREEZE.json` 必须**显式纳入** manifest 覆盖集（计划把它连同 takeover 顶层一起排除是错的 —— 它是 base identity 的唯一来源）
- Task 3: ruling (R24): tier 判定改**整词精确匹配**，未知 token → PROPOSED（fail-closed），杜绝 `NOT APPROVED` 被判为 APPROVED
- Task 3: ruling (R25): `Approvals:` 必须真被解析，且**非 ADR token 必须拒绝而非静默过滤**；正则改为行内任意位置匹配
- Task 3: ruling (R26): 变异臂改用**合成 tier map**（003 → APPROVED_DIRECTION）自包含地测档位越权分支
- Task 3: ruling (R27): HARA digest 锚点必须是 `--finalize` **无法重写**的（取自冻结基线提交的内容）
- Task 3: ruling (R28): ADR 目录规模改为**阶段绑定**的期望状态（记录在已签署/已提交处），不用 `{10,11}` 常量带
- Task 3: consequence (影响 Task 4): R25 使 `Approvals:` 只能放**纯 ADR 引用**（计划模板里的 `OWNER-DECISION-C-ROUTE` 会被拒）—— 计划 Task 4 Step 2 已同步修订（R22）
- Task 3: consequence (影响 Task 4): 冻结记录新增 `adr_catalogue` 期望字段，Task 4 需把 011 纳入并更新期望
- Task 3: consequence (影响 Task 4): `runtime-contract/20261120-pr5915/00_DECISION_README.md` 有一条指向 ADR-410-011 的悬空链接，Task 4 移走 011 后需修
- Task 3: fix round 2/5 (前向破坏修复: 两个 tier 变异臂各自 pin 自己的 tier map，不再依赖仓内当前措辞 —— 否则 Task 4 写完 003 的真实档位后 `--check` 必红; 档位限定词只能收紧不能放宽; `Approvals` 逐项 fullmatch 裸 ADR id; 夹具改 hermetic; 提交 bbf512b798 / 7ddbee63fb)
- Task 3: re-review round 2 → **Approved**（Important + A–F 全 ADDRESSED；reviewer 逐条枚举 10 个变异臂确认其拒绝原因均可追溯到自 pin 或 tier-无关来源）
- Task 3: complete (commits 3d7006432d..7ddbee63fb, 2 fix rounds, review clean)
- Task 3: minor (deferred): tier 收窄只认已登记词汇 —— `Status: APPROVED（不得视为批准）` 这种词汇外表述仍判为 APPROVED。**Task 4 必须写档位 token 本身**（已写对）
- Task 3: minor (deferred): `Approvals` 只取第一个匹配行；多行列表形式会被硬失败（fail-closed，可接受）
- Task 3: minor (deferred): 夹具依赖 ADR-001/003 的当前 status 行 → Task 4 改写后必须 re-base（Task 4 已做）
- Task 4: implemented by subagent `a5d0580` (sonnet). 提交 0261d4ba6f / 7c4aae8425 / 45b8c678bb
  11 份 ADR 档位落地（APPROVED×3 = 001/002/005；APPROVED_DIRECTION×6 = 003/004/007/008/009/010；CONDITIONALLY_APPROVED×2 = 006/011）；ADR-410-011 并入 `adrs/`；18 处 `approved_adr_refs` 填完（17 非空，C01-15 空 → 记入 `ADAPTER-MAPPING-PENDING.md`）；冻结记录 `adr_catalogue` 期望更新为 11 份；manifest 重签 305/305 匹配
- Task 4: review → **Approved**（0 Critical / 0 Important / 7 Minor）；reviewer **独立重算** 11 个 tier 与 18 个映射（从各 ADR 的「目标不变量与影响」行 × 各包的 `invariant_ids`），结果与写入完全一致；42 条 Markdown 链接 0 悬空；重签后 305/305 逐字节匹配
- Task 4: ruling (R22 已实施): `Approvals` 只能是裸 ADR id；裁决路由与 sha256 绑定放独立 `Approval:` 字段（不被解析、不被断言）
- Task 4: ruling (R28 已实施): ADR 目录期望改为**阶段绑定** —— 记在 `BASELINE_FREEZE.json` 的 `adr_catalogue`（expected_ids/expected_size/adr_410_011_in_adrs），不再是 `{10,11}` 带
- Task 4: ruling (R30): ADR 正文的 `Approval:` 绑定指向 **BASE 下的 ADR digest**（reviewer 复核 10 个 digest 与 BASE 字节逐一对上；ADR-011 的 digest 与 runtime-contract 包自记的 `adr_sha256` 一致），不指向 ledger 绑定的 `45da6968ae`/`b681b74ca3` —— 二者字节等价，但 ADR 正文未声明等价，属**追溯性缺口**
- Task 4: ruling (R31): ADR 正文的**纯元数据编辑**（`Status:`/`Approvals:`/`Approval:`/`conflict_resolution` 行）**不触发** C6 的「内容变更须重审」—— 实质 ADR 正文未动；若将来改动「选择与否决」「数据与身份契约」等实质条款则必须重审
- Task 4: minor (deferred): `runtime-contract/20261120-pr5915/00_DECISION_README.md:22` 同句内前半段仍称「十份 PROPOSED ADR 原字节保持不变」—— 现已不成立
- Task 4: minor (deferred): ADR 正文深处仍有旧状态断言（如 `ADR-410-001.md:33`「原则接受C不等于此ADR已批」）与新的 `Status: APPROVED` 并存 —— 头部改了、正文没改，处理半吊子
- Task 4: minor (deferred): `ADR-410-009.md:7` 在自己的 `Approvals` 里列了 `ADR-410-009`（自指）；`ADR-410-008.md:7` 列了条件批准的 `ADR-410-006`（依赖陈述，非包引用，不违 R4 但字段名易误读）
- Task 4: minor (deferred): `ADAPTER-MAPPING-PENDING.md` 位于 `takeover/` 顶层 → **不在 manifest 哈希范围内**，属主据以裁决的治理记录未被 receipt 绑定（gate 仍以 `tasks_pending_adr_approval` 机器可读地报出 C01-15）
- Task 4: minor (deferred): `ADR_INDEX.md` 的 11 个状态格无任何门禁交叉校验（编目只解析 `adrs/*.md`），二者可静默漂移（今日一致）
- Task 4: minor (deferred): runtime-contract 包自带的 `handoff/MANIFEST.json` / `SHA256SUMS.txt` / `verify_packet.py` 仍绑 ADR-011 旧路径与旧字节 → `verify_packet.py` 抛 FileNotFoundError。**判定为可接受**：它是 PR #5915 的历史交付收据，重签等于伪造历史，且无任何 CI/门禁引用它（Task 2/4 的重签已使其此前就红）
- Task 4: complete (commits 7ddbee63fb..45b8c678bb, review clean)
- Task 5: implemented by subagent `a12bb9e` (sonnet). 提交 38c748d077 / 32bdd0ca62。矩阵 75 条 invariant 映射完成（13 组 declared_invariant → 11 个 INV id，MAPPED）；`case_id` 唯一化（T6-001B/002B）；`status_vocabulary_mapping` 落盘；迁入 pilot 的 `test_factory.py`（字节相同）与 `tf_t1_undo_invariants.rs`（已在基线）；新建 `tools/matrix_driver.py`（`run_slice` + 薄 CLI）、`tools/verify_matrix.py`（M1–M6）、`tools/test_matrix_driver.py`
  驱动层对**真工厂实跑**：T6-001/002 → PASS、T4-101 → KNOWN_FAIL（复现 D-001 RED）；三个 bespoke → NOT_RUN
- Task 5: review round 1 → ❌ 3 Important（**I1 注册 RED 在 pilot 记录缺失时会塌陷成 `FAIL`** —— 正是设计 anti-goal；**I2 bespoke 探针路径是死路**（探针迁入后仍只会 NOT_RUN + 假理由）；**I3 `PILOT-T5-101` 上游 COVERED_RED 但矩阵写 NOT_RUN → 对 M6 不可见且未升级给属主**）+ 4 Minor
- Task 5: fix round 1/5 (3 Important + 4 Minor addressed; 提交 7ed9c61730 / 7fb973b1f8)。断言 26→88，变异 9/9 bite
- Task 5: re-review round 1 → **Approved**（reviewer 重建 4 条对抗场景，确认 RED 塌陷在所有路径上不可达；`DriverError` 是循环体第一条语句，无分支可短路）
- Task 5: complete (commits 45b8c678bb..7fb973b1f8, 1 fix round, review clean)
- Task 5: ruling (R32): RED 判定**双信号**（矩阵 `source.claimed_status` + pilot `rerun_status`）；pilot 记录缺失 → `DriverError` fail-closed；`status_mapping()` 断言 FAILED_RED/COVERED_RED → KNOWN_FAIL
- Task 5: ruling (R33): bespoke 探针走矩阵级 `probe_registry`；已绑定则执行，未绑定则 NOT_RUN（理由准确）；专有 RED 身份不全时只能 NOT_RUN
- Task 5: ruling (R34): `PILOT-T5-101` 携带 4 个可复原要素 + `source_sha256: null` + `identity_gap` 显式标记 → 缺口可审计而非不可见；状态分歧升级为属主裁决项
- Task 5: ruling (R35): 驱动的映射副本必须与矩阵 `status_vocabulary_mapping` 块**交叉校验**，不一致即 `DriverError`（消除第二份规范副本）
- Task 5: ruling (R36): M4 语料改读**登记表 id 列**（结构化），不再正则扫散文 —— 原 review 的 `ADR-010` 实例经复核**不成立**（Python `\b` 在 CJK 与 "A" 之间不触发），但**类别缺陷**（语料是偶然的而非结构性的）已修
- Task 5: minor (deferred): `find_test` 改为「精确优先、否则唯一子串匹配」→ 他人测试的判定可能被记到本 case（不涉 RED，不会造假 PASS）
- Task 5: minor (deferred): 单个 pilot 记录缺失 → `DriverError` **中止整个 slice**（R32 所要求，但把逐 case 降级变成整 slice 中止；今日 75 条记录无缺口）
- Task 5: minor (deferred): 映射漂移检查是**精确 dict 相等** → 未来任何合法扩展都要同步改代码副本
- Task 5: minor (deferred): slug 约定可在无属主 registry 条目时绑定探针（与 registry 注释的「属主决定」表述有张力；今日 glob 无命中）
- Task 5: minor (deferred): 修复轮的 commit 引用是 10 位缩写而非完整 SHA（C5 讲的是 receipt 的完整 SHA 绑定）
- Task 5: minor (deferred): 提交 `7ed9c61730` 的 message body 有一处 heredoc 损坏字符（subject 干净，未 amend）
- Task 6: implemented by subagent `a73e655` (sonnet). 提交 601c026f48。L1 = 44 条（T1:30/T2:6/T3:8）；BASE 上 `cargo test -p sqlrustgo-storage --lib -- --list` 得 926 个测试，42/44 名存在、0 消失；driver exit 1（fail-closed 约定）→ 42 PASS / 2 NOT_RUN，耗时 6.44s；差异类型 42 一致 + 2 NOT_RUN，0 新增 RED / 0 PASS→FAIL / 0 RED 消失 / 0 状态降级
- Task 6: review round 1 → ❌ 2 Important（**1 两条 NOT_RUN 行基于对 pilot 基线的错误陈述** —— pilot 确实跑过（`rerun-tf_t1_undo_invariants-20261120T171057.log` 4 passed），真实差异是「pilot 跑过、BASE 完全跑不了」，因为 `[[test]]` 注册只存在于 pilot 的**未提交本地编辑**（pilot HEAD f23f237bb1 比声明 base e9ff21fca3 领先 2 个提交，`git log -S` 查无该注册）—— pilot 用本地改动掩盖了**基线装配缺口**；**2 42 条 PASS 与 DSH-B 验收口径冲突** —— `REQUIREMENT-TEST-COVERAGE-MATRIX.md:23` 明写「assembly=UNVERIFIED 的不得记 PASS」，而 75 条全带该标记，必须升级属主）+ 5 Minor
- Task 6: 重要事实: **L1 切片内不含任何已知 RED** —— 台账 17 项与矩阵 5 个注册 RED 全在 T4/T5 或根本不是 75 场景矩阵成员 → **该切片不检验任何 RED 身份机制**
- Task 6: 重要事实: **ledger 台账实为 18 行，而其正文自称 17**
- Task 6: minor (deferred): 矩阵 `actual` 膨胀 +134%（42 条各 ~4.9KB、含机器特定绝对路径），56% 的文件体积
- Task 6: minor (deferred): §四「结论」列把「artifact 仍在树里」与「RED 在 BASE 上仍复现」混为一谈（7/18 行受影响）
- Task 6: minor (deferred): `REQUIREMENT-TEST-COVERAGE-MATRIX.md:13` 的「复跑通过数 0」已过时
- Task 6: minor (deferred): `L1-RERUN-*.json` 无自身 SHA 绑定（仅 12 位前缀在文件名里）
- Task 6: minor (deferred): 3 个 evidence 文件被 force-add（`.gitignore` 的「无 log/tbl/json 测试产物」catch-all 命中，但 manifest 哈希整棵 PLAN 树 → 未跟踪会让新克隆的 `--check` 因缺文件而红；同树已有 13 个 tracked .log / 98 个 tracked .log+.json 先例）
- Task 6: fix round 1/5 (2 Important + 4 Minor addressed; 提交 6bf3cb2f3e)。**implementer 纠正了自己的错误陈述** —— 原报告称「pilot 从未跑过那两条」为假，pilot 确实跑过（4 passed）
- Task 6: fix round 2/5 (1 Important + 2 Minor addressed; 提交 2923612f51)。C5 绑定记录的执行点改为**真实跑过的那棵树** `executed.sha=7fb973b1f87e89…` / `executed.tree=06565d61a2ea…`（由工厂自记的 `base_sha` 推导，非抄写）；产物提交 SHA 隔离到 `artefact_commits` 块并带 `ordering_evidence` 祖先证明；`product_code_evidence` 改跨 `fd849ae20541..7fb973b1f8`；矩阵↔回执 `actual` 关系登记为**可运行的派生规则**（非逐字相等）；8 个 digest 在内容定稿后全部重算
- Task 6: re-review round 2 → **Approved**（reviewer 逐条独立复算：`git rev-parse 7fb973b1f8^{tree}` = 06565d61a2ea ✓、祖先序 ✓、38 提交 ✓、`product_code_evidence` 路径集 0 变更 ✓、8 个 digest 与磁盘字节逐一对上 ✓、manifest 条目 = binding 记录自身 digest ✓、**矩阵与 round-1 树字节相同 → `red_identity` 结构上不可能被改写** ✓）
- Task 6: complete (commits 7fb973b1f8..2923612f51, 2 fix rounds, review clean)
- Task 6: ruling (R37): 两条 NOT_RUN 行的归因改为「pilot 用一次**未提交**的本地 `[[test]]` 注册让它们可执行；BASE 里没有该注册 → 基线装配缺口」；verdict 仍为 `NOT_RUN`（非新缺陷、非回归）
- Task 6: ruling (R38): 42 条 PASS 与 DSH-B 验收口径（「assembly=UNVERIFIED 的不得记 PASS」）的冲突必须写入 `L1-DIFF` §五-5、§六 判定表的备注列、以及 DSH-B 规则文件本身 —— **两种读法并列陈述，由属主裁定，不得被差异表默默吸收**
- Task 6: ruling (R39): C5 绑定记录必须区分三个事实 —— **执行的 SHA+tree**（由工厂自记的 `base_sha` 推导）、**产物提交的 SHA**（隔离到独立块）、**排序证据**（祖先关系证明）；只读 JSON 的读者也必须能分辨

## 累积的 Owner 裁决项（Task 1–6 期间产生，均已记录在对应交付物中）

| # | 事项 | 出处 | 阻塞 |
|---|---|---|---|
| O2 | 75 场景中哪些标记 `required_for_certification=true` | Task 5/6 | A5 判据 6 |
| O6 | Oracle 签署人（分层：先签 T1 试点，再批量签 T2/T3）—— **属主已裁决不介入签署环节**，由独立 Reviewer 承担 | Task 3 后的裁决 | Task 7 |
| O13 | 是否允许加 `jsonschema` 依赖 | Task 3 | 已按默认落地（requirements.txt 钉 4.23.0） |
| O16 | 「测试已消失」的分类 | Task 6 | 已按 NOT_RUN + 原因记录 |
| **N1** | **在基线里注册 `[[test]] tf_t1_undo_invariants` vs 把 `PILOT-T1-101/102` 记为不可认证** | Task 6 | L1 切片能否作为验收证据 |
| **N2** | **`assembly=UNVERIFIED` 的场景可否记 PASS** —— 两种读法并列；DSH-B 字面读法下「42 PASS」不可作验收凭据，而该切片又不含任何 RED → 按字面读法它没有任何验收价值 | Task 6 | L1 切片的证据效力 |
| **N3** | ledger #10/#11 的 `source_sha256` 滞后一次改写（RED fn 完好） | Task 6 | 台账自洽 |
| **N4** | ledger 实为 18 行而其正文自称 17 | Task 6 | 台账自洽 |
| O1/O3/O4/O5/O10/O11/O12/O14/O15/O17 | 见 spec §6.2 | — | 不阻塞对应步骤 |

- Task 7: implemented by subagent `aa87e03` (sonnet). **本任务推翻了整备路径的一个前提**：44 条 L1 断言逐条判定 → **可判定 6 / ASSERTION_UNREVIEWABLE 38（R1 12 · R2 22 · R3 4）/ 已副签 0**
- Task 7: **根因（已独立复核为真）**：矩阵的 `declared_invariant` 是**按 feature 分桶的标签**（全 75 条 13 种取值，逐 feature T1 3 / T2 1 / T3 1 / T4 3 / T5 2 / T6 3；L1 子集 5 种；T1 的 28/30 共用同一句），Task 5 按其明写规则「declared_invariant 相同的整组用同一组 id」贴 `invariant_ids` —— **从未逐用例核对**。例：T1 组含 `test_classify_sql_transaction`、`test_semi_sync_master_record_transactions`，与其声明的「undo log 语义」无关
- Task 7: 另有 3 条测试与所声称的不变量矛盾/无效：`PILOT-T1-002`（注释写明回滚后页仍留在缓存 —— 钉的是反面）、`PILOT-T3-002`（`assert!(!s.dirty)` 空过 —— flush 前从未写入）、`PILOT-T1-101`（文档注释与唯一断言相反）
- Task 7: review round 1 → ❌ 2 Critical（**C-1 签署者不符合资格** —— 签署者是 Task 7 执行者本人，不在属主授权的 Codex/DSH Reviewer 池，且计划的 BLOCKED 兜底未启用；且 Task 9 自动判据只查 `assertion_reviewed_by` 非空 → 这 8 个签名会被读成有效 Oracle 签名。**C-2 继续第 2 段的授权条款在 brief/plan/design 中根本不存在**（「澄清」一词零命中）—— 该措辞来自我的 dispatch 提示词，**是我的错**）+ 3 Important + 6 Minor
- Task 7: fix round 1/5 (2 Critical + 3 Important + 5 Minor addressed; 提交 d971dacd22 / cea04bdc9b / e605dc97a5)。8 个签署**全部撤回**（44 条无一带签署人）；Oracle 步骤记 **BLOCKED**；判定记录保留在矩阵新字段 `assertion_review_verdict` / `assertion_review_performed_by` / `assertion_review_session` / `assertion_reviewed_commit` / `assertion_reviewed_at` / `assertion_source_sha256` / `reason`；`assertion_source_sha256` 保留（真实文件摘要，副签人需要）；3 个第 2 段签署撤回、判定保留并标注；残余 R-T7-1/2/3 登记给 Task 9
- Task 7: re-review round 1 → Critical 1/2、Important 1/2/3 全 ADDRESSED；Minor 1–5 ADDRESSED；**Minor 6 NOT ADDRESSED**（仅报告文字层面）
- Task 7: complete（实质交付 Approved）—— 钉死计数：**已审查 44 / 可判定待副签 6 / 不可独立判定 38 / 已副签 0**
- Task 7: ruling (R40): 无人可证明属于属主授权的 Reviewer 池 → **副签必须来自属主**；因此撤回全部签署、把 Oracle 步骤记 BLOCKED 并登记残余，**不以任何形式留下会被 Task 9 自动判据读成有效签名的字段**
- Task 7: ruling (R41): 「澄清」条款是我在 dispatch 提示词里写错的措辞（brief/plan/design 三处均为绝对前提）→ 第 2 段作业是在错误指示下进行的，必须在记录中如实写明并撤回其签署
- Task 7: ruling (R42): 不可判定的 38 条**不得**为了让数字好看而签署；诚实的 `ASSERTION_UNREVIEWABLE` 优于完整的签名外观
- Task 7: minor (deferred, 报告文字): 报告 `:363-365`/`:584` 称「44 条 `note` 行因 JSON 重新序列化被重发」—— **与 diff 矛盾**（36 个 hunk 零删除、8 个 hunk 各 2 条删除，全是 `assertion_reviewed_by`/`assertion_source_sha256`；文件总量只在不含这些行的前提下对得上）
- Task 7: minor (deferred, 报告文字): 报告 `:430` 仍留旧的「实际只有 7 条」，与钉死计数（0 副签 / 5 条 PASS 待副签）矛盾 —— **Task 9 会读这份报告，续跑时须先修正**
- Task 7: minor (deferred, 字段语义): `assertion_reviewed_at` 现携带**判定**时间而非签署时间（治理定义为签署时间）→ 任何只查 `reviewed_at`+`source_sha256` 而不查 `reviewed_by` 的临时读者会把 42 条 PASS 读成已签
- Task 7: minor (deferred, 字段语义): `assertion_reviewed_commit` 填的是冻结 `base_sha` fd849ae… 而非实际读取的 commit 2923612f…；且 17 个测试源文件的字节同一性论证**不覆盖** `TRANSACTION_INVARIANTS.md`（判定判据 (c)/(d) 也依赖它）
- Task 7: minor (deferred): `verify_matrix.py` M5 仍打印「the mapping covers 9 word(s)」—— 正是先前误量化的来源，下一个读者仍会踩
- Task 7: minor (deferred): `ORACLE-REVIEW-LOG-T1.md` 仍在 manifest 哈希范围之外（矩阵被绑定、叙述性记录未被绑定）—— 扩范围属主专属，已登记 R-T7-3

- Task 10: **在 Task 7 的发现之后追加**（属主裁决「先修矩阵映射与断言」）——Task 7 推翻了「矩阵映射可用」这一前提，故必须先修
- Task 10: 实现 = **逐用例从测试源重建 `invariant_ids`**（不再沿用按 feature 分桶的 `declared_invariant` 标签）+ 修复 38 条不可独立判定的断言
- Task 10: 成果 —— 75 条中 **64 条 `invariant_ids` 改动 / 11 条不变**；**有源派生覆盖的仅 12 条用例 / 5 条不变量**（INV-01, 02, 04, 07, 14）；**13 条不变量零覆盖**；L1 内 8 条有 id、L1 的 42 条 PASS 中 6 条有 id
- Task 10: 撤下 3 条标签派生映射（T5-101 / T6-001B / T6-002B）→ `invariant_ids: []` + 登记 GAP-14；两条 R3 测试体未改（该能力在冻结基线上不存在）→ id 留空 + 登记 GAP-13
- Task 10: 38 条断言修复 → **35 条可判定**（2 条作为不变量覆盖、33 条作为缺口提案）、**3 条仍不可判定**（T1-009 无断言 / T3-002 空过 / T3-003 无断言）
- Task 10: `TRANSACTION_INVARIANTS.md:30` 登记 `NO_INVARIANT_MATCH` 映射状态词表；38 条对账表落在 `ORACLE-REVIEW-LOG-T1.md:505-511`；`verify_matrix.py` 升到 7/7（M7 逐用例引用 + 双向缺口索引）
- Task 10: review round 1 → ❌（详见 `review-task10-round1.diff`）；fix round 1/5 → re-review **Approved**（8/8 findings ADDRESSED，新出 2 Minor：报告前半段自述自相矛盾、M7 词表校验在 registry 缺失时走 fail-open `skip()` 路径）
- Task 10: complete (commits e605dc97a5..d67fd2264b, 1 fix round, review clean)。提交：`e9d1214d92`（主体）/ `9173fe0f81`（binding 重算）/ `3fb58a745b`（计数修正）/ `0f00da6a1a`（撤下 + M7 收紧 + 词表）/ `d67fd2264b`（自述口径修正）
- Task 10: ruling (R43): INV-10 与 INV-12 的唯一「覆盖」来自标签派生而非源码派生 → **撤回**，宁可留下 13 条零覆盖也不保留无法复核的覆盖
- Task 10: ruling (R44): 断言修复只允许**改断言文本与映射**，不得改测试体逻辑——R3 两条测试体保持原样，缺失能力登记为 GAP-13 而非补写实现
- Task 10: ruling (R45): `invariant_ids` 必须是**逐用例**、从**测试源码**派生的；按 feature 分桶的标签不再是可接受的映射来源
- Task 10: ruling (R46): 13 条零覆盖的不变量进入 Task 9 的判据与 R2 包的缺口清单 —— **这是「可以开始重构」与「可以放行」之间的实质差距的量化**

## 当前分支状态

`base/v4.1.0-frozen`，tip = `d67fd2264b`，已推送 Gitea。**8/9 计划任务完成**（Task 1–7 + Task 10）。
剩余：**Task 8**（ALPHA 阶段执行点 + 7 道负向夹具）、**Task 9**（12 条判据汇总 + R2 包 + 放行裁决记录 → 属主介入点）。

### 待 Task 9 消费的残余项（Task 7 / Task 10 产生）

| 残余 | 内容 | 出处 |
|---|---|---|
| R-T7-1 | Oracle 副签人身份（属主授权的独立 Reviewer）—— 无可用 Reviewer → Oracle 步骤 BLOCKED | Task 7 |
| R-T7-2 | 第 2 段签署是在**错误指示**下进行的（我在 dispatch 里写了 plan/brief/design 都没有的「澄清」条款），3 条签署已撤回 | Task 7 |
| R-T7-3 | `ORACLE-REVIEW-LOG-T1.md` 在 manifest 哈希范围之外 | Task 7 |
| GAP-13 | 两条 R3 测试所需能力在冻结基线上不存在 | Task 10 |
| GAP-14 | 3 条用例的原映射为标签派生，已撤下 | Task 10 |
| 零覆盖 | 13 条不变量无任何源派生覆盖 | Task 10 |
| 不可判定 | 3 条断言仍不可独立判定（T1-009 / T3-002 / T3-003） | Task 10 |











## Task 8 —— 派发前的裁决（2026-11-24）

计划正文（Task 8 Step 2/3/5）与仓库事实有四处出入，按「spec 是权威、计划是论证、我的判断裁定两者都没回答的部分」处理：

- **R47（Step 2 的派生口径 —— 以夹具实物为准，具体到哪份实物）**：`gov_probe.NEG_FIXTURE_RE` 现在按**门禁脚本自身源码**里的关键字（`selftest|self_test|negative|反例|反向变异|canary|负向`）判定 `neg_fixture`，语义是「门禁**自带** selftest」。而 `GATE_CATALOG.jsonl` 的人工 `effect.reason` 说的是「由**外部**夹具 fxNN 证明（kind=mutation-verified）」——**两个字段在说两件事**，于是 7 行出现 `neg_fixture=false` + `effect.catches` 非空的表面矛盾。仓库自己早已诊断过同一问题（Gitea issue #5598 正文：「关键字统计会把「提到过反例」当成「有反例夹具」。台账按门禁逐行记账才是可核对的口径」），且 `docs/releases/v4.1.0/NEG_FIXTURES_v4.1.0.jsonl`（57 行）就是那份台账。
  **裁决**：口径基准 = **台账 + `audit/v4.1.0/neg/fx*.sh` 实物**；`neg_fixture` 的**原义（门禁自带 selftest）不变**（改义会让 `OWNER_REPORT_V4.1.0_2026-11-05.md:53`、`IL6_AUDIT_GATE_SELFREFERENCE_2026-11-05.md:127`、`GATE_BASELINE_V410.jsonl` 的 24/168 与历史审计口径全部失真），**新增**一个独立字段承载「外部夹具」事实，使 7 行从「矛盾」变成「三个各自为真的正交事实」。`gate_registry.py check` 的 C2 判据同步新增这条双向核对。
  **代价**：登记表多一个字段，`GATE_BASELINE_V410.jsonl` 与 4 处历史文档是否同步由实施者核。

- **R48（要造几道夹具 —— 计划说 4，实际是 6）**：计划 Step 3 称「已有 fx02/fx08/fx14 瞄准 ALPHA 阶段门禁」，这**不成立**：`fx02 → check_anti_ignore_gate.sh`（确为 ALPHA）；`fx08 → check_alpha_entry_v4.10.sh`（`stages=[]`）；`fx14 → check_stage.sh`（`stages=[]`）。**只有 1/7 已有夹具**，需新造 **6** 道：`check_alpha_v410.sh` / `check_arch_invariants.sh` / `check_arch3_no_bypass.sh` / `check_coverage_v312.sh` / `check_sql_corpus_gate.sh` / `check_g1_tpch_baseline.sh`。计划正文「其余 4 道」是笔误。

- **R49（每道夹具的缺陷规格不新造）**：这 6 道的缺陷规格**仓库已写好**——`NEG_FIXTURES_v4.1.0.jsonl` 里对应行的 `target` 字段就是台账给的关闭条件（如「删 docs/releases/v4.1.0/STAGE.yaml 后 composite 必须变红」「新增一处绕过 ExecutionEngine 的 DML 调用 ⇒ 必须变红」「把某个 crate 的覆盖率表替换成低于阈值的值 ⇒ 必须变红」）。实施者按 `target` 施工，**不得自创另一套规格**。

- **R50（timeout 不得当成变红）**：仓库既有规则（issue #5598 关闭计划）：夹具泳道必须「预热 target + 每夹具预算 30 分钟 + **超时即报 INCONCLUSIVE，不允许把 timeout 的 124 当成「门禁变红」**」。`verify_neg_fixtures.sh` 必须区分 RED 与 INCONCLUSIVE，**不得**凑成 7/7；台账对应行在夹具未成立时**保持** `pending-fixture`，不得为对账写成 `mutation-verified`。

- **R51（O17 裁决 —— ALPHA 执行面用哪套工具）**：**CI 执行点用 `check_stage.sh`（两态）**。理由：(a) `check_stage.sh` 是 SSOT 的唯一 driver，BETA/RC/GA 都用它 → 单一执行面（C2）；(b) 三态 runner 的**执行**接线被 design §A4.1 G2 明确移交 kanban IL-7（属「已知红分支」策略决定，不在 A4 范围内）。
  **代价（必须登记，不得默默吞掉）**：`check_stage.sh` 丢失 INCOMPLETE 语义（超时/信号/脚本缺失全塌成非零）→ 列为 Task 9 的残余项。
  但**夹具的两层 RED 证明用三态工具**：`run_gates.py plan/execute` + `gate_report.py report`（**既有**工具，非新建，不违反 C2）——因为它给出机器可读的 `results.jsonl`，且汇总是**独立重解析**权威集合后得出的，不是同一进程的自述。

- **R52（C8 与 ALPHA 执行点的触发方式 —— 计划 Step 5 与 C8 冲突，裁定如下）**：计划 Step 5 提的
  `if: startsWith(github.ref,'refs/heads/develop/')` + `check_stage.sh --stage ALPHA` **直接违反 C8**（每次 push 全量）。
  **裁定**：ALPHA 全量清单**不由 push 触发**；落在**独立 workflow** `.gitea/workflows/ci-alpha-stage.yml`，`on: workflow_dispatch`（promotion 手动触发），**独立 concurrency group 且 `cancel-in-progress: false`**。
  **为什么不能放 ci.yml**：ci.yml 的 concurrency 组是 `sqlrustgo-ci-pipeline-v2` 且 `cancel-in-progress: true`；任何落在 ci.yml 的非 push 触发器（`schedule` 或 `workflow_dispatch`）都会**取消别人正在跑的 run**——`ci-trusted-baseline.yml` 的注释明确记录了这条禁令（它就是因为这个才另开自己的组）。而把 `develop/` 加进 ci.yml 的 job 级 `if:` 同样会每次 push 触发全量 ALPHA ⇒ 违反 C8。
  **连带**：Task 9 判据 7 的**检索范围**由「仅 `.gitea/workflows/ci.yml`」扩为「`.gitea/workflows/*.yml`」，**判据强度不变**（仍要求：真实执行、非 `--dry-run`、退出码可传播、触发条件可指认）。此为口径变更，已登记，Task 9 执行时按新范围核。

- **R53（不新建第二套执行面）**：ALPHA 执行点**只有一处**（新 workflow）。不在 ci.yml 里另加 ALPHA step —— 两处执行点 = C2「第二套 harness」的变体。

- **R54（两层 RED 的可核断言）**：两层必须来自**同一次** `run_gates.py execute` 产生的同一份 `results.jsonl`：
  (a) 门禁层 = 该文件中目标门禁那一行 `state == FAIL` 且 `exit_code != 0`；
  (b) 汇总层 = `gate_report.py report` 对**同一份** results.jsonl 的**独立重解析**结论 `!= PASS`；
  (c) **因果链闭合**：汇总层必须把**目标门禁**标为 FAIL（不是仅因无关理由而红）；
  (d) 目标门禁的退出码**不得**是 timeout(124)/信号 ⇒ 否则该夹具判 **INCONCLUSIVE**，不计入 7/7；
  (e) 脚本汇总必须**分别**报 RED 与 INCONCLUSIVE，不得把 INCONCLUSIVE 报成 RED。

- **R55（新夹具的落点与登记）**：新夹具落 `audit/v4.1.0/neg/fxNN-<gate>-<缺陷类型>.sh`（沿用现有 fxNN 编号续到 fx21），形状沿用既有夹具约定：`<scratch-tree>` 作参数 → 注入故障 → 跑**真实门禁** → 断言 rc 非零 → 还原 scratch → rc 0 = FIXTURE-PASS / rc 3 = FIXTURE-FAIL。**每个新夹具必须在 `docs/releases/v4.1.0/NEG_FIXTURES_v4.1.0.jsonl` 登记对应行**（`kind`/`fixture`/`fixture_id`/`evidence_run`/`note`），且 `python3 audit/v4.1.0/probe_neg_fixtures.py check` 必须仍然通过。

- **R56（不得放宽门禁）**：本任务**不新增**门禁、不改 `STAGE_CONFIG.yaml` 的任何阶段清单/阈值、不改 168 分母。ALPHA 的 10 条 required_gates 一个不动。

## Task 8 —— 评审第 1 轮（reviewer `a1d7b56`，opus）

评审结论：**❌ 0 Critical / 6 Important / 7 Minor** → 质量判定 **Needs fixes**。

- **做对的（reviewer 逐条独立复核通过）**：两层 RED 机制复用仓库既有三态工具（`run_gates.py plan/execute` + `gate_report.py report`），**同一份 `results.jsonl`** 出两层；超时纪律真实（`run_gates.py:162-175` 124/信号→INCOMPLETE、`gate_report.py:95-97` FAIL 只认 rc==1、`fx_two_layer.sh` 独立再拒 124）；覆盖率夹具 fx18 带**对照臂**，判 INCONCLUSIVE 有实证（`logs/014` 干净树即红：`measured 7 of 8 crates`）；发现并如实登记两处门禁本体缺陷（`check_arch3_no_bypass.sh:34-46` 只数 marker；`crates/sql-corpus/src/lib.rs:1118` 解析出的 `expected_rows` 是字面量 `"rows"`，期望行数从未被比较）而**未擅改门禁**；`neg_fixture` 原义未动、新增 `external_fixture` 拆分两个事实，历史 24/168 口径保持；`swallow_hits` 对齐方向正确（让登记值等于现场重算值），且 `swallow_n` 实测 57 前后一致（任务书的 56 是过期快照）。
- **Important 1**: `audit/v4.1.0/neg/fx_two_layer.sh` 命中 `probe_neg_fixtures.py` 的 `fx*.sh` glob → `probe_neg_fixtures.py fixtures` 由 rc 0 变 **rc 1**（`UNDECLARED-EXPECT`）——**回归**。
- **Important 2**: `audit/v4.1.0/gen_neg_ledger.py` 未更新 → 重跑生成器会把本任务提为 `mutation-verified` 的 5 行**静默改回** `pending-fixture`，并把 fx02 的 `evidence_run` 指回旧 run。
- **Important 3**: `check_sql_corpus_gate.sh` 的 `effect.catches: ["expected-tampered"]` **与证据相反**——fx19 的 arm C 证明篡改 1476 条 `-- EXPECT` 之后门禁**仍绿**；真正成立的是「截断语料 ⇒ 变红」。
- **Important 4**: `check_arch3_no_bypass.sh` 行的 `target` 文本仍写「新增绕过 ⇒ 必须变红」而 `kind` 已是 `mutation-verified`；`check_g1_tpch_baseline.sh` 行的「该门禁在冻结基线上本来就因基线哈希漂移而红」**是错的**——reviewer 实跑 `check_g1_tpch_baseline.sh` → `G1 PASS`（exit 0）。
- **Important 5**: brief Step 6 未落实——`check_stage.sh --version v4.1.0 --stage ALPHA` **从未端到端跑过**（只跑了 `--dry-run` + 逐道真实执行）。
- **Important 6**: manifest 改名 `manifest.task8.yaml` 使该证据 run **无法被仓库自己的 `audit_run.py verify` 读取**（`audit_run.py:104,113` 硬编码 `manifest.yaml`）。

### Task 8 修订 1 的裁决

- **R57（Important 6 的解法 —— 恢复文件名，扩冻结集，不绕 glob）**：`verify_sha_scope.py` 的 S5 设计意图是「**历史**记录不得被改写」，它**不是**「run 目录不得增加」。S5 自己的 docstring 写明两条合法路径：「a *new* audit run manifest must be added to the freeze record (or the gate must be re-ruled), it must not silently widen the set」——**但实现只支持第二条**（`expected` 由 `git ls-tree` 在 BASE 上派生，不读冻结记录）。
  **裁决**：走第一条 —— ① 文件名**恢复** `manifest.yaml`（否则证据链断了：`audit_run.py verify` 读不到 = 新证据不可复核）；② `docs/refactor/v4.1.0-c/takeover/BASELINE_FREEZE.json` 增加一个**显式**的历史集增补字段（如 `historical_set_additions`），把本轮 run 的 manifest 路径登记进去；③ `verify_sha_scope.py` 的 S5 `expected` = 冻结集 ∪ 增补集，**仍必须拒绝**任何未登记的新增与任何历史条目的字节漂移；④ 用 `verify_handoff.py --finalize` 重签（冻结记录在 manifest 哈希范围内）。
  **代价**：动 Task 1（冻结记录）与 Task 2（S5）的产物，并在 Task 3 的重签链上跑一遍——这是让证据链自洽的最小改动，不是放宽门禁。
- **R58（Important 3/4 —— 机器可读的声明必须等于证据建立的东西）**：台账行与 `GATE_CATALOG.jsonl` 的 `effect.catches` 只允许写**夹具真正证明**的性质；夹具同时证明反面（该门禁**看不见**某类故障）时，那必须写成 **known hole**，不能写成 `catches`。这与 Task 7 的 R42、Task 8 的 R50 同一精神。
- **R59（Important 2 —— 台账是生成物，生成器必须能复现它）**：`NEG_FIXTURES_v4.1.0.jsonl` 由 `gen_neg_ledger.py` 生成；生成器与产物不一致 = 一次重跑就能把本轮成果静默抹掉。生成器必须更新到能复现当前产物，且它的输出必须与提交进来的台账**逐字节一致**。
- **R60（Important 5 —— 端到端跑一次，预算有界）**：必须实跑一次 `bash scripts/gate/check_stage.sh --version v4.1.0 --stage ALPHA`（热 target，期望非零退出）并把输出挂进证据 run。**若在有界预算内跑不完**，如实报 ⚠️ 给 Task 9，**不得**用 dry-run 冒充。

### Task 8 的 Minor（不进修订循环，留给最终整支评审分诊）

- Minor 1: fx02 台账行的 `evidence_log` 仍指旧 run 的日志（**升级进修订**：机器可读指针指错，属 Important 3/4 同族）
- Minor 2: 5 条新台账行缺 `evidence_log`（**升级进修订**：同族，指针缺失）
- Minor 3: 阶段汇总层用的是夹具自造的 `--authoritative-file` 单门禁清单 ⇒ `gate_report.py` 的 C1/C7 在该泳道里是空判
- Minor 4: 两层判决是从**打印出来的行**里 grep 出来的，不是从 `results.jsonl` 读的
- Minor 5: fx18 的对照臂用的是 GA 门槛（85）而非 ALPHA 门槛（50）
- Minor 6: fx02 尾部的表征臂已成死代码
- Minor 7: 两个共享 helper 放在 `audit/v4.1.0/neg/` 里（Important 1 的成因）

## Task 8 —— 修订第 1 轮的 re-review（reviewer `a1cf606`，opus）

- **8/8 findings ADDRESSED**（6 Important + 2 条升级 Minor），**无新增 Critical/Important**。re-reviewer 独立复核：生成器在沙箱里重跑 `cmp` ⇒ 台账**逐字节一致**；S5 新增两条变异臂（「未登记的新 manifest ⇒ 非零」「已登记增补项的 frozen_commit 绑定被抹掉 ⇒ 非零」）都过；`audit_run.py verify` ⇒ `evidence_hash=sha256:05672028…` 与 manifest 记录一致；端到端 ALPHA 跑 **27:06.58 / 退出码 1 / `Stage ALPHA (v4.1.0): 13 PASS, 4 FAIL`**（10 条 required_gates 含 3 条 cargo 全跑），红是预期的（composite、覆盖率、anti_ignore、`cargo fmt --check`）。
- **R61（第四种期望值 `inconclusive` —— 接受）**：re-reviewer 用两条实跑臂证明它**不能**冒充红（`check` 的 `FIXTURE-EXPECT-MISMATCH`、`fixtures` 要求恰好 exit 4、`known_hole` 必须是 `green-characterization`），覆盖率门禁那行仍是 `pending-fixture` 且 `effect.catches: []` ⇒ 没有任何机器可读声明依赖未建立的性质。**唯一的松弛**：`fixtures` 泳道把 exit 4 计入 `pass=` 桶（汇总行会读成「19 个夹具证明了它的性质」而其中一个什么都没证明）—— 建议给该泳道单列 `inconclusive=` 计数器（非阻塞，留给最终分诊）。
- **Task 8: fix round 1/5 (8 addressed, 0 open — 提交 d67fd2264b..bf8f038dce)**

### 修订 1 带出的证据保真残余（**升级进修订 2**，理由：这份证据 run 就是 Task 9 交给属主与 R2 外审包的东西，保真度是承重的）

- **New Breakage 1（升 Important）**：证据 run 里 `verify-handoff` 那条记着 `exit_code: 1`，日志是 `FAIL AssertionError: …/test_verify_sha_scope.py` —— **是重签之前抓到的旧红**；重签提交 `d6b6343955` 发生在 run 建成之后，run 没跟着重建。检查本身今天是通过的（314 个 hash 全对）。即：**提交进仓库的证据记录比事实更悲观，且没有任何注记解释**。在整个计划的主题（证据不得乐观、也不得不悲观的失真）下，这不是小事。
- **New Breakage 3（升 Minor→修订 2）**：`fx21` 的通过率正则永远匹配不上（夹具按 `[0-9]+\.[0-9]+% pass rate` 取，门禁实际打印 `SQL Corpus pass rate: 98.2%`）⇒ 日志里是 `pass rate=<none>`；判决取自退出码（正确的判别器），但**夹具头部声称的那句话没有被机器检查**。
- **New Breakage 5（升 Important）**：run 内**三条命令跑在三个不同 commit 上**（driver `fbfe4c7400`、001 `366d2cd2dd`、manifest 声明 `frozen_commit: d6b6343955`），而 manifest 只声明一个 ⇒ **run 的 commit 绑定不描述它的命令何时跑的**。

### Task 8 修订 1 留下的 Minor（不进循环，留给最终整支评审分诊）

- New Breakage 2：已登记的历史增补项**只由它自己声明的 `frozen_commit` 锚定**（S5 只查绑定**存在**，不查其**值**；冻结记录登记的是路径不是哈希）⇒ 该 manifest 的字节在外部无锚。R57 明确要求的就是「登记路径」，故按已知残余登记。
- New Breakage 4：`BASELINE_FREEZE.json` 被整体重排版（1 个语义字段的增补带出 ~66 行格式噪声）—— 可读性代价，非语义。
- re-review 的 out-of-scope：`audit_integrity_report.py` 今天因一个**既有** run（`20261119-168h-v410-r4/manifest.yaml:29`）的 YAML 解析崩溃；同文件 `:39` 从 `commands[]` 算 `exit0_claims` 而 schema-v1 的退出码在 `results[]` ⇒ 恒为 0（既有毛病）；`logs/fixtures/*.log` 被 `.gitignore:175 (*.log)` 命中 ⇒ **未跟踪、未被哈希绑定**，但 manifest 的 note 却在宣传它们；`audit/v4.1.0/neg/` 里仍有两个非夹具 helper（glob 今天安全，但「非 fx 的 helper 不得住在 neg/ 下」这条不变式只是隐式的）；实施者自列的 Minor 4 **已过时**（`neg_two_layer.sh:104-127` 确实是从 `results.jsonl` 用 Python 解析的）。

## Task 8 —— 修订 2 与修订 3（re-reviewer `af00a62` / `a17855a`）

- **修订 2**（F-A/F-B/F-C：证据 run 的保真度）：re-review → F-A/F-B/F-C **全 ADDRESSED**，但修订 2 自己引入 **1 Important + 3 Minor**。
  - **Important**：重建后的 manifest 把 016（R60 端到端）记成 `2026-11-25T00:05→00:29Z`，**晚于** manifest 自己的 `completed_at`(19:30:46Z) 与它自己的提交(19:30:56Z) ⇒ 记录声称一条命令在 manifest 写下时还没跑。真实窗口由三条独立证据推定（stdout 与 `gate_alpha_b556199c86_20261124_190447Z.log` 逐字节相同；该名来自 `check_stage.sh:353` 的 `date -u`；mtime 19:28:45Z = 起 + 23m58.64s）。
  - **Minor**：`commands[].timeout_sec` 记 5400 而实际跑的是 `timeout 10800`（照抄会重现两次 rc=124 撞墙）；notes[8] 指向**不存在的** `logs/fixtures/`；fx21 的 exit-4（INCONCLUSIVE）路径在夹具泳道里被记成 `fail`。
- **修订 3**（G-1…G-4）：**全 ADDRESSED，无新增 Critical/Important**。
  - G-1：016 时间窗改成 `2026-11-24T19:04:47Z → 19:28:46Z`（**从证据推定**，非编造）；两个字段不在 `evidence_hash` 覆盖范围内 ⇒ verify / S5 均未破。
  - G-2：`commands[]` 记实际跑的命令（`timeout 10800`），其余 16 条 `timeout_sec: null`（无上限）；`results[].exit_code` 一条未动。
  - G-3：七份逐道夹具日志**真的入库** `logs/fixtures/` 并由 `artifacts.fixture_logs` 的 sha256 绑定（artifacts 在 hash 覆盖内）。
  - G-4（**R62**）：夹具泳道对 **exit 4 一律记 `inconclusive`**，不看声明的期望值（exit 4 永远不是红）；汇总行**单列** `inconclusive=` 计数，不并进 `pass=` 桶。
- **R62**：第四种期望值 `inconclusive` 的语义定案 —— 它**不能**冒充红，也**不并进通过数**。
- **R63（⚠️ 判据 8 的口径必须改写 —— 这是 Task 9 的硬输入）**：夹具泳道在**发布树**上的可复现结果是 **RED=5/7 + INCONCLUSIVE=2/7**（退出码 3）；**冻结提交那一次**记的是 6/7 + 1/7。两次都不是 7/7。差异是**机器速度**（fx17 动过 Rust 源码后 composite 的 release 重建要 1–2 小时，fx15 的 7200s 预算这次没够上），**不是**断言被放松（`git diff b556199c86..HEAD -- audit/v4.1.0/neg/` 为空）。⇒ **判据 8 的结论对机器速度敏感**：Task 9 必须记**实测值 + 时间敏感性**，不得断言 7/7。

### Task 8 修订后的残余（进最终整支评审分诊）

- Minor：`logs/fixtures/` 的七份日志被 `evidence_hash` 封住，但 `audit_run.py verify` 只重算 `results[]` 的路径 ⇒ 日志被改不会被 verify 抓到（今天 7 份全对）。
- Minor：G-4 使「泳道绿」不再等于「每个夹具都证明了它的性质」—— 读汇总行的人必须看 `inconclusive=`。
- Minor：重跑那一次的逐道日志留在临时目录，未入库（无 manifest 注记指向它 ⇒ 无悬空引用）。
- Minor：报告称「仓里有 116 个已入库 `.log` 的先例」，实测 123。
- Minor：`commands[verify-handoff].cmd` 记的是临时 venv 路径（修订 2 引入，非本轮）。
- out-of-scope：证据 run 绑的是 `frozen_commit b556199c86`（G-4 之前的树）⇒ **发布版 `probe_neg_fixtures.py` 的执行证据是 `artifacts.rerun_after_g4` 那一对**，Task 9 要认这个。
- out-of-scope：`NEG_FIXTURE_PROGRAM_2026-11-05.md:78` 引用的旧汇总格式是历史引用（run 20261105-negfix-004），未失真但可能被误读为当前格式。

- Task 8: fix round 2/5 (3 addressed, 1 new Important + 3 Minor open; 提交 b5369fca18..d358289919)
- Task 8: fix round 3/5 (4 addressed, 0 open; 提交 b5369fca18..09ba360f6c)
- Task 8: complete (commits d67fd2264b..09ba360f6c, 3 fix rounds, review clean)。分支 tip `09ba360f6c`（+ 控制者提交的 `e007fe7114` 记忆文档）。

## Task 9 —— 实施（实施者 `a1da18c`，opus）

- **产物**：`docs/refactor/v4.1.0-c/takeover/` 下三个新文件 —— `V410-READINESS-VERDICT.json`（12 条判据逐条
  `id`/`criterion`/`command`/`observed`/`verdict`）、`R2-RELEASE-PACKAGE.md`、`V410-OWNER-RULING.md`（**裁决请求**，
  属主裁决位留空，实施者与控制者均未代签）；并按 R67 的例外更新了 `V410-READINESS-STATE.md`。
- **12 条判据实测判定：PASS 9 · FAIL 2（#6、#8）· WAIVED 1（#12）**。计划正文的「11 PASS + 1 WAIVED」是
  Task 7/10 之前的乐观预期，差额**恰好**来自 #6 与 #8，按 R63 不凑数。
  - **#6 FAIL（签署层）**：44 条有终态 ✓、双向差异表 ✓，但 **Oracle 副签 0 条**（38 条 `ASSERTION_UNREVIEWABLE`
    + 6 条 `JUDGED_PENDING_COUNTERSIGNATURE`；`assertion_review_performed_by` 是实施者本人）⇒ 需属主指认独立 Reviewer。
  - **#8 FAIL（证据层）**：夹具泳道 **RED=5/7 + INCONCLUSIVE=2/7（退出码 3）**；冻结提交那次 6/7 + 1/7；
    两次都不是 7/7；差异是机器速度（fx15 超 7200s 预算、fx18 覆盖率门禁在完整树上本就红），**不是**断言被放松。
- **R1 逐字节未变已核实**：生成前后 21 个文件的 sha256 清单 `diff` 无输出；`FREEZE.json` = `8363343618a6e659…`、
  R1 zip = `4f6c3ad01789b1a7…`，mtime 仍为 11月23。R2 生成在**兄弟目录** `sqlrustgo-v4.1.0-r2/`
  （R68 的路线），构建链在 `…/DSH-EXTERNAL-REVIEW-001/r2-build/`（`build_r2.sh` 唯一入口、幂等）。
- **R2 三套统计实测 71 条 / 等级分布 24-41-6**，`verify_r2_package.py` 23/23 PASS；`owner/approvals/` **空**，
  `record_approval.py` 一次未跑 ⇒ 没有代属主签署。
- **R2 的 `source_commit` 保留两个值**（包级 BASE + 逐条 `cd8cefe623` 采集树）并显式命名：逐条改成 BASE 等于
  谎称 47 条 REPORTED/NOT_RUN 已在 BASE 复跑。
- **DSH 仓无 remote** ⇒ 两个本地提交（`a031404` 发布 R2、`19f999e` 补齐 7 处残余 R1 身份残留）**无推送对象**。

### Task 9 的两条待属主裁决的口径冲突（实施者主动提出，未自行扩权）

- **判据 4**：`ADAPTER-MAPPING-PENDING.md` 清单非空（C01-15）⇒ 计划正文说 #4 不能判 PASS；但其可核方式
  （`--check` 退出 0 + 11 条变异臂全拒）全绿。实施者按**可核方式**判 PASS，冲突原样写进 `observed`/O1/裁决记录。
- **判据 1 的字面检查被并入反向**：`develop/v4.1.0` 在本任务期间并入了 `base/v4.1.0-frozen`
  （PR #5928，merge `ea285c5fda`）⇒ 远端 develop tip 现在是 BASE 的**后代**而非祖先；冻结时点的 develop tip
  `8cbde5b6d3` 仍是 BASE 的祖先 ✓。远端 `base/v4.1.0-frozen` 当时比本地 tip 落后 2 个提交（现已由控制者推送解决）。

## Task 9 —— 评审第 1 轮与修订

- **评审第 1 轮**（reviewer `ae3cce7`，sonnet）：**0 Critical / 1 Important / 8 Minor** → Needs fixes。
  - **Important**：判据 #4 判成 PASS，但计划正文（`task-9-brief.md:64`）「若非空则 A5 #4 不能判 PASS」的条件**成立**
    （`ADAPTER-MAPPING-PENDING.md` 有 `C01-15` 行，其 `approved_adr_refs` 为 `[]`）；且 A2.2 **R1**（「每个 task 的
    `approved_adr_refs` 至少 1 项」）被客观违反；`verify_handoff.py:383-384` 在列表为空时**直接返回** ⇒ `--check` 退出 0
    **不担保 R1**；校验器自身输出 `adr_approval_state: PENDING`（「the owner's ADR decision is PENDING, not verified」）。
    ⇒ 按「工具退出 0」判 PASS = 给判据**换指称对象**，R63 明令禁止。
  - 8 Minor：断链指针 ×3、`R2-RELEASE-PACKAGE.md:188` 一句假话（说 R2 未提交，其实已提交于 `a031404`/`19f999e`）、
    判据 5 的 `observed` 缺变异臂、判据 1 的 `command` 带占位符、头部缺「冻结时点」限定词、6/7 那次未钉住树、
    R2 构建链在无 remote 的仓里（可复现性缺口）、STATE 措辞过强。
- **修订 1**（实施者 `a1da18c`）：I-1 与 7 条 Minor 关闭、**M-4 半开**；新增 **3 Important + 8 Minor**。
  - 根因：生成器 `gen_verdict.py` 只活在 git-ignored 区 ⇒ 它里的**硬编码字符串**（"差额来自判据 6 与判据 8 两条"、变异臂数
    "11"）改了树、没改源 ⇒ 下一次重跑又把陈旧字符串写回裁决文件。
- **R69（控制者裁定 —— 除根因）**：把 `gen_verdict.py` **提交进本仓**（`docs/refactor/v4.1.0-c/takeover/gen_verdict.py`，
  不在 `MANIFEST_ROOTS` 内 ⇒ 不需重签 manifest），并把生成器里的文本**改成派生**（note 由实际 FAIL 集合生成、
  变异臂数由 `--check` 输出的 `len()` 得出、判据 8 的退出码由脚本退出码语义推出），再**用它重出**裁决 JSON。
  ⇒ 裁决文件从此**可从已提交的树复现**，这一类失真不再复发。
- **修订 2**：re-review（`a433efe`）→ **全部 findings ADDRESSED，无新增 Critical/Important**。
  - 改后判据计数：**PASS 8 · FAIL 3（#4、#6、#8）· WAIVED 1（#12）**。
  - 变异臂真实条数 = **10**（四个独立来源互证：`mutations.append` 源码、`--check` 实跑输出、`:618` 发出数、台账 `:104`）；
    原写「11 条」是**假数字**，三个文件已改。
  - 复核确认：R1 仍逐字节未变（21 文件 sha256 前后 `diff` 无输出）；`owner/approvals/` 仍空；属主裁决位仍留空。

### Task 9 的最终分层归因（交给属主）

| 判据 | 层 | 缺什么 | 谁能补 |
|---|---|---|---|
| **#4** | **映射层** | `ADAPTER-MAPPING-PENDING.md` 的 `C01-15` 未绑定 ADR ⇒ A2.2 R1 未满足（ADR 机器字段本身是绿的） | 属主裁决 O1（补映射 or 豁免） |
| **#6** | **签署层** | Oracle 副签 **0 条**（38 条 `ASSERTION_UNREVIEWABLE`） | 属主指认独立 Reviewer（R-T7-1） |
| **#8** | **证据层** | 夹具泳道 **RED=5/7 + INCONCLUSIVE=2/7**（fx15 超 7200s 预算、fx18 覆盖率门禁在完整树上本就红） | 属主裁决（提高预算 / 豁免 / 修导入的 RED 测试） |

### Task 9 修订 2 留下的残余（进最终整支评审分诊）

- Minor：裁决 JSON 的判据 1 记的「本地分支 tip」比 HEAD 落后一个提交（两段式生成的必然结果）⇒ 「重跑逐项一致」这句话
  需要加限定语「在生成它的那棵树上」。
- Minor：`gen_verdict.py` 在找不到负向夹具日志时会**静默去跑那条 1–2 小时的泳道**，且可能抛未捕获的 `FileNotFoundError`。
- Minor：派生出来的 note 仍写「**本次实测**的 FAIL」，而判据 8 的测量是**沿用** 2026-11-25 的实测值（逐条 `observed` 里已澄清）。
- out-of-scope：`V410-READINESS-STATE.md:185` 仍写「修订第 1 轮已处理 1 Important + 8 Minor」（未含第 2 轮）；
  Task 9 行只列 4 件产物（漏第 5 件 `gen_verdict.py`）；`R2-RELEASE-PACKAGE.md:216` 的表格有一行畸形。
- **两份 `verify_r2_package.py` 仍未同步**（DSH 那份 `75de8cce…` vs 本仓 `e8155879…`，内容等价、字节不同）⇒
  **需属主/控制者放行「同步这一个文件」**；R2 构建链仍无异地副本（DSH 仓无 remote）。
- 判据 8 的证据**无法从已提交的树单独复现**（日志 git-ignored，日期与树是生成器参数）⇒ 记为可复现性残余。

- Task 9: fix round 1/5 (8 addressed, 1 partial + 3 new Important + 8 Minor open; 提交 e2d98149d2..33f1c7ceb9)
- Task 9: fix round 2/5 (11 addressed, 0 Important open, 3 Minor + 3 out-of-scope open; 提交 33f1c7ceb9..a2d0e3ecec)

## 最终整支评审与修复波

- **最终整支评审**（reviewer `a86c3fd`，opus，范围 = 27 个计划自撰代码文件 + 179 份文档/数据清单 + 35 条 deferred 分诊）：
  **0 Critical / 9 Important / 11 Minor**。
  - **I-1** 判据 8 的「实测树」SHA 是**假的**（三份属主可见文档照抄生成器参数 `e2d98149d2…`；泳道实际跑在 `09ba360f6c`）
    ⇒ C5 缺陷类出现在**属主要签的那份文件**里。
  - **I-2** 判据 6 的判定只看一个字段、**不看是谁签的**：`rc6`、差异表存在性与表头、终态检查全只打印不判定
    ⇒ **属主指认 Reviewer 这个动作本身就会把 #6 翻成 PASS**（Task 7 评审点名的义务从未兑现）。
  - **I-3/#33** Oracle 判定的 `assertion_reviewed_commit` 指向 BASE，而实际读到的树是 `2923612f51`；
    且 Task 10 后来改了判定所依赖的 `TRANSACTION_INVARIANTS.md`。
  - **I-4** 生成器 docstring 声称「Nothing is hand-written」，实际 6 条判据的 `observed` 嵌着手写散文。
  - **I-5** R19 要求的披露（放弃 B3 的内容比对、靠 manifest 逐文件 sha256 顶上）在裁决记录里**缺失**。
  - **I-6** 本支的校验器与变异探针自测**一个都没进 CI**；台账 `:62` 的 deferred minor 把它派给 Task 8，Task 8 没兑现，
    也没有任何文档记着它还开着 ⇒ **12 条判据是一次性人工测量**。
  - **I-7** 记忆锚点的续跑指令指向 git-ignored 的生成器 ⇒ 新会话会把 R69 修掉的回归写回去。
  - **I-8** 唯一的豁免，其 `approved_by` 指向一份自称 **PROPOSED** 的设计稿 ⇒ 全支唯一一处治理声明是执行方自证的。
  - **I-9** 属主可见的 tip 与提交数陈述过时，**合并指令是错的**（说合 2 个提交，实际缺 7 个 —— 合了会丢掉裁决文件与生成器）。
  - deferred 分诊：35 条里 **5 条必修**（CI 接线、ADR 正文旧断言披露、`assertion_reviewed_at` 语义、`assertion_reviewed_commit` 绑定），
    22 条可作残余，8 条已不适用。
- **修复波**（实施者 `ad25207`）：11 条 finding **全 ADDRESSED**，无新增 Critical/Important；
  **判定结果不变**（PASS 8 · FAIL 3 · WAIVED 1）。三条关键改动：
  ① 判据 8 的树身份改成 `09ba360f6c…` 并**明写「推定/重构，不是观测值」**（泳道日志不含树身份）；
  ② 判据 6 改成**六个条件同时成立**才 PASS（`rc6==0` + 差异表 + 终态 + 属主授权名单 + 签署人 ≠ 实施者），
     **名单为空 ⇒ FAIL**；名单一授权 + 合法副签 ⇒ **自然变 PASS**（探针实测）；
  ③ 44 条 Oracle 判定的 `assertion_reviewed_commit` 重指 `2923612f51…`、`assertion_reviewed_at` 改名
     `assertion_judged_at`、新增 `assertion_signed_at`（44 条全 null），并**重签 manifest**（314 绑定文件、`16f87dff…`）。
- **回归清扫**（修复波**自己**引入的 8 条 Minor，其中 4 条是 Owner 可见文档里的**新不实陈述**）：8/8 ADDRESSED。
- **末轮两行更正**（C-1 判据 6 口径「四条」→「六条」与生成器对齐；C-2 STATE 的 Task 9 评审状态改成「已结束」）：2/2 ADDRESSED。

### 最终残余（交给属主，本轮不再修）

- 判据 8 的树身份**仍是推定**（reflog + mtime 两条独立证据很硬，但不是观测）⇒ 要变成观测需重跑泳道，或让脚本今后把 HEAD 写进日志。
- 判据 (c)/(d) 相对于当前矩阵**未重新求值**（Task 10 改了 38 条 `invariant_ids`）⇒ 已如实登记为受影响。
- **CI 仍未接线** ⇒ 12 条判据仍是一次性人工测量，2026-11-25 之后的回归可能无人察觉（属主专属决策，C1）。
- 两份 `verify_r2_package.py` 仍未同步（DSH 那份 `75de8cce…` vs 本仓 `e8155879…`，内容等价、字节不同）；
  R2 构建链无异地副本（DSH 仓无 remote）⇒ 需属主放行「同步这一个文件」并决定是否给 DSH 仓配 remote。
- `ORACLE-REVIEW-LOG-T1.md` 仍在 manifest 哈希范围之外（R-T7-3）⇒ 属主可见的 Oracle 叙述仍无收据绑定。
- 生成器 note 里的树 SHA 与三个时刻**仍是硬编码字面量**（已加适用范围限定，未参数化）。
- 计划正文与设计稿仍写 `assertion_reviewed_at` 与旧绑定值（不在允许改动清单内，未动）；台账由控制者同步。
- 「25 条待裁决项」沿用上一波修复波 §7 的计数，回归清扫未重数 ⇒ 属主按 §3 逐条数可能对不上。

### 收尾状态

- 9 个计划任务全部完成；每条任务都经过独立评审（Task 8 三轮修订、Task 9 两轮修订 + 修复波 + 回归清扫）。
- **最终判定：PASS 8 · FAIL 3（#4 映射层 / #6 签署层 / #8 证据层）· WAIVED 1（#12）** ⇒ **不放行**。
- 剩余待办**全部是属主裁决**：`docs/refactor/v4.1.0-c/takeover/V410-OWNER-RULING.md` §4 的裁决位（留空）。

## Task 9 —— 收尾（控制者授权的漂移同步 + 台账更正）

- **R70（控制者裁定 —— 漂移同步）**：实施者请求控制者放行「把仓内那份 `verify_r2_package.py` 同步回 DSH 仓」。
  **裁定：放行**（属主授权范围内 —— O8「发 R2，R1 不动」，且这是**只读校验器**的同步，不是 R2 包内容变更）。
  **执行**：仓内那份**复制**到 DSH 的 `r2-build/` 与 `sqlrustgo-v4.1.0-r2/` ⇒ **三处字节完全相同**；
  两家自解析都正确（三处实跑 **23 PASS / 0 FAIL**）；R1 的 `package/` 未触碰；R2 的 `package/` 与 zip
  **不含**该校验器（zip 21 条目、检索 0 命中）⇒ **R2 包哈希未变**。
  DSH 仓提交：`08742fc` → `0964dda681a8`（该仓**无 remote**，只有本地提交）。
- **R71（哈希记录纪律 —— 一条新规则）**：控制者自己犯过一次：**在关于某个文件的文档里记录那个文件的 sha256**，
  而该文件的头注释是它字节的一部分 ⇒ 哈希随下一次编辑立即过期（实测：同步前 `e8155879…` → 改注释后 `5ebe694d…`
  → 再改后 `fa9f1978…`，两次都陈旧）。
  **裁定**：**不在任何文档里记录一个「其字节包含自己的文档注释」的文件的 sha256**；
  改成**可复算的校验入口**（`diff` + `shasum` 三条路径，当场可验）。已落到：
  校验器自身的头注释、`R2-RELEASE-PACKAGE.md` §9.1、`V410-OWNER-RULING.md` §3.1.5。
  **代价**：读者不能从文档里直接看到一个哈希，必须跑一条命令 —— 这是刻意的（假哈希比没有哈希更坏）。
- **实施者复核时改掉了控制者写错的一处**：`r2-build/` 那份校验器**并不**自解析到自己的目录（它下面没有 `package/`），
  而是落到兜底的 DSH 默认位置 ⇒ 注释已按事实改写。**落点相同、机制不同** —— 这正是「以实测为准」的例证。
- **台账更正条目（本节即为更正）**：
  ① 上文「两份 `verify_r2_package.py` 仍未同步」**已关闭**（见 R70）；
  ② **字段改名更正**：Task 7 记录的 `assertion_reviewed_at` 在修复波里已改名为 **`assertion_judged_at`**，
     并新增 `assertion_signed_at`（44 条全 null）；`assertion_reviewed_commit` 已由 BASE 重指到 **`2923612f51…`**
     （实际读到的树）。台账正文里仍写旧名与旧绑定值 —— **那是历史记录，不改写**；以本条与
     `ORACLE-REVIEW-LOG-T1.md` §11 为准。
  ③ 计划正文与设计稿仍写 `assertion_reviewed_at` 与旧绑定值（不在允许改动清单内，未动）。
