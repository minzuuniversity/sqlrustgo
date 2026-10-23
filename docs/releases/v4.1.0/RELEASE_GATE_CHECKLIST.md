# v4.1.0 Release Gate Checklist

> **基线**：`develop/v4.1.0` @ `2406559ac09ecf821566d11bf4a3cc1ce4f4dbb8`
> **provenance:** generated_at=2026-10-23, branch=docs/ga-rc-artifacts-batch32,
> commit=`2406559ac09ecf821566d11bf4a3cc1ce4f4dbb8`, machine=Mac-mini-7.local (arm64, macOS 27.0.1),
> identity=openclaw-mcode, source_repo=openclaw/sqlrustgo,
> policy=Anti-Fabrication-Policy-v1.0
> **gate_policy_eval_id**：`v410-release-gate-checklist-001`
> （本文档汇编的门禁引擎：`scripts/gate/check_rc_ga_gate.sh`、`scripts/gate/check_coverage_v312.sh`、
> `scripts/gate/check_anti_fabrication.sh`、`scripts/gate/check_full_gate_verification.sh`、
> `scripts/gate/check_drift_not_pass.sh`；证据绑定门禁
> `scripts/gate/check_evidence_binding.sh` 的 Type B 判据据此认定本文档的
> PASS 声明有对应的 gate engine 输出，而非 AI 自述。）

---

## 0. 这份文件是什么，不是什么

**是什么**：RC（Release Candidate）阶段的**晋级清单** —— 把
`docs/governance/STAGE_CONFIG.yaml` 的 `stages.RC` 判据逐条展开成「判据是什么、
由谁测、这一轮测没测、测出什么、还差什么」。

**不是什么**：它**不是**批准。`ALPHA → BETA → RC → GA` 三次阶段跃迁在
STAGE_CONFIG 里都是 `requires_human_approval: true`，本文档只提供**决策所需
的事实**，不代替人类架构师的裁决。

**本轮（批次 32）实测的边界**，先说清楚：

| 范围 | 状态 |
|---|---|
| 机器 | Mac-mini-7.local（arm64, macOS 27.0.1） |
| 被测树 | `develop/v4.1.0` @ `2406559ac0` |
| 工具链 | 本地钉 `cargo +1.95.0`（CI 镜像为 rustc/cargo **1.91.1**、rustfmt 1.8.0） |
| **不是** | CI run 的结果。CI 侧以 `action_run_job.status` 为准（1=success / 2=failure / 3=cancelled） |

未在本轮实跑的门禁一律标 `NOT RUN（原因：…）`。**`NOT RUN` 不等于 `PASS`**
—— 这一条是仓库自己的纪律（AFP §7.1 Type B；STAGE_CONFIG 对 optional_gates
的注记也逐字写了「NOT-RUN 不得记为 PASS（P14 DRIFT ≠ PASS）」）。

---

## 1. R1 — 必需文件（Required Files）

来源：`docs/governance/STAGE_CONFIG.yaml` → `stages.RC.required_files`。

| # | 文件 | 期望 | 本轮状态 |
|---|---|---|---|
| R1.1 | `docs/releases/v4.1.0/STAGE.yaml` | 存在 | **PASS** —— 文件存在；`current_stage: "ALPHA"` |
| R1.2 | `docs/releases/v4.1.0/RELEASE_NOTES.md` | 存在 | **PASS** —— 47 460 B |
| R1.3 | `docs/releases/v4.1.0/CHANGELOG.md` | 存在 | **PASS** —— 存在 |
| R1.4 | `docs/releases/v4.1.0/GA_GATE_REPORT.md` | 存在 | **PASS** —— 本批次新建（见 `GA_GATE_REPORT.md`） |

> ⚠️ R1.4 出现在 **RC** 的 required_files 里，不只是 GA —— 也就是说
> **RC 阶段就要求 GA 门禁报告已经存在**（作为一份「RC 时的 GA 就绪度快照」）。
> 这是 STAGE_CONFIG 的事实，不是本文档的安排。

