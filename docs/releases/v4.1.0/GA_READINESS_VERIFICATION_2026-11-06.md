# 4.1.0 GA 就绪核实（2026-11-06）

> 本文按仓内规则**逐项核实**「从 draft 到 GA 的全部过程」，目的是回答一个问题：
> **现在能不能按规则重新发布 GA？** 结论先说：**不能** —— 而且不能靠「再推几个 PR」解决，
> 有 5 件**必须由属主完成**的事卡住。下面每一条都带现场证据。

## 0. 结论

| # | 结论 | 依据 |
|---|---|---|
| 1 | **阶段仍是 ALPHA**，从未合法进入 BETA / RC ⇒ 「直接发 GA」不是规则允许的路径 | `docs/releases/v4.1.0/STAGE.yaml`：`current_stage: "ALPHA"`（`last_transition` = DRAFT→ALPHA 重新进入，2026-10-06）；`AGENTS.md` §2.7 阶段跃迁必须人批准；GA 阶段的 `branch_pattern` 是 `main`/`release/v*`/`hotfix/v*`（`carrier_enforced: true`）⇒ **GA 必须由 main / release 分支承载**，而 main 封存到 GA |
| 2 | **没有任何门禁真正执行过**：CI 侧每个 run 都是 `completed/failure` 且 **job 数 = 0** | 实测：run 10956（合并 #5612 的 push，08:29:21Z 起 18 秒完成）、10947–10959 同款；runner 1735/1736/1737 均 online；同样形态在 2026-11-05 的 run 10884/10885 就存在 ⇒ 不是本轮引入 |
| 3 | **GA 门禁清单**（`STAGE_CONFIG.yaml` 的 `stages.GA.required_gates`：34 道门禁 + 5 条 cargo 命令）本轮**全量实跑**，结果见 §3 | 归档 run `20261106-ga-gates-001`（本轮实跑，结论随门禁逐条落库） |
| 4 | 有一个**未闭合的 P0 产品缺陷**（#5584 崩溃边界）⇒ GA 清单「E2E / 回归测试全部通过」不成立 | 看板 ENG-13；归档 run `20261106-p22fl1-001` 的 `crash-wal-index` 仍红 |
| 5 | **双血统证据链断在 deepseek** ⇒ 看板 IL-2 的「≥47 条双血统」达不到（现值 0） | 属主裁定「两专家是 deepseek 与 chatGPT」；deepseek 席位 HTTP 500（EX-5） |
| 6 | 上一版 v4.0.0 的 GA 判定**已被作废**（从未合法 GA）⇒ 「上一次发布过 GA」不能拿 v4.0.0 充数 | `docs/releases/v4.0.0/STAGE.yaml`：`ga_promotion_status.verdict = "REVOKED 2026-09-30"`、`current_stage: "DRAFT"` |

## 1. 阶段阶梯（规则的原文与现场）

| 阶段 | 规则要求 | 现场 |
|---|---|---|
| DRAFT | — | 已过（DRAFT→ALPHA 2026-09-29，2026-10-06 重新进入 ALPHA） |
| ALPHA | `STAGE_CONFIG.yaml` 的 `stages.ALPHA` 门禁全绿；`ALPHA→BETA` 需**人批准** | **当前阶段**；ALPHA 门禁（`check_stage.sh --stage ALPHA`）本轮随 GA 门禁批一起实跑 |
| BETA | `stages.BETA` 门禁全绿；**人批准** | **从未进入** |
| RC | `stages.RC` 门禁全绿；**人批准**；RC 期间禁功能新增、只允许 bugfix/文档 | **从未进入** |
| GA | `stages.GA` 门禁全绿（34 门禁 + 5 cargo 命令）；`current_stage` 必须 GA；发布文件 5 份齐备；由 `main`/`release/v*` 承载；tag + 发布目录 + 宣布 | 门禁未全绿（§3）；`current_stage` 仍是 ALPHA |

证据出处：ALPHA 阶段门禁的现场实跑见归档 run `20261106-ga5594-003`（含 `gate-executability`、
`cargo fmt --check`、`cargo clippy --all-targets -D warnings`、`honesty.py check`、
`gate_registry.py check`、看板一致性、CI checkout 一致性）；GA 门禁清单的逐条实跑见
`20261106-ga-gates-001`。上表的「规则要求」列取自 `docs/governance/STAGE_CONFIG.yaml` 与
`AGENTS.md`（仓库根）§2.7，「现场」列取自 `docs/releases/v4.1.0/STAGE.yaml`。

