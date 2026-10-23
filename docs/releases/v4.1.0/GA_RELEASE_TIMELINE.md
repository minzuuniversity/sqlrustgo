# v4.1.0 — GA 发布时间线

> **基线**：`develop/v4.1.0` @ `2406559ac09ecf821566d11bf4a3cc1ce4f4dbb8`
> **provenance:** generated_at=2026-10-23, branch=docs/ga-rc-artifacts-batch32,
> commit=`2406559ac09ecf821566d11bf4a3cc1ce4f4dbb8`, machine=Mac-mini-7.local (arm64, macOS 27.0.1),
> identity=openclaw-mcode, source_repo=openclaw/sqlrustgo,
> policy=Anti-Fabrication-Policy-v1.0
> **gate_policy_eval_id**：`v410-ga-release-timeline-001`
>
> **当前阶段**：`ALPHA`（`docs/releases/v4.1.0/STAGE.yaml` → `current_stage: "ALPHA"`）
> **GA 承载分支**：`release/v4.1.0`（该分支**尚不存在**；GA **不得**在
> `develop/v4.1.0` 上声明 —— 见 §2.6）
> **本文件是**：`docs/governance/STAGE_CONFIG.yaml` → `stages.GA.required_files`
> 与 `stages.GA.doc_artifacts` 要求的产物。
>
> ⚠️ **本文件不构成 GA 晋升声明。** 补全本文件只是满足 GA 阶段的
> `required_files` 清单；阶段是否晋升由**门禁实跑结果 + 人类签字**决定，
> **不由文件是否存在决定**。`STAGE_CONFIG.yaml` →
> `transitions.RC_to_GA.requires_human_approval: true`。
> 本文件由 `openclaw-mcode` 自动生成，**晋级审批由人类架构师执行**。

---

## 1. 已发生的事实时间线

> 全部条目取自 `git log` 实测（commit 与日期均为仓库真实记录），不是推演。
> 分支基线：`develop/v4.1.0` @ `2406559ac09ecf821566d11bf4a3cc1ce4f4dbb8`。