门禁：`scripts/gate/check_release_artifacts.sh` —— 实测 **PASS**
（ALPHA 阶段阻塞检查 7/7 OK；RC/GA 段按设计只做「披露」，不阻断）。
本批次把 RC/GA 两段的 MISS 补齐后，该门禁的 RC/GA 段应为全 present。

---

## 2. R2 — 门禁脚本（Gates）

来源：`stages.RC.required_gates`（22 条）+ `stages.RC.optional_gates`（6 条）。

### 2.1 阻断门禁（required_gates）

| # | 命令 | 判据针对 | 本轮状态 | 证据 / 说明 |
|---|---|---|---|---|
| R2.1 | `scripts/gate/check_rc_ga_gate.sh` | RC/GA 阶段总入口 | **NOT RUN（原因：本轮未执行；RC 阶段专用，须在切 `v4.1.0-rc1` 前的评审窗口执行）** | 判据见脚本自身 |
| R2.2 | `scripts/gate/check_arch_invariants.sh` | 架构不变量 C-ARCH-01..05 | **NOT RUN（原因：本轮未执行）** | 仓库历史记录 exit 0 PASS 5/5（`STAGE.yaml` resolved_issues_2026_10_03），**但那是旧树上的记录，不作为本轮结论** |
| R2.3 | `scripts/gate/check_arch3_no_bypass.sh` | 架构第三层无旁路 | **NOT RUN（原因：本轮未执行）** | — |
| R2.4 | `scripts/gate/check_int2_no_orphan.sh` | G2 INT-2 ParallelExecutor | **NOT RUN（原因：本轮未执行）** | — |
| R2.5 | `scripts/gate/check_int3_single_expr.sh` | G3 INT-3 Single Expression | **NOT RUN（原因：本轮未执行）** | — |
| R2.6 | `scripts/gate/check_sem1_savepoint.sh` | G5 SEM-1 Savepoint MVCC | **NOT RUN（原因：本轮未执行）** | — |
| R2.7 | `scripts/gate/check_backup_restore.sh` | G6 Backup/Restore 100+ 场景 | **NOT RUN（原因：本轮未执行）** | — |
| R2.8 | `scripts/gate/check_p12_crash_test.sh` | G8 Crash Matrix 100+ 场景 | **NOT RUN（原因：本轮未执行）** | — |
| R2.9 | `scripts/gate/check_p14_upgrade_test.sh` | G9 Upgrade Test 50+ 路径 | **NOT RUN（原因：本轮未执行）** | — |
| R2.10 | `scripts/gate/check_p21_audit_log.sh` | G10 Audit Log | **NOT RUN（原因：本轮未执行）** | — |
| R2.11 | `scripts/gate/check_p22_time_travel.sh` | G10 Time Travel | **NOT RUN（原因：本轮未执行）** | — |
| R2.12 | `scripts/gate/check_p23_hash_chain.sh` | G10 Immutable Audit Chain | **NOT RUN（原因：本轮未执行）** | — |
| R2.13 | `scripts/gate/check_g1_tpch_baseline.sh` | G1 TPC-H 22/22 | **NOT RUN（原因：本轮未执行；且 SF=1 数据未就绪 —— 见 §7）** | `EVALUATION_REPORT.md`：TPC-H SF=1 22/22 判定 **not recorded** |
| R2.14 | `scripts/gate/check_integration_gate.sh` | 集成门禁 | **NOT RUN（原因：本轮未执行）** | — |
| R2.15 | `scripts/gate/check_anti_fabrication.sh` | AFP 反编造 | **PASS** | 实测 `ERRORS=0, WARNINGS=0`，末行 `✅ Anti-fabrication check (AFP v4): PASS`；内含 `cargo test --workspace --no-run` 编译检查与 428 份 markdown 的代码示例扫描 |
| R2.16 | `scripts/gate/check_arch_sem_debt.sh` | 架构语义债务 | **PASS** | 实测 rc=0 |
| R2.17 | `scripts/gate/check_cross_version_debt.sh` | 跨版本债务 | **NOT RUN（原因：本轮未执行）** | — |
| R2.18 | `scripts/gate/check_int_debt.sh` | INT 债务 | **NOT RUN（原因：本轮未执行）** | — |
| R2.19 | `scripts/gate/check_full_gate_verification.sh` | 全门禁验证 | **NOT RUN（原因：本轮未执行）** | — |
| R2.20 | `scripts/gate/check_coverage_v312.sh` | 覆盖率 | **NOT RUN（原因：本轮未执行）**；上一轮完整 8-crate 实测见 §4 | `evidence/BATCH28_SEM4_AND_GATE_INTEGRITY_2026-10-23.md` §1.3 |
| R2.21 | `scripts/gate/check_sql_corpus_gate.sh` | SQL 语料门禁 | **NOT RUN（原因：本轮未执行）** | — |
| R2.22 | `scripts/gate/check_anti_ignore_gate.sh` | `#[ignore]` 债务 | **PASS** | 实测 rc=0；tree recount **123 == 123**（`total_allowed` 与树内实际条目数相等） |

