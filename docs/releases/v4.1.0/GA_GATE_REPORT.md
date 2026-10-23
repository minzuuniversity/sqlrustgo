# SQLRustGo v4.1.0 GA Gate Report

> **基线**：`develop/v4.1.0` @ `2406559ac09ecf821566d11bf4a3cc1ce4f4dbb8`
> **provenance:** generated_at=2026-10-23, branch=docs/ga-rc-artifacts-batch32,
> commit=`2406559ac09ecf821566d11bf4a3cc1ce4f4dbb8`, machine=Mac-mini-7.local (arm64, macOS 27.0.1),
> identity=openclaw-mcode, source_repo=openclaw/sqlrustgo,
> policy=Anti-Fabrication-Policy-v1.0
> **gate_policy_eval_id**：`v410-ga-gate-report-g1-g6-001`
> （本文档汇编的门禁引擎：`scripts/gate/check_rc_ga_gate.sh`、`scripts/gate/check_coverage_v312.sh`、
> `scripts/gate/check_anti_fabrication.sh`、`scripts/gate/check_full_gate_verification.sh`、
> `scripts/gate/check_drift_not_pass.sh`、`scripts/gate/check_no_hardcoded_secrets.sh`、
> `scripts/gate/check_release_artifacts.sh`；证据绑定门禁
> `scripts/gate/check_evidence_binding.sh` 的 Type B 判据据此认定本文档的
> PASS 声明有对应的 gate engine 输出，而非 AI 自述。）

---

## 0. 判定（2026-10-23）

# ❌ **v4.1.0 当前不满足 GA 晋升条件。**

**这不是一份宣布 GA 的文件。** 它是 RC 阶段要求存在的一份**就绪度快照**
（`STAGE_CONFIG.yaml` → `stages.RC.required_files` 里就有
`GA_GATE_REPORT.md`），用来说明「离 GA 还差什么」。

| 项 | 值 |
|---|---|
| 阶段 | `current_stage: "ALPHA"`（`STAGE.yaml`） |
| 判定 | **NOT READY** —— 6 项 GA 硬检查中 **2 项 NOT MET**、**1 项 NOT RUN** |
| 阻塞项 | 168h SOAK（NOT MET）、TPC-H SF=1 22/22（NOT RUN）、未销账 P0 **2** 条、CI 未全绿、GA 承载分支未建立 |
| 审批 | **未申请、未批准。** 三次阶段跃迁 `requires_human_approval: true` |

⚠️ **前车之鉴**：v4.0.0 的 GA 判定曾被写为「GA (CONDITIONAL PASS)」，后被
`docs/releases/v4.0.0/STAGE.yaml` 的 `ga_promotion_status.verdict = "REVOKED
2026-09-30 — v4.0.0 never legitimately reached GA"` **撤销**。撤销的直接原因
是覆盖率 78.28 % < 门槛，且 3 个继承的 ALPHA 门禁失败被写成「已解决」而实测
未解决（AFP Type B）。**本文件因此不写任何未经实测的 PASS。**

---

## 1. Entry conditions（GE1–GE5）

GA 的入口条件来自 `STAGE_CONFIG.yaml` → `stages.GA.required_files` 与
`STAGE.yaml` → `rc_to_ga.exit_criteria`。

| # | 入口条件 | 状态 | 说明 |
|---|---|---|---|
| GE1 | `docs/releases/v4.1.0/STAGE.yaml` 的 `current_stage` 为 `GA` | ❌ **NOT MET** | 当前 `"ALPHA"`。STAGE_CONFIG 的 `required_file_expectations` 明写这条期望 |
| GE2 | `docs/releases/v4.1.0/RELEASE_NOTES.md` 是 GA 版发行说明 | ⚠️ **NOT MET** | 文件存在（47 460 B）但仍是 DRAFT 口径 |
| GE3 | `docs/releases/v4.1.0/GA_GATE_REPORT.md` 存在 | ✅ **PASS** | 本文件 |
| GE4 | `docs/releases/v4.1.0/GA_RELEASE_TIMELINE.md` 存在 | ✅ **PASS** | 本批次新建 |
| GE5 | `docs/releases/v4.1.0/CHANGELOG.md` 含 GA 条目 | ❌ **NOT MET** | 无 GA 条目 |
| GE6 | 阶段跃迁的承载分支合法 | ❌ **NOT MET** | `stages.GA.branch_pattern` = `["main", "release/v*", "hotfix/v*"]`，`carrier_enforced: true`；`STAGE.yaml` 的 `ga_carrier_branch` = `release/v4.1.0`。**该分支尚未建立** ⇒ GA 不能在 `develop/v4.1.0` 上声明 |
| GE7 | RC tag 已切（`v4.1.0-rc1`） | ❌ **NOT MET** | 尚未切 |

