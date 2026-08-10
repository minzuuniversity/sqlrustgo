# SQLRustGo v3.12.0 版本计划

> **版本**: v3.12.0
> **状态**: 规划中
> **日期**: 2026-08-09
> **产品目标**: 面向 `~/gmp-platform` 的 GMP 内审检索数据库
> **commit**: 1903545df6d036f7f6d5035a0503b5fa932aac51

## 1. 版本定位

v3.12.0 是一个受控生产场景版本，目标是让 SQLRustGo 可以作为 `~/gmp-platform` 的 GMP 内审检索数据库，管理受监管文档元数据、chunk、embedding、audit log 和 evidence relation。

本版本不宣称 SQLRustGo 已成为通用向量数据库或通用图数据库。v3.12.0 中的向量检索和图遍历只作为 GMP/RAG 内部能力交付。

v3.12.0 同时承担 v3.11.0 GA 后的生产硬化任务：TPC-H correctness、coverage methodology、MySQL wire protocol、LOAD DATA、crash recovery、backup/restore、upgrade/downgrade，以及 v3.10 已规划但 v3.11 未阻断集成的 SQLite SQLLogicTest oracle gate。

本版本还承接 v3.6.0-v3.11.0 文档中已经规划但未完成闭环的功能和测试：`CREATE SEQUENCE` executor、Window/GIS/JSON 受控 SQL 功能、MySQL 兼容性 backlog、执行路径和优化器债务、存储/索引/WAL 工具债务、禁用测试和 API 漂移测试恢复、TPC-H SF=10、Sysbench、Prometheus metrics、Slow Query Log、SQL corpus all-target gate、架构 invariant 和双 reviewer 签核。

## 2. 对 v3.11.0 的依赖

v3.12.0 不能继承 v3.11.0 中未验证的生产声明。进入 GA 前，必须关闭或显式承接以下弱项：

| 阻断项 | 必须作出的决定 |
|---|---|
| G3 Coverage | 产生可复现的 per-crate 覆盖率证据，或作为 v3.12 P0 继续跟踪 |
| G4 TPC-H SF=1 | 产生 22/22 run evidence，并补齐跨引擎 row-count/SHA256 correctness，或作为 v3.12 P0 继续跟踪 |
| v3.11.0 168h SOAK | 完成、复跑，或明确限制 prior production claim 的适用范围 |
| SQLLogicTest / SQLite oracle | 从 v3.11 TBD 提升为 v3.12 可执行 gate |
| MySQL wire / LOAD DATA / recovery | 在扩大 MySQL 替代声明前补齐生产硬化 gate |
| debt registry drift | 按 truth audit 对齐 F-25/F-26 和 extension crate 状态 |
| `CREATE SEQUENCE` executor gap | 作为 v3.12 P0 完成执行器、并发、事务和 recovery 验证 |
| Window/GIS/JSON planned features | 作为受控 P1 功能交付，必须有 fixture 和 unsupported boundary |
| disabled/API-drift/flaky tests | 建立 test debt manifest，恢复、重写、隔离或有证据退休 |
| TPC-H SF=10 / Sysbench / observability | 作为性能和观测性 baseline，而不是生产性能宣传 |
| SQL corpus / architecture invariant / reviewer sign-off TBD | 纳入 RC/GA 阻断门禁 |
| v3.6.0 Beta PENDING 与 coverage/test compile 遗留 | 建立历史债务总账，不能引用旧文档 PASS 作为当前证据 |
| v3.7.0 SHOW/auth/prepared/transaction routing 限制 | 进入 MySQL compatibility regression；已关闭项必须有当前执行证据 |
| v3.8.0 frozen/backlog 功能 | 按 GMP 相关性进入 v3.12 实现、测试、延期或退休清单 |
| v3.9.0 real SOAK、ignored long tests、tx_wal autocommit | 进入 mixed SOAK、test-debt 和事务语义复核 |
| v3.10.0 SQLancer/test-runner/test-registry/E2E/anti-fabrication 缺口 | 进入测试基础设施激活工作包 |

## 3. 产品范围

