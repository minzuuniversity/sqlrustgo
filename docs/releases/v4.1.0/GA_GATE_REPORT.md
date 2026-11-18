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
<!-- BEGIN 2026-11-17 REFRESH BLOCK

## 🔄 刷新块（2026-11-18T05:00:55Z）—— 取代下方 2026-10-23 的判定

> **基线**：`develop/v4.1.0` @ `e5c42c7fd02d3d2e24086d419920bcea938c19d6`（tree `761e3491a8ce91b89d8d79f60fa0f906fc3b09d5`）—— 当前 develop tip
> **认证轮**：run `20261117-ga-gates-011`（plan sha256 `sha256:6466a0ec156075daf3bc53ece379c4ecc2b13b63da27cd17a1f39cb2254da809`），
> 被测树 `fa2b41abfa6b`；其 `git_head` 与**当时**的基线一致 ⇒ **那一轮测的是它自己的被交付树**。
> ⚠️ **但 `fa2b41abfa6b` 的 crates 与当前 tip 差一个 KILL CONNECTION 修复（#5842，`crates/mysql-server/src/lib.rs` +131 行）**
> ⇒ **35/35 的结论不能直接继承到当前 tip**（见 GA-REWIND-1 的剩余义务）——
> 最终认证轮仍须在 `e5c42c7fd02d3d2e24086d419920bcea938c19d6`（或其后继 tip）上重跑。
> **材料提交**：本块随材料提交 M 一起落地；M 只新增 `docs/` 与 `audit/`
> （`git diff --stat e5c42c7fd02d3d2e24086d419920bcea938c19d6..M -- crates scripts tests` 为空 ⇒ M 不改变被测代码面）
> **本块刷新说明**：上一版刷新块为 2026-11-17T15:13:56Z；本版只刷新本块内容，
> 下方 2026-10-23 的判定与第 9 段**原样保留为历史记录**，签署栏**仍留空**。

### 判定：**NOT READY —— 只剩物理与人类约束**

| 项 | 值 |
|---|---|
| 阶段 | `current_stage: "ALPHA"`（`STAGE.yaml`）—— **未跃迁** |
| GA 门禁 | **35/35 项：PASS 35 / FAIL 0 / INCOMPLETE 0 / 其他 0**（run `20261117-ga-gates-011`，被测树 `fa2b41abfa6b`）。⚠️ **该树的 crates 与当前 tip 差一个 KILL CONNECTION 修复（#5842）** ⇒ 最终认证轮仍须在 `e5c42c7fd02d3d2e24086d419920bcea938c19d6`（或其后继 tip）上重跑（见 GA-REWIND-1） |
| 覆盖率 | `✓ PASS A5 Coverage: 87.69% (min: 85%, measured 8/8 crates)`（min 85%，measured 8/8 crates）—— 真实测量 |
| 168h SOAK | **权威轮已在独占窗口内起跑**（run `20261118-168h-v410`，冻结提交 `e5c42c7fd02d3d2e24086d419920bcea938c19d6`，起跑 2026-11-18T03:56:06Z，计划结束 2026-11-25T03:56:06Z）。上一轮诊断轮 `20261116-168h-v410`（attempt-33）**已出局**：累计 644,800 查 / 20,850 错 ⇒ 3.234 %（> 1 % 阈值，且远超历史 0 错门槛），panic **0**，心跳 max_gap 24.0 s ⇒ 仓内校验器 PASS；诊断轮在独占窗口出现后**主动中止**，为权威轮让位（独占窗口下两轮不能并存）。⚠️ **2026-11-18T05:06:49Z 起 252 整机脱网**（掉线前最后一笔实测 2026-11-18T04:56:23Z：138,029 查 / 0 错、panic 00、双臂存活）⇒ **该轮生死待判**（网络闪断 vs 整机重启取决于恢复后的心跳连续性，见 §7.19 与 `GA-SOAK-14`）⇒ 该项**既不是 PASS 也不是 FAIL**，仍是 **NOT-RUN**；合格判据 = **168 h 内 0 错 + 0 panic**（见 B1b）。⚠️ **后续判定**：attempt-11 已于 2026-11-18T05:02:03Z 被**宿主整机重启**打断（boot_id / 内核 7.0.0-42-generic→7.0.0-43-generic / 心跳与 boot 结束差 3 s 三项独立对上）⇒ 终态 elapsed **4,130 s（0.68 %）**、139,568 查 / 0 错、panic 00、双臂存活 ⇒ 如实记 **INCOMPLETE**（非跑满、非通过、非被测缺陷），已归档于 `audit/v4.1.0/runs/20261118-168h-v410/`（见 §7.21 / `GA-SOAK-16`）。**重跑已就位**：守望器按同一把尺等「root 编译进程 0 且 load1 ≤ 40 连续 ≥2 h」，成立即自动起跑（当下 root 编译进程 5 个、load1 18.40 ⇒ **尚非独占窗口，不抢跑**） |
| 审批 | **未申请、未批准。** `STAGE_CONFIG` 的 `transitions.RC_to_GA.requires_human_approval: true`；`manual_only_criteria` 明文：「批准记录写进本文件的变更记录，**不能由 Agent 生成**」 |

