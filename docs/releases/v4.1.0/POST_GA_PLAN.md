# SQLRustGo v4.1.0 发布后计划（POST_GA_PLAN）

> **基线**：`develop/v4.1.0` @ `2406559ac09ecf821566d11bf4a3cc1ce4f4dbb8`
> **provenance:** generated_at=2026-10-23, branch=docs/ga-rc-artifacts-batch32,
> commit=`2406559ac09ecf821566d11bf4a3cc1ce4f4dbb8`, machine=Mac-mini-7.local (arm64, macOS 27.0.1),
> identity=openclaw-mcode, source_repo=openclaw/sqlrustgo,
> policy=Anti-Fabrication-Policy-v1.0
> **gate_policy_eval_id**：`v410-post-ga-plan-001`
> （本文档汇编的门禁引擎：`scripts/gate/check_release_artifacts.sh`、
> `scripts/gate/check_stage.sh`、`scripts/gate/check_drift_not_pass.sh`、
> `scripts/gate/check_main_freshness.sh`、`scripts/gate/check_docs_links.sh`、
> `scripts/gate/check_evidence_binding.sh`、`scripts/gate/check_anti_ignore_gate.sh`、
> `scripts/gate/check_coverage_v312.sh`、`scripts/gate/check_g13_stability.sh`、
> `scripts/gate/check_g14_real_crash.sh`、`scripts/gate/check_g15_perf_report.sh`、
> `scripts/gate/check_g11_qps.sh`、`scripts/gate/check_g12_sysbench.sh`、
> `scripts/gate/check_g1_tpch_baseline.sh`、`scripts/gate/check_p13_soak_test.sh`；
> 证据绑定门禁 `scripts/gate/check_evidence_binding.sh` 的 Type B 判据据此认定本文档的
> PASS 声明有对应的 gate engine 输出，而非 AI 自述。）

---

## 0. 这份文件是什么，不是什么

**是什么**：v4.1.0 **正式发布（GA）之后**要做的事情的预登记计划 ——
监控窗口、必须补做但 GA 不包含的事项、hotfix 流程、对外口径、后续版本移交条件、
回滚与撤回预案。它是 `docs/governance/STAGE_CONFIG.yaml` →
`stages.GA.doc_artifacts` 点名要求的 GA 阶段发布产物之一。

**不是什么**：

* **不是 GA 声明**。见 §1。
* **不是批准**。`STAGE_CONFIG.yaml` 的 `GA_to_NEXT_VERSION_DRAFT` 跃迁是
  `requires_human_approval: true`；本文档只提供决策事实，不代替人类架构师裁决。
* **不是绩效承诺**。§2–§4 的每个窗口都给了「触发条件」，触发条件不满足时
  **不动作**，不靠承诺驱动。

---

## 1. 生效前提：v4.1.0 目前**尚未** GA

本计划的所有「发布后」都以 **GA 实际发生**为前提。据 `docs/releases/v4.1.0/STAGE.yaml`：

| 事实 | 值 | 出处 |
|---|---|---|
| 当前阶段 | `current_stage: "ALPHA"`（2026-10-06 重新进入） | `STAGE.yaml` `current_stage` / `last_transition` |
| GA 是否已声明 | `ga_declared: false` | `STAGE.yaml` `current_state` |
| GA 承载分支 | `ga_carrier_branch: "release/v4.1.0"` —— 且 `ga_carrier_note` 写明「**no release/v4.1.0 branch exists yet; GA has NOT been declared and MUST NOT be claimed on develop/v4.1.0**」 | `STAGE.yaml` `current_state` |
| 承载分支规则 | GA 的 `branch_pattern` 是 `["main", "release/v*", "hotfix/v*"]`，**不含 `develop/*`**；`carrier_enforced: true` ⇒ 在 `develop/v4.1.0` 上把阶段推到 GA，门禁会直接红 | `STAGE_CONFIG.yaml` `stages.GA` |
| GA 的硬判据 | 「All 5 dimensions + stability gates PASS」「GA gate report signed by human architect」「GA tag cut: v{VER}」「Merge to main + release branch created」 | `STAGE_CONFIG.yaml` `stages.GA.exit_criteria` |
| 未满足的稳定性判据 | `known_gaps.V400_09_soak.status: "NOT MET"`；`decision: "OPEN — 是否重跑 168h SOAK 属架构师裁决，本 SSOT 不代为决定。"` | `STAGE.yaml` `known_gaps` |
| 未销账缺陷 | P0 × 2（P0-03 = v4.0.0 GA `REVOKED`，是**状态**不是缺陷；P0-04 = 5 远端漂移）、P1 × 6、P2 × 7，合计 **15** | `DEFECTS_AND_ISSUES.md` §0（2026-10-23 更新） |
| 未销账 issue | `STAGE.yaml` `open_issues` 剩 **1** 条（P1-02 / #4540 TPC-H Q20 oracle 缺产物） | `STAGE.yaml` `open_issues` |
| GA 阶段 doc 产物 | `RELEASE_NOTES.md` / `GA_GATE_REPORT.md` / `GA_RELEASE_TIMELINE.md` / `STAGE.yaml` / `EVIDENCE_STATUS.md` / `POST_GA_PLAN.md`（本文档） | `STAGE_CONFIG.yaml` `stages.GA.doc_artifacts` |

