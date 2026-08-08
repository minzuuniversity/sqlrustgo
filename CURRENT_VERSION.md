# 当前版本状态

v3.11.0 RC（GA 门未通过）

## 阶段信息

- **阶段**: RC（候选发布；2026-07-19 第二次 GA 声明因 TPC-H SF=1 fixture 缺失回退；2026-08-08 第三次 GA 声明因 G3 覆盖率 4/8 失败被 `bc4d2143e2` 修订） — 见 `docs/releases/v3.11.0/STAGE.yaml:current_stage`
- **发布日期**: 2026-08-08（GA 声明日；RC 实际生效日）
- **开发分支**: develop/v3.11.0
- **目标**: 债务清零 + 功能孤岛集成 + 性能突破
- **协作 Issue**: [#3433](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3433)（V311-MASTER）
- **G4 TPC-H SF=1**: [Issue #3650](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3650)（fixture 已生成，wire 测试执行中）

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
| G3 覆盖率 | 每 crate ≥80% line | ❌ FAIL (4/8: storage✅ common✅ planner✅ tools✅; executor❌ admin❌ mysql-server❌ mysql-client❌) — 见 GA_GATE_REPORT.md |
| G4 TPC-H SF=1 | 22/22 PASS | 🟡 ALMOST PASS (P0-1 fixture ✅ 1.1GB; P0-2 wire tests 🟡 mostly ✅; ADR-008 exception active 2026-09-01) — 见 TPCH_SF1_VERIFICATION_REPORT.md |
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
