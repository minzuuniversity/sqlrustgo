# v4.1.0 整备 —— 我替属主做的全部裁决（索引）

> 本文件是 `V410-READINESS-LEDGER.md` 里全部 `Ruling:` 行的**逐条索引**（47 条），
> 全文与每条的「若错了要付什么代价」在台账原处。属主读这一份就能看到
> 我替属主做的每一个决定。

> 生成于 2026-11-25。

ruling (R13): `cd8cefe623` 的 state 改 `already-present` —— 计划写的「唯一真 delta」被事实推翻，账本必须诚实

ruling (R14): 计数/枚举断言一律钉 frozen `base_sha`；HEAD 侧改 presence/subset —— 否则 Task 4 移 ADR-011 会让冻结校验器变红

ruling (R15): B4 的 applied 分支改 blob-ID 内容比对，不再用 commit-message 字符串匹配

ruling (R16): 跳过项单独计数并进 verdict；加 `--strict` —— 校验器必须能区分「核过了」与「没核」

ruling (R17): `verify_baseline.py` 的 B3 改为路径存在性断言（导入路径集合 ⊆ HEAD 树路径集合），不再要求 blob 相同 —— freeze 记录的职责是冻结*身份*，不是冻结*内容*；内容级保证由下游各任务的校验器（`verify_sha_scope.py` / `verify_matrix.py` / `verify_handoff.py`）承担。这与 Task 1 的 R14（HEAD 侧改 presence-only）同向

concern C3 → ruling (R18): DAG 的第 19 处 `artifact_context.source_code_base_sha` = `e9ff21fca3` 归类为来源/provenance 绑定，保持原值不改 —— 它记录外部 test-factory 包是在哪个源码基线上产生的，与本次实施基线无关，改它等于伪造来源记录。`verify_sha_scope.py` 应新增一条断言把它固定住

ruling (R19): 接受「B3 不再对 179 个导入路径做内容比对」这一覆盖损失 —— 导入材料的内容完整性由 `10_HANDOFF_MANIFEST.json` 的逐文件 sha256 + `verify_handoff.py --check` 复算保证（当前 233 绑定文件、0 漂移），B3 的职责是身份与在场而非内容。此项须在 Task 9 的放行裁决记录中写明

ruling (R23): `BASELINE_FREEZE.json` 必须显式纳入 manifest 覆盖集（计划把它连同 takeover 顶层一起排除是错的 —— 它是 base identity 的唯一来源）

ruling (R24): tier 判定改整词精确匹配，未知 token → PROPOSED（fail-closed），杜绝 `NOT APPROVED` 被判为 APPROVED

ruling (R25): `Approvals:` 必须真被解析，且非 ADR token 必须拒绝而非静默过滤；正则改为行内任意位置匹配

ruling (R26): 变异臂改用合成 tier map（003 → APPROVED_DIRECTION）自包含地测档位越权分支

ruling (R27): HARA digest 锚点必须是 `--finalize` 无法重写的（取自冻结基线提交的内容）

ruling (R28): ADR 目录规模改为阶段绑定的期望状态（记录在已签署/已提交处），不用 `{10,11}` 常量带

ruling (R30): ADR 正文的 `Approval:` 绑定指向 BASE 下的 ADR digest（reviewer 复核 10 个 digest 与 BASE 字节逐一对上；ADR-011 的 digest 与 runtime-contract 包自记的 `adr_sha256` 一致），不指向 ledger 绑定的 `45da6968ae`/`b681b74ca3` —— 二者字节等价，但 ADR 正文未声明等价，属追溯性缺口

ruling (R31): ADR 正文的纯元数据编辑（`Status:`/`Approvals:`/`Approval:`/`conflict_resolution` 行）不触发 C6 的「内容变更须重审」—— 实质 ADR 正文未动；若将来改动「选择与否决」「数据与身份契约」等实质条款则必须重审

ruling (R32): RED 判定双信号（矩阵 `source.claimed_status` + pilot `rerun_status`）；pilot 记录缺失 → `DriverError` fail-closed；`status_mapping()` 断言 FAILED_RED/COVERED_RED → KNOWN_FAIL

ruling (R33): bespoke 探针走矩阵级 `probe_registry`；已绑定则执行，未绑定则 NOT_RUN（理由准确）；专有 RED 身份不全时只能 NOT_RUN

ruling (R34): `PILOT-T5-101` 携带 4 个可复原要素 + `source_sha256: null` + `identity_gap` 显式标记 → 缺口可审计而非不可见；状态分歧升级为属主裁决项

ruling (R35): 驱动的映射副本必须与矩阵 `status_vocabulary_mapping` 块交叉校验，不一致即 `DriverError`（消除第二份规范副本）

ruling (R36): M4 语料改读登记表 id 列（结构化），不再正则扫散文 —— 原 review 的 `ADR-010` 实例经复核不成立（Python `\b` 在 CJK 与 "A" 之间不触发），但类别缺陷（语料是偶然的而非结构性的）已修