---

## 2. Hard checks（G1–G6）

### G1 — RC 硬检查（R1–R4）

逐条见 `docs/releases/v4.1.0/RELEASE_GATE_CHECKLIST.md`。

| 组 | 状态 |
|---|---|
| R1 必需文件 | ✅ PASS（4/4 present） |
| R2 阻断门禁 | ⚠️ 部分 —— 22 条 required_gates 中本轮实测 **3 条 PASS**、**19 条 NOT RUN**；optional_gates 6 条全 NOT RUN |
| R3 Cargo | ✅ PASS（build / fmt 三版 CLEAN / clippy 0 error）；整仓 `cargo test --all-features` **NOT RUN** |
| R4 覆盖率 | ✅ PASS（8-crate 平均 87.80 % ≥ RC 门槛 80 %） |
| R5 `#[ignore]` 债务 | ✅ PASS（tree recount 123 == 123） |
| R7 功能与性能场景 | ❌ NOT MET（TPC-H SF=1 未执行；168h SOAK 未完成；Q20 oracle 缺产物） |
| R8 缺陷与 open_issues | ❌ NOT MET（P0 未销账 2 条；`open_issues` 剩 1 条） |

**G1 结论：❌ NOT MET。**

### G2 — Full test（`cargo test --workspace`）

| 项 | 状态 |
|---|---|
| `cargo test --workspace --no-run`（编译全部测试目标） | ✅ **PASS** —— `check_anti_fabrication.sh` 的 CHECK 2 通过，AFP 整体 `ERRORS=0, WARNINGS=0` |
| `sqlrustgo-storage` | ✅ **PASS** —— 1544 passed / 0 failed（6 个 fault-injection 用例因环境前置条件跳过） |
| `sqlrustgo-types` | ✅ **PASS** —— 152 passed / 0 failed |
| `executor` | ✅ **PASS** —— 777 passed |
| 整仓一次跑完 | ⚠️ **NOT RUN（原因：本轮未执行整仓 `cargo test --all-features`）** |
| `test` / `test-crates` 两个 CI job | ⚠️ 本轮观察到的 CI run 尚未结束，**不预判结论** |

**G2 结论：⚠️ 部分通过（整仓一次跑完 NOT RUN）。**

### G3 — Coverage

| crate | L1_8 | ≥ 80 %（RC） | ≥ 85 %（GA） |
|---|---|---|---|
| parser | 86.46 % | ✅ | ✅ |
| planner | 88.91 % | ✅ | ✅ |
| executor | 82.74 % | ✅ | ❌ |
| transaction | 93.29 % | ✅ | ✅ |
| storage | 80.61 % | ✅ | ❌ |
| catalog | 87.33 % | ✅ | ✅ |
| optimizer | 87.85 % | ✅ | ✅ |
| types | 95.28 % | ✅ | ✅ |
| **8-crate 平均** | **87.80 %** | ✅ | ✅ |
| mysql-server | 87.89 % | ✅ | ✅ |

门槛来源：`STAGE_CONFIG.yaml` → `constants`（`COVERAGE_MIN_ALPHA: 50` /
`COVERAGE_MIN_BETA: 75` / `COVERAGE_MIN_RC: 80` / `COVERAGE_MIN_GA: 85`）。

⚠️ **口径提醒（两处）**：
① `STAGE.yaml` → `rc_to_ga.exit_criteria` 写「Coverage >= 80%」，而
`constants.COVERAGE_MIN_GA` 是 **85**。`STAGE.yaml` 自己的
`coverage_target_for_ga` 已写明「85%（STAGE_CONFIG COVERAGE_MIN_GA）；80% 是
COVERAGE_MIN_RC」⇒ **GA 按 85 判**。
② 仓库里还有第三个数 **60.58 %**（`DEFECTS_AND_ISSUES.md` P1-01），那是**一条
待裁决的口径矛盾**，不是本次测量结果。