⇒ **本文件是「如果 GA 发生，之后就这样做」的预登记**，不是「GA 已经发生」的陈述。
本文件的存在**不推进** `current_stage`，也不构成任何晋级结论 —— 这与
`docs/releases/v4.0.0/GA_RELEASE_TIMELINE.md` 开头那句完全同构：
「补全本文件只是满足 GA 阶段 `required_files` 清单；阶段是否晋升由门禁实跑结果 +
人工签字决定，**不由文件是否存在决定**」。

---

## 2. 发布后 0–7 天（稳定期，T+0 ~ T+7）

**窗口目标**：把「发布当天没崩」升级成「发布后一周没退化」。这一周只做**观测与
止血**，不做功能改动。

| # | 监控项 | 监控方式 | 触发条件 | 动作 | 责任人（角色） |
|---|---|---|---|---|---|
| M01 | 崩溃 / 错误 / 重连计数 | 读取部署侧 server log（`--log-level warn`）与驱动日志 | 任一部署出现 `FATAL` / panic / 非预期 reconnect | 立即按 §6 开 hotfix 分支；同步在 `STAGE.yaml` 登记 | Release Manager（发起）+ Human Architect（批准） |
| M02 | **内存是否收敛** | 连续采集 RSS（每 15 s 一采样，写 `metrics.csv`） | RSS **连续 3 天单调上升、无平台期** | 升级为 hotfix 候选。**基线参照**：v4.1.0 1h SOAK 的 RSS 从 106.0 MB 单调升到 662.8 MB，报告自述「v4.1.0 at HEAD has not demonstrated a stable memory equilibrium」 | Codeowner |
| M03 | 文件描述符数 | `lsof` / `/proc` 计数 | FD 数持续增长（1h SOAK 基线：恒定 28，停服后 12） | 按 FD 泄漏路径排查，登记为 issue | Codeowner |
| M04 | 延迟分位 | 驱动侧逐查询 CSV（1h SOAK 基线：p50 79.9 / p90 204.3 / p95 251.3 / p99 370.1 / max 2 933.4 ms） | p95 相对基线退化 > 10% 且可复现 | 先看是否命中已知瓶颈（写路径 O(rows) 遍历 / `engine.write()` 全局独占锁），再决定 hotfix 或记为已知边界 | Codeowner |
| M05 | **重启→可服务时间** | 灌 N 行 → 停服 → 重启 → 计时到 accept、再计时首条全表查询 | 重启→可服务 > 1 s，或首条全表查询 > 1 s | 对照 P0-08 的关闭条件（`restart_to_ready = 0.17 s`、`first_full_table_query = 0.08 s`）。超时说明该次部署的恢复路径**形状**回退，重新开 P0-08 | Codeowner |
| M06 | 数据不丢 | 重启前后 `COUNT(*)` 对比 | 计数不等 | **最高优先级**：按 P0-08 关闭条件第 4 项处理，立即 hotfix | Codeowner + Human Architect |
| M07 | 5 远端 `main` 漂移 | `scripts/sync/5remotes_drift_check.sh`（当前 `--strict-main` 只对 `main` 严） | `main` 在任一远端不一致 | 按 §6 的同步流程处理（force-with-lease）；**develop 漂移当前只报告不阻断**，需人工决定是否升级为阻断 | Governance |
| M08 | `main` 新鲜度 | `scripts/gate/check_main_freshness.sh`（C-MAIN-01/02/03 全 fail-closed） | exit ≠ 0 | 修正 main 与 `release/v4.1.0` 的关系 | Governance |
| M09 | 门禁漂移 | `scripts/gate/check_drift_not_pass.sh`；`scripts/gate/check_evidence_binding.sh` | 任一 exit ≠ 0 或 FAIL > 0 | 先修门禁再谈发布后功能；**NOT-RUN 一律不得记为 PASS** | Governance |
| M10 | 文档链接与发布产物 | `scripts/gate/check_docs_links.sh`；`scripts/gate/check_release_artifacts.sh --version v4.1.0 --stage GA` | 断链 > 0 或 GA 产物缺失 | 补齐 / 修复；`check_release_artifacts.sh` 的设计就是「缺产物必须报缺，**不许伪造、不许 stub、不许把 skip 报成 pass**」 | Governance |
| M11 | 覆盖率不回退 | `scripts/gate/check_coverage_v312.sh`（**必须 `--test-threads=1`**） | L1_8 均值 < 80%（`COVERAGE_MIN_RC`） | 阻塞下一轮晋级；补测后附实跑输出 | Codeowner |