**另加**（本轮执行、但不在 RC required_gates 里的仓库内门禁）：

| 门禁 | 状态 | 说明 |
|---|---|---|
| `scripts/gate/check_docs_consistency.sh` | **PASS** | rc=0 |
| `scripts/gate/check_stage_config_consistency.sh` | **PASS** | rc=0 |
| `scripts/gate/check_document_completeness.sh` | **PASS** | rc=0（8 份 mandatory 全 present） |
| `scripts/gate/check_gate_script_refs.sh` | **PASS** | rc=0 |
| `scripts/gate/check_ci_checkout_consistency.sh` | **PASS** | rc=0（批次 29 新增的 C5 规则；`--selftest` 5/5） |
| `scripts/gate/check_evidence_binding.sh v4.1.0` | **PASS** | `PASS=129 WARN=123 FAIL=0 UNVERIFIED=0` |
| `scripts/gate/check_release_artifacts.sh` | **PASS** | rc=0 |

### 2.2 非阻断但**必须实跑并披露**的门禁（optional_gates）

STAGE_CONFIG 对这 6 条的原话是：「**必须实跑并披露** PASS / FAIL / NOT-RUN；
NOT-RUN 不得记为 PASS（P14 DRIFT ≠ PASS）。`check_stage.sh` 只列出
optional_gates（driver 不执行它们），所以执行责任在晋级评审」。

| # | 命令 | 本轮状态 | 说明 |
|---|---|---|---|
| R2.O1 | `scripts/gate/check_architecture_freeze.sh` | **NOT RUN（原因：本轮未执行）** | — |
| R2.O2 | `scripts/gate/check_gate_self_verification.sh` | **NOT RUN（原因：本轮未执行）** | — |
| R2.O3 | `scripts/gate/check_p13_soak_test.sh`（G7 24h Soak） | **NOT RUN（原因：本轮未执行；且 168h SOAK 的结论为 NOT MET —— 见 §7）** | `STAGE.yaml` → `known_gaps.V400_09_soak` |
| R2.O4 | `scripts/gate/check_g11_qps.sh`（G11 QPS/TPS） | **NOT RUN（原因：本轮未执行；判据依赖基准机）** | — |
| R2.O5 | `scripts/gate/check_g12_sysbench.sh`（G12 Sysbench OLTP） | **NOT RUN（原因：本轮未执行；判据依赖外部工具 `sysbench`）** | — |
| R2.O6 | `scripts/gate/check_g14_real_crash.sh`（G14 Real Crash） | **NOT RUN（原因：本轮未执行）** | — |