| 领域 | v3.12.0 范围 |
|---|---|
| GMP relational store | documents、chunks、versions、status、metadata、audit rows |
| GMP vector retrieval | embedding 存入 SQLRustGo，并由 `sqlrustgo-vector` 建索引 |
| GMP graph projection | 以 SQL-backed nodes/edges 支持审计证据导航 |
| Hybrid retrieval | SQL filters + keyword + vector + graph relation boost + RRF |
| Compliance | ALCOA+、audit hash chain、access control、backup/restore evidence |
| Migration | 替代 GMP 生产路径中必需的 SQLite/Chroma/PostgreSQL runtime dependencies |
| SQL correctness | SQLite SQLLogicTest oracle、TPC-H cross-engine checks、sqllogictest baseline reports |
| Production hardening | wire protocol、LOAD DATA、crash recovery、backup/restore、upgrade/downgrade |
| SQL feature close-out | `CREATE SEQUENCE`、Window Functions、JSON path、GIS subset |
| Test debt close-out | disabled/API-drift tests、低覆盖 crate、flaky tests、无 target 测试 |
| Performance and observability | TPC-H SF=10、Sysbench OLTP、bulk-load baseline、Prometheus、Slow Query Log |
| Release governance | SQL corpus all-target、architecture invariant、reviewer sign-off |
| Historical backlog | v3.6-v3.10 规划任务、综合测试和遗留问题 disposition |
| MySQL compatibility backlog | SHOW、auth、prepared statements、ALTER、TIMESTAMP、connection pool、advanced functions、permissions |
| Execution and optimizer backlog | DML route、VTU、Parallel/SIMD、Hash Semi/Anti Join、subquery decorrelation、CBO/histogram |
| Storage/index/WAL backlog | WAL checkpoint、wal-verification、composite index、index statistics、checksum/torn-write |
| Test infrastructure | SQLancer、test-runner、test-registry、E2E scripts、anti-fabrication binaries |

## 4. 非目标

- 不宣称通用 Cypher 兼容。
- 不宣称独立通用向量数据库产品。
- 不宣称多租户 HA / distributed storage。
- 没有 168h mixed SOAK 前，不作生产可用声明。
- 不把“crate 存在”当成“feature production-ready”。

## 5. 工作包

| ID | 工作包 | 优先级 | 退出证据 |
|---|---|---|---|
| V312-01 | v3.11 弱项关闭与 debt registry 对齐 | P0 | 更新后的 GA/debt evidence |
| V312-02 | GMP schema v3.12 | P0 | schema tests PASS |
| V312-03 | 从 `~/gmp-platform/gmp-md` 导入 GMP markdown | P0 | corpus ingestion report |
| V312-04 | embedding provider 与 vector persistence | P0 | rebuildable vector index |
| V312-05 | 带 citation bundle 的 hybrid retrieval | P0 | retrieval quality report |
| V312-06 | SQL-backed graph projection | P0 | node/edge/path tests |
| V312-07 | audit hash chain 与 ALCOA+ controls | P0 | tamper tests fail closed |
| V312-08 | GMP/RAG/graph projection backup/restore | P0 | restore verification report |
| V312-09 | mixed workload SOAK | P0 | 168h report |
| V312-10 | GMP compliance docs 与用户运维指南 | P1 | signed compliance matrix |
| V312-11 | SQLite SQLLogicTest oracle gate | P0 | runner build/run output、SQLite corpus manifest、baseline report |
| V312-12 | TPC-H SF=1 correctness close-out | P0 | cross-engine row-count 与 SHA256 report |
| V312-13 | MySQL wire + LOAD DATA hardening | P0 | wire e2e、LOAD DATA row/hash/memory evidence |
| V312-14 | crash recovery 与 upgrade/downgrade verification | P0 | WAL replay、restore、upgrade、rollback reports |
| V312-15 | `CREATE SEQUENCE` executor close-out | P0 | sequence DDL/DML、并发、事务、WAL/recovery reports |
| V312-16 | Window/GIS/JSON controlled SQL features | P1 | SQL feature corpus 和 unsupported boundary report |
| V312-17 | coverage 与 disabled-test debt close-out | P0 | per-crate coverage、disabled/API-drift/flaky disposition |
| V312-18 | SF=10、Sysbench 与 observability baseline | P1 | benchmark、metrics、slow-query logs 和趋势报告 |
| V312-19 | SQL corpus、architecture invariant 与 reviewer sign-off | P0 | all-target corpus、R2.1-R2.8 output、2 reviewer evidence hash |
| V312-20 | v3.6-v3.10 cross-version backlog disposition | P0 | historical backlog disposition manifest |
| V312-21 | MySQL compatibility 与 SQL surface backlog | P1 | SHOW/auth/prepared/ALTER/TIMESTAMP/functions/permissions regression |
| V312-22 | execution architecture 与 optimizer debt close-out | P1 | route invariants、Parallel/SIMD traces、Q4 optimization disposition |
| V312-23 | storage/index/WAL tooling backlog | P1 | checkpoint、wal-verification、index、checksum/torn-write reports |
| V312-24 | test infrastructure activation | P0 | SQLancer/test-runner/test-registry/E2E/anti-fabrication artifacts |