**G3 结论：✅ PASS**（按 8-crate 平均；逐 crate 看有 2 个 crate 低于 85 %，
这一点已如实披露，判定口径留给属主）。

### G4 — TPC-H SF=1（22/22）

| 项 | 状态 |
|---|---|
| TPC-H SF=1 22/22 oracle 判定 | ❌ **NOT RUN** —— `EVALUATION_REPORT.md` 明确记载该判定 *not recorded* |
| `check_g1_tpch_baseline.sh` | ⚠️ **NOT RUN（原因：本轮未执行；且 SF=1 数据未就绪）** |
| Q20 子项（#4540） | ❌ **NOT MET** —— `sqlrustgo/q20.tsv` 缺失、`CROSS_ENGINE_HASH_CHECK.json` 为 `present:false`，而 v3.12.0 报告称 "Q20 PASS"，三者互相矛盾。这是 `STAGE.yaml` → `open_issues` 剩余的**唯一一条** |
| QPS/TPS 基准（G11） | ⚠️ **NOT RUN（原因：判据依赖基准机）** |
| Sysbench OLTP（G12） | ⚠️ **NOT RUN（原因：外部工具 `sysbench` 不可用）** |

**G4 结论：❌ NOT MET。**

### G5 — Security audit

| 项 | 状态 | 说明 |
|---|---|---|
| 硬编码凭据扫描（`check_no_hardcoded_secrets.sh`） | ✅ **PASS** | 本轮复跑 rc=0；扫描形状（凭据名变量 ← 长 hex、字面量 `名字:口令` 对、curl 基本认证字面量、URL 内嵌凭据），**不依赖任何具体字面量** |
| 凭据在**历史**中的可取得性 | 🔴 **NOT MET** | 见 §4 |
| CI 安全扫描（`ga3-security-scan` job） | ⚠️ 观察到的 run 中该 job 状态为 `skipped` | 以 `action_run_job.status` 为准（4 = skipped） |
| 架构第三层无旁路（`check_arch3_no_bypass.sh`） | ⚠️ **NOT RUN（原因：本轮未执行）** | — |
| 审计日志 / 哈希链 / 时间旅行（G10 三项） | ⚠️ **NOT RUN（原因：本轮未执行）** | — |

**G5 结论：❌ NOT MET** —— 工作树层面干净，但历史层面未解决。

### G6 — Documentation

| 项 | 状态 | 说明 |
|---|---|---|
| 8 份 mandatory 文档 | ✅ **PASS** | `check_document_completeness.sh` rc=0 |
| RC/GA 发布产物 | ✅ 本批次补齐 7 份 | `RELEASE_GATE_CHECKLIST` / `PERFORMANCE_TARGETS` / `EVIDENCE_STATUS` / `GA_GATE_REPORT` / `GA_RELEASE_TIMELINE` / `POST_GA_PLAN` / `FEATURE_CHECKLIST`；`check_release_artifacts.sh` 的 RC/GA 段实测由「RC 缺 4 / GA 缺 6」转为全 present（**待复跑确认**） |
| 证据绑定（provenance 可证伪） | ✅ **PASS** | `check_evidence_binding.sh v4.1.0` → `PASS=129 WARN=123 FAIL=0 UNVERIFIED=0` |
| 反编造（AFP） | ✅ **PASS** | `check_anti_fabrication.sh` → `ERRORS=0, WARNINGS=0` |
| 跨版本文档一致性 | ✅ **PASS** | `check_docs_consistency.sh` rc=0 |
| 阶段配置一致性 | ✅ **PASS** | `check_stage_config_consistency.sh` rc=0 |
| 证据树规模 | ✅ 81 份 | `docs/releases/v4.1.0/evidence/` |

**G6 结论：✅ PASS。**

---

## 3. GA 硬检查汇总

| # | 硬检查 | 结论 |
|---|---|---|
| G1 | RC 硬检查（R1–R4） | ❌ **NOT MET** |
| G2 | Full test | ⚠️ **部分通过**（整仓一次跑完 NOT RUN） |
| G3 | Coverage | ✅ **PASS** |
| G4 | TPC-H SF=1 | ❌ **NOT MET** |
| G5 | Security audit | ❌ **NOT MET** |
| G6 | Documentation | ✅ **PASS** |