**0–7 天的固定动作**

1. 每日一次发布后健康巡检，产出当日 `metrics.csv` + 巡检记录，归档到
   `docs/releases/v4.1.0/evidence/`（**新建目录需要人工授权**，见 §10 说明）。
2. 把 GA 后**第一次真实部署**的重启→可服务时间、首条全表查询、行数校验记下来 ——
   这是 P0-08 修复在生产形态下的第一次验证。
3. 不在 0–7 天窗口内合入任何非 hotfix 改动（`STAGE_CONFIG.yaml` GA 的
   `merge_policy` 要求 `reviewers: 3` + `codeowner + governance + human-architect +
   release-manager` 四方批准）。

---

## 3. 发布后 8–30 天（观察期，T+8 ~ T+30）

**窗口目标**：把 GA **不包含**的那几项硬证据补齐 —— 这是 §5 的执行窗口。

| # | 动作 | 触发条件 | 产出物 | 责任人（角色） |
|---|---|---|---|---|
| A01 | **168h SOAK 重跑（若 GA 前未完成）** | 架构师已就 `known_gaps.V400_09_soak.decision` 作出裁决且裁决为「重跑」 | 一份正文写明「completed run」的终期报告（**文件存在不等于跑完** —— v4.0.0 那次就是因为「文件存在」被误销账） | Human Architect（裁决）+ Codeowner（执行） |
| A02 | **24h+ 稳定性（G13）** | 专用主机（Z6G4）可用 | `STABILITY_REPORT.md`（≥ 24h）+ 让 `scripts/gate/check_g13_stability.sh` 变绿 | Codeowner |
| A03 | **进程级崩溃测试（G14）** | 专用主机 + sysbench 可用 | `CRASH_TEST_REPORT.md`（`kill -9` 级） | Codeowner |
| A04 | **性能汇总报告（G15）** | A05 完成 | `PERFORMANCE_REPORT.md` + 5 份子报告 + `PERFORMANCE_BASELINE.md`（三者对 v4.1.0 **目前都不存在**） | Codeowner + Release Manager |
| A05 | **QPS/TPS 基准（G11）与 Sysbench（G12）** | 专用基准主机 + sysbench 二进制 | `PERFORMANCE_BASELINE.md`；sysbench OLTP 轮次记录 | Codeowner |
| A06 | **TPC-H SF=1 22/22 正式执行（G1）** | 数据与 oracle 就位 | 22/22 的执行记录 + 与 oracle 的哈希比对 | Codeowner |
| A07 | **Q20 oracle 产物补齐（#4540）** | A06 执行时同步产出 | `sqlrustgo/q20.tsv` 真实产物 + 重算 SHA256 + 改写 `GA5_TPCH_SF1_REPORT.md` 的「Q20 PASS」 | Codeowner |
| A08 | 内存稳态专项 | M02 触发过，或 A01/A02 的 RSS 曲线仍无平台期 | 一份「内存是否收敛」的专项结论（当前 **无**） | Codeowner |
| A09 | 5 远端同步固化 | 每次 `main` 变更后 | 5 远端 `main` 一致 + drift gate 通过 | Governance |
| A10 | 缺陷台账收敛 | 每周 | `DEFECTS_AND_ISSUES.md` §0 计数刷新（当前 P0×2 / P1×6 / P2×7 = 15） | Codeowner |

**8–30 天的固定动作**

* 每周一次发布后稳定性复盘，把 §5 的清单逐项更新状态（MET / NOT MET / NOT RUN）。
* 每周一次 `check_drift_not_pass.sh` + `check_evidence_binding.sh` + `check_coverage_v312.sh` 三连跑，
  输出归档。**任何一项 NOT RUN 都要写明原因，不得记为 PASS。**
* 不做 v4.2.0 的功能开发（见 §8 的移交条件）。

---

## 4. 发布后 31–90 天（收敛期，T+31 ~ T+90）

**窗口目标**：把发布期的临时结论固化成**下一版**的输入。

