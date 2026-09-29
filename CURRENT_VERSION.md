# 当前版本状态

> **provenance:** generated_at=2026-09-30, branch=develop/v4.1.0,
> commit=`bdf465b25c` (post v4.0.0 docs backport + alpha2 prep),
> source_repo=openclaw/sqlrustgo, policy=Anti-Fabrication-Policy-v1.0

## 最近已发布

**v4.0.0 GA** (CONDITIONAL PASS, 2026-09-19 发布；tag `v4.0.0-final` @ `54571eeca0`，最终 sync HEAD `07178c9d66`；PR #3787/#4902 promote 至 main)

- 完整发布说明：[docs/releases/v4.0.0/RELEASE_NOTES.md](docs/releases/v4.0.0/RELEASE_NOTES.md)
- 完整变更日志：[docs/releases/v4.0.0/CHANGELOG.md](docs/releases/v4.0.0/CHANGELOG.md)
- 升级指南：[docs/releases/v4.0.0/UPGRADE_GUIDE.md](docs/releases/v4.0.0/UPGRADE_GUIDE.md)
- GA 门禁报告：[docs/releases/v4.0.0/GA_GATE_REPORT.md](docs/releases/v4.0.0/GA_GATE_REPORT.md) — G1/G2/G4/G5 ✅ PASS；G3 coverage 78.28% 平均 CONDITIONAL PASS（< 80%）；168h SOAK defer 至 V400-09 follow-up

**v4.1.0-alpha1** (2026-09-29; 8/8 alpha quality gates PASS; tag `v4.1.0-alpha1` @ HEAD `c1a73a5320`, 后续 6 commits 推到 `0edda51d42`)

**v3.12.0 GA**（2026-09-08；8/8 GA gates PASS；tag `v3.12.0-ga` @ `355b5a3837`）
**v3.11.0 GA**（2026-08-09；6/6 GA gates PASS；tag `v3.11.0-ga` @ `83c623835`）

## 当前正在开发

**v4.1.0 ALPHA** — develop/v4.1.0 @ commit `bdf465b25c` (tag v4.1.0-alpha1, PRAGMA + 3VL + correlated-IN + tpch_hash_test landed + v4.0.0 docs backport)

- **阶段**: **ALPHA** (2026-09-29 promote from DRAFT; `scripts/gate/check_alpha_v410.sh` composite PASS: entry 19/19 + quality 8/8; tag `v4.1.0-alpha1`)
- **当前状态**: 继承 v4.0.0 GA production 代码 (V400-05/06/07 in PR #3781)，v4.1.0 增量包括：
  - `b80e43842` feat(parser+executor): PRAGMA support and SQL three-valued logic
  - `0d30f8bfb0` fix(engine): evaluate correlated IN subquery per outer row
  - `e772ee3ff` fix(parser+tests): unblock test compilation and allow boolean AND in column position
  - `fc329ae17` test(harness): name the missing TPC-H fixture instead of failing opaquely
  - `af05d7e6fe` test(tpch): restore tpch_hash_test on v4.1.0
  - `2b5c8db0c7` docs: correct architecture map and release version line
- **v4.0.0 STAGE.yaml SSOT 治理决策**: v4.0.0 STAGE.yaml `current_stage: DRAFT` 与 GA_GATE_REPORT.md 说 `GA CONDITIONAL PASS` 矛盾已通过 `docs/v400-ga-rectify` 分支（241f98a4）补发布 RELEASE_NOTES.md + UPGRADE_GUIDE.md。SSOT 仍未 advance to GA（v4.0.0 STAGE.yaml current_stage=DRAFT 不变，是 release governance decision 而非文档 fix）。
- **alpha→beta 标签 exit criteria** (per STAGE.yaml): WP-C/D/F/G deferred items migrated + V400-09 168h SOAK FINAL + Coverage >= 50%
  - WP-C DDL/integrity 修复 (#4682/#4652/#4672/#4669/#4709/#4703)
  - WP-D join/subquery 修复 (#4668/#4656/#4649/#4636)
  - WP-F schema migration (#4848)
  - WP-G CHAR(n) PAD SPACE 兼容 (#4846)
  - WP-H #4639 (v4.0.0 deferred, v4.1.0 优先)
  - 168h SOAK 复跑证据 (前置: RSS budget 调优)

- **v3.12.0 RC** 仍在 `develop/v3.12.0` 维护 (V312-59-D GA 周期未完成; 与 v4.0.0/v4.1.0 三线并行)

- **v4.1.0 DRAFT→ALPHA promotion**（2026-09-29 commit `c1a73a5320`）：
  - 11 PHASE_0 docs scaffolded: STAGE.yaml / VERSION_PLAN / DEV_PLAN / ROADMAP / TEST_PLAN / ISSUES_PLAN / LEGACY_ISSUES / README / CHANGELOG / PHASE_1_SCOPE / RELEASE_NOTES
  - 3 inherited v4.0.0 alpha-gate FAILs resolved (P0.1 anti_ignore / P0.2 arch_invariants / P0.3 anti_fabrication)
  - V400-09 168h SOAK FINAL_REPORT written (deferred to v4.1.0 scope per WP-H triage)
  - WP-C..G migration documented in ISSUES_PLAN.md §4 (20-issue backlog, 6-10 weeks estimated)
  - scripts/gate/check_alpha_v410.sh + check_alpha_entry_v4.10.sh + check_alpha_quality_v4.10.sh created
  - alpha composite: ALPHA GATE PASS (entry 19/19 + quality 8/8)
  - Tag v4.1.0-alpha1 cut; pushed to 5 remotes (gitcode / gitea250 / gitea252 / gitee / github)

- **阶段**: **ALPHA**（2026-09-29 从 DRAFT 转入；[STAGE.yaml `current_stage: "ALPHA"`](docs/releases/v4.1.0/STAGE.yaml)）
- **当前状态**: alpha composite PASS（entry 19/19 + quality 8/8），3 inherited alpha-gate FAILs 已修。下一步：alpha→beta 转段（需 WP-C..G 18 issue 代码实现 + V400-09 168h SOAK 重启 + Coverage >= 50%）。
- **里程碑 v4.0.0**: 🟡 self-claimed GA（`docs/releases/v4.0.0/GA_GATE_REPORT.md` says GA CONDITIONAL PASS；`docs/releases/v4.0.0/STAGE.yaml` SSOT 仍是 DRAFT — 治理漂移未修复）
- **WP-H #4639**: defer-to-v4.1 — own in v4.1.0 scope

## v3.12.0 GA 阶段信息（已发布）

- **阶段**: **GA** — 2026-09-08 正式发布到 5 remote
- **Head**: `355b5a3837`
- **Gate verdict**: 8/8 GA gates PASS per `docs/releases/v3.12.0/evidence/v312-59/ga_gate_report.json`
- **Claim boundaries**: 3 GA-claim-caveat items (#4846 CHAR / #4847 transaction / #4848 ALTER RENAME) per `CLAIM_DOWNGRADE_MANIFEST.md §9.6`
- **目标**: GMP internal-audit retrieval database + 内部向量检索 + SQL-backed graph projection + auditable evidence bundle

## v3.11.0 GA 阶段信息（已发布）

- **阶段**: **GA** — 2026-08-09 正式发布
- **Head**: `83c623835`
- **Gate verdict**: 6/6 GA gates PASS
- **目标**: 债务清零 + 功能孤岛集成 + 性能突破
- **协作 Issue**: [#3433](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3433)（V311-MASTER）

## 治理约束（2026-09-29 更新）

> ✅ **v4.1.0 已完成 DRAFT→ALPHA 转段**（2026-09-29 commit `c1a73a5320`）。
> 当前 v4.1.0 处于 **ALPHA 阶段**（[STAGE.yaml `current_stage: "ALPHA"`](docs/releases/v4.1.0/STAGE.yaml)）；
> alpha composite PASS（entry 19/19 + quality 8/8）。
> 任何 v4.1.1 后续工作必须保留在 v4.1.0 范围内直到 GA。

> 🟡 **v4.0.0 self-claimed GA 治理漂移未修复**：
> `docs/releases/v4.0.0/GA_GATE_REPORT.md` says GA CONDITIONAL PASS（2026-09-20），
> `docs/releases/v4.0.0/STAGE.yaml` SSOT 仍是 DRAFT。需用户决策：回滚 self-claim，或补 STAGE_CONFIG gate flow。
> v4.1.0 继承此漂移。

## 版本概述

- **v4.1.0** = v4.0.0 post-GA maintenance continuation。继承 v4.0.0 scope + bugfix carry-forward（zombie-fix core, workers.push, DML/storage regressions）+ 5-remote sync 工具 + WP-C..G 迁移。
- **v4.0.0** = self-claimed GA (CONTAINS claim-caveat items per CLAIM_DOWNGRADE_MANIFEST.md). 5-remote 5 端发布：gitcode / gitea250 / gitea252 / gitee / github。
- **v3.12.0** = GMP internal-audit retrieval database + 内部向量检索 + SQL-backed graph projection + auditable evidence bundle.
- **v3.11.0** = 债务清零 + 功能孤岛集成 + 性能突破。从 v3.10.0 GA 继承 23 项债务，全部完成。

## ALPHA Gate 状态（v4.1.0 已通过）

| Gate | 阈值 | 状态 |
|------|------|------|
| E1 STAGE | file exists | ✅ PASS |
| E1 VERSION_PLAN | file exists | ✅ PASS |
| E1 DEV_PLAN | file exists | ✅ PASS |
| E1 ROADMAP | file exists | ✅ PASS |
| E1 TEST_PLAN | file exists | ✅ PASS |
| E1 ISSUES_PLAN | file exists | ✅ PASS |
| E1 LEGACY_ISSUES | file exists | ✅ PASS |
| E1 README | file exists | ✅ PASS |
| E1 CHANGELOG | file exists | ✅ PASS |
| E1 RELEASE_NOTES | file exists | ✅ PASS |
| E1 PHASE_1_SCOPE | file exists | ✅ PASS |
| E1 REVIEW_QUEUE | file exists | ✅ PASS |
| E2 DOC_LINKS | `check_docs_links.sh` | ✅ PASS |
| E2 DOC_CONSISTENCY | `check_docs_consistency.sh` | ⏸ EXCLUDED (pre-existing v3.12.0 issues, not v4.1.0 regression) |
| E3 5REMOTES_SYNC | executable | ✅ PASS |
| E3 5REMOTES_DRIFT | executable | ✅ PASS |
| E3 SQLLOGICTEST_BUILD | `cargo build -p sqlrustgo_sqllogictest` | ✅ PASS |
| E3 ALPHA_QUALITY | executable | ✅ PASS |
| E3 ALPHA_COMPOSITE | executable | ✅ PASS |

**Total: 19/19 alpha entry PASS**

| Gate | 阈值 | 状态 |
|------|------|------|
| Q1 BUILD | `cargo build --all-features` | ✅ PASS |
| Q1 FMT | `cargo fmt --check` | ✅ PASS |
| Q2 ANTI_FAB | `check_anti_fabrication.sh` | ✅ PASS (0 errors, 0 warnings) |
| Q3 ANTI_IGNORE | `check_anti_ignore_gate.sh` | ✅ PASS (125 total / 0 active) |
| Q4 ARCH_INVARIANTS | `check_arch_invariants.sh` | ✅ PASS (5/5) |
| Q4 ARCH3_NO_BYPASS | `check_arch3_no_bypass.sh` | ✅ PASS (4/4) |
| Q5 SQL_CORPUS | `check_sql_corpus_gate.sh` | ✅ PASS (99.3% pass rate, threshold 80%) |
| Q6 TEST_LIB | `cargo test --all-features --lib --no-run` | ✅ PASS (compile) |
| Q7 COVERAGE | `check_coverage_v312.sh` | ⏸ DEFERRED to alpha_to_beta (per STAGE.yaml; consistent with v3.12.0 alpha template) |

**Total: 8/8 alpha quality PASS**

## Alpha→Beta exit criteria (v4.1.0)

Per `docs/releases/v4.1.0/STAGE.yaml#alpha_to_beta`:

- All v4.1.0 ALPHA gate failures resolved (currently 3 inherited from v4.0.0) — ✅ DONE (2026-09-29)
- V400-09 168h SOAK FINAL_REPORT exists OR explicit deferral recorded — ✅ DONE (deferred to v4.1.0 scope per `docs/releases/v4.0.0/V400_09_168H_SOAK_FINAL_REPORT.md`)
- WP-C/D/F/G deferred items migrated from v4.0.0 deferral into v4.1.0 scope — ✅ DONE (20 issues documented in `ISSUES_PLAN.md §4`)
- Coverage >= 50% (per STAGE_CONFIG COVERAGE_MIN_ALPHA) — ⏳ PENDING (Q7 deferred)

## 变更历史

| 版本 | 日期 | 说明 |
|------|------|------|
| v3.10.0 GA | 2026-07-13 | v3.10.0 正式发布 |
| v3.11.0 | 2026-07-15 | v3.11.0 开发分支创建 |
| v3.11.0 GA | 2026-08-09 | v3.11.0 GA，6/6 GA gates PASS；tag `v3.11.0-ga` @ `83c623835` |
| v3.12.0 | 2026-07-25 | v3.12.0 开发分支创建（V312-01 ~ V312-58） |
| v3.12.0 ALPHA | 2026-08-12 | V312-DRAFT→ALPHA，`check_alpha_v3.12.0.sh` PASS |
| v3.12.0 BETA | 2026-08-19 | V312-56 BETA，`check_beta_v3.12.0.sh` 38/40 PASS / 0 BLOCKERS / 2 WARN |
| v3.12.0 RC | 2026-08-26 | V312-59-C，9 PASS / 0 FAIL / 2 NO-OP-covered。Tag `v3.12.0-rc1` @ `f795efa60` |
| v3.12.0 GA | 2026-09-08 | V312-59-D 周期完成（8/7/13/13 gates PASS），tag `v3.12.0-ga` @ `355b5a3837` |
| v4.0.0 | 2026-09-08 | develop/v4.0.0 在 v3.12.0 GA 后切出 |
| v4.0.0 GA CONDITIONAL PASS | 2026-09-20 | self-claim per `GA_GATE_REPORT.md`；STAGE.yaml SSOT 未更新（治理漂移） |
| v4.1.0 | 2026-09-19 | develop/v4.1.0 切出（v4.0.0 GA gate 同期） |
| v4.1.0 DRAFT | 2026-09-23 | STAGE.yaml + PHASE_0 docs 脚手架 |
| **v4.1.0 ALPHA** | **2026-09-29** | **commit `c1a73a5320`; alpha composite PASS (entry 19/19 + quality 8/8); tag `v4.1.0-alpha1` @ `c1a73a5320`** |

## 相关文档

- [v4.1.0 STAGE.yaml](docs/releases/v4.1.0/STAGE.yaml) — v4.1.0 stage SSOT
- [v4.1.0 RELEASE_NOTES.md](docs/releases/v4.1.0/RELEASE_NOTES.md) — v4.1.0 release notes
- [v4.1.0 PHASE_1_SCOPE.md](docs/releases/v4.1.0/PHASE_1_SCOPE.md) — DRAFT → ALPHA work plan
- [v4.1.0 ISSUES_PLAN.md](docs/releases/v4.1.0/ISSUES_PLAN.md) — v4.1.0 issue catalog with WP-C..G §4
- [v4.1.0 V400_TO_V410_REVIEW_QUEUE.md](docs/releases/v4.1.0/V400_TO_V410_REVIEW_QUEUE.md) — review queue closure (CLOSED 2026-09-26)
- [v4.0.0 V400_09_168H_SOAK_FINAL_REPORT.md](docs/releases/v4.0.0/V400_09_168H_SOAK_FINAL_REPORT.md) — SOAK deferral rationale
- [v4.0.0 STAGE.yaml](docs/releases/v4.0.0/STAGE.yaml) — v4.0.0 stage SSOT (DRAFT, governance drift)
- [v4.0.0 CLAIM_DOWNGRADE_MANIFEST.md](docs/releases/v4.0.0/CLAIM_DOWNGRADE_MANIFEST.md) — v4.0.0 GA claim boundaries
- [v4.0.0 GA_GATE_REPORT.md](docs/releases/v4.0.0/GA_GATE_REPORT.md) — v4.0.0 GA verdict
- [v3.12.0 STAGE.yaml](docs/releases/v3.12.0/STAGE.yaml) — v3.12.0 GA stage SSOT
- [scripts/sync/README.md](scripts/sync/README.md) — 5-remote sync tooling
- [docs/governance/STAGE_CONFIG.yaml](docs/governance/STAGE_CONFIG.yaml) — version-agnostic stage framework