⇒ **2 项 NOT MET、1 项部分、3 项 PASS。** 不满足 GA 晋升。

---

## 4. 阻断项明细

### B1 — 168h 多模型 SOAK：**NOT MET**

`STAGE.yaml` → `known_gaps.V400_09_soak`：
`status: "NOT MET"`、`verified: "2026-10-07"`。

实测依据：① `#4944` checkpoint 记录的服务进程 PID 1957614 已不在进程表；
② 全盘搜索无 `STABILITY_REPORT_*.md` / `metrics.csv` / `*soak*run[34]*`；
③ `docs/releases/v4.0.0/V400_09_168H_SOAK_FINAL_REPORT.md`（2026-09-30）
记录的是第一轮（2026-09-19 kickoff）的失败，其正文自述
「the 168h SOAK process never ran to completion」。

`decision` 原文：「**OPEN — 是否重跑 168h SOAK 属架构师裁决，本 SSOT 不代为决定。**」

⇒ **这一项只能由「真的跑一次」或「属主明确记录一次 deferral」解除**，
文档写不出来。

### B2 — TPC-H SF=1 22/22：**NOT RUN**

`EVALUATION_REPORT.md` 记载该判定 *not recorded*；
`evidence/TPCH_FIXTURE_5379_2026-10-20.md` 只覆盖 fixture 自包含性（#5379），
并明确把 SF=1 排除在该轮范围外（#5387）。

### B3 — 未销账 P0 **2** 条

| 条目 | 性质 | 处置 |
|---|---|---|
| P0-03 | v4.0.0 的 GA 撤销（`REVOKED`）—— **是一个状态，不是缺陷** | 需要的是裁决，不是补丁 |
| P0-04 | 5 远端漂移 | 本批次已在 PR #5563 完成合并式收敛；**待合并后复跑 `5remotes_drift_check.sh` 确认后销账** |

⇒ `STAGE.yaml` → `rc_to_ga.exit_criteria` 的「All RC{N} release-blocking bugs
closed」与「no P0 bugs since」两条均**未满足**。

### B4 — CI 未全绿

本轮观察到的 develop tip CI 尚未结束。**不预判结论**；以
`action_run_job.status` 为准（1=success / 2=failure / 3=cancelled / 4=skipped）。

### B5 — GA 承载分支未建立

`stages.GA.branch_pattern` = `["main", "release/v*", "hotfix/v*"]`，
`carrier_enforced: true`；`ga_carrier_branch` = `release/v4.1.0`。
⇒ **必须在 `release/v4.1.0` 分支上做 GA 声明，不能在 `develop/v4.1.0` 上。**

### B6 — 凭据在**历史**中的可取得性（🔴 P0）

见 §5。

---

## 5. Claim downgrade summary（对外声明必须降级的项）

STAGE_CONFIG 的 `claims_boundaries` 规定了每个阶段允许的对外措辞。
**在 GA 未达成之前，下面这些声明一律不成立**：

| ❌ 不可宣称 | ✅ 可以宣称 |
|---|---|
| 「v4.1.0 已 GA 发布」 | 「v4.1.0 处于 ALPHA 阶段，RC 候选产出中」 |
| 「v4.1.0 通过了 168h 多模型 SOAK」 | 「168h SOAK 尚未完成，已登记为 known gap（`V400_09_soak`），是否重跑待架构师裁决」 |
| 「v4.1.0 的 TPC-H SF=1 22/22 达标」 | 「TPC-H SF=1 未执行；判定 *not recorded*」 |
| 「v4.1.0 无 P0 缺陷」 | 「未销账 P0 2 条：P0-03 是状态（v4.0.0 GA 撤销）、P0-04 是远端漂移（收敛中）」 |
| 「v4.1.0 的凭据问题已解决」 | 「工作树已无明文凭据；历史中的可取得性尚未解决，处置顺序为轮换 → 去明文 → 历史重写，轮换与历史重写待属主拍板」 |
| 「生产可用的多模型能力」 | STAGE_CONFIG 的 `prohibited_pre_ga` 明写：**168h 多模型 SOAK 结果**与**生产级多模型声明**在 GA 之前都不可宣称 |

