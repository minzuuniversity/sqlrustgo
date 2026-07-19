# 当前版本状态

v3.9.0 GA

## 阶段信息

- **阶段**: GA（正式发布）
- **发布日期**: 2026-07-10
- **开发分支**: develop/v3.9.0
- **目标**: Production Readiness — TPC-H 22/22 + Q9 6.7x 加速 + 168h SOAK PASS
- **协作 Issue**: [#3266](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3266)（168h SOAK）

## 版本概述

v3.9.0 是 Production Readiness 版本，聚焦三个核心目标：TPC-H 22/22 全通、Q9 6.7x 加速（600ms→90ms）、Q13 子查询三值逻辑修正，以及 168h SOAK 稳定性验证。

## 核心里程碑

| 里程碑 | 状态 |
|--------|------|
| TPC-H 22/22 in-process（SF=0.1）| ✅ |
| TPC-H 22/22 wire round-trip | ✅ |
| Cell-level MATCH 21/22（vs SQLite）| ✅ |
| Q9 6.7x 加速（600ms → 90ms）| ✅ |
| Q13 子查询修正 | ✅ |
| 72h SOAK 119h57m，0 错误，0 重连 | ✅ |
| 168h SOAK PASS | ✅ |
| G13 deadlock 修复（parking_lot RwLock）| ✅ |
| execution_engine.rs 2630 → 1471 行 | ✅ |
| Statement cache（1.7x 热路径加速）| ✅ |
| SCRAM-SHA-256 加固 | ✅ |
| TLS 1.3 默认启用 | ✅ |

## 已知限制（GA 条件通过）

| 项目 | 状态 | 说明 |
|------|------|------|
| 覆盖率均值 | ⚠️ ~67% < 85% | 条件通过；目标 v3.10.0 GA ≥80% per crate |
| TPC-H H/22 |

## v3.9.0 vs v3.8.0

| 方面 | v3.8.0 | v3.9.0 |
|------|---------|---------|
| TPC-H SF=0.1 | 22/22 | 22/22 ✅ |
| TPC-H SF=1 | 6/10 | 6/10（parser 限制）|
| Q9 耗时 | 600ms | **90ms**（6.7x）|
| Q1 耗时 | 150ms | **50ms**（3x）|
| SOAK | 72h 有 G13 deadlock | 168h PASS ✅ |
| Cell-level 匹配 | 18/22 | **21/22** |

## 开发时间线

| 版本 | 日期 | 目标 |
|------|------|------|
| v3.9.0-alpha | 2026-06-05 | 功能开发 |
| v3.9.0-beta | 2026-06-10 | RC 门禁开始 |
| v3.9.0-rc8 | 2026-07-08 | 最后 RC |
| v3.9.0 GA | 2026-07-10 | 正式发布 |

## 相关文档

- [v3.9.0 文档入口](docs/releases/v3.9.0/README.md)
- [v3.9.0 GA 发行说明](docs/releases/v3.9.0/ga/GA_RELEASE_NOTES.md)
- [GA 门禁报告](docs/releases/v3.9.0/ga/GA_GATE_REPORT.md)
- [v3.9.0 升级指南](docs/releases/v3.9.0/MIGRATION_GUIDE.md)

## 变更历史

| 版本 | 日期 | 说明 |
|------|------|------|
| 1.0 | 2026-02 | 首个正式版本 |
| ... | ... | ... |
| 4.0 | 2026-06-05 | 创建 v3.9.0 开发分支，基于 v3.8.0 GA |
| 5.0 | 2026-07-10 | v3.9.0 GA 发布 |