| # | 动作 | 触发条件 | 产出物 | 责任人（角色） |
|---|---|---|---|---|
| B01 | GA 后真实负载 vs v4.1.0 基线对比 | T+30 已过 | 一份对比报告（用 §5 A04 的 `PERFORMANCE_BASELINE.md` 作基线） | Codeowner |
| B02 | v4.1.1 范围确定 | T+45 | hotfix 清单 + 是否需要 v4.1.1 的结论 | Release Manager + Human Architect |
| B03 | v4.1.2 / v4.2.0 范围确定 | T+60 | 版本范围说明（哪些 WP 项进 v4.2.0） | Human Architect |
| B04 | 24 条 backlog 重新排期 | T+60 | `ISSUES_PLAN.md` §4 的更新（当前估算 6–10 周） | Codeowner + Governance |
| B05 | 债务台账更新 | T+60 | `docs/governance/debt/debt-registry.yaml` 与 v4.1.0 实际状态对齐 | Governance |
| B06 | 文档矛盾裁决 | T+75 | `STAGE.yaml` `legacy_chain_audit_2026_09_30.doc_contradictions_unresolved` 里那 **19 条**矛盾逐条给出「有证据」或「承认未闭合」（该字段自己的 `recommended_next` 就是这么写的） | Governance |
| B07 | 分支清理 | T+90 | `rc/v4.1.0` 按 `BRANCH_GOVERNANCE.md` 处理；`develop/v4.1.0` 转只读归档；`release/v4.1.0` 保留至 v4.2.0 GA | Governance |
| B08 | 下一版本分支 | T+90 或触发条件满足 | `STAGE_CONFIG.yaml` `transitions.GA_to_NEXT_VERSION_DRAFT` 的两个条件：①`POST_GA_PLAN.md` 存在（本文档）②下一版本分支已创建 | Human Architect（跃迁批准） |
| B09 | 支持期承诺公布 | T+90 | v4.1.0 的支持级别与终止日期（口径参照 §7.4） | Release Manager |

---

## 5. GA **不包含**、但发布后必须补做的事项

这一节是本文档的核心诚实性条款。**下面每一项在 v4.1.0 GA 时都必须是
「未完成」状态**；如果某项在 GA 前已完成，此表相应行改写为「GA 已包含」并
附证据 —— 但**不得**在没有产物的情况下划掉。

| # | 事项 | 仓库现状（截至 2026-10-23） | 为什么 GA 不包含 | 补做窗口 | 责任人 |
|---|---|---|---|---|---|
| N01 | **168h 多模型 SOAK（V400-09）** | `known_gaps.V400_09_soak.status: "NOT MET"`（verified 2026-10-07）；进程 PID 1957614 已终止；全盘无 run3/run4 的 `STABILITY_REPORT_*.md` / `metrics.csv`；v4.0.0 那份 `V400_09_168H_SOAK_FINAL_REPORT.md` 日期为 2026-09-30 且正文自述「the 168h SOAK process never ran to completion」 | `STAGE.yaml` `exit_criteria.rc_to_ga` 明写「168h multi-model SOAK PASS」——它是一条**判据**，不是事后可补的装饰 | A01（T+8 ~ T+30） | Human Architect 裁决 + Codeowner 执行 |
| N02 | **长时间稳定性观察（≥ 24h）** | 仓库记录的运行是 5 min / 1 h / 8 h；**没有** 24 h+ 的 `STABILITY_REPORT.md`；G13 门禁当前 exit 1（`tests/soak_test_harness.rs not found`） | G13 的 `requirement` 明确要 Z6G4 主机 + `STABILITY_REPORT.md` + Beta 72h 报告，三者对 v4.1.0 都不齐 | A02（T+8 ~ T+30） | Codeowner |
| N03 | **TPC-H SF=1 22/22 的正式执行** | `EVALUATION_REPORT.md` §7.1 记为 **not recorded**；`evidence/TPCH_FIXTURE_5379_2026-10-20.md` 把 SF=1 排除在本轮范围外（#5387） | G1 是从 ALPHA 起累计到 GA 的阻断门禁；v4.1.0 从未执行过 | A06（T+8 ~ T+30） | Codeowner |
| N04 | **Q20 oracle 产物（#4540）** | `sqlrustgo/q20.tsv` 不存在（21/22 文件）；`CROSS_ENGINE_HASH_CHECK.json` 记 `present:false`；与 `GA5_TPCH_SF1_REPORT.md:40` 的「Q20 PASS」直接矛盾 | 这是 `STAGE.yaml` `open_issues` 里**唯一**还 open 的一条 | A07（与 A06 同步） | Codeowner |
| N05 | **内存稳态结论** | 1h SOAK 的 RSS 从 106.0 MB 单调升到 662.8 MB，无 GC 平台期；报告自述「未证明稳定内存平衡」 | 现有 SOAK 全是短时；8h SOAK 的 RSS 峰值只有 85.3 MB，但那是**单库、无 kill -9、无重启**的场景，证据权重已被 IAIA-410 下调 | A08（T+8 ~ T+30） | Codeowner |
| N06 | **吞吐随并发提升** | 修复前并发 4 → 64 **吞吐不升反降**（1 连接 136 QPS → 8 连接 17 QPS）；修复后 10 000 行 QPS 1 242~1 295，仍不随并发提升 | 瓶颈已转移到写路径 O(rows) 遍历与 `engine.write()` 全局独占锁，属结构性问题，不是 GA 前的补测能解决 | B01 结论 + v4.2.0 范围 | Codeowner |
| N07 | **Q20 去相关接线** | `crates/optimizer/src/decorrelate.rs:417` 的 `try_decorrelate()` 在 `crates/` 内 **0 调用点** | planner 未接线；`#4429/#4426/#4435` 的性能债根因未动，expiry 2027-03-31 | v4.2.0 | Codeowner |
| N08 | **`PERFORMANCE_BASELINE.md` / `PERFORMANCE_REPORT.md`** | v4.1.0 目录下**都没有**（只有 v3.9.0 / v3.10.0 有 baseline） | G11 与 G15 的 `requirement` 以这两份产物为前提 | A04 / A05（T+8 ~ T+30） | Codeowner + Release Manager |
| N09 | **`CRASH_TEST_REPORT.md`（G14）** | 不存在 | G14 `requirement` | A03（T+8 ~ T+30） | Codeowner |
| N10 | **C-ARCH-05 覆盖扩展 / 三个大文件拆分** | `expr/mod.rs` 5 104 行 / `stored_proc.rs` 4 248 行 / `trigger.rs` 2 549 行，**都不在门禁覆盖内**；门禁只看 `execution_engine.rs`（1 034 行） | `STAGE.yaml` `gate_policy` 明写：扩展 C-ARCH-05 覆盖**必须等 REF-01..03 拆分先落地**，否则门禁立刻红 | v4.2.0（REF-01..03 记为 P1/P1/P2） | Codeowner |
| N11 | **`#5167`（`COUNT(*)` / 范围扫描返回 0）** | **未修**，且是**正确性**问题 | 会污染所有性能测量 | v4.1.1 或 v4.2.0 | Codeowner |
| N12 | **P0-04 5 远端 `develop/v4.1.0` 漂移** | 缺陷目录 P0-04 记为 `OPEN`；drift gate 当前只对 `main` 严，对 develop 仅报告 | 跨远端 reviewer 看到不同的「当前 v4.1.0」 | T+8 前完成同步，或明确裁决为「接受为已知边界」并记录 | Governance + Human Architect |

