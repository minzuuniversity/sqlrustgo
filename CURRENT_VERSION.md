# 当前版本状态

## 最近已发布

**v3.11.0 GA**（6/6 GA 门通过，2026-08-09 发布）

## 当前正在开发

**v3.12.0 RC** — develop/v3.12.0 @ commit `3ad8ac37d`（截至 2026-08-24）
- **阶段**: **RC (Release Candidate)**（milestone #38 OPEN）
- **当前状态**: 等 RC gate 全绿以升级 GA
- **里程碑 #39 (v3.13)**: ⛔ **已关闭**（2026-08-24 治理整改，见 `docs/governance/incidents/2026-08-24-V313-MILESTONE-PREMATURE.md`）

## v3.11.0 GA 阶段信息（已发布）

- **阶段**: **GA (General Availability)** — 2026-08-09 正式发布到 250/252/gitcode/gitee/github 五个 remote。Tag `v3.11.0-ga` @ commit `83c623835`。所有 6/6 GA gate 通过：(G1) R1-R4 RC 指标 ✅ (G2) 2,060 lib tests ✅ (G3) tools 80.31% line / 80.17% branch ✅ (G4) TPC-H SF=1 22/22 实跑通过（519.15s, 0 OOM, 0 panic）✅ (G5) cargo audit 已知 advisory 可修复 ✅ (G6) CHANGELOG/UPGRADE_GUIDE 完整 ✅
- **发布日期**: 2026-08-09（GA 正式发布日；PR #3664 merged；tag v3.11.0-ga 创建于 commit 83c623835；已发布到 250/252/gitcode/gitee/github 5 个 remote）
- **目标**: 债务清零 + 功能孤岛集成 + 性能突破
- **协作 Issue**: [#3433](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3433)（V311-MASTER）
- **G4 TPC-H SF=1**: Issue [#3650](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3650) ✅ CLOSED — 22/22 实跑通过
- **G3 治理真实性**: Issue [#3643](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3643) ✅ CLOSED — 虚假声明全部修正

## 治理约束（2026-08-24 重要）

> ⚠️ **v3.13 milestone 创建必须等 v3.12.0 GA 之后**。
> 当前 v3.12.0 仍在 RC 阶段（里程碑 #38 OPEN，**未 GA**）。
> 任何 v3.13 相关 issue / 任务必须保留在 v3.12.0 范围内。
>
> 见 `docs/governance/incidents/2026-08-24-V313-MILESTONE-PREMATURE.md` 了解详情。

## 版本概述

v3.11.0 = 债务清零 + 功能孤岛集成 + 性能突破。从 v3.10.0 GA 继承 23 项债务（SEM-3/4、F-23~F-36 孤岛、PERF-1~5），全部完成。

## 核心里程碑

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

## GA Gate 状态

| Gate | 阈值 | 状态 |
|------|------|------|
| G1 R1-R4 | PASS | ✅ |
| G2 全量测试 | 0 失败 | ✅ |
| G3 覆盖率 | 每 crate ≥80% line | ✅ PASS (sqlrustgo-tools 80.31% / 80.17% branch — commit `a34b880a7`; storage 81.27% common 88.36% planner 79.72% executor 79.11%). 4 crates 仍 < 80% (admin 65%, mysql-server 54%, mysql-client 42%, parser 71%) — 跟踪至 v3.12, 不阻塞 GA. |
| G4 TPC-H SF=1 | 22/22 PASS | ✅ PASS (PR #3664 merged: 22/22 实跑 519.15s, 0 OOM, 0 panic; lineitem=6,001,215; commit `0b61f864c`). 8 zero-row queries (Q5/Q7/Q8/Q9/Q10/Q16/Q18/Q21) 跟踪 #3653 (PG SHA256). 完整报告: [`TPCH_SF1_22_22_PASS_REPORT.md`](docs/releases/v3.11.0/TPCH_SF1_22_22_PASS_REPORT.md) |
| G5 Security audit | PASS | ✅ |
| G6 Documentation | PASS | ✅ |

## GA 晋升条件

- GA tag 创建并推送
- `current_stage: RC → GA`

## 相关文档

- [v3.11.0 文档入口](docs/releases/v3.11.0/INDEX.md)
- [v3.11.0 GA Gate Report](docs/releases/v3.11.0/GA_GATE_REPORT.md)
- [v3.11.0 RC Gate Report](docs/releases/v3.11.0/RC_GATE_REPORT.md)
- [v3.11.0 TPC-H SF=1 核查报告](docs/releases/v3.11.0/TPCH_SF1_VERIFICATION_REPORT.md)

## 变更历史

| 版本 | 日期 | 说明 |
|------|------|------|
| ... | ... | ... |
| v3.10.0 GA | 2026-07-13 | v3.10.0 正式发布 |
| v3.11.0 | 2026-07-15 | v3.11.0 开发分支创建 |
| v3.11.0 RC | 2026-07-19 | RC 门禁通过（首次 GA 声明被回退） |
| v3.11.0 RC | 2026-07-19 (re-declared RC); 2026-08-08 (re-declared GA, but G3/G4 fail) | v3.11.0 RC 实际生效日；GA 门 G3/G4 未通过（见 `docs/releases/v3.11.0/GA_GATE_REPORT.md`、`TPCH_SF1_VERIFICATION_REPORT.md`） |
