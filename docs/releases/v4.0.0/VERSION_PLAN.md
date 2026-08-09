# SQLRustGo v4.0.0 版本计划

> **版本**: v4.0.0
> **状态**: 规划中
> **日期**: 2026-08-08
> **产品目标**: 面向 SQL + Vector + Graph + GMP knowledge workloads 的生产级多模型数据库

## 1. 版本定位

v4.0.0 的目标是把 SQLRustGo 从“SQL 数据库 + GMP/RAG 内部能力”推进为一等公民级多模型数据库。它必须把关系行、向量记录、图节点/边、GMP audit event 放到统一的 transaction、WAL、backup/restore、access-control 和 observability 模型之下。

## 2. 对 v3.12.0 的依赖

v4.0.0 只有在 v3.12.0 用执行证据证明 GMP 内审检索产品之后，才应进入 RC。

| v3.12 结果 | v4.0 依赖 |
|---|---|
| GMP schema and ingestion | 成为 system-table 和 migration baseline |
| Internal vector retrieval | 升级为一等公民 vector DB surface |
| SQL-backed graph projection | 升级为一等公民 graph store |
| Audit chain and ALCOA+ controls | 升级为全局 data-integrity layer |
| Mixed SOAK | 成为 multi-model SOAK 的最低 baseline |

## 3. v4.0.0 产品范围

| 领域 | 范围 |
|---|---|
| SQL database | MySQL-compatible relational core |
| Vector database | vector column/index syntax、Flat/HNSW/IVF、metadata filtering |
| Graph database | property graph node/edge storage 与 traversal query layer |
| GMP knowledge layer | document、audit、CAPA、deviation、SOP、evidence graph |
| Unified storage | WAL-backed SQL/vector/graph/GMP writes |
| Unified operations | backup/restore、access control、audit、observability |

## 4. 工作包

| ID | 工作包 | 优先级 | 退出证据 |
|---|---|---|---|
| V400-01 | vector column 与 vector index SQL syntax | P0 | parser/executor E2E PASS |
| V400-02 | WAL-backed vector storage 与 rebuild | P0 | crash/rebuild tests |
| V400-03 | graph crate revival 或 rewrite | P0 | storage and traversal tests |
| V400-04 | graph query surface | P0 | bounded path query tests |
| V400-05 | cross-model transaction semantics | P0 | rollback and crash tests |
| V400-06 | unified backup/restore | P0 | restore SQL/vector/graph/GMP equality |
| V400-07 | unified ACL and audit | P0 | permission bypass tests fail |
| V400-08 | multi-model optimizer 与 metadata filters | P1 | query plan evidence |
| V400-09 | multi-model SOAK | P0 | 168h mixed report |

## 5. 发布里程碑

| 里程碑 | 目标 | 必需证据 |
|---|---|---|
| Alpha | vector SQL + storage prototype | vector E2E tests |
| Beta | graph store + traversal prototype | graph E2E tests |
| RC | unified transaction/backup/security | cross-model gates |
| GA | multi-model production | 168h SOAK + full GA report |

## 6. GA 声明边界

允许声明：

> SQLRustGo v4.0.0 是面向受控 SQL、vector、graph 和 GMP knowledge workloads 的生产级多模型数据库。

该声明只有在 vector 和 graph 都成为一等公民能力，并且具备 WAL-backed recovery、permission、backup、restore 与 SOAK 测试证据后才允许使用。

## 附录：英文原文

> 本附录保留本文件改写前的英文原文，便于追溯历史语义；当前正式阅读与执行口径以上方中文正文为准。

# SQLRustGo v4.0.0 Version Plan

> **Version**: v4.0.0
> **Status**: PLANNED
> **Date**: 2026-08-08
> **Product target**: production multi-model database for SQL + Vector + Graph + GMP knowledge workloads

## 1. Positioning

v4.0.0 promotes SQLRustGo from a SQL database with GMP/RAG internal capabilities into a first-class multi-model database. It must unify relational rows, vector records, graph nodes/edges, and GMP audit events under one transaction, WAL, backup/restore, access-control, and observability model.

## 2. Dependency on v3.12.0

v4.0.0 should only start RC after v3.12.0 proves the GMP internal-audit retrieval product with execution evidence.

| v3.12 outcome | v4.0 dependency |
|---|---|
| GMP schema and ingestion | becomes system-table and migration baseline |
| Internal vector retrieval | becomes first-class vector DB surface |
| SQL-backed graph projection | becomes first-class graph store |
| Audit chain and ALCOA+ controls | become global data-integrity layer |
| Mixed SOAK | becomes minimum multi-model SOAK baseline |

## 3. v4.0.0 Product Scope

| Area | Scope |
|---|---|
| SQL database | MySQL-compatible relational core |
| Vector database | vector column/index syntax, Flat/HNSW/IVF, metadata filtering |
| Graph database | property graph node/edge storage and traversal query layer |
| GMP knowledge layer | document, audit, CAPA, deviation, SOP, evidence graph |
| Unified storage | WAL-backed SQL/vector/graph/GMP writes |
| Unified operations | backup/restore, access control, audit, observability |

## 4. Work Packages

| ID | Package | Priority | Exit evidence |
|---|---|---|---|
| V400-01 | Vector column and vector index SQL syntax | P0 | parser/executor E2E PASS |
| V400-02 | WAL-backed vector storage and rebuild | P0 | crash/rebuild tests |
| V400-03 | Graph crate revival or rewrite | P0 | storage and traversal tests |
| V400-04 | Graph query surface | P0 | bounded path query tests |
| V400-05 | Cross-model transaction semantics | P0 | rollback and crash tests |
| V400-06 | Unified backup/restore | P0 | restore SQL/vector/graph/GMP equality |
| V400-07 | Unified ACL and audit | P0 | permission bypass tests fail |
| V400-08 | Multi-model optimizer and metadata filters | P1 | query plan evidence |
| V400-09 | Multi-model SOAK | P0 | 168h mixed report |

## 5. Release Milestones

| Milestone | Goal | Required evidence |
|---|---|---|
| Alpha | vector SQL + storage prototype | vector E2E tests |
| Beta | graph store + traversal prototype | graph E2E tests |
| RC | unified transaction/backup/security | cross-model gates |
| GA | multi-model production | 168h SOAK + full GA report |

## 6. GA Claim

Allowed GA claim:

> SQLRustGo v4.0.0 is a production multi-model database for controlled SQL, vector, graph, and GMP knowledge workloads.

This claim is only allowed after vector and graph are first-class, WAL-backed, permissioned, backed up, restored, and SOAK-tested.