> **纪律**：N01–N12 中任何一项在 GA 后被「顺手做了」而没有**独立产物**，
> 一律按未完成处理。**文件存在 ≠ 跑完**（这是 v4.0.0 168h SOAK 被误销账的直接原因）。

---

## 6. Hotfix 流程

### 6.1 分支拓扑（仓库既有规定）

`docs/governance/BRANCH_GOVERNANCE.md` §5.5：

```
main
   ↓
hotfix/*
   ↓ PR
main
   ↓ 回流
rc/vX.Y.Z
   ↓ 回流
develop/vX.Y.Z
```

规则原文：**「必须从 main 创建 hotfix 分支」**、「修复后 PR 到 main」、
「**必须回流到 rc 和 develop，否则会产生分叉灾难**」。

`docs/governance/VERSION_PROMOTION_SOP.md` §12 给的是同一件事的命令形态：

```bash
git checkout release/vX.Y.Z
git checkout -b hotfix-X.Y.Z+1
# ... 修复 ...
git checkout release/vX.Y.Z
git merge --no-ff hotfix-X.Y.Z+1
git tag vX.Y.Z+1
git push origin release/vX.Y.Z --tags
# 然后必须同步回：
git checkout feature/vX.Y+1.0-alpha
git merge release/vX.Y.Z
```

### 6.2 v4.1.0 的具体规定

| 项 | 规定 | 出处 |
|---|---|---|
| 分支名 | **必须**匹配 `hotfix/v*` | `STAGE_CONFIG.yaml` `stages.GA.branch_pattern`（`["main", "release/v*", "hotfix/v*"]`）+ `carrier_enforced: true` ⇒ `scripts/gate/check_stage.sh` 会直接判红 |
| 基线分支 | 从 `main`（GA 后的 `main`）切出 | `BRANCH_GOVERNANCE.md` §5.5 |
| 审批 | `reviewers: 3`；`approval: codeowner + governance + human-architect + release-manager`（**四方**） | `STAGE_CONFIG.yaml` `stages.GA.merge_policy` |
| CI 必需项 | `lint, build, test, full-gate, anti-fab, soak, stability` | 同上 `ci_required` |
| 版本号 | `vX.Y.Z+1`（补丁号 +1） | `BRANCH_GOVERNANCE.md` §6.2；`VERSION_PROMOTION_SOP.md` §12 |
| 合并方式 | `--no-ff`（保留 hotfix 的合并节点） | `VERSION_PROMOTION_SOP.md` §12 |
| 回流 | hotfix → `main` → `rc/v4.1.0` → `develop/v4.1.0` | `BRANCH_GOVERNANCE.md` §5.5 |
| Tag | `git tag vX.Y.Z+1`，并 `push origin release/vX.Y.Z --tags` | `VERSION_PROMOTION_SOP.md` §12 |