## 2. GA 要求的发布文件（`required_files`）—— 全部在位

| 文件 | 状态 |
|---|---|
| `docs/releases/v4.1.0/STAGE.yaml` | ✅ 在位（但 `current_stage` 仍是 ALPHA ⇒ 阶段未到 GA） |
| `docs/releases/v4.1.0/RELEASE_NOTES.md` | ✅ 在位 |
| `docs/releases/v4.1.0/GA_GATE_REPORT.md` | ✅ 在位（**内容需按本轮实跑刷新**） |
| `docs/releases/v4.1.0/GA_RELEASE_TIMELINE.md` | ✅ 在位 |
| `docs/releases/v4.1.0/CHANGELOG.md` | ✅ 在位 |

证据出处：归档 run `20261106-ga5594-003` 的 `ga-required-files` 命令（逐份 `test -f` 断言，
任一缺失即 rc=1）。`current_stage` 字段取自 `docs/releases/v4.1.0/STAGE.yaml`。

## 3. GA 门禁清单实跑

见归档 run `20261106-ga-gates-001`（逐条 rc 与日志）与本文 §6 的清单。

## 4. 关门原则（`RC_TO_GA_GATE_CHECKLIST.md` §0）

| 原则 | 状态 | 依据 |
|---|---|---|
| 功能冻结（RC 期间禁功能新增） | **不适用/未到 RC** | 当前 ALPHA |
| 修改限制（只允许 bugfix / 文档修复） | **本轮已破例**：合并了 F-08 列查找（功能修复）与三态自检脚本（工具） | 这是 ALPHA 阶段的合法修改，但**进入 RC 后必须冻结** |
| 流程合规（所有修改走 PR） | ✅ | 本会话 12 个 PR 全部走 Gitea PR + 归档 run + 自审评论（#5608–#5619；证据 run 例：`20261106-mcode-001`、`20261106-readbackmerge-003`、`20261106-ga5594-003`） |
| 质量保证（必须通过全部 CI） | ❌ | CI 侧 0 job（§0 第 2 条）⇒ 「全部 CI 通过」**无法取证** |

## 5. open ISSUE 的整改状态（本轮实测）

| ISSUE | 状态 | 说明 |
|---|---|---|
| #5601（BIN-1 binary 假成功） | ✅ **已闭合** | 修复 + 两条回归 + probe `false_success_count 3→0` + 反向变异证据 + 对方独立复测 ⇒ 验收 5 条全满足 |
| #5604（DRIFT-2 看板/工具漂移） | ✅ **已闭合** | P2-1/P2-2 已完成、P1-4 已迁待复核，三个工具现值全绿 ⇒ 漂移消除 |
| #5594（执行点门禁把注释当调用） | ✅ **已闭合**（本批） | 行尾注释里的门禁路径不再算执行点；反向变异：`--selftest` D2 臂变红 ⇒ 矩阵假通过被拦住 |
| #5598（P1-3 23 道门禁无夹具 + 2 处 fail-open） | ⏳ 待裁定 + 待做 | 裁定问题 `audit/v4.1.0/adjudications/owner-5598-p1-3-failopen.md` 已写好；两处 fail-open 的修法待双血统裁定 |
| #5603（LEDGER-2 claims 台账三套并存） | ⏳ 待裁定 | 裁定问题 `owner-5603-claims-ledger-ssot.md` 已写好 |
| #5605（KAN-2 看板写入口径） | ⏳ 待裁定 | 裁定问题 `owner-5605-kanban-writeback.md` 已写好；合并口径已先行落 `BRANCH_GOVERNANCE.md` §7.4 |
| #5596（7 条审计记录不可复核） | ⏳ 待裁定（属主） | 裁定问题 `owner-5596-audit-records.md` 已写好 |
| #5584（崩溃测试边界） | ⏳ 归属待裁定 | 认领在另一条线且长期无动作；裁定问题 `owner-5584-ownership-and-fix.md` 已写好 |
| #5597（CI 执行面） | ❌ **属主动作** | runner-03 缺 `sqlrustgo-runner:v1` 镜像 ⇒ job 从未进入执行 |
| #5595（83 条不会运行的门禁交代） | ⏳ 待裁定 | 34 条归档候选 + principles 三份并存，需属主裁 |