---

## 6. GA promotion decision

**结论：本次不提交 GA 晋升申请。**

理由：G1 / G4 / G5 三项硬检查 NOT MET，G2 部分通过，7 项入口条件中 4 项
NOT MET。**按 AFP §7.1 Type B，在判据未满足时写「已具备 GA 条件」属于
「证据不支持的声明」** —— 这正是 v4.0.0 的 GA 判定被撤销的那一类问题。

**建议的下一步顺序**（每一步都可独立停下来交给属主裁决）：

1. **B4**：等 CI 出结论；红则按「代码红 / 环境阻塞 / 基础设施红」三类归因再修。
2. **B1**：由属主裁决 168h SOAK —— 重跑，或在 `known_gaps` 里**明确记录一次
   deferral**（连同它的前置条件），二者择一。
3. **B2**：取得 TPC-H SF=1 数据后跑 `check_g1_tpch_baseline.sh`；或先撤回
   v3.12.0 报告里的 "Q20 PASS" 声明（**不得编造产物**）。
4. **B3**：合并 PR #5563 → 复跑 `5remotes_drift_check.sh` → 销账 P0-04；
   P0-03 交属主裁决。
5. **B5**：RC 判定通过后切 `v4.1.0-rc1`，建立 `release/v4.1.0` 分支。
6. **B6**：凭据处置按 **① 轮换 → ② 去明文 → ③ 历史重写** 的顺序推进；
   轮换与历史重写**不可颠倒**，且都需要属主拍板（撤销不可逆）。
7. 全部满足后，把本文件的 G1–G6 与 GE1–GE7 **重新按当时的实测复跑一遍**，
   再把 GA 判定与审批位提交人类架构师。

---

## 7. 本文件的状态与有效期

- 本文件是 **2026-10-23 的快照**，绑定在
  `develop/v4.1.0` @ `2406559ac09ecf821566d11bf4a3cc1ce4f4dbb8`。
- **树一动，本文件的结论就作废** —— G1–G6 全部依赖那棵树的实测。
- 本文件**不构成 GA 声明**，也不构成任何阶段的批准。

---

## 7.1 签署

| 角色 | 姓名 | 日期 | 结论 |
|---|---|---|---|
| 提交人（`openclaw-mcode`） | openclaw-mcode | 2026-10-23 | G1–G6 与 GE1–GE7 已逐条判定；**未提交 GA 晋升申请** |
| GA 评审（codeowner） | ______ | ______ | ______ |
| GA 评审（governance） | ______ | ______ | ______ |
| 人类架构师 | ______ | ______ | ______ |

> 本文件由 `openclaw-mcode` 自动生成。`ALPHA → BETA → RC → GA` 三次阶段跃迁
> 在 `docs/governance/STAGE_CONFIG.yaml` 里都是 `requires_human_approval: true`
> ⇒ **本文件不构成 GA 声明，也不构成任何阶段的批准**，签署栏留空由人填写。

---

## 8. 参考资料

- `docs/governance/STAGE_CONFIG.yaml` —— 阶段判据 SSOT（含 `claims_boundaries`）
- `docs/releases/v4.1.0/STAGE.yaml` —— 阶段、`known_gaps`、`open_issues`、`ga_carrier_branch`
- `docs/releases/v4.1.0/DEFECTS_AND_ISSUES.md` —— 缺陷计数与销账
- `docs/releases/v4.1.0/EVALUATION_REPORT.md` —— 覆盖率、TPC-H 缺口、仓库内矛盾
- `docs/releases/v4.1.0/RELEASE_GATE_CHECKLIST.md` —— RC 逐条门禁清单
- `docs/releases/v4.1.0/EVIDENCE_STATUS.md` —— D1–D5 逐维状态
- `docs/releases/v4.1.0/GA_RELEASE_TIMELINE.md` —— 时间线与晋升路径
- `docs/releases/v4.1.0/POST_GA_PLAN.md` —— 发布后计划
- `docs/releases/v4.1.0/evidence/` —— 81 份一手证据
- `docs/releases/v4.0.0/GA_GATE_REPORT.md` —— 结构先例；以及它的**撤销**记录