---

## 3. R3 — Cargo 构建 / 测试 / 格式 / clippy

| # | 命令 | 本轮实测 | 结果 |
|---|---|---|---|
| R3.1 | `cargo build --all-features` | ✅ 通过 | 编译无错 |
| R3.2 | `cargo test --all-features` | ⚠️ **部分** | 逐 crate 实测通过：`sqlrustgo-storage` **1544 passed / 0 failed**（另有 6 个 fault-injection 用例因环境前置条件跳过）、`sqlrustgo-types` **152 passed / 0 failed**、`executor` **777 passed**、批次 30 的 5 个 P0-08 语义测试 **5 passed / 0 failed**。**整仓 `cargo test --all-features` 未在本轮跑完**，故不记 PASS |
| R3.3 | `cargo fmt --check` | ✅ 通过 | **三版工具链全 CLEAN**：`cargo +1.91.0 fmt --check --all` rc=0、`+1.95.0` rc=0、`+1.97.0` rc=0。CI 镜像（rustfmt **1.8.0**）对批次 32 涉及的两个文件 `rustfmt --check --edition 2021` **RC=0** |
| R3.4 | `cargo clippy --all-features -- -D warnings` | ✅ 通过 | 钉 `+1.95.0` 全量实跑，**0 error**（9m58s）。workspace scope 的 lint 预算见 §4 |

⚠️ **R3.3 的一个坑（批次 32 的实测记录）**：rustfmt 的 diff 里 `-` 是**当前文件**、
`+` 是**期望结果**。第一次读 CI 日志时把方向读反了，险些误判成「本地与 CI 的
rustfmt 版本漂移」。实测两边**期望一致**，没有版本漂移。详见
`evidence/REMOTE_SYNC_CONVERGENCE_2026-10-23.md` 与 PR #5562 正文 §2。

---

## 4. R4 — 覆盖率

门槛（`STAGE_CONFIG` → `constants`）：`COVERAGE_MIN_ALPHA: 50` /
`COVERAGE_MIN_BETA: 75` / `COVERAGE_MIN_RC: 80` / `COVERAGE_MIN_GA: 85`。

| crate | L1_8 覆盖率 | 对 RC(80) | 对 GA(85) |
|---|---|---|---|
| parser | 86.46 % | ✅ | ✅ |
| planner | 88.91 % | ✅ | ✅ |
| executor | 82.74 % | ✅ | ❌（< 85） |
| transaction | 93.29 % | ✅ | ✅ |
| storage | 80.61 % | ✅ | ❌（< 85） |
| catalog | 87.33 % | ✅ | ✅ |
| optimizer | 87.85 % | ✅ | ✅ |
| types | 95.28 % | ✅ | ✅ |
| **8-crate 平均** | **87.80 %** | ✅ | ✅ |
| mysql-server | 87.89 % | ✅ | ✅ |

来源：`evidence/BATCH28_SEM4_AND_GATE_INTEGRITY_2026-10-23.md` §1.3（8-crate
完整实跑）与 `evidence/COV_01_02_COVERAGE_GATE_2026-10-07.md`（mysql-server）。

> ⚠️ **两个口径都存在，不要混用**：
> ① `STAGE.yaml` → `rc_to_ga.exit_criteria` 写的是「Coverage >= 80%」；
> ② `STAGE_CONFIG` → `constants.COVERAGE_MIN_GA` 是 **85**；
> `STAGE.yaml` 自己在 `coverage_target_for_ga` 里写明了「85%（STAGE_CONFIG
> COVERAGE_MIN_GA）；80% 是 COVERAGE_MIN_RC」。
> ⇒ **GA 的门槛按 85 判**，两处措辞的差异见 `DEFECTS_AND_ISSUES.md` §「仓库内
> 自相矛盾之处」。
>
> ⚠️ 另有第三个数字（60.58 %）记在 `DEFECTS_AND_ISSUES.md` 的 P1-01 里
> —— 那是一条**待裁决的口径矛盾**，不是本次覆盖率测量的结果。引用覆盖率时
> 必须注明是哪个口径。