闭合三行的证据出处：#5601 见归档 run `20261106-bin1-002`（binary 臂 DDL 明确报错的两条回归 +
`binmode_probe.py` 的 `false_success_count` 计数）与 `20261106-lineage-001`；#5604 见
`20261106-board3-001`（看板行状态与三个工具现值同源）；#5594 见 `20261106-ga5594-003`
（`gate_executability.py --selftest` 的 A/B/C/D/**D2**/E/E2 臂 + 门禁本体 NO-EXECUTOR 0）。

**已备好、等 deepseek 恢复即裁的 8 份问题**：`audit/v4.1.0/adjudications/owner-*.md`
（5601 binary 口径 / 5603 claims 台账 / 5605 看板写入口径 / 5584 归属与修法 /
HO-2 附录 A / 10 条分支批次优先级 / 5598 两处 fail-open / 5596 审计记录）。

## 6. **只有属主必须完成**的事（Agent 不得代做）

| # | 动作 | 为什么必须属主做 | 解锁什么 |
|---|---|---|---|
| 1 | **deepseek 席位重新登录**（`http://127.0.0.1:9655/v1/chat/completions` HTTP 500，上游 token 过期） | 凭据动作（看板 EX-5 明写「Agent 不得代做凭据动作」）；且属主已指定两专家是 deepseek + chatGPT | 8 份待裁定问题 + 台账双血统（IL-2 的 47 条） |
| 2 | **恢复 CI 执行面**（runner-03 缺 `sqlrustgo-runner:v1` 镜像；回退 docker.io 被拒） | 需要镜像/基础设施权限 | GA 门禁里「所有 CI Job 通过」才有取证；否则 GA 无法按规则闭环 |
| 3 | **阶段跃迁批准**：ALPHA→BETA→RC→GA 逐级（`AGENTS.md` §2.7 每次跃迁需人批准） | 规则要求人批准 | 合法路径 |
| 4 | **启用分支保护 / merge 拦截**（P0-3；仓库当前**无**分支保护 ⇒ 两线 PR 都是无 review 直接合的） | Gitea 仓库设置 | GA 清单「无 pending review / 无 failing check」才能机械成立 |
| 5 | **#5584 的 claim 释放或移交**（另一条线长期未动） | 多 Agent 归属规则（先认领者拥有） | 崩溃边界缺陷有人修 |
| 6 | **GA 的不可变动作**：tag `v4.1.0`、生成发布目录、把 release 分支合入 `main`、发布公告 | 清单 §5/§6 的冻结与宣布动作 | GA 真正落地 |

## 7. Agent 侧（不需要属主动作）可继续推进的

1. 8 份待裁定问题一旦 deepseek 恢复即跑双血统裁定，并把裁定写回看板/ISSUE。
2. 10 条「需要专门批次」的分支（session-context-5057 / 5262-b2 / omp-ignore-registry-5098 /
   ci-* / 4597-chaos / e5-r9-r11 / rustfmt-4597 / 5320-5327 / omp-engine-phase3 /
   branch-batch-priority）—— 详见 `docs/releases/v4.1.0/BRANCH_MERGE_VERDICTS_2026-11-06.md` §2.5
   的逐条判定与依据（其中 4 条的有效增量本轮已取）。
3. #5598 的两处 fail-open：等裁定后修（改门禁判据需反向变异证据）。
4. GA 门禁清单里**与 CI 无关**的门禁继续逐条实跑，把红的修掉（本轮已修：执行点门禁 #5594、
   clippy 4 处 lint、rustfmt 欠账、目录策略唯一新违规、bench 目录归位、F-08 列查找）。
5. 台账：待 deepseek 恢复后跑一次批量 deepseek 血统分诊（104 条）⇒ IL-2 的 47 条达标。

## 8. 本文的事实来源

- `docs/releases/v4.1.0/BRANCH_MERGE_VERDICTS_2026-11-06.md`（365 个分支的逐条裁定）
- 归档 run：`20261106-ci-002` / `20261106-mcode-001` / `20261106-readbackmerge-003` /
  `20261106-p22fl1-001` / `20261106-lineage-001` / `20261106-ga5594-001` /
  `20261106-board3-001` / `20261106-branchbatch-001`
- 现场量测：Gitea Actions API（runs / runners）、`gov_probe.py metrics/issues`、
  `kanban_render.py counts`、`verify_ledger.py`、`probe_ledger_freshness.py`、
  `gate_executability.py --selftest`、`cargo clippy --workspace --all-features --all-targets -- -D warnings`