ruling (R37): 两条 NOT_RUN 行的归因改为「pilot 用一次未提交的本地 `[[test]]` 注册让它们可执行；BASE 里没有该注册 → 基线装配缺口」；verdict 仍为 `NOT_RUN`（非新缺陷、非回归）

ruling (R38): 42 条 PASS 与 DSH-B 验收口径（「assembly=UNVERIFIED 的不得记 PASS」）的冲突必须写入 `L1-DIFF` §五-5、§六 判定表的备注列、以及 DSH-B 规则文件本身 —— 两种读法并列陈述，由属主裁定，不得被差异表默默吸收

ruling (R39): C5 绑定记录必须区分三个事实 —— 执行的 SHA+tree（由工厂自记的 `base_sha` 推导）、产物提交的 SHA（隔离到独立块）、排序证据（祖先关系证明）；只读 JSON 的读者也必须能分辨

ruling (R40): 无人可证明属于属主授权的 Reviewer 池 → 副签必须来自属主；因此撤回全部签署、把 Oracle 步骤记 BLOCKED 并登记残余，不以任何形式留下会被 Task 9 自动判据读成有效签名的字段

ruling (R41): 「澄清」条款是我在 dispatch 提示词里写错的措辞（brief/plan/design 三处均为绝对前提）→ 第 2 段作业是在错误指示下进行的，必须在记录中如实写明并撤回其签署

ruling (R42): 不可判定的 38 条不得为了让数字好看而签署；诚实的 `ASSERTION_UNREVIEWABLE` 优于完整的签名外观

ruling (R43): INV-10 与 INV-12 的唯一「覆盖」来自标签派生而非源码派生 → 撤回，宁可留下 13 条零覆盖也不保留无法复核的覆盖

ruling (R44): 断言修复只允许改断言文本与映射，不得改测试体逻辑——R3 两条测试体保持原样，缺失能力登记为 GAP-13 而非补写实现

ruling (R45): `invariant_ids` 必须是逐用例、从测试源码派生的；按 feature 分桶的标签不再是可接受的映射来源

ruling (R46): 13 条零覆盖的不变量进入 Task 9 的判据与 R2 包的缺口清单 —— 这是「可以开始重构」与「可以放行」之间的实质差距的量化

R47（Step 2 的派生口径 —— 以夹具实物为准，具体到哪份实物）：`gov_probe.NEG_FIXTURE_RE` 现在按门禁脚本自身源码里的关键字（`selftest|self_test|negative|反例|反向变异|canary|负向`）判定 `neg_fixture`，语义是「门禁自带 selftest」。而 `GATE_CATALOG.jsonl` 的人工 `effect.reason` 说的是「由外部夹具 fxNN 证明（kind=mutation-verified）」——两个字段在说两件事，于是 7 行出现 `neg_fixture=false` + `effect.catches` 非空的表面矛盾。仓库自己早已诊断过同一问题（Gitea issue #5598 正文：「关键字统计会把「提到过反例」当成「有反例夹具」。台账按门禁逐行记账才是可核对的口径」），且 `docs/releases/v4.1.0/NEG_FIXTURES_v4.1.0.jsonl`（57 行）就是那份台账。

R48（要造几道夹具 —— 计划说 4，实际是 6）：计划 Step 3 称「已有 fx02/fx08/fx14 瞄准 ALPHA 阶段门禁」，这不成立：`fx02 → check_anti_ignore_gate.sh`（确为 ALPHA）；`fx08 → check_alpha_entry_v4.10.sh`（`stages=[]`）；`fx14 → check_stage.sh`（`stages=[]`）。只有 1/7 已有夹具，需新造 6 道：`check_alpha_v410.sh` / `check_arch_invariants.sh` / `check_arch3_no_bypass.sh` / `check_coverage_v312.sh` / `check_sql_corpus_gate.sh` / `check_g1_tpch_baseline.sh`。计划正文「其余 4 道」是笔误。

R49（每道夹具的缺陷规格不新造）：这 6 道的缺陷规格仓库已写好——`NEG_FIXTURES_v4.1.0.jsonl` 里对应行的 `target` 字段就是台账给的关闭条件（如「删 docs/releases/v4.1.0/STAGE.yaml 后 composite 必须变红」「新增一处绕过 ExecutionEngine 的 DML 调用 ⇒ 必须变红」「把某个 crate 的覆盖率表替换成低于阈值的值 ⇒ 必须变红」）。实施者按 `target` 施工，不得自创另一套规格。

R50（timeout 不得当成变红）：仓库既有规则（issue #5598 关闭计划）：夹具泳道必须「预热 target + 每夹具预算 30 分钟 + 超时即报 INCONCLUSIVE，不允许把 timeout 的 124 当成「门禁变红」」。`verify_neg_fixtures.sh` 必须区分 RED 与 INCONCLUSIVE，不得凑成 7/7；台账对应行在夹具未成立时保持 `pending-fixture`，不得为对账写成 `mutation-verified`。