### 非绿项明细

无（35 项门禁全部 PASS）。**但 168h SOAK 尚未跑满、且起跑后宿主整机脱网 ⇒ 那一项仍未取得**，
不得记为 PASS，**也不得记为 FAIL**（是环境事件，不是被测系统的判定）。

### 硬检查现状（G1–G6）

| # | 硬检查 | 结论 | 依据 |
|---|---|---|---|
| G1 | RC 硬检查（R1–R4） | ✅ MET | run 20261117-ga-gates-011：35/35 项全 PASS；D1 Result: 10/10 (not-run: 0, blockers: 0)；D2 Result: 6/6；D3 Result: PASS=5 | FAIL=0 | DRIFT=0；D4 Result: 5/5 passed (failures: 0, exit 0)；D5 Result: 7/10 |
| G2 | Full test | ✅ MET | run 20261117-ga-gates-011 seq33 `cargo test --all-features` **PASS**（exit=0，耗时 968.9 s） |
| G3 | Coverage | ✅ PASS | run 20261117-ga-gates-011 seq1 实测行：`✓ PASS A5 Coverage: 87.69% (min: 85%, measured 8/8 crates)`（8-crate 平均；逐 crate 见 §7.13） |
| G4 | TPC-H SF=1 22/22 | ⚠️ **NOT MET（已知且已登记）** | SF=1 的 22/22 用例在 `#[ignore]` 下（ADR-008 例外，**已过期且从未签署**）；当前 SSOT（`STAGE_CONFIG`）各阶段要求的是 `check_g1_tpch_baseline.sh`（hermetic SF=0.001，**由独立 SQLite oracle 逐条担保**，实测 `G1 PASS: hash matches (7439ce80…)`）—— 已 MET |
| G5 | Security audit | ❌ NOT MET | 工作树层面已收敛（凭据 SSOT + `check_no_hardcoded_secrets.sh` 已接 CI）；**历史层面未解决**：旧 Gitea token 仍有效（ENG-8）⇒ 需属主在服务端删除/轮换 |
| G6 | Documentation | ✅ PASS | 6 份 GA doc_artifacts 齐备（`check_release_artifacts.sh` ⇒ PASS，GA 段 11 份 present）；看板 183 行、`kanban_render.py check` 一致、`kanban_merge_policy` verdict OK |

### 阻断项（剩余全部为物理 / 人类约束）