> ⚠️ **一处必须先解决的口径不一致（本文档登记，不代为裁决）**：
> `VERSION_PROMOTION_SOP.md` §12 用的分支名是 `hotfix-X.Y.Z+1`（**无斜杠**），
> `BRANCH_GOVERNANCE.md` §5.5 用的是 `hotfix/*`（**有斜杠**），
> 而 `STAGE_CONFIG.yaml` 的 GA `branch_pattern` 只认 `hotfix/v*`。
> `hotfix-X.Y.Z+1` **既不匹配** `hotfix/*`（无斜杠）**也不匹配** `hotfix/v*`。
> ⇒ **建议**：第一次 hotfix 之前，由 Governance 统一为一个命名，并让
> `check_stage.sh` 的 `branch_pattern` 与之对齐。**在统一之前，
> 任何 hotfix 分支都可能被承载分支门禁判红。**

### 6.3 Hotfix 的准入与回滚

* **准入**：只有 §5 里 N01–N12 之外的**新发现缺陷**才走 hotfix。
  已登记的已知边界（§7）**不走 hotfix**，走 v4.2.0。
* **hotfix 内允许的改动**：仅缺陷修复；**不得**夹带 feature、不得夹带重构
  （`BRANCH_GOVERNANCE.md` §5.5 之外的通用约束：rc 分支只接受 bugfix / hotfix）。
* **回滚**：见 §9。

---

## 7. 已知问题的对外沟通口径

### 7.1 唯一权威口径

对外口径的 SSOT 是 `docs/releases/v4.1.0/CLAIM_DOWNGRADE_MANIFEST.md`
（+ 其继承的 `docs/releases/v4.0.0/CLAIM_DOWNGRADE_MANIFEST.md`）、
`docs/releases/v4.1.0/STAGE.yaml` 的 `claims_boundaries`、
以及 `docs/releases/v4.1.0/COMPREHENSIVE_ASSESSMENT_REPORT.md` §9。
**任何对外材料（README / RELEASE_NOTES / 官网 / 演讲）必须逐字复用这些
claim boundary 句子，不得改写、不得扩写。**

### 7.2 可以说的（known gap，可以公开声明为已知边界）

| 项 | 对外口径 |
|---|---|
| 3 条继承的 GA-claim-caveat（`#4846` / `#4847` / `#4848`） | 「`CHAR(n)` 字节填充主键点查」「显式 `BEGIN`/`COMMIT`/`ROLLBACK`」「`ALTER TABLE ... RENAME COLUMN`」三项**排除在 v4.1.0 的发布声明之外**（原文见 v4.0.0 manifest §3.5 / §3.4） |
| `#5304` BufferPool / Page / disk B+Tree | 「v4.1.0 **不发布**所宣称的 `BufferPool` / `Page` / disk B+Tree 能力；这三个类型是 `pub use` 导出的**仅测试用途**，生产 `FileStorage` 是内存 `HashMap` + JSON `insert_buffer`（非分页）」 |
| `crates/planner` 物理计划层不可达 | 「planner 的物理计划层没有 `execute()`，生产路径不可达 ⇒ 其单元测试**不作为**生产正确性证据」 |
| 短时 SOAK 的观测值 | 可以说「在单库、无 kill -9、无重启的 8 小时运行里，0 FATAL / 0 errors / 0 reconnects」——**但必须同时说清这不是「v4.1.0 可用」的证据**（IAIA-410 已下调其证据权重） |

### 7.3 **不能**说的（禁止对外宣称）

| 禁止宣称 | 依据 |
|---|---|
| 「v4.1.0 已达到 ALPHA/BETA/RC/GA 任何一级的发布质量」 | `COMPREHENSIVE_ASSESSMENT_REPORT.md` §9 明确**禁止**；`EVALUATION_REPORT.md` §8 亦写明「**v4.1.0 is not GA and is not releasable as GA**」 |
| 「v4.1.0 是 SQLite / MySQL / PostgreSQL 的替代品」 | 同上 |
| 「事务隔离 / AUTO_INCREMENT / CHAR PAD SPACE / ALTER TABLE RENAME COLUMN / SQLite 元数据已闭合」 | 同上 |
| **任何 168h 多模型 SOAK 的结论** | `STAGE.yaml` `claims_boundaries.prohibited_pre_ga`：「168h multi-model SOAK result」 |
| 生产多模型声明（未到 GA 前） | `claims_boundaries.prohibited_pre_beta` |
| 「TPC-H SF=1 22/22 通过」 | v4.1.0 **从未执行**（`EVALUATION_REPORT.md` §7.1） |
| 「Q20 oracle 校验通过」 | `sqlrustgo/q20.tsv` 缺失，`present:false`；`GA5_TPCH_SF1_REPORT.md:40` 的「Q20 PASS」是 AFP Type B |
| 「CI 全绿 ⇒ 可合并」 | `evidence/CI_TRUTH_BATCH24_2026-10-22.md` §1.5：CI 的 `test` job **从未成功完成过一次**（1 452 次机会中 0 次） |
| 「v4.0.0 是 GA」 | `docs/releases/v4.0.0/STAGE.yaml` → `ga_promotion_status.verdict = "REVOKED 2026-09-30"`；代码确实已发布（2026-09-19，tag `v4.0.0-final` @ `54571eeca0`，5 远端 `main` 均含该 commit），但**从未合法达到 GA** |
| 任何「已修复 / 已完成」而没有产物与实跑输出支撑的说法 | `ANTI_FABRICATION_POLICY.md`；`ADR-001`；`ADR-008` |