## 6. 发布里程碑

| 里程碑 | 目标 | 必需证据 |
|---|---|---|
| Alpha | schema + representative ingestion + sqllogictest runner build | build/fmt/clippy + schema tests + SLT runner smoke |
| Beta | hybrid retrieval + graph projection + SLT smoke corpus | retrieval fixture + graph tests + SLT report |
| RC | compliance hardening + TPC-H correctness + wire/LOAD DATA/recovery + test-debt close-out + historical backlog disposition | audit、ACL、backup/restore、cross-engine、wire、LOAD DATA、sequence、coverage、SQL corpus、invariant、compat、test-infra reports |
| GA | GMP internal-audit production | 168h mixed SOAK + GA gate report + no unresolved v3.6-v3.11 carried P0 + 2 reviewer sign-off |

## 7. GA 声明边界

允许声明：

> SQLRustGo v3.12.0 可用于受控 GMP 内审检索生产工作负载，使用 SQLRustGo 管理关系存储、内部向量检索、SQL-backed graph projection 和可审计 evidence bundle。

禁止声明：

> SQLRustGo v3.12.0 是通用专用向量数据库或图数据库替代品。

同时禁止：

> 在 TPC-H correctness、SQLLogicTest、wire protocol、LOAD DATA、crash recovery、backup/restore 和 upgrade evidence 全部通过前，宣称 SQLRustGo v3.12.0 是广义 MySQL 5.7 替代品。

## 附录：英文原文

> 本附录保留本文件改写前的英文原文，便于追溯历史语义；当前正式阅读与执行口径以上方中文正文为准。

# SQLRustGo v3.12.0 Version Plan

> **Version**: v3.12.0
> **Status**: DRAFT
> **Date**: 2026-08-09
> **Product target**: GMP internal-audit retrieval database for `~/gmp-platform`
> **Planning source**: `docs/plans/2026-08-08-sqlrustgo-v312-v400-gmp-rag-graph-plan.md`

## 1. Positioning

v3.12.0 is a controlled production release for GMP internal-audit retrieval. It should make SQLRustGo usable by `~/gmp-platform` as the main database for regulated document metadata, chunks, embeddings, audit logs, and evidence relations.

v3.12.0 is not a general-purpose vector database or graph database release. Vector retrieval and graph traversal are internal GMP/RAG capabilities in this version.

v3.12.0 also carries a production-hardening track from the v3.11.0 GA assessment: TPC-H correctness, coverage methodology, MySQL wire protocol, LOAD DATA, crash recovery, backup/restore, upgrade/downgrade, and the previously deferred SQLite SQLLogicTest oracle gate.

## 2. Dependency on v3.11.0

v3.12.0 must not inherit unverified v3.11.0 production claims. Before v3.12.0 enters GA, the project must either close or explicitly carry the following v3.11.0 weak points:

| Blocker | Required decision |
|---|---|
| G3 Coverage | Produce reproducible per-crate coverage evidence or carry as v3.12 P0 |
| G4 TPC-H SF=1 | Produce 22/22 run evidence plus cross-engine row-count/SHA256 correctness, or carry as v3.12 P0 |
| v3.11.0 168h SOAK | Complete, repeat, or explicitly scope prior production claim |
| SQLLogicTest / SQLite oracle | Promote from v3.11 TBD to v3.12 executable gate |
| MySQL wire / LOAD DATA / recovery | Add missing production hardening gates before broader MySQL replacement claims |
| debt registry drift | Reconcile F-25/F-26 and extension crate state with truth audit |