| # | 阻断 | 归属 |
|---|---|---|
| B1 | 168h SOAK 的**合格**证据需要跑满 168 h —— 权威轮已于 2026-11-18T03:56:06Z 在**实测独占窗口**内起跑（root 编译进程连续 ≥2 h 为 0，见 `GA-SOAK-13`），但**尚未跑满** ⇒ 该项仍未取得 | 执行（起跑完成；剩余物理时长） |
| B1a | **252（Z6G4）整机脱网 —— 已判定为宿主重启** —— 2026-11-18T05:06:49Z 起不可达，2026-11-18T06:30:52Z 恢复；根因 = **宿主整机重启**（boot_id `1287e51dc1494edaae9fce1c583b72f9`、内核 `7.0.0-42-generic → 7.0.0-43-generic`、`unattended-upgrade（2026-11-18 06:57:18Z 处理 linux-hwe-7.0-headers-7.0.0-38）`），权威轮 attempt-11 因此在 2026-11-18T05:02:03Z 中止 ⇒ **INCOMPLETE**（归档见 §7.21 / `audit/v4.1.0/runs/20261118-168h-v410/`） | 已判定；**重跑由守望器自动执行** |
| B1b | **「独占窗口的合格错误率阈值」已裁定（2026-11-18T05:47:26Z）：维持 0 errors / 0 panics（不改判据、不抬阈值、不改 35 项门禁分母）** —— 门槛原样保留（debt #3265/#3266），**本次提交零判据变更**。⚠️ **裁定的后果**：实测安静档（`load1<4`）错误率仍为 **0.111 %** ⇒ **「独占窗口」是必要条件、不是充分条件**；权威轮合格判据 = 168 h 内 **0 错 + 0 panic**，带错结束即不是 GA 证据（见 §7.20 / `GA-SOAK-15`） | 已裁定（属主，2026-11-18T05:47:26Z）；后续「换一台 0 错节点」与「是否给书面例外」**仍是属主决策**（Agent 不代做） |
| B1c | **在 252 上禁用 `unattended-upgrade` 的自动内核升级/自动重启** —— attempt-11 就是被它打断的（内核 `7.0.0-42-generic → 7.0.0-43-generic` 触发重启，停机 88.7 分钟）；**不处理则重跑仍会被同样打断** | **属主**（Agent 无 root，改不了该配置） |
| B1d | **gitea 侧 `refs/heads/` 下 22 个 ref 目录曾为 `root:root`** ⇒ 带斜杠的新分支无法创建 lock（`cannot lock ref … Permission denied`）；本 PR 改用无斜杠分支名 `docs-v410-soak-daily-20261118` 发布（PR #5855），**未动属主服务器**；该批目录属主现已恢复为 `git:git` | 已绕过；**属主可自行确认**（后续 PR 可正常用 `docs/*` 命名） |
| B2 | 人类架构师签批本报告 | **属主**（`requires_human_approval: true`） |
| B3 | GA 承载分支 `release/v4.1.0` + tag + 并入 main | **属主**（按 `RELEASE_LIFECYCLE` §2.5：审批 → tag → 并入 main → 建承载分支，顺序不可越） |
| B4 | 旧 Gitea token 在服务端失效 | **属主**（仓库侧已收敛） |
| B5 | 两条已过期且从未签署的 ADR-008 例外 + 61 道死门禁逐个处置 | **属主**（P0-4：逐个门禁处置必须属主决定） |

### 签署（**本栏留空 —— 签署是人的动作，Agent 不代签**）

```
GA gate report reviewed and approved by human architect:

  姓名 / 身份：______________________________
  签署时间（UTC）：______________________________
  结论（PASS / CONDITIONAL / NOT APPROVED）：______________________________
  备注（若为 CONDITIONAL，必须逐条列出条件与到期日）：______________________________
```

<!-- END 2026-11-17 REFRESH BLOCK -->

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

门槛来源：`STAGE_CONFIG.yaml` → `thresholds`（`COVERAGE_MIN_ALPHA: 50` /
`COVERAGE_MIN_BETA: 75` / `COVERAGE_MIN_RC: 80` / `COVERAGE_MIN_GA: 85`）。

⚠️ **口径提醒（两处）**：
① `STAGE.yaml` → `rc_to_ga.exit_criteria` 写「Coverage >= 80%」，而
`thresholds.COVERAGE_MIN_GA` 是 **85**。`STAGE.yaml` 自己的
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

---

## 9. 2026-11-16 追加段：GA 门禁基线自有化重盘与实跑状态

> 依 `docs/governance/DOC_CHECK_CORRECTION_RULES.md`：**上文原始记录一字未删**，
> 本段只追加实测到的内容（不订正上文任何一句，因为上文没有与实测冲突的表述）。

### 9.1 基线升级裁决（用户「4.0.0 基线是否升级为 4.1.0 自有基线」的答复）

**裁决：升级 —— 判据继承、清单全量重盘。**

- v4.0.0 的 GA 判定已于 2026-09-30 撤销（`docs/releases/v4.0.0/STAGE.yaml` →
  `ga_promotion_status.verdict = REVOKED — v4.0.0 never legitimately reached GA`），
  其继承清单**不得**充当 v4.1.0 的合格证据；仍有效的技术判据继续沿用。
