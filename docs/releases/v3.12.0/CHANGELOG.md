# SQLRustGo v3.12.0 变更日志

> **状态**: 规划中
> **日期**: 2026-08-09

## v3.12.0-planned

这是面向 GMP 合规内审检索场景的初始规划条目。v3.12.0 的目标不是扩张宣传口径，而是在 v3.11.0 GA 的基础上补齐生产弱项，并为 `~/gmp-platform` 提供可审计、可恢复、可验证的数据库底座。

## 2026-08-09 规划更新

v3.12.0 被调整为双主线版本：

| 主线 | 目标 |
|---|---|
| A：v3.11.0 弱项补强 | 在扩大生产声明前，关闭或显式重门禁 v3.11.0 的覆盖率、TPC-H、wire protocol、LOAD DATA、恢复、升级和 SQLLogicTest 缺口 |
| B：GMP 内审检索数据库契约 | 为 `~/gmp-platform` 提供 SQLRustGo 管理的关系存储、向量检索、图谱投影和证据包输出 |

从 v3.11.0 综合评估报告继承的 P0 补强范围包括：

- TPC-H SF=1 跨引擎 row-count 与 SHA256 正确性验证。
- 覆盖率测量口径统一，并形成唯一 G3 命令。
- MySQL wire protocol E2E 硬化。
- `LOAD DATA` / bulk import 的行数、hash、内存和耗时验证。
- crash recovery、backup/restore、upgrade/downgrade 证据。
- dependency audit 复跑和例外登记。
- SQLite SQLLogicTest oracle gate。

SQLLogicTest 背景：

- v3.10.0 已在 `TESTING_SYSTEM_BETA_REPORT.md` 和 V310-14 中规划 SQLite 官方 SQLLogicTest 集成。
- v3.11.0 阶段要求中写过 `sqllogictest runner all targets PASS`，但 `RELEASE_GATE_CHECKLIST.md` 仍显示为 TBD，没有成为阻断门禁。
- v3.12.0 将 `crates/sqlrustgo_sqllogictest` 提升为显式 gate，要求 build/run 输出、语料 manifest、排除清单和 baseline report。
- 2026-08-09 本地基线：`cargo build -p sqlrustgo_sqllogictest` 可完成但依赖仍有 warning；本地 smoke corpus 可运行，但当前仅 6/16 文件通过，通过率 27.3%，因此这是失败基线，不是 gate PASS。

### 已规划内容

- GMP document、chunk、embedding、audit、relation schema。
- 从 `~/gmp-platform/gmp-md` 幂等导入 GMP 文档。
- 由 SQLRustGo 管理 embedding 持久化和 vector index rebuild。
- 混合检索：精确 SQL filter、keyword score、vector score、graph relation boost 和 RRF。
- SQL-backed graph projection，用于 GMP 证据导航。
- ALCOA+ 合规矩阵、RBAC/ACL 检查、audit hash-chain tamper test。
- GMP/RAG/graph projection 状态的 backup/restore 验证。
- 覆盖 SQL、ingestion、retrieval、audit、backup/restore 的 168h mixed SOAK。
- SQLite SQLLogicTest smoke 和 curated corpus gate。
- TPC-H correctness、MySQL wire、LOAD DATA、recovery、upgrade gate。

### v3.12.0 不规划的声明

- 不声明通用向量数据库。
- 不声明通用图数据库。
- 不声明 Cypher 兼容。
- 不声明分布式 HA。

## 版本历史

| 版本 | 日期 | 阶段 | 说明 |
|---|---|---|---|
| v3.12.0 | TBD | PLANNED | GMP 内审检索数据库版本，附带 v3.11.0 弱项补强 |

## 附录：英文原文

> 本附录保留本文件改写前的英文原文，便于追溯历史语义；当前正式阅读与执行口径以上方中文正文为准。

# Changelog -- SQLRustGo v3.12.0

> **Status**: PLANNED
> **Date**: 2026-08-09

## v3.12.0-planned

Initial planning entry for the GMP internal-audit retrieval release.

## 2026-08-09 Planning Update

Updated v3.12.0 as a dual-track release:

- Track A: close v3.11.0 weak points before broader production claims.
- Track B: deliver the GMP internal-audit retrieval database contract for `~/gmp-platform`.

Added P0 hardening scope from the v3.11.0 comprehensive assessment:

- TPC-H SF=1 cross-engine row-count and SHA256 correctness.
- Coverage methodology reconciliation and single G3 command.
- MySQL wire protocol e2e hardening.
- `LOAD DATA` / bulk import benchmark and count/hash validation.
- Crash recovery, backup/restore, and upgrade/downgrade evidence.
- Dependency audit refresh.
- SQLite SQLLogicTest oracle gate.

SQLLogicTest context:

- v3.10.0 planned SQLite official SQLLogicTest integration in `TESTING_SYSTEM_BETA_REPORT.md` and V310-14.
- v3.11.0 kept `sqllogictest runner all targets PASS` in stage requirements, but `RELEASE_GATE_CHECKLIST.md` still marked it TBD.
- v3.12.0 promotes `crates/sqlrustgo_sqllogictest` to an explicit gate with build/run output, corpus manifest, exclusion registry, and baseline reports.
- 2026-08-09 local baseline: `cargo build -p sqlrustgo_sqllogictest` completes with dependency warnings; local smoke corpus runs but currently reports 6/16 files passing and 27.3% pass rate, so this is a failing baseline rather than a gate PASS.

### Planned

- GMP document, chunk, embedding, audit, and relation schema.
- Idempotent ingestion from `~/gmp-platform/gmp-md`.
- SQLRustGo-managed embedding persistence and vector index rebuild.
- Hybrid retrieval with exact SQL filters, keyword score, vector score, graph relation boost, and RRF.
- SQL-backed graph projection for GMP evidence navigation.
- ALCOA+ compliance matrix, role-based access checks, and audit hash-chain tamper tests.
- Backup/restore verification for GMP/RAG/graph projection state.
- 168h mixed SOAK covering SQL, ingestion, retrieval, audit, and backup/restore.
- SQLite SQLLogicTest smoke and curated corpus gates.
- TPC-H correctness, MySQL wire, LOAD DATA, recovery, and upgrade gates.

### Not Planned For v3.12.0

- General-purpose vector database claim.
- General-purpose graph database claim.
- Cypher compatibility claim.
- Distributed HA claim.

## Version History

| Version | Date | Stage | Notes |
|---|---|---|---|
| v3.12.0 | TBD | PLANNED | GMP internal-audit retrieval database |