## 3. v3.12.0 Product Scope

| Area | Scope |
|---|---|
| GMP relational store | Documents, chunks, versions, status, metadata, audit rows |
| GMP vector retrieval | Embeddings stored in SQLRustGo and indexed by `sqlrustgo-vector` |
| GMP graph projection | SQL-backed nodes and edges for audit evidence navigation |
| Hybrid retrieval | SQL filters + keyword + vector + graph relation boost + RRF |
| Compliance | ALCOA+, audit hash chain, access control, backup/restore evidence |
| Migration | Replace required SQLite/Chroma/PostgreSQL runtime dependencies for GMP production path |
| SQL correctness | SQLite SQLLogicTest oracle, TPC-H cross-engine checks, sqllogictest baseline reports |
| Production hardening | Wire protocol, LOAD DATA, crash recovery, backup/restore, upgrade/downgrade |

## 4. Non-Goals

- No general Cypher compatibility claim.
- No standalone vector database product claim.
- No multi-tenant HA/distributed storage claim.
- No production claim without 168h mixed SOAK.
- No link between "crate exists" and "feature is production-ready" without gate evidence.

## 5. Work Packages

| ID | Package | Priority | Exit evidence |
|---|---|---|---|
| V312-01 | v3.11 weak-point closure and debt registry reconciliation | P0 | Updated GA/debt evidence |
| V312-02 | GMP schema v3.12 | P0 | Schema tests PASS |
| V312-03 | GMP markdown ingestion from `~/gmp-platform/gmp-md` | P0 | Corpus ingestion report |
| V312-04 | Embedding provider and vector persistence | P0 | Rebuildable vector index |
| V312-05 | Hybrid retrieval with citation bundle | P0 | Retrieval quality report |
| V312-06 | SQL-backed graph projection | P0 | Node/edge/path tests |
| V312-07 | Audit hash chain and ALCOA+ controls | P0 | Tamper tests fail closed |
| V312-08 | Backup/restore for GMP/RAG/graph projection | P0 | Restore verification report |
| V312-09 | Mixed workload SOAK | P0 | 168h report |
| V312-10 | GMP compliance docs and user operations guide | P1 | Signed compliance matrix |
| V312-11 | SQLite SQLLogicTest oracle gate | P0 | Runner build/run output, SQLite corpus manifest, baseline report |
| V312-12 | TPC-H SF=1 correctness close-out | P0 | Cross-engine row-count and SHA256 report |
| V312-13 | MySQL wire + LOAD DATA hardening | P0 | Wire e2e, LOAD DATA row/hash/memory evidence |
| V312-14 | Crash recovery and upgrade/downgrade verification | P0 | WAL replay, restore, upgrade and rollback reports |

## 6. Release Milestones

| Milestone | Goal | Required evidence |
|---|---|---|
| Alpha | Schema + representative ingestion + sqllogictest runner build | Build/fmt/clippy + schema tests + SLT runner smoke |
| Beta | Hybrid retrieval + graph projection + SLT smoke corpus | Retrieval fixture + graph tests + SLT report |
| RC | Compliance hardening + TPC-H correctness + wire/LOAD DATA/recovery | audit, ACL, backup/restore, cross-engine, wire, LOAD DATA tests |
| GA | GMP internal-audit production | 168h mixed SOAK + GA gate report + no unresolved v3.11 carried P0 |

## 7. GA Claim

Allowed GA claim:

> SQLRustGo v3.12.0 is production-ready for controlled GMP internal-audit retrieval workloads using SQLRustGo-managed relational storage, internal vector retrieval, SQL-backed graph projection, and auditable evidence bundles.

Disallowed GA claim:

> SQLRustGo v3.12.0 is a general-purpose replacement for dedicated vector databases or graph databases.

Also disallowed:

> SQLRustGo v3.12.0 is a broad MySQL 5.7 replacement unless TPC-H correctness, SQLLogicTest, wire protocol, LOAD DATA, crash recovery, backup/restore, and upgrade evidence all pass.