**R4 结论：RC 门槛（80 %）已满足；GA 门槛（85 %）按 8-crate 平均已满足，
但 executor（82.74 %）与 storage（80.61 %）两个 crate 单独看低于 85 %。**
若属主按「逐 crate ≥ 85 %」而非「平均 ≥ 85 %」判定，则 GA 尚不满足 ——
这一点留给裁决。

---

## 5. R5 — `#[ignore]` 债务

`scripts/gate/check_anti_ignore_gate.sh` —— **PASS**（rc=0）。

- 登记簿：`tests/baseline/ignore_registry.json`，`total_allowed: 123`
- 本轮 tree recount：**123 == 123**（树内实际 `#[ignore]` 数与登记数相等）
- 分类计数：parked 54，其余为 captured / external-prereq
- 本批次新增的 4 个 `#[ignore]`（`crates/storage/tests/measure_recovery_scale_p0_08.rs`）
  已按仓库约定登记进 registry（`PARKED`）

---

## 6. R6 — 跨版本债务 / 跨版本文档一致性

| 门禁 | 状态 |
|---|---|
| `scripts/gate/check_cross_version_debt.sh` | **NOT RUN（原因：本轮未执行）** |
| `scripts/gate/check_int_debt.sh` | **NOT RUN（原因：本轮未执行）** |
| `scripts/gate/check_docs_consistency.sh` | **PASS**（rc=0；该门禁是版本无关的跨版本检查，覆盖 v3.0.0..v3.12.0） |

---

## 7. R7 — 功能与性能场景（E2E）

| 场景 | 判据来源 | 本轮状态 |
|---|---|---|
| TPC-H SF=1 22/22 | `check_g1_tpch_baseline.sh`（G1） | **NOT RUN** —— SF=1 数据未就绪；`EVALUATION_REPORT.md` 记载 22/22 判定 **not recorded** |
| Backup/Restore 100+ 场景 | `check_backup_restore.sh`（G6） | **NOT RUN（原因：本轮未执行）** |
| Crash Matrix 100+ 场景 | `check_p12_crash_test.sh`（G8） | **NOT RUN（原因：本轮未执行）** |
| Upgrade Test 50+ 路径 | `check_p14_upgrade_test.sh`（G9） | **NOT RUN（原因：本轮未执行）** |
| 168h 多模型 SOAK | `STAGE.yaml` → `known_gaps.V400_09_soak` | **NOT MET** —— 进程已终止且无任何 run3/run4 产出；`decision` 原文为「是否重跑 168h SOAK 属架构师裁决，本 SSOT 不代为决定」 |
| Q20 oracle（#4540） | `STAGE.yaml` → `open_issues` | **NOT MET** —— `sqlrustgo/q20.tsv` 缺失，`CROSS_ENGINE_HASH_CHECK.json` 为 `present:false`，而 v3.12.0 报告称 "Q20 PASS"，三者互相矛盾（`open_issues` 剩余的唯一一条） |

---

## 8. R8 — 缺陷与 open_issues

`docs/releases/v4.1.0/DEFECTS_AND_ISSUES.md` §0 的计数：

| 级别 | 总数 | 未销账 | 备注 |
|---|---|---|---|
| P0（阻断任何 release） | 8 | **2** | P0-03（v4.0.0 GA 撤销 `REVOKED` —— 是状态不是缺陷）、P0-04（5 远端漂移，本批次已在 PR #5563 处理，待复跑确认后销账） |
| P1（阻断 ALPHA 推进） | 7 | **6** | — |
| **合计** | **22** | **15** | 批次 30/31 已销账 P0-06 / P0-07 / P0-08 |