### 7.4 支持期口径（参照 v3.10.0 的先例，按 v4.1.0 实际写）

| 版本 | 支持级别 | 期限 | 备注 |
|---|---|---|---|
| v4.1.0 | Critical + Security 修复 | 至 v4.2.0 GA | — |
| v4.1.x | Security-only | v4.2.0 GA + 90 天 | — |
| v4.0.0 | **不作为 GA 版本支持**（判定 `REVOKED`） | — | 需在对外材料里明确区分「已发布的代码」与「已撤销的 GA 判定」 |

---

## 8. 后续版本的移交条件

### 8.1 阶段跃迁的框架条件

`STAGE_CONFIG.yaml` `transitions.GA_to_NEXT_VERSION_DRAFT`：

* `trigger`: 「POST_GA_PLAN.md exists + next version branch created」
* `requires_human_approval`: `true`

⇒ **本文档的存在满足该触发的第一条**，但**第二条（下一版本分支已创建）与人工批准
都不在本文件的能力范围内**。

### 8.2 v4.1.1（补丁）

| 移交条件 | 状态 |
|---|---|
| 从 `main` 切出 `hotfix/v4.1.1`（见 §6.2 的命名统一前提） | 待 §6.2 的口径不一致解决 |
| 只含 §5 之外的新发现缺陷修复 | — |
| `reviewers: 3` + 四方批准 + `ci_required` 全绿 | `STAGE_CONFIG.yaml` GA `merge_policy` |
| §2–§4 的三个窗口已各跑完一次并归档 | — |
| `merge --no-ff` + `git tag v4.1.1` + 推 `release/v4.1.0` 与 tags | `VERSION_PROMOTION_SOP.md` §12 |
| 回流到 `develop/v4.1.0`（以及下一版本的 alpha 分支） | `BRANCH_GOVERNANCE.md` §5.5 |

### 8.3 v4.1.2（补丁）

| 移交条件 | 说明 |
|---|---|
| v4.1.1 已在生产运行 ≥ 14 天且无新 hotfix | 参照 v3.10.0 `POST_GA_PLAN.md` 的 hotfix 判定窗口 |
| §5 的 N01–N12 中仍未完成项已重新分类（补做 / 接受为已知边界 / 顺延） | 分类结论需 Human Architect 签字 |
| `DEFECTS_AND_ISSUES.md` §0 的未销账计数已刷新 | 当前 P0×2 / P1×6 / P2×7 = 15 |

### 8.4 v4.2.0（次版本）

| 移交条件 | 说明 |
|---|---|
| 下一版本分支已创建（`transitions.GA_to_NEXT_VERSION_DRAFT` 第二条） | `VERSION_PROMOTION_SOP.md` §11：从 `release/vX.Y.Z` 切 `feature/vX.Y+1.0-alpha` |
| 24 条 backlog（`ISSUES_PLAN.md` §4）重新排期完成 | 当前估算 6–10 周 |
| N03（TPC-H SF=1）、N04（Q20 oracle）、N08（性能基线与汇总报告）已补齐 | 这三项决定 v4.2.0 能否声称性能与兼容性 |
| N07（Q20 去相关接线）、N10（C-ARCH-05 覆盖扩展 + 三个大文件拆分）已排期 | `STAGE.yaml` `inherited_from_v4000_revocation` REF-01..03 |
| 19 条文档矛盾已逐条裁决 | `STAGE.yaml` `legacy_chain_audit_2026_09_30.doc_contradictions_unresolved` |
| 5 远端 `develop` 与 `main` 均无漂移（P0-04 处置完成） | `scripts/sync/5remotes_drift_check.sh` |

---

## 9. 回滚与撤回预案

### 9.1 原则（沿用 `docs/releases/v4.0.0/GA_RELEASE_TIMELINE.md` §4）

v4.0.0 记录的**回滚原则**四条，本计划逐条继承：

1. 明确受影响的 claim 与证据缺口。
2. 在 `STAGE.yaml` 或相关 gate 报告中降级或标注限制。
3. 修复后重新执行对应 gate，**附实跑输出**。
4. 使用 PR / commit 记录整改过程。

### 9.2 v4.1.0 的具体情形与动作

