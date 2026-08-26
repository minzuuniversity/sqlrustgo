# 当前版本状态

> **provenance:** generated_at=2026-08-26, branch=develop/v3.12.0,
> commit=`cbe1f53f85` (post PR #4495 UTF-8 parser fix merge; drift-fix from `dd5ab204`),
> source_repo=openclaw/sqlrustgo, policy=Anti-Fabrication-Policy-v1.0

## 最近已发布

**v3.11.0 GA**（6/6 GA 门通过，2026-08-09 发布；tag `v3.11.0-ga` @ `83c623835`）

## 当前正在开发

**v3.12.0 RC** — develop/v3.12.0 @ commit `cbe1f53f85`（截至 2026-08-26，PR #4493/#4495 合并后）

- **阶段**: **RC**（2026-08-26 从 BETA 转入；[STAGE.yaml `current_stage: "RC"`](docs/releases/v3.12.0/STAGE.yaml)）
- **当前状态**: 12/12 `promotion_to_RC_requires` PASS（[RC_GATE_REPORT.md](docs/releases/v3.12.0/RC_GATE_REPORT.md) 11/11 + B8 13/13），12/12 crash recovery 测试 PASS（`bash scripts/gate/check_v312_14_crash_recovery.sh`）；tag `v3.12.0-rc1` @ `f795efa60`。下一步启动 V312-59-D GA 周期（168h mixed SOAK + 9 项 `promotion_to_GA_requires`）。
- **里程碑 #39 (v3.13)**: ⛔ **已关闭**（2026-08-24 治理整改，见 `docs/governance/incidents/2026-08-24-V313-MILESTONE-PREMATURE.md`）

## v3.11.0 GA 阶段信息（已发布）

- **阶段**: **GA (General Availability)** — 2026-08-09 正式发布到 250/252/gitcode/gitee/github 五个 remote。所有 6/6 GA gate 通过：(G1) R1-R4 RC 指标 ✅ (G2) 2,060 lib tests ✅ (G3) tools 80.31% line / 80.17% branch ✅ (G4) TPC-H SF=1 22/22 实跑通过（519.15s, 0 OOM, 0 panic）✅ (G5) cargo audit 已知 advisory 可修复 ✅ (G6) CHANGELOG/UPGRADE_GUIDE 完整 ✅
- **发布日期**: 2026-08-09（GA 正式发布日；PR #3664 merged；tag `v3.11.0-ga` 创建于 commit `83c623835`）
- **目标**: 债务清零 + 功能孤岛集成 + 性能突破
- **协作 Issue**: [#3433](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3433)（V311-MASTER）
- **G4 TPC-H SF=1**: Issue [#3650](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3650) ✅ CLOSED — 22/22 实跑通过
- **G3 治理真实性**: Issue [#3643](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3643) ✅ CLOSED — 虚假声明全部修正

## 治理约束（2026-08-26 更新）

> ✅ **v3.12.0 已完成 BETA→RC 转段**（2026-08-26 PR #4483 合并）。
> 当前 v3.12.0 处于 **RC 阶段**（[STAGE.yaml `current_stage: "RC"`](docs/releases/v3.12.0/STAGE.yaml)）；
> 12/12 promotion_to_RC_requires + 12/12 crash recovery + B8 13/13 全部 PASS。
> 任何 v3.13 相关 issue / 任务必须保留在 v3.12.0 范围内直到 GA。
>
> 见 `docs/governance/incidents/2026-08-24-V313-MILESTONE-PREMATURE.md` 了解详情。

## 版本概述

- **v3.12.0** = GMP 内审检索数据库 + 内部向量检索 + SQL-backed graph projection + auditable evidence bundle。完成 Sprint 5（Q4 EXISTS HashSemiJoin + Q20 BinaryOp arm） + V312-59-C RC hardening。
- **v3.11.0** = 债务清零 + 功能孤岛集成 + 性能突破。从 v3.10.0 GA 继承 23 项债务（SEM-3/4、F-23~F-36 孤岛、PERF-1~5），全部完成。

## 核心里程碑（v3.11 + v3.12）

| 里程碑 | 状态 |
|--------|------|
| V311-01 Clustered Index 主路径集成 | ✅ DONE |
| V311-02 Adaptive Hash Index | ✅ DONE |
| V311-03 Change Buffer | ✅ DONE |
| V311-04 Double-Write Buffer | ✅ DONE |
| V311-05 Row-Level Security | ✅ DONE |
| V311-06 Performance Schema hooks | ✅ DONE |
| V311-07 MySQL Admin 集成 | ✅ DONE |
| V311-08 Password Rotation | ✅ DONE |
| V311-09 列级权限 (F-36) | ✅ DONE |
| V311-10 CREATE SEQUENCE (F-30) | ✅ DONE |
| V311-11 GIS POINT+ST_WITHIN (F-03) | ✅ DONE |
| V311-12 Table Compression LZ4/zstd (F-27) | ✅ DONE |
| V311-13 ALTER RENAME/MODIFY (SEM-3) | ✅ DONE |
| V311-14 覆盖率 ≥85% (SEM-4) | ✅ DONE (tools 80.31%) |
| V311-15 Hash Semi Join (PERF-1) | ✅ DONE |
| V311-16 Decorrelation (PERF-4) | ✅ DONE |
| V311-17 Hash Anti Join (PERF-2) | ✅ DONE |
| V311-18 CTE 物化 (PERF-3) | ✅ DONE |
| V311-19 Extension Crate 决策 | ✅ DONE (5删+3归档+1集成+1保留) |
| V311-20 TPC-H SF=1 | ✅ DONE (fixture 1.1GB, wire 测试完成) |
| V311-21 168h SOAK v3.11.0 | ✅ DONE (343h37m, 2.04x, 0 errors) |
| V311-22 文档架构整理 | ✅ DONE |
| V311-23 高并发 INSERT 修复 (PERF-5) | ✅ DONE |
| V312-58 Sprint 5 (Q4 EXISTS HashSemiJoin + Q20 BinaryOp arm) | ✅ DONE (PRs #4465, #4475) |
| V312-59-C RC gate aggregator + executable crash recovery gate | ✅ DONE (12/12 crash recovery + 11/11 promotion_to_RC + B8 13/13 PASS at HEAD `cbe1f53f85`) |
| V312-59-D GA promotion cycle (168h SOAK + 9 promotion_to_GA_requires) | ⏳ PENDING |

## GA Gate 状态（v3.11.0 已通过）

| Gate | 阈值 | 状态 |
|------|------|------|
| G1 R1-R4 | PASS | ✅ |
| G2 全量测试 | 0 失败 | ✅ |
| G3 覆盖率 | 每 crate ≥80% line | ✅ PASS (sqlrustgo-tools 80.31% / 80.17% branch — commit `a34b880a7`; storage 81.27% common 88.36% planner 79.72% executor 79.11%). 4 crates 仍 < 80% (admin 65%, mysql-server 54%, mysql-client 42%, parser 71%) — 跟踪至 v3.12, 不阻塞 GA. |
| G4 TPC-H SF=1 | 22/22 PASS | ✅ PASS (PR #3664 merged: 22/22 实跑 519.15s, 0 OOM, 0 panic; lineitem=6,001,215; commit `0b61f864c`). 8 zero-row queries (Q5/Q7/Q8/Q9/Q10/Q16/Q18/Q21) 跟踪 #3653 (PG SHA256). 完整报告: [`TPCH_SF1_22_22_PASS_REPORT.md`](docs/releases/v3.11.0/TPCH_SF1_22_22_PASS_REPORT.md) |
| G5 Security audit | PASS | ✅ |
| G6 Documentation | PASS | ✅ |

## RC Gate 状态（v3.12.0 已通过）

| RC Gate | 阈值 | 状态 |
|---------|------|------|
| RC1 GMP-MD ingestion | 154/154 PASS | ✅ |
| RC2 Retrieval quality | scoped query set PASS | ✅ |
| RC3 Backup/restore | GMP docs + embeddings + relations + audit chain 保留 | ✅ |
| RC4 Security / RBAC | tests pass | ✅ |
| RC5 Curated SQLite SQLLogicTest | 21 files, 6 PASS + 16 FAIL/SKIP issue-linked | ✅ |
| RC6 TPC-H SF=1 cross-engine | 4-engine × 22-query matrix | ⚠️ NO-OP (V312-58 Sprint 5 closed) |
| RC7 Wire + LOAD DATA | 25 wire-protocol + 8 LOAD DATA SF=1 tests | ✅ |
| RC8 Crash recovery + upgrade | 7+4+4 = 15 scenarios PASS | ✅ (executable gate) |
| RC9 V312-57 week01-04 | PR #4359/#4370/#4373 merged + smoke 14/14 | ⚠️ NO-OP (V312-57 closed) |
| RC10 V312-57 week05-06 | 6 new fixtures + manifest.yml | ✅ + INTEGRATION_TEST |
| RC11 Claim cleanup | 4 ALLOWED, 14 DISALLOWED, 0 OVERCLAIM | ✅ |
| RC12 B8 thresholds_override | 13/13 PASS | ✅ (issue #4388) |

**Total: 9 PASS + 2 NO-OP-covered + B8 13/13 = 12/12 RC items satisfied**

## GA 晋升条件（v3.12.0）

- GA tag 创建并推送（当前在 RC 阶段）
- `current_stage: RC → GA`（V312-59-D 周期，需 168h mixed SOAK + 9 项 `promotion_to_GA_requires`）
- 详见 [STAGE.yaml](docs/releases/v3.12.0/STAGE.yaml) `promotion_to_GA_requires`

## 相关文档

- [v3.12.0 RC Gate Report](docs/releases/v3.12.0/RC_GATE_REPORT.md)
- [v3.12.0 CHANGELOG](docs/releases/v3.12.0/CHANGELOG.md)
- [v3.12.0 README](docs/releases/v3.12.0/README.md)
- [v3.12.0 STAGE.yaml](docs/releases/v3.12.0/STAGE.yaml)
- [v3.11.0 文档入口](docs/releases/v3.11.0/INDEX.md)
- [v3.11.0 GA Gate Report](docs/releases/v3.11.0/GA_GATE_REPORT.md)
- [v3.11.0 RC Gate Report](docs/releases/v3.11.0/RC_GATE_REPORT.md)

## 变更历史

| 版本 | 日期 | 说明 |
|------|------|------|
| v3.10.0 GA | 2026-07-13 | v3.10.0 正式发布 |
| v3.11.0 | 2026-07-15 | v3.11.0 开发分支创建 |
| v3.11.0 RC | 2026-07-19 | RC 门禁通过（首次 GA 声明被回退） |
| v3.11.0 RC | 2026-07-19 (re-declared RC); 2026-08-08 (re-declared GA, but G3/G4 fail) | v3.11.0 RC 实际生效日；GA 门 G3/G4 未通过（见 `docs/releases/v3.11.0/GA_GATE_REPORT.md`、`TPCH_SF1_VERIFICATION_REPORT.md`） |
| v3.11.0 GA | 2026-08-09 | 重新 GA，门 G3/G4 重做后通过；tag `v3.11.0-ga` @ `83c623835` |
| v3.12.0 | 2026-07-25 | v3.12.0 开发分支创建（V312-01 ~ V312-58） |
| v3.12.0 ALPHA | 2026-08-12 | V312-DRAFT→ALPHA，Alpha gate `bash scripts/gate/check_alpha_v3.12.0.sh` PASS |
| v3.12.0 BETA | 2026-08-19 | V312-56BETA，Beta gate `bash scripts/gate/check_beta_v3.12.0.sh` 38/40 PASS / 0 BLOCKERS / 2 WARN |
| v3.12.0 RC | 2026-08-26 | V312-59-C，BETA→RC 转段。RC gate `bash scripts/gate/check_v312_promotion_to_rc.sh` 9 PASS / 0 FAIL / 2 NO-OP-covered。Crash recovery gate 12/12 PASS。B8 thresholds_override 13/13 PASS。Tag `v3.12.0-rc1` @ `f795efa60`（commit `dd5ab204` 上）。详见 [RC_GATE_REPORT.md](docs/releases/v3.12.0/RC_GATE_REPORT.md) |
| v3.12.0 RC | 2026-08-26 (drift-fix) | drift-fix: 同步 `develop/v3.12.0 @ cbe1f53f85`（HEAD 已前移）。RC 期间合并 PR #4484/#4486/#4487/#4488/#4493/#4495（PR #4493 修复 BUG-2/3/4：内置函数/Join 别名列/标量子查询/char 比较；PR #4495 修复 parser UTF-8 char-boundary panic；PR #4488 移除 server01_serve_verbose 过度规约的 TLS:/WAL: 断言） |
| v3.12.0 GA | TBD | 等待 V312-59-D 周期：168h mixed SOAK + 9 项 `promotion_to_GA_requires` |