- v4.1.0 的 `STAGE.yaml` 三段 `required_gates_for_beta/_rc/_ga` 此前只是
  「Deferred / 继承」占位 note，且**全仓无任何消费方**（实测
  `grep -rn "required_gates_for" scripts/ docs/governance/` 零命中 ⇒ 执行面只读
  `docs/governance/STAGE_CONFIG.yaml`）⇒ 现已换成**实际清单镜像**：
  **BETA 19 项 / RC 26 项 / GA 35 项**（31 脚本门禁 + 4 条 cargo 命令）。
- 168 道门禁逐条归属基线：`docs/releases/v4.1.0/GATE_BASELINE_V410.jsonl`（168 行，
  与 `gov_probe.metrics.check_n=168` 严格对齐）。
  **REQUIRED 134 / HISTORICAL 32 / RETIRED 2 / NOT_APPLICABLE 0**；
  execution_point：STAGE_LIST 34 / CI_DECLARED 51 / **NONE_PENDING 37** /
  **NONE_BLOCKED_EXTERNAL 12**；`ga_critical` = 31。
  ⇒ **49 道门禁「应在 v4.1.0 范围内执行但当前无执行点」**；
  其中含 `check_audit_manifest.sh`、`check_security.sh`、`check_architecture.sh`、
  `check_test_count_monotonic.sh`、`check_a04_evidence_reproducible.sh`、
  `check_attack_surface.sh`。

### 9.2 存量红清零（本轮）

| 门禁 | 改前 | 改后 | 根因 |
|---|---|---|---|
| `cargo fmt --check --all` | rc=1（10 处 / 6 文件） | **rc=0** | 纯格式漂移（已复验 `git diff -w` 仅 2 处非空白且均为 rustfmt 语义等价改写） |
| `scripts/gate/check_commit_message.sh` | rc=1（1 条 BAD_SHAPE） | **rc=0** | **类型表不完整**：全历史实测 `evidence` 32 次 / `bench` 51 次 / `spec` 43 次都在用却不在表内 ⇒ L2 把合规提交判成 BAD_SHAPE。按 2026-11-08 补 `audit` 的**同型先例**补入规范表与机器镜像（不放宽形态规则、不动 L1、不动分母） |
| `scripts/gate/check_architecture_freeze.sh` | rc=0 但**自报 finding**（A7-4 读到 0 个 DriftGate 测试） | **rc=0 且 PASS 5/5** | 两处：①「部分检查未通过」仍 `exit 0`（fail-open）⇒ 三态化；② A7-4 只扫 `crates/*/tests/`，漏掉 src 内 `#[cfg(test)]` ⇒ **测量假阴性**（DriftGate 的负面测试确实存在于 `crates/executor/src/execution/drift_gate.rs`） |
| `scripts/gate/check_int_debt.sh` | rc=0 但**可被绕过** | rc=0（正常态） | 两处 fail-open：① 状态列未知时无条件回退到「整行扫关键字」；② `FAILED_ITEMS` 收集失败却**从不参与退出码判定** ⇒ 债务行被改名/删除时门禁仍报 PASS |

后两门的 fail-open 闭合有**反向变异证明**：fx06 / fx09 / fx10 三道负向夹具实测
全部 FIXTURE-PASS（门禁读数分别为 rc=2 / rc=1 / rc=1），两条特征化夹具的期望随之
由 `green-characterization` 翻转为 `red`。

### 9.3 实跑状态（run `20261116-ga-gates-002`）

- 冻结提交：`7e26c2b7c6840e3bbec804a0bf3bbcc706a646d7`
- 计划：`plan.jsonl`，35 项，plan sha256 `sha256:6466a0ec156…`
- 执行器：`scripts/gate/run_gates.py`（三态：exit 0 ⇒ PASS；exit 1 ⇒ FAIL；
  其它（2/124/信号/脚本缺失/无法启动）⇒ **INCOMPLETE**）
- **逐项结果以该 run 的 `results.jsonl` 为准**（本段不预写结论）。

### 9.4 结论

**GA 今日未发布。** 四条硬约束（SOAK-168H / HUMAN-SIGNOFF / CARRIER / BASELINE-GAP）
见 `GA_RELEASE_REPORT.md` §3；阶段治理与签批行见 `STAGE_GOVERNANCE_RC_TO_GA.md`。

> 本段由 `mcode-macmini` 线 2026-11-16 追加。证据：PR #5804 / #5805 / #5806；
> 归档 run `20261116-ga-gates-002`；看板行 `GA-BASELINE-1`。