| 情形 | 判据 | 动作 | 禁止动作 |
|---|---|---|---|
| **R1：GA 判定被证伪**（AFP Type B：声称门禁通过但无 policy engine 输出支撑） | 晋级评审发现某个 PASS 声明没有对应的门禁实跑输出 | ① 立即把 `STAGE.yaml` 的 `ga_promotion_status.verdict` 置为 `REVOKED <日期>` 并列出阻塞事实；② 在 `GA_GATE_REPORT.md` 顶部加判定更正块（保留原正文作历史记录）；③ 同步 `CURRENT_VERSION.md` 与 v4.0.0 / v4.1.0 两份 `RELEASE_NOTES.md` 的措辞（v4.1.0 侧的先例是 2026-10-23 那次对 `CURRENT_VERSION.md` 6 处引用的对齐）；④ 重新执行对应 gate 并附实跑输出 | **不得**删除 tag（v4.0.0 的 `v4.0.0-final` tag 与 `release/v4.0.0` 分支**保留为审计轨迹**）；**不得**把「已发布的代码」改写成「未发布」 |
| **R2：发布后出现 P0 缺陷** | §2 的 M01 / M06 触发 | ① 走 §6 的 hotfix 流程；② 若缺陷使某条 GA 声明失效，同时触发 R1 | 不得在 `main` 上直接改代码（`main` 受保护） |
| **R3：发布后性能退化** | M04 触发（p95 退化 > 10% 且可复现） | ① 先确认是否命中已知瓶颈（N06）；② 若是**已知边界**⇒ 不回滚，按 §7.2 对外声明；③ 若是**新缺陷**⇒ 走 §6 | 不得用「重跑一遍就好了」覆盖退化 |
| **R4：远端状态不一致** | M07 / M08 触发 | ① 按 `BRANCH_GOVERNANCE.md` §14 收敛 `main`（v4.0.0 的先例：先做 `backup/main-pre-convergence-<日期>` 分支 + `archive/main-pre-convergence-<日期>` tag 双备份，再单向驱动）；② 重跑 drift gate | 不得直接 force-push 而不做双备份 |
| **R5：门禁本身失效** | M09 触发（例如门禁又是「空转」） | ① 参照 v4.1.0 自己的先例（`check_anti_ignore_gate.sh` 的 tree recount、`check_coverage_v312.sh` 的 fail-closed）：门禁必须**扫描被检对象**并在不满足时 **fail-closed**；② 修完重跑并附输出 | 不得把「exit 0」当作「门禁有效」 |

### 9.3 回滚的记录义务

任何一次回滚 / 撤回，必须留下三样东西：

1. `STAGE.yaml` 里的状态变更（带日期与理由）；
2. `docs/governance/incidents/` 下的事件记录（先例：
   `docs/governance/incidents/2026-09-30-V410-ALPHA-UNSUPPORTED-BY-GATE-EVIDENCE.md`）；
3. 对应的门禁实跑输出（**输出，不是叙述**）。

> v4.1.0 的历史已经演示过这条路径的成本：2026-09-29 自我声明 ALPHA，2026-09-30
> 被自身的 AFP Type B 自查推翻并回滚到 DRAFT，代价是 **6 个 issue 重新开过、
> 一个 tag 留在树上与 SSOT 长期矛盾**（该矛盾到 2026-10-23 才销账）。**先跑门禁、
> 后声明阶段**，比事后回滚便宜得多。

---

## 10. 签署块

> **本文件由 `openclaw-mcode` 自动生成，晋级审批由人类架构师执行
> （`STAGE_CONFIG.yaml` 三次阶段跃迁 `requires_human_approval: true`）。**
> **本文件不代替签字，也不构成 GA 晋级或版本跃迁的结论。**

| 角色 | 姓名 | 日期 | 结论 |
|---|---|---|---|
| 计划编制（Post-GA 负责人） | `______` | `______` | `______` |
| Release Manager | `______` | `______` | `______` |
| Governance | `______` | `______` | `______` |
| Human Architect（GA → 下一版本跃迁签字） | `______` | `______` | `______` |

**未决事项（签字前必须先有裁决）**

| # | 事项 | 裁决人 | 结论 | 日期 |
|---|---|---|---|---|
| 1 | 是否重跑 168h SOAK（`known_gaps.V400_09_soak.decision` 现为 `OPEN`） | `______` | `______` | `______` |
| 2 | §6.2 的 hotfix 分支命名口径不一致（`hotfix-X.Y.Z+1` / `hotfix/*` / `hotfix/v*`） | `______` | `______` | `______` |
| 3 | §5 的 N01–N12 逐项处置（补做 / 接受为已知边界 / 顺延） | `______` | `______` | `______` |
| 4 | §7.3 的禁止宣称清单是否作为对外沟通的最终口径 | `______` | `______` | `______` |
| 5 | §9.2 的回滚情形分级（R1–R5）是否被接受 | `______` | `______` | `______` |