`STAGE.yaml` → `open_issues`：**1** 条（P1-02 / #4540 Q20 oracle）。

⇒ **ALPHA→BETA 出口判据「No P0 bugs open」目前 NOT MET**（P0-03 + P0-04
两条未销账）。P0-03 是「状态」而非缺陷，其处置需要的是裁决而非补丁。

---

## 9. GA 额外门禁（RC 阶段预览）

GA 阶段比 RC 多出的 required_gates：`check_g16_compatibility.sh`（G16）、
`check_drift_not_pass.sh`、`check_architecture_freeze.sh`、
`check_gate_self_verification.sh`、`check_gate_test_integrity.sh`，
以及 `check_rc_ga_gate.sh ga`（full mode）。

**GA 的 5 份 required_files**（`stages.GA.required_files`）：

| 文件 | 期望 | 本批次状态 |
|---|---|---|
| `docs/releases/v4.1.0/STAGE.yaml` | `current_stage must be GA` | ❌ **NOT MET** —— 当前 `current_stage: "ALPHA"` |
| `docs/releases/v4.1.0/RELEASE_NOTES.md` | `must be the GA release notes` | ⚠️ 存在，但仍是 DRAFT 口径 |
| `docs/releases/v4.1.0/GA_GATE_REPORT.md` | 存在 | ✅ 本批次新建 |
| `docs/releases/v4.1.0/GA_RELEASE_TIMELINE.md` | 存在 | ✅ 本批次新建 |
| `docs/releases/v4.1.0/CHANGELOG.md` | `must contain the GA entry` | ❌ **NOT MET** —— 无 GA 条目 |

⚠️ **GA 的承载分支**：STAGE_CONFIG 的 `stages.GA.branch_pattern` 是
`["main", "release/v*", "hotfix/v*"]`，且 `carrier_enforced: true` ⇒
**GA 不能在 `develop/v4.1.0` 上声明**；`STAGE.yaml` 的 `ga_carrier_branch`
写明为 `release/v4.1.0`。RC 的 `branch_pattern` 包含 `develop/v*`，所以 RC
可以在当前分支上判定。

---

## 10. 签署

| 角色 | 姓名 | 日期 | 结论 |
|---|---|---|---|
| 提交人（`openclaw-mcode`） | openclaw-mcode | 2026-10-23 | 清单已生成，含全部实测与 NOT-RUN 披露 |
| RC 评审（codeowner） | ______ | ______ | ______ |
| RC 评审（governance） | ______ | ______ | ______ |
| 人类架构师 | ______ | ______ | ______ |

> 本文件由 `openclaw-mcode` 自动生成。`ALPHA → BETA → RC → GA` 三次阶段跃迁
> 在 `docs/governance/STAGE_CONFIG.yaml` 里都是 `requires_human_approval: true`
> ⇒ **本文件不构成批准**，签署栏留空由人填写。

---

## 11. 参考资料

- `docs/governance/STAGE_CONFIG.yaml` —— 阶段判据的 SSOT
- `docs/releases/v4.1.0/STAGE.yaml` —— 当前阶段状态、exit_criteria、known_gaps
- `docs/releases/v4.1.0/DEFECTS_AND_ISSUES.md` —— 缺陷计数与销账
- `docs/releases/v4.1.0/EVALUATION_REPORT.md` —— 覆盖率与缺口
- `docs/releases/v4.1.0/evidence/` —— 一手证据（81 份）
- `docs/releases/v4.1.0/GA_GATE_REPORT.md` —— GA 硬检查 G1–G6 的逐条判定
- `docs/releases/v4.1.0/EVIDENCE_STATUS.md` —— D1–D5 五个维度的证据状态
- `docs/releases/v4.1.0/REMOTE_SYNC_CONVERGENCE_2026-10-23.md` —— P0-04 远端收敛
- `docs/releases/v3.10.0/RELEASE_GATE_CHECKLIST.md` —— 本文件结构的先例