| 日期 | 事件 | commit（短 SHA + 主题） | 证据 |
|---|---|---|---|
| 2026-09-19 | v4.0.0 GA FINAL 提升到 `main`（tag `v4.0.0-final`） | `558aa61385` — Merge 'v4.0.0: GA FINAL — promote develop/v4.0.0 to main' (#3787) | `docs/releases/v4.0.0/STAGE.yaml` |
| 2026-09-19 | V400-09 168h SOAK 启动（**后未跑完，见 §2.1**） | —（非代码提交） | `docs/releases/v4.1.0/STAGE.yaml` → `known_gaps.V400_09_soak` |
| 2026-09-20 | v4.0.0 `GA_GATE_REPORT.md` 判定 **GA CONDITIONAL PASS** | —（文档提交，见 v4.0.0 线） | `docs/releases/v4.0.0/GA_GATE_REPORT.md` |
| 2026-09-21 | 5 远端收敛报告 | `136f232643` — docs(v4.0.0): SYNC_AUDIT_v4.1.0_2026-09-20 | `docs/releases/v4.1.0/CHANGELOG.md` |
| 2026-09-22 | 空合并图修复：把 3 个 v4.0.0 doc-only 提交带入 v4.1.0 图 | `fca71cb525` / `a65497e276` / `7e30fbc0cb` | `docs/releases/v4.1.0/CHANGELOG.md` §"Empty-merge graph repair" |
| 2026-09-23 | v4.0.0 → v4.1.0 review queue 立卷（4 个提交待人工评审） | `2bd69b223f` — docs(v4.1.0): V400_TO_V410_REVIEW_QUEUE | `docs/releases/v4.1.0/V400_TO_V410_REVIEW_QUEUE.md` |
| 2026-09-26 | zombie-fix / Windows 兼容 / BINARY collation 评审队列提交并入 | `9c6767a512` — merge: bring v4.0.0 Windows compat + BINARY collation review-queue commits into v4.1.0 | `docs/releases/v4.1.0/FEATURE_MATRIX.md` D-01 |
| 2026-09-28 | review queue 关闭（4 个提交全部有结论） | `0bbb044da3` — docs(v4.1.0): close V400_TO_V410_REVIEW_QUEUE — all 4 commits resolved | `docs/releases/v4.1.0/ROADMAP.md` M-3 |
| 2026-09-29 | PHASE_0 DRAFT 文档脚手架（9 份） | `8ea6d0e592` — docs(v4.1.0): PHASE_0 DRAFT docs scaffold (STAGE SSOT compliance) | `docs/releases/v4.1.0/ROADMAP.md` M-1 |
| 2026-09-29 | DRAFT → ALPHA 自我声明（tag `v4.1.0-alpha1` @ `c1a73a5320`）—— **该声明于 2026-09-30 被回退** | `c1a73a5320` — feat(v4.1.0): promote DRAFT to ALPHA (tag v4.1.0-alpha1) | `docs/releases/v4.1.0/STAGE.yaml` → `last_rollback_record` |
| 2026-09-30 | **v4.0.0 GA 判定作废** + 补跑证据（7/8 gates exit 0）+ 新建 `v4.0.0/GA_RELEASE_TIMELINE.md` | `2c729af161` — v4.0.0: GA 判定作废 + 补跑证据(7/8 gates exit 0) | `docs/releases/v4.0.0/STAGE.yaml` → `ga_promotion_status.verdict = "REVOKED 2026-09-30"` |
| 2026-09-30 | WP-A 回填（backlog 20 → 24）；发现 `check_anti_ignore_gate.sh` 是空转门禁 | `86c2ba6ac2` — v4.1.0: WP-A 回填 20→24 + check_anti_ignore_gate 空转发现 | `docs/releases/v4.1.0/STAGE.yaml` → `legacy_chain_audit_2026_09_30.hollow_gate_anti_ignore` |
| 2026-10-04 | 销账 6 条已解决 open_issues；重写陈旧 `next_action` / `last_status_refresh` | `27f646097f` — docs(v4.1.0): STAGE.yaml 销账 6 条已解决 open_issues | `docs/releases/v4.1.0/STAGE.yaml` → `resolved_issues_2026_10_03` |
| 2026-10-04 | 记录 15 个 open issue 的并行路径与三处根因更正 | `af64d834c8` — docs(v4.1.0): 记录 15 个 open issue 的并行路径与三处根因更正 | `docs/releases/v4.1.0/STAGE.yaml` → `issue_coordination_2026_10_04` |
| 2026-10-06 | **重新进入 ALPHA**（`current_stage: "ALPHA"`，tag `v4.1.0-alpha2` 未实际切出） | `3ae177381d` — chore(stage): DRAFT -> ALPHA promotion at v4.1.0-alpha2 (#4950) | `docs/releases/v4.1.0/STAGE.yaml` → `last_transition` |
| 2026-10-07 | 更正 ALPHA 晋升理由中的**失实门禁证据** | `845cfe5f7f` — docs(v4.1.0): 更正 STAGE.yaml ALPHA 晋升理由中的失实门禁证据 (#5078) | `docs/releases/v4.1.0/STAGE.yaml` → `last_transition.reason` |
| 2026-10-07 | **V400-09 168h SOAK 明确登记为 NOT MET**，撤回两条基于「文件存在」的销账 | `8b55e7f16e` — docs(v4.1.0): V400-09 168h SOAK 明确登记为 NOT MET (#5102) | `docs/releases/v4.1.0/STAGE.yaml` → `known_gaps.V400_09_soak` |
| 2026-10-07 | COV-01 / COV-02 门禁证据归档，`STAGE.yaml` 状态置 DONE | `51486b79dc` — docs(v4.1.0): COV-01/COV-02 门禁证据归档 + STAGE.yaml 状态 DONE | `docs/releases/v4.1.0/evidence/COV_01_02_COVERAGE_GATE_2026-10-07.md` |
| 2026-10-08 | 修正 `alpha_to_beta` 判据：**文件存在性不是 SOAK 完成的证据** | `4e02e29f17` — docs(#5102): 修正 alpha_to_beta 判据 | `docs/releases/v4.1.0/STAGE.yaml` → `exit_criteria.alpha_to_beta` |
| 2026-10-08 | v4.1.0 HEAD 的 5min / 1h SOAK 实测产物入库（**非** 168h） | `62f1fe3cb1` — docs(v4.1.0): 5min/1h SOAK 实测 + P0-08 WAL 恢复逐条整表重写 | `docs/releases/v4.1.0/SOAK_V410_1H_2026-10-08/`、`SOAK_V410_5MIN_2026-10-08/` |
| 2026-10-09 | #5209 等值谓词路由到索引（二次列谓词 111×）；BPlusTree 支持一 key 多 row_id | `ccabad3c18` / `bcaee8b03c` | `docs/releases/v4.1.0/CHANGELOG.md` |
| 2026-10-09 | #5193 架构路线裁决 = **选项 C**（文档与实现一致） | `5326c77137` — docs+code(#5193): 采纳选项 C | `docs/releases/v4.1.0/ARCHITECTURE_ROUTE_DECISION_5193.md` |
| 2026-10-10 | MVCC 版本按**行身份**而非首列做键（#5277） | `ccf1409397` | `docs/releases/v4.1.0/CHANGELOG.md` |
| 2026-10-11 | 删除 `current_db` 共享可变状态，会话库改 per-connection（#5057） | `0a2a186c7c` | `docs/releases/v4.1.0/evidence/SESSION_DB_READPATH_5057_2026-10-07.md` |
| 2026-10-13 | 三处正确性缺陷修复：#5303 二级索引丢失、#5301 GROUP BY、#5300 隐式转换 | `1cb784dd5f` / `6a8878c9b0` / `3765dbec33` | `docs/releases/v4.1.0/CHANGELOG.md` |
| 2026-10-14 | #5281 行身份架构缺陷修复（RowId 由链保真） | `48f62d1906` | `docs/releases/v4.1.0/CHANGELOG.md` |
| 2026-10-14 | #5192 WAL 行条目记调用方指定的库（多库崩溃恢复 E2E 闭环） | `cd5149c8b4` | `docs/releases/v4.1.0/CHANGELOG.md` |
| 2026-10-15 | **门禁路径漂移修复**（V310 → V3.12 目录重构后未同步）：B5 必需测试文件、G7 soak harness、P12 crash、int2 orphan、其余 10 个 gate | `15ec658e11` / `f1253f8b07` / `009a68e4f7` / `b2bdf9f9b3` | `docs/releases/v4.1.0/CHANGELOG.md` |
| 2026-10-15 | 协议层改用 engine schema（#5341）；SHOW PROCEDURE STATUS（#5346）；BEFORE INSERT 触发器改新行（#5349） | `ab6fc8af81` / `bf2bed26d3` / `144e5df982` | `docs/releases/v4.1.0/CHANGELOG.md` |
| 2026-10-16 | CI 修复：`cargo fmt --check` 漂移、clippy `collapse_commit`、CARGO_HOME/RUSTUP_HOME 覆盖、evidence-graph 短 hash panic | `d2ea946867` / `01def11053` / `06e2a4f36c` / `fb109aa170` | `docs/releases/v4.1.0/evidence/CI_RESILIENCE_BATCH26_2026-10-22.md` |
| 2026-10-16 | #5228/#5233/#5234：CLUSTERED 表不再丢已提交行 | `f03c81d047` | `docs/releases/v4.1.0/CHANGELOG.md` |
| 2026-10-21 | **门禁判据唯一化**（#5437 / #5438 / #5439 / #5440 / #5441）：G 编号降级为别名、GA 门禁清单唯一、`check_release_artifacts.sh` 第一次真的被执行；`FEATURE_MATRIX.md` 建成 | `4d8c8deb10` | `docs/releases/v4.1.0/FEATURE_MATRIX.md`；`docs/releases/v4.1.0/evidence/STAGE_SSOT_AND_ARTIFACT_GATES_BATCH10_2026-10-21.md` |
| 2026-10-22 | **GA 的两个前置落地**：发布线版本锚点（#5442）+ 承载分支规则（#5436，`carrier_enforced: true`） | `2593712d8d`（合入 `66b6f4559a`） | `docs/releases/v4.1.0/evidence/GA_VERSION_AND_CARRIER_BATCH25_2026-10-22.md` |
| 2026-10-22 | **覆盖率测量确定性化**（#5537）：钉 `--test-threads=1`，消除跨机台读数漂移 | `2fef945260`（合入 `b68671895c`） | `docs/releases/v4.1.0/evidence/BATCH28_SEM4_AND_GATE_INTEGRITY_2026-10-23.md` §1 |
| 2026-10-22 | CI 第一次真的跑测试（needs 死锁 + checkout 两个缺陷） | `1b24f048c6`（合入） | `docs/releases/v4.1.0/evidence/CI_TRUTH_BATCH24_2026-10-22.md` |
| 2026-10-22 | 根包单元测试不再改写进程级 env（#5423） | `e91079bfc8`（合入） | `docs/releases/v4.1.0/evidence/PARALLEL_ENV_RACE_BATCH17_2026-10-22.md` |
| 2026-10-23 | **SEM-4 销账**：根因从字面占位符 `XYZ` 换成实测事实（并行 test 线程导致读数漂移） | `231652c205` | `docs/governance/debt/debt-registry.yaml`（SEM-4，`state: CLOSED`） |
| 2026-10-23 | **WAL 恢复去重扫描改为每表一次索引**（消除二次项）+ `Value::Hash` 与 `PartialEq` 对齐 | `a685661448` / `93e0d92519` | `docs/releases/v4.1.0/evidence/WAL_RECOVERY_SCALE_P0_08_2026-10-23.md` |
| 2026-10-23 | **P0-08 销账**（实测更正归因：真正的二次项是去重扫描，不是 `save_table_full`） | `59ed39108e` | `docs/releases/v4.1.0/evidence/WAL_RECOVERY_SCALE_P0_08_2026-10-23.md` |
| 2026-10-23 | P0-06 / P0-07 销账（两条的**前提**被实测推翻） | `aa28c49c19` | `docs/releases/v4.1.0/DEFECTS_AND_ISSUES.md` §0 |
| 2026-10-23 | 补齐 4 份从未存在的 mandatory 文档；FEATURE_MATRIX / RELEASE_NOTES 扩写至门禁阈值 | `f23f567447` / `962d3ba50d` | `docs/releases/v4.1.0/evidence/GA_MANDATORY_DOCS_BATCH27_2026-10-23.md` |
| 2026-10-23 | 11 份 mandatory 文档补齐 + postcheck 两个真实阻塞点（#5552 / #5553） | `88ab01c6c9`（合入） | `docs/releases/v4.1.0/evidence/GA_MANDATORY_DOCS_BATCH27_2026-10-23.md` |
| 2026-10-23 | **16 个 G 关注点全部接进阶段门禁清单，废止孤儿 `check_g_all.sh`** | `504e0f6ec4` / `01275c72ac`（合入） | `docs/governance/STAGE_CONFIG.yaml` → `gate_definitions`（#5505） |
| 2026-10-23 | 同步工具不再吞错 + 凭据不再进进程列表 + 三态区分「远端没有该分支」与「查不到」 | `34f6e70cd0` / `e71468d942` / `cd1fdbfc79`（合入） | `docs/releases/v4.1.0/evidence/SYNC_TOOLS_STRICT_5520_2026-10-23.md` |
| 2026-10-23 | CI 门禁跑它自己被创建的那个 commit（#5534） | `a8d8e30877` / `2406559ac0`（合入） | `docs/releases/v4.1.0/evidence/CI_WIRING_HONESTY_5416_5484_2026-10-21.md` |
| 2026-10-23 | **本批次**：补齐 `FEATURE_CHECKLIST.md`（BETA 产物）与 `GA_RELEASE_TIMELINE.md`（GA 产物） | 本分支 `docs/ga-rc-artifacts-batch32`（**未提交**） | 本文件 §2.1 |

### 1.1 远端状态（2026-10-23 本轮 `git ls-remote` 实测）

| 远端 | `main` | `develop/v4.1.0` |
|---|---|---|
| gitea250（`origin`） | `515ab236d0bf` | `878e1d62c3a5` |
| gitea252 | `515ab236d0bf` | **`2406559ac0`** |
| gitcode | `515ab236d0bf` | `cc075f264fdd` |
| gitee | `515ab236d0bf` | `cc075f264fdd` |
| github | `515ab236d0bf` | `2406559ac0` |

⇒ **`main` 在 5 远端已收敛**（全部 `515ab236d0bf`）；**`develop/v4.1.0` 尚未收敛**
（3 个不同值）⇒ **P0-04（5 远端漂移）仍未销账**。

---

## 2. GA 的硬性阻塞项

> 逐条对照 `docs/governance/STAGE_CONFIG.yaml` → `stages.GA.exit_criteria`（5 条）
> 与 `docs/releases/v4.1.0/STAGE.yaml` → `exit_criteria.rc_to_ga`（6 条），
> 外加 `stages.GA.required_files`（5 份）与 `stages.GA.required_gates`（31 项）。
> **本轮实跑的只有 3 个门禁**（见 §2.7），其余一律 `NOT RUN`。
> **NOT RUN ≠ PASS**（`STAGE_CONFIG.yaml` 对 `optional_gates` 的注记：
> 「NOT-RUN 不得记为 PASS（P14 DRIFT ≠ PASS）」）。

### 2.1 168h 多模型 SOAK — **NOT MET**（决定性阻塞）

| 项 | 内容 |
|---|---|
| 判据 | `STAGE.yaml` → `exit_criteria.rc_to_ga`：「168h multi-model SOAK PASS」；`known_gaps.V400_09_soak.criterion`：「V400-09 168h SOAK FINAL_REPORT exists OR explicit deferral recorded」 |
| 状态 | **NOT MET**（`known_gaps.V400_09_soak.status`，核验日期 2026-10-07） |
| 实测事实 | (1) #4944 checkpoint 记录的服务进程 PID 1957614 已不在进程表；(2) 全盘搜索无 `STABILITY_REPORT_*.md` / `metrics.csv` / `*soak*run[34]*`；(3) `docs/releases/v4.0.0/V400_09_168H_SOAK_FINAL_REPORT.md` 日期为 2026-09-30，记录的是第一轮（2026-09-19 kickoff）的**失败**，其正文自述 "the 168h SOAK process never ran to completion" |
| 已有的短时 SOAK（**不满足本判据**） | `SOAK_V410_5MIN_2026-10-08/`、`SOAK_V410_1H_2026-10-08/`（含 `STABILITY_REPORT.md` / `metrics.csv` / `summary.json`）；另 `evidence/SOAK_8H_AND_CAPACITY_2026-10-09.md`（8h） |
| 裁决状态 | `decision: "OPEN — 是否重跑 168h SOAK 属架构师裁决，本 SSOT 不代为决定。"` |
| 是否可由属主裁决解除 | **可以**——但代价是**放弃 GA 判据本身**，不是补一份文档。属主可以选择：(a) 重跑 168h SOAK（需专用主机与真实时长）；(b) 正式把该判据从 `rc_to_ga` 移除并记录理由（属框架级变更，影响所有版本）。**本文件不代为决定。** |
| 附带约束 | `STAGE.yaml` → `claims_boundaries.prohibited_pre_ga`：**GA 之前不得宣称 168h 多模型 SOAK 结果** |
| 证据 | `docs/releases/v4.1.0/STAGE.yaml` → `known_gaps.V400_09_soak`；`docs/releases/v4.0.0/V400_09_168H_SOAK_FINAL_REPORT.md` |

### 2.2 TPC-H SF=1 22/22 — **not recorded**（判据不可判定）

| 项 | 内容 |
|---|---|
| 判据 | `stages.GA.required_gates` → `scripts/gate/check_g1_tpch_baseline.sh`（G1，`stage_min: ALPHA`，累计到 GA） |
| 状态 | **not recorded**（不是 PASS，也不是 FAIL——仓库里**没有这个判定**） |
| 原文 | `docs/releases/v4.1.0/EVALUATION_REPORT.md` §7.1 第 378 行：「TPC-H SF=1 22/22 verdict for v4.1.0 \| **not recorded**」 |
| 原因 | `evidence/TPCH_FIXTURE_5379_2026-10-20.md` 只覆盖 fixture 自包含性（#5379），该轮明确把 SF=1 排除在范围外（#5387） |
| 本轮状态 | **NOT RUN**（SF=1 数据集未就绪） |
| 是否可由属主裁决解除 | **不能靠裁决解除**——它要么被跑出来，要么继续 `not recorded`。属主可以决定把 G1 从 GA 的 `required_gates` 移出（框架级变更）。 |
| 关联缺口 | **Q20 oracle（#4540）**：`sqlrustgo/q20.tsv` 缺失（22 个文件缺 1），`CROSS_ENGINE_HASH_CHECK.json` 记 `present:false`，与 `docs/releases/v3.12.0/evidence/v312-59/GA5_TPCH_SF1_REPORT.md:40` 的 "Q20 PASS" **直接矛盾**。当前树里两个 oracle 产物都不存在。这是 `STAGE.yaml` → `open_issues` **复核后剩余的唯一一条** |
| 证据 | `docs/releases/v4.1.0/EVALUATION_REPORT.md` §7.1；`docs/releases/v4.1.0/DEFECTS_AND_ISSUES.md` P1-02 |

### 2.3 覆盖率门槛 — **MET**（按 8-crate 平均；逐 crate 口径需裁决）

| 口径 | 数值 | 判定 |
|---|---|---|
| `STAGE_CONFIG.yaml` → `thresholds.COVERAGE_MIN_GA` | **85 %** | — |
| `STAGE.yaml` → `rc_to_ga.exit_criteria` | 「Coverage >= 80%」 | ⚠️ **措辞与 SSOT 不一致**（见下） |
| **8-crate（L1_8）平均实测** | **87.80 %** | ✅ **≥ 85，MET** |
| mysql-server 实测 | 87.89 % | ✅ |

分 crate（L1_8）：

| crate | 覆盖率 | 对 GA(85) |
|---|---|---|
| parser | 86.46 % | ✅ |
| planner | 88.91 % | ✅ |
| executor | 82.74 % | ❌（单独看 < 85） |
| transaction | 93.29 % | ✅ |
| storage | 80.61 % | ❌（单独看 < 85） |
| catalog | 87.33 % | ✅ |
| optimizer | 87.85 % | ✅ |
| types | 95.28 % | ✅ |

> ⚠️ **三个口径并存，引用时必须注明**：
> ① `STAGE.yaml` 的 `rc_to_ga` 写「Coverage >= 80%」（那是 `COVERAGE_MIN_RC` 的值）；
> ② `STAGE_CONFIG.yaml` 的 `COVERAGE_MIN_GA` 是 **85**，且 `STAGE.yaml` 自己的
> `coverage_target_for_ga` 明写「85%（STAGE_CONFIG COVERAGE_MIN_GA）；80% 是
> COVERAGE_MIN_RC」⇒ **GA 门槛按 85 判**；
> ③ `DEFECTS_AND_ISSUES.md` P1-01 记的 **60.58 %** 是 2026-10-01 的旧测量，
> 2026-10-23 **不复现**（parser 实测 86.46 %），该条已销账但数字仍在文档里。
>
> ⚠️ **逐 crate vs 平均**：8-crate 平均 87.80 % ≥ 85 已满足，但 **executor
> （82.74 %）与 storage（80.61 %）单独看低于 85 %**。若属主按「逐 crate ≥ 85 %」
> 判定，则 GA **尚不满足**。**这一点留给裁决。**
>
> ⚠️ 另注：`stages.GA.required_gates` 含 `scripts/gate/check_coverage_v312.sh`
> —— 门槛由脚本按 `thresholds` 判定；本轮该脚本 **NOT RUN**，§2.7 记录了原因。

来源：`docs/releases/v4.1.0/evidence/BATCH28_SEM4_AND_GATE_INTEGRITY_2026-10-23.md` §1.3
（8-crate 完整实跑，L1_8 = 87.80 %）；
`docs/releases/v4.1.0/evidence/COV_01_02_COVERAGE_GATE_2026-10-07.md`（mysql-server 87.89 %）。
测量已确定性化：commit `2fef945260`（2026-10-22）钉死 `--test-threads=1`，
同机同代码连续两遍测 `sqlrustgo-transaction` 得 93.29 % / 93.29 % 完全一致。

### 2.4 缺陷与 P0 — **NOT MET**（P0 尚有 2 条未销账）

| 级别 | 总数（2026-10-01 原始） | 仍未销账（2026-10-23） | 主要类别 |
|---|---|---|---|
| **P0** | 8 | **2** | v4.0.0 GA 撤销（`REVOKED`）、5 远端漂移 |
| **P1** | 7 | **6** | 覆盖率 60.58 %（已销账，计数为历史）、Q20 oracle 缺产物、WP-A RE-OPENED + 20 条 backlog、crate 状态描述错误、G13 门禁红 |
| **P2** | 7 | 7 | 门禁假阳性、Q20 decorrelate 零调用点、大文件无门禁、v3.12 失联、未核实遗留、运营债 |
| **合计** | **22** | **15** | |

**两条未销账 P0 逐条**：

| ID | 性质 | 内容 | 是否可由属主裁决解除 |
|---|---|---|---|
| **P0-03** | **状态，不是缺陷** | v4.0.0 的 GA 判定被撤销：`docs/releases/v4.0.0/STAGE.yaml` → `ga_promotion_status.verdict = "REVOKED 2026-09-30 — v4.0.0 never legitimately reached GA"`，`actual_status: "DRAFT"`。<br>⚠️ **但 v4.0.0 的代码确已发布**：2026-09-19，tag `v4.0.0-final` @ `54571eeca0` → `07178c9d66`，5 远端 `main` 均含该 commit。⇒ v4.0.0 **从未合法达到 GA**，但**确实是已发布的代码**。 | **是**——它需要的是**裁决**（如何表述「v4.0.0 已发布但从未合法 GA」），不是补丁。`STAGE.yaml` → `resolved_issues_2026_10_23` 首条已把 `CURRENT_VERSION.md` 与 `v4.0.0/RELEASE_NOTES.md` 的 6 处旧口径对齐 |
| **P0-04** | 运营级缺陷 | 5 远端漂移。2026-10-23 实测（§1.1）：`main` 已在 5 远端收敛为 `515ab236d0bf`；但 **`develop/v4.1.0` 仍有 3 个不同值**（gitea250 `878e1d62c3a5` / gitcode+gitee `cc075f264fdd` / gitea252+github `2406559ac0`）⇒ **未销账** | **部分可以**——`main` 那部分已完成；`develop/v4.1.0` 的收敛是**推送操作**，不需裁决，只需执行与验证 |

⇒ `exit_criteria.alpha_to_beta` 的「No P0 bugs open」与 `rc_to_ga` 的
「All RC{N} release-blocking bugs closed / no P0 bugs since」**目前 NOT MET**。

`STAGE.yaml` → `open_issues` 复核后**剩 1 条**：P1-02 / #4540 Q20 oracle（见 §2.2）。

### 2.5 GA 阶段 required_files — 产物位已补齐，但 **3/5 仍 NOT MET**

| required_file | 期望 | 当前状态 | 判定 |
|---|---|---|---|
| `docs/releases/v4.1.0/STAGE.yaml` | `current_stage must be GA` | `current_stage: "ALPHA"` | ❌ **NOT MET** |
| `docs/releases/v4.1.0/RELEASE_NOTES.md` | `must be the GA release notes` | 存在，但是 ALPHA/DRAFT 口径 | ⚠️ **NOT MET**（措辞） |
| `docs/releases/v4.1.0/GA_GATE_REPORT.md` | 存在 | **本文件落盘时已存在**（同批次并行产出，2026-10-23 本 worktree 内实测存在）；其**内容是否已覆盖全部 GA 判据不由本文件裁决** | ✅ 产物位已在 |
| `docs/releases/v4.1.0/GA_RELEASE_TIMELINE.md` | 存在 | ✅ **本文件**（本批次补齐） | ✅ 产物位已补 |
| `docs/releases/v4.1.0/CHANGELOG.md` | `must contain the GA entry` | 无 GA 条目 | ❌ **NOT MET** |

GA `doc_artifacts` 另需：`EVIDENCE_STATUS.md`（同批次并行产出，本文件落盘时已存在）、
`POST_GA_PLAN.md` —— 后者在本文件落盘时**不存在**
（`check_release_artifacts.sh` 实测：`GA: 10 present, 1 missing` — 仅 `POST_GA_PLAN.md`）。

> ⚠️ **产物存在 ≠ 判据满足**。上表只回答「文件在不在」。`STAGE.yaml` 的
> `current_stage` 仍是 `ALPHA`，`CHANGELOG.md` 仍无 GA 条目 —— 这两项是
> `required_file_expectations`（`STAGE_CONFIG.yaml` #5439）里**带状态期望**的，
> 只有推进阶段 + 写 GA 条目才能满足，不是补文件能解决的。

### 2.6 承载分支 — **NOT MET**（机器可执行的门禁，不是判断题）

| 项 | 内容 |
|---|---|
| 判据 | `stages.GA.exit_criteria` 第 5 条：「GA judged on a branch matching GA.branch_pattern (main / release/v* / hotfix/v*)」；`stages.GA.branch_pattern = ["main", "release/v*", "hotfix/v*"]`；`carrier_enforced: true` |
| 现状 | `STAGE.yaml` → `current_state.ga_carrier_branch = "release/v4.1.0"`；`ga_declared: false`；`ga_carrier_note: "no release/v4.1.0 branch exists yet; GA has NOT been declared and MUST NOT be claimed on develop/v4.1.0"` |
| 后果 | **在 `develop/v4.1.0` 上把 `current_stage` 推到 GA，`scripts/gate/check_stage.sh` 会直接红。** 「在开发线上宣布 GA」从此不是一个判断问题，而是一个**可执行的门禁**（`STAGE_CONFIG.yaml` #5436） |
| 解除方式 | 切出 `release/v4.1.0`（或 `hotfix/v4.1.0` / `main`）并在**该分支上**声明 GA |
| 证据 | `docs/governance/STAGE_CONFIG.yaml` → `stages.GA`；`docs/releases/v4.1.0/STAGE.yaml` → `current_state`；`docs/releases/v4.1.0/evidence/GA_VERSION_AND_CARRIER_BATCH25_2026-10-22.md` |

### 2.7 GA 门禁清单（31 项 = 27 个门禁脚本 + 4 条 cargo 命令）—— 本轮实测边界

**本轮实跑 3 个**（2026-10-23，Mac-mini-7.local / arm64 / macOS 27.0.1）：

| 门禁 | 实测输出摘录 | 结论 |
|---|---|---|
| `scripts/gate/check_release_artifacts.sh --version v4.1.0` | **本文件落盘前**：`stage: ALPHA`；ALPHA 段 7/7 `OK`；`BETA: 5 present, 2 missing`（MISS `FEATURE_CHECKLIST.md` ×2）；`RC: 5 present, 4 missing`；`GA: 5 present, 6 missing`（MISS `GA_RELEASE_TIMELINE.md` ×2、`GA_GATE_REPORT.md` ×2、`EVIDENCE_STATUS.md`、`POST_GA_PLAN.md`）；`RESULT: PASS`；`EXIT=0`。<br>**本文件落盘后复跑**：`BETA: 7 present, 0 missing`；`RC: 8 present, 1 missing`（仅 `PERFORMANCE_TARGETS.md`）；`GA: 10 present, 1 missing`（仅 `POST_GA_PLAN.md`）；`RESULT: PASS`；`EXIT=0` | **PASS**（ALPHA 段阻塞检查）。⚠️ 该门禁在 ALPHA 阶段**只做披露**，不判定 GA 段 |
| `scripts/gate/check_evidence_binding.sh v4.1.0 <out_dir>` | `PASS=137, WARN=123, FAIL=0, UNVERIFIED=0`；`✅ Evidence binding check PASSED`；`EXIT=0` | **PASS**。⚠️ 该门禁枚举发布目录下的文档集合，**PASS 计数随文档集合变化**（本轮两次实测：补齐前 `PASS=127`、补齐后 `PASS=137`，WARN 恒为 123、FAIL 与 UNVERIFIED 恒为 0）。本行是**补齐后**的实测值 |
| `scripts/gate/check_anti_fabrication.sh` | `ERRORS=0 WARNINGS=0`；末行 `✅ Anti-fabrication check (AFP v4): PASS` | **PASS** |

**其余 28 项：全部 `NOT RUN`**。原因分类：

| 原因 | 涉及 |
|---|---|
| 本轮范围之外（批次 32 只交付两份文档） | `check_rc_ga_gate.sh ga`、`check_arch_invariants.sh`、`check_arch3_no_bypass.sh`、`check_int2_no_orphan.sh`、`check_int3_single_expr.sh`、`check_sem1_savepoint.sh`、`check_backup_restore.sh`、`check_p12_crash_test.sh`、`check_p14_upgrade_test.sh`、`check_p21_audit_log.sh`、`check_p22_time_travel.sh`、`check_p23_hash_chain.sh`、`check_g16_compatibility.sh`、`check_integration_gate.sh`、`check_arch_sem_debt.sh`、`check_cross_version_debt.sh`、`check_int_debt.sh`、`check_full_gate_verification.sh`、`check_coverage_v312.sh`、`check_sql_corpus_gate.sh`、`check_drift_not_pass.sh`、`check_architecture_freeze.sh`、`check_gate_self_verification.sh`、`check_gate_test_integrity.sh` |
| 前置数据 / 硬件 / 外部工具未就绪 | `check_g1_tpch_baseline.sh`（SF=1 数据未就绪）、`check_p13_soak_test.sh`、`check_g11_qps.sh`、`check_g12_sysbench.sh`、`check_g13_stability.sh`、`check_g14_real_crash.sh`、`check_g15_perf_report.sh` |
| cargo 命令（`cargo build --all-features` / `cargo test --all-features` / `cargo fmt --check` / `cargo clippy --all-features -- -D warnings`） | 本轮**未**在整仓范围实跑，故不记 PASS |

**非阻断但「必须实跑并披露」的门禁**（`stages.GA.optional_gates`，6 个）—— 本轮同样
全部 `NOT RUN`：

| 门禁 | G | requirement（`STAGE_CONFIG.yaml` 原文） | 本轮 |
|---|---|---|---|
| `check_p13_soak_test.sh` | G7 | 长时 soak（真实 24h 需要专用主机 / 真实时长） | **NOT RUN**（且 168h SOAK 判定 NOT MET，§2.1） |
| `check_chaos_soak.sh` | — | 混沌压测 | NOT RUN |
| `check_g11_qps.sh` | G11 | 专用基准主机 + `PERFORMANCE_BASELINE.md` | NOT RUN（需基准机） |
| `check_g12_sysbench.sh` | G12 | `sysbench` 二进制（脚本在缺失时 FAIL） | NOT RUN（缺外部工具） |
| `check_g13_stability.sh` | G13 | Z6G4 主机 + `STABILITY_REPORT.md` + Beta 72h 报告（缺失时脚本 FAIL） | NOT RUN |
| `check_g14_real_crash.sh` | G14 | Z6G4 主机 + sysbench + `CRASH_TEST_REPORT.md`（缺失时脚本 FAIL） | NOT RUN |
| `check_g15_perf_report.sh` | G15 | `PERFORMANCE_REPORT.md` + 5 份子报告 + `PERFORMANCE_BASELINE.md`（缺失时脚本 FAIL） | NOT RUN |

> `check_stage.sh`（driver）**只列出** `optional_gates`，**不执行**它们 ⇒
> **执行责任在晋级评审**。这是 SSOT 唯一的声明处。

### 2.8 GA 硬性阻塞项汇总

| # | 阻塞项 | 状态 | 属主可否裁决解除 |
|---|---|---|---|
| 1 | 168h 多模型 SOAK | **NOT MET** | 可（但等于改判据或重跑） |
| 2 | TPC-H SF=1 22/22 | **not recorded / NOT RUN** | 不可（只能跑出来） |
| 3 | Q20 oracle（#4540） | **OPEN** | 不可（缺产物） |
| 4 | 覆盖率 ≥ 85 % | **MET**（8-crate 平均 87.80 %）；逐 crate 口径下 executor/storage 不满足 | 可（口径裁决） |
| 5 | P0 未销账 | **2 条**（P0-03 状态 / P0-04 远端漂移） | P0-03 可裁决；P0-04 需执行 |
| 6 | GA required_files | **产物位已补齐**（`GA_RELEASE_TIMELINE.md` 本文件、`GA_GATE_REPORT.md` 同批次并行产出）；但 **`STAGE.yaml` 的 `current_stage` 仍是 `ALPHA`**、`RELEASE_NOTES.md` 仍是 ALPHA 口径、`CHANGELOG.md` 无 GA 条目 ⇒ **3/5 仍 NOT MET** | 需推进阶段 + 写 GA 条目，非裁决 |
| 7 | GA doc_artifacts | `EVIDENCE_STATUS.md` 已存在（同批次并行产出）；**`POST_GA_PLAN.md` 仍缺** | 需产出，非裁决 |
| 8 | 承载分支 | **NOT MET**（无 `release/v4.1.0`） | 需执行，非裁决 |
| 9 | GA required_gates（31 项） | **仅 3 项实跑，28 项 NOT RUN** | 需执行，非裁决 |
| 10 | 人工架构师签字 | **未取得**（`transitions.RC_to_GA.requires_human_approval: true`） | **必须由人执行** |

---

## 3. 三条晋升路径（均需人工决策）

> 三条路径**都不是**本文件的推荐结论。本文件只列出**每条路径的前提、代价与
> 不可回避的裁决点**。**本文件不代为决定，也不代为签字。**

### 路径 A — 按现有门槛完整推进（ALPHA → BETA → RC → GA 全程合规）

| 项 | 内容 |
|---|---|
| 做法 | ① 依次补齐 BETA / RC / GA 的 `required_files` 与 `doc_artifacts`；② 在 `release/v4.1.0` 上完成 `ALPHA → BETA → RC` 的**三次**跃迁（每次都需人批）；③ 重跑 168h SOAK 与 TPC-H SF=1 并归档真实产物；④ 实跑 GA 的 31 项 required_gates + 7 项 optional_gates；⑤ 收敛 5 远端 `develop/v4.1.0`；⑥ 人类架构师签署 `GA_GATE_REPORT.md` 后切 tag `v4.1.0` |
| 代价 | **最长**。168h SOAK 需专用主机 + 真实时长；TPC-H SF=1 需先造 fixture 并跑 22 条 query；GA 的 31 项门禁需在承载分支上完整实跑一遍 |
| 不可回避的裁决点 | 逐 crate 还是平均判覆盖率（§2.3）；`STAGE.yaml` 的 `rc_to_ga` 措辞「Coverage >= 80%」与 SSOT 的 85 哪份为准 |
| 风险 | 时间成本；期间 `develop/v4.1.0` 继续漂移会扩大 P0-04 |

### 路径 B — 先完成阶段跃迁，GA 延后（把 GA 判据与 GA 发布解耦）

| 项 | 内容 |
|---|---|
| 做法 | ① 先把 `ALPHA → BETA` 的 4 条出口判据逐条闭合（含 V400-09 的裁决、以及 WP-C..G 的处置）；② 再走 `BETA → RC`（B1-B5 硬检查 + B-F1..B-F7 功能检查 + 首个 RC tag）；③ **GA 暂缓**，GA 相关文档（本文件）与 GA 门禁留到 RC 稳定后再做 |
| 代价 | 中等。好处是**每一次跃迁都有可审的最小判据集**，不必一次面对 GA 的 31 项 |
| 不可回避的裁决点 | `alpha_to_beta` 判据第 2 条本身要求「FINAL_REPORT exists **OR** explicit deferral recorded」，且第 3 条要求 `known_gaps.V400_09_soak.decision` **不再是 OPEN** ⇒ **即使走这条路径，168h SOAK 也必须先有一个明确裁决**（重跑 / 正式 defer） |
| 风险 | 阶段停留时间变长；期间 `open_issues` 若增加，`ALPHA → BETA` 判据会再次变红 |

### 路径 C — 以限定口径发布（明示已知缺口，不宣称满足现有门槛）

| 项 | 内容 |
|---|---|
| 做法 | 承认 §2.8 的未满足项，在 `STAGE.yaml` 与 `GA_GATE_REPORT.md` 中**明确记录降级口径**（v4.0.0 的先例：`CLAIM_DOWNGRADE_MANIFEST.md` §4「Full PASS target is v4.0.1」），以限定口径发布并在 `POST_GA_PLAN.md` 里列出待补项 |
| 代价 | **最低，但治理风险最高**。v4.0.0 走的正是这条路，其代价是 `STAGE.yaml` 与 `GA_GATE_REPORT.md` **互相矛盾**，最终导致 2026-09-30 的 GA 判定作废（`ga_promotion_status.verdict = "REVOKED"`）与一次 ALPHA 回滚 |
| 不可回避的裁决点 | ① `STAGE_CONFIG.yaml` 的 `COVERAGE_MIN_GA: 85` 是否下调（**框架级变更，影响所有版本**）；② `transitions.RC_to_GA.requires_human_approval: true` 不可绕过 ⇒ 即便走 C，**签字仍是必须的人工动作** |
| 风险 | **高**。若降级口径**没有**写进 `STAGE.yaml`（v4.0.0 的原始失败点），会重演「SSOT 与 gate 报告矛盾」⇒ 下一个审计仍会作废 |

> **三条路径共同的硬约束**：`docs/governance/STAGE_CONFIG.yaml` →
> `transitions` 段里 `ALPHA_to_BETA` / `BETA_to_RC` / `RC_to_GA`
> **三次跃迁全部是 `requires_human_approval: true`**。
> **本文件只记录判据与建议，不代审批。**

---

## 4. 回滚原则

### 4.1 触发条件

若 GA 发布后判定其证据不成立（例如 §2.1 的 SOAK 判据被事后证明不成立、
或 §2.2 的 oracle 缺失被发现影响正确性声明），按仓库已有的先例
（`docs/releases/v4.0.0/GA_RELEASE_TIMELINE.md` §4 与
`docs/governance/incidents/2026-09-30-V410-ALPHA-UNSUPPORTED-BY-GATE-EVIDENCE.md`）
执行回滚。

### 4.2 回滚步骤

1. **明确受影响的 claim 与证据缺口** —— 写清哪一条声明、哪一份证据不成立。
2. **在 SSOT 里降级或标注限制** —— 改 `docs/releases/v4.1.0/STAGE.yaml`
   （`current_stage` / `ga_promotion_status` / `known_gaps`），
   **不只在发布说明里改**。v4.0.0 的教训：口径只写在
   `CLAIM_DOWNGRADE_MANIFEST.md` 而没写进 `STAGE.yaml`，就是被作废的直接原因。
3. **修正后重新执行对应 gate，并附实跑输出** —— 不接受「文件存在」作为证据
   （#5102 的原话：*file existence alone is NOT evidence*）。
4. **用 PR / commit 记录整改过程** —— 保留审计轨迹，不做 force-push 抹除。
5. **同步 5 远端** —— 用 `scripts/sync/5remotes_sync.sh` + `scripts/sync/5remotes_drift_check.sh`
   收敛并复验（`scripts/gate/check_main_freshness.sh` 做 `main` 新鲜度检查）。

### 4.3 tag / 分支 / 远端的处置

| 对象 | 处置 | 依据 / 先例 |
|---|---|---|
| GA tag（`v4.1.0`） | **不删除、不 force-push**。v4.0.0 的先例：`v4.0.0-final` 与 `release/v4.0.0` **都保留**，作为审计轨迹存在（`DEFECTS_AND_ISSUES.md` P0-03：「仍存在（保留为审计轨迹）」） | `docs/releases/v4.0.0/STAGE.yaml` → `ga_promotion_status` |
| 承载分支（`release/v4.1.0`） | **保留**，不删除；回滚以 `current_stage` 的降级 + 新 commit 表达 | 同上 |
| `main` | **不 force-push**。`scripts/gate/check_main_freshness.sh`（C-MAIN-01/02/03 fail-closed）会红 | `docs/releases/v4.1.0/MAIN_DIVERGENCE_RESOLUTION_2026-09-30.md` |
| 备份 | 收敛前先打备份分支 / tag（2026-09-30 的做法：`backup/main-pre-convergence-2026-09-30` + `archive/main-pre-convergence-2026-09-30`） | `docs/releases/v4.1.0/STAGE.yaml` → `resolved_issues_2026_10_03` |
| 5 远端 | 用同步工具收敛 + `5remotes_drift_check.sh` 复验；凭据走环境变量，不进进程列表 | `docs/releases/v4.1.0/evidence/SYNC_TOOLS_STRICT_5520_2026-10-23.md` |

### 4.4 谁有权执行

| 动作 | 执行者 |
|---|---|
| 修改 `STAGE.yaml` 的 `current_stage` / `ga_promotion_status` | **人类架构师**（`STAGE_CONFIG.yaml` → `transitions.*.requires_human_approval: true`） |
| 声明 / 撤销 GA 晋升 | **人类架构师** |
| 签署 `GA_GATE_REPORT.md` | **人类架构师**（`stages.GA.exit_criteria`：「GA gate report signed by human architect」） |
| 打 GA tag / 合入 `main` | **release-manager** + `codeowner + governance + human-architect`（`stages.GA.merge_policy.approval`，3 名 reviewer） |
| 推送 / 收敛 5 远端 | **release-manager**（执行动作，非裁决） |
| 修改门禁脚本或阈值 | **governance**（属框架级变更，影响所有版本） |
| 本文件由 | `openclaw-mcode` **自动生成** —— **不代签、不代审批** |

---

## 5. 阶段跃迁纪律

`docs/governance/STAGE_CONFIG.yaml` → `transitions` 段：

| 跃迁 | trigger | `requires_human_approval` |
|---|---|---|
| `DRAFT_to_ALPHA` | first alpha tag cut: `v{VER}-alpha1` | **`true`** |
| `ALPHA_to_BETA` | all alpha-targeted issues closed + coverage >= `COVERAGE_MIN_ALPHA` | **`true`** |
| `BETA_to_RC` | all B-F1~B-F7 functional checks PASS + first RC tag cut | **`true`** |
| `RC_to_GA` | GA gate report signed by human architect | **`true`** |
| `GA_to_NEXT_VERSION_DRAFT` | `POST_GA_PLAN.md` exists + next version branch created | **`true`** |

**⇒ 五次跃迁全部需要人工批准。本文件只记录判据与建议，不代审批。**

v4.1.0 当前的跃迁位置：`DRAFT → ALPHA` 已完成（2026-10-06 重新进入 ALPHA，
commit `3ae177381d`）；**`ALPHA → BETA` 未完成**（`known_gaps.V400_09_soak.decision`
仍是 `OPEN`；`open_issues` 剩 1 条；P0 未销账 2 条 ⇒「No P0 bugs open」不满足）。
`BETA → RC` / `RC → GA` **均未开始**（无 `v4.1.0-beta1` / `v4.1.0-rc1` tag）。

---

## 6. 签署

| 角色 | 姓名 | 日期 | 结论 |
|---|---|---|---|
| 提交人（`openclaw-mcode`） | openclaw-mcode | 2026-10-23 | 时间线与阻塞项已生成；全部 NOT MET / NOT RUN 均注明原因 |
| GA 评审（codeowner） | ______ | ______ | ______ |
| GA 评审（governance） | ______ | ______ | ______ |
| GA 评审（release-manager） | ______ | ______ | ______ |
| **人类架构师** | ______ | ______ | ______ |

> 本文件由 `openclaw-mcode` 自动生成，**晋级审批由人类架构师执行**。
> `STAGE_CONFIG.yaml` → `transitions.RC_to_GA.requires_human_approval: true`
> ⇒ **本文件不构成 GA 晋升批准**，签署栏留空由人填写。

---

## 7. 证据与参考

- `docs/governance/STAGE_CONFIG.yaml` —— 阶段框架 SSOT（`stages.GA.required_files` /
  `required_gates` / `doc_artifacts` / `exit_criteria`、`thresholds`、`transitions`、
  `gate_definitions` 的 `stage_min` 规则）
- `docs/releases/v4.1.0/STAGE.yaml` —— v4.1.0 per-version SSOT（`current_stage`、
  `exit_criteria.rc_to_ga`、`known_gaps.V400_09_soak`、`open_issues`、
  `resolved_issues_2026_10_23`、`current_state.ga_carrier_branch`、
  `claims_boundaries`）
- `docs/releases/v4.1.0/FEATURE_MATRIX.md` —— v4.1.0 相对 v4.0.0 的权威变更矩阵
- `docs/releases/v4.1.0/DEFECTS_AND_ISSUES.md` —— 缺陷目录与销账状态（§0 计数、P0-03、P0-04、P1-02）
- `docs/releases/v4.1.0/EVALUATION_REPORT.md` —— §7.1「not recorded」清单（TPC-H SF=1）
- `docs/releases/v4.1.0/RELEASE_NOTES.md` / `CHANGELOG.md` / `VERSION_PLAN.md` / `ROADMAP.md` —— 发布口径与里程碑
- `docs/releases/v4.1.0/evidence/` —— 81 份一手证据（关键：
  `BATCH28_SEM4_AND_GATE_INTEGRITY_2026-10-23.md`、`COV_01_02_COVERAGE_GATE_2026-10-07.md`、
  `WAL_RECOVERY_SCALE_P0_08_2026-10-23.md`、`SOAK_8H_AND_CAPACITY_2026-10-09.md`、
  `TLS_READ_SPIN_HANG_5099_2026-10-09.md`、`GA_MANDATORY_DOCS_BATCH27_2026-10-23.md`、
  `GA_VERSION_AND_CARRIER_BATCH25_2026-10-22.md`、`SYNC_TOOLS_STRICT_5520_2026-10-23.md`、
  `CI_TRUTH_BATCH24_2026-10-22.md`、`STAGE_SSOT_AND_ARTIFACT_GATES_BATCH10_2026-10-21.md`）
- `docs/releases/v4.1.0/SOAK_V410_1H_2026-10-08/`、`SOAK_V410_5MIN_2026-10-08/` —— 已入库的短时 SOAK 产物
- `docs/releases/v4.0.0/STAGE.yaml` / `GA_GATE_REPORT.md` / `GA_RELEASE_TIMELINE.md` —— 继承基线与 GA 撤销判定
- `docs/governance/incidents/2026-09-30-V410-ALPHA-UNSUPPORTED-BY-GATE-EVIDENCE.md` —— 关联的 AFP Type-B 违规记录
- `docs/releases/v4.1.0/MAIN_DIVERGENCE_RESOLUTION_2026-09-30.md` —— 5 远端收敛做法与备份约定
- `docs/releases/v4.1.0/FEATURE_CHECKLIST.md` —— 同批次产出的 BETA 阶段功能清单