R51（O17 裁决 —— ALPHA 执行面用哪套工具）：CI 执行点用 `check_stage.sh`（两态）。理由：(a) `check_stage.sh` 是 SSOT 的唯一 driver，BETA/RC/GA 都用它 → 单一执行面（C2）；(b) 三态 runner 的执行接线被 design §A4.1 G2 明确移交 kanban IL-7（属「已知红分支」策略决定，不在 A4 范围内）。

R52（C8 与 ALPHA 执行点的触发方式 —— 计划 Step 5 与 C8 冲突，裁定如下）：计划 Step 5 提的

R53（不新建第二套执行面）：ALPHA 执行点只有一处（新 workflow）。不在 ci.yml 里另加 ALPHA step —— 两处执行点 = C2「第二套 harness」的变体。

R54（两层 RED 的可核断言）：两层必须来自同一次 `run_gates.py execute` 产生的同一份 `results.jsonl`：

R55（新夹具的落点与登记）：新夹具落 `audit/v4.1.0/neg/fxNN-<gate>-<缺陷类型>.sh`（沿用现有 fxNN 编号续到 fx21），形状沿用既有夹具约定：`<scratch-tree>` 作参数 → 注入故障 → 跑真实门禁 → 断言 rc 非零 → 还原 scratch → rc 0 = FIXTURE-PASS / rc 3 = FIXTURE-FAIL。每个新夹具必须在 `docs/releases/v4.1.0/NEG_FIXTURES_v4.1.0.jsonl` 登记对应行（`kind`/`fixture`/`fixture_id`/`evidence_run`/`note`），且 `python3 audit/v4.1.0/probe_neg_fixtures.py check` 必须仍然通过。

R56（不得放宽门禁）：本任务不新增门禁、不改 `STAGE_CONFIG.yaml` 的任何阶段清单/阈值、不改 168 分母。ALPHA 的 10 条 required_gates 一个不动。

R57（Important 6 的解法 —— 恢复文件名，扩冻结集，不绕 glob）：`verify_sha_scope.py` 的 S5 设计意图是「历史记录不得被改写」，它不是「run 目录不得增加」。S5 自己的 docstring 写明两条合法路径：「a *new* audit run manifest must be added to the freeze record (or the gate must be re-ruled), it must not silently widen the set」——但实现只支持第二条（`expected` 由 `git ls-tree` 在 BASE 上派生，不读冻结记录）。

R58（Important 3/4 —— 机器可读的声明必须等于证据建立的东西）：台账行与 `GATE_CATALOG.jsonl` 的 `effect.catches` 只允许写夹具真正证明的性质；夹具同时证明反面（该门禁看不见某类故障）时，那必须写成 known hole，不能写成 `catches`。这与 Task 7 的 R42、Task 8 的 R50 同一精神。

R59（Important 2 —— 台账是生成物，生成器必须能复现它）：`NEG_FIXTURES_v4.1.0.jsonl` 由 `gen_neg_ledger.py` 生成；生成器与产物不一致 = 一次重跑就能把本轮成果静默抹掉。生成器必须更新到能复现当前产物，且它的输出必须与提交进来的台账逐字节一致。

R60（Important 5 —— 端到端跑一次，预算有界）：必须实跑一次 `bash scripts/gate/check_stage.sh --version v4.1.0 --stage ALPHA`（热 target，期望非零退出）并把输出挂进证据 run。若在有界预算内跑不完，如实报 ⚠️ 给 Task 9，不得用 dry-run 冒充。

R61（第四种期望值 `inconclusive` —— 接受）：re-reviewer 用两条实跑臂证明它不能冒充红（`check` 的 `FIXTURE-EXPECT-MISMATCH`、`fixtures` 要求恰好 exit 4、`known_hole` 必须是 `green-characterization`），覆盖率门禁那行仍是 `pending-fixture` 且 `effect.catches: []` ⇒ 没有任何机器可读声明依赖未建立的性质。唯一的松弛：`fixtures` 泳道把 exit 4 计入 `pass=` 桶（汇总行会读成「19 个夹具证明了它的性质」而其中一个什么都没证明）—— 建议给该泳道单列 `inconclusive=` 计数器（非阻塞，留给最终分诊）。

R63（⚠️ 判据 8 的口径必须改写 —— 这是 Task 9 的硬输入）：夹具泳道在发布树上的可复现结果是 RED=5/7 + INCONCLUSIVE=2/7（退出码 3）；冻结提交那一次记的是 6/7 + 1/7。两次都不是 7/7。差异是机器速度（fx17 动过 Rust 源码后 composite 的 release 重建要 1–2 小时，fx15 的 7200s 预算这次没够上），不是断言被放松（`git diff b556199c86..HEAD -- audit/v4.1.0/neg/` 为空）。⇒ 判据 8 的结论对机器速度敏感：Task 9 必须记实测值 + 时间敏感性，不得断言 7/7。

R69（控制者裁定 —— 除根因）：把 `gen_verdict.py` 提交进本仓（`docs/refactor/v4.1.0-c/takeover/gen_verdict.py`，