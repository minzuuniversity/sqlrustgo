# SQLRustGo v3.12.0 Issue 计划

> **版本**: v3.12.0
> **状态**: 规划中
> **日期**: 2026-08-09

本文件把 v3.12.0 计划拆分为 issue 粒度的工作包。每个条目只有在产生执行证据后，才可以标记为完成。

## V312-01：前序版本阻断项处置

**优先级**: P0
**目标**: 确保 v3.12.0 不继承 v3.11.0 隐藏生产弱项。
**范围**: 对齐 G3 coverage、G4 TPC-H SF=1、168h SOAK、`debt-registry.yaml` 与 truth audit，重点关注 F-25/F-26 和 extension crate 状态。
**验收**: blocker disposition report、stage gate output，以及所有 PASS claim 的 evidence hash。

## V312-02：GMP Schema v3.12

**优先级**: P0
**目标**: 定义由 SQLRustGo 管理的 GMP document、chunk、embedding、audit log 和 relation tables。
**验收**: schema creation 幂等；保留 document version history；audit rows 包含 previous hash 和 event hash；relation rows 能表达 SOP、clause、CAPA、deviation、role、equipment 关系。

## V312-03：GMP Corpus Ingestion

**优先级**: P0
**目标**: 将 `~/gmp-platform/gmp-md` 导入 SQLRustGo 管理的 GMP tables。
**验收**: ingestion report 包含 document/chunk/relation/embedding/skipped/failure 计数；re-ingestion 幂等；源文件变更产生新 document version；保留 source path 和 source hash。

## V312-04：Embedding Provider 与 Vector Persistence

**优先级**: P0
**目标**: 用 SQLRustGo 管理的 embedding storage 和 vector index rebuild 替代 GMP 生产路径对 Chroma runtime 的必需依赖。
**验收**: embedding 存入 SQLRustGo tables；fixture 可重建 Flat 或 HNSW index；记录 model name、dimension、vector hash、chunk id；支持 BGE-M3/Ollama-compatible flows。

## V312-05：Hybrid Retrieval

**优先级**: P0
**目标**: 使用 SQL filters、keyword score、vector similarity、graph relation boost 和 RRF fusion 实现 GMP 内审检索。
**验收**: 每个 result 含 source path、document id、version、chunk id、chunk hash、score components、citation text；支持按 document type、chapter、status、effective date、relation type 过滤；固定 audit question fixture 结果确定。

## V312-06：SQL-backed Graph Projection

**优先级**: P0
**目标**: 为 GMP evidence navigation 实现 SQL-backed graph projection。
**验收**: nodes/edges 存储 document、clause、SOP、CAPA、deviation、equipment、role、audit finding；支持 depth <= 3 的 neighbor/path query；每条 path 输出 evidence bundle。

## V312-07：RAG Evidence Bundle

**优先级**: P0
**目标**: 让 GMP RAG 的每个回答都可追溯、可复核。
**验收**: 生成回答只能使用已引用 chunk；每个 answer/result 带 citation bundle；缺失 citation 时 fail closed；answer envelope 传递 evidence hash。

## V312-08：Compliance、Audit Trail 与 Access Control

**优先级**: P0
**目标**: 将 SQLRustGo 行为映射到 GMP/ALCOA+ 控制。
**验收**: import/search/export/approve/backup/restore/review 均进入 audit hash chain；ACL 覆盖 SQL/vector/graph/GMP retrieval；tamper 和 unauthorized access 测试 fail closed。

## V312-09：Backup/Restore 与 Upgrade Path

**优先级**: P0
**目标**: 验证 GMP/RAG/graph projection 状态可备份、恢复、升级和回滚。
**验收**: restore 后 row count、hash、embedding count、graph edge count 一致；覆盖 v3.10/v3.11 到 v3.12 的升级和 rollback fixture。

## V312-10：Mixed Workload SOAK

**优先级**: P0
**目标**: 运行覆盖 SQL、ingestion、retrieval、audit、backup/restore 的 168h mixed SOAK。
**验收**: 0 crash；audit-chain 不断裂；memory growth 有界；检索质量 fixture 周期性复核。

## V312-11：SQLite SQLLogicTest Oracle Gate

**优先级**: P0
**目标**: 把 v3.10 规划、v3.11 未集成到 gate 的 SQLite SQLLogicTest 变成 v3.12 阻断门禁。
**背景**: v3.10 报告提出使用 SQLite 官方 SQLLogicTest corpus；V310-14 记录 runner 已实现但官方 suite 下载受阻；当前 `crates/sqlrustgo_sqllogictest` 存在，含 22 个本地 `.test` 文件；v3.11 `RELEASE_GATE_CHECKLIST.md` 仍把 sqllogictest runner 标为 TBD。
**验收**: `cargo build -p sqlrustgo_sqllogictest` 成功；本地 smoke corpus 产生报告；官方/cached SQLite corpus 有 manifest、hash、file count、exclusion policy；新增或规划 `scripts/gate/check_sqllogictest_v312.sh`。

## V312-12：TPC-H SF=1 Correctness Close-out

**优先级**: P0
**目标**: 从“22/22 可运行”推进到“row-count 和 SHA256 正确性可解释”。
**验收**: SQLRustGo 与 SQLite/PostgreSQL/MySQL 至少一个外部基准引擎完成 22 query row-count/SHA256 对比；zero-row query 有独立 issue、owner、期限和解释。

## V312-13：MySQL Wire + LOAD DATA Hardening

**优先级**: P0
**目标**: 补齐 MySQL wire protocol 和 bulk import 生产风险。
**验收**: COM_QUERY、COM_STMT_PREPARE/EXECUTE/CLOSE、error packet、reset、TLS/compression 有 E2E artifact；SF=1/SF=10 `LOAD DATA` 有 row count、hash、memory cap、duration 证据。

## V312-14：Crash Recovery 与 Upgrade/Downgrade Verification

**优先级**: P0
**目标**: 证明 SQLRustGo 在 crash、restore、upgrade、rollback 场景下不破坏 GMP/RAG/graph 数据。
**验收**: kill -9、WAL replay、dirty page recovery、backup/restore checksum、v3.10/v3.11 fixture upgrade to v3.12 和 rollback verification 均有日志与 hash。

## V312-15：CREATE SEQUENCE Executor Close-out

**优先级**: P0
**目标**: 关闭 v3.11.0 综合评估中记录的 `CREATE SEQUENCE` executor gap。
**范围**: sequence DDL、`NEXTVAL`、default expression、并发取值、事务 rollback、WAL replay、backup/restore 后继续取值。
**验收**: sequence 正反例 SQL fixture 全部有输出；并发测试无重复值或回退；crash/recovery 后 sequence state 与 row/hash summary 一致。

## V312-16：Window/GIS/JSON 受控 SQL 功能交付

**优先级**: P1
**目标**: 按 v3.11.0 后续计划交付受控 SQL feature，而不是在 release note 中泛化声明。
**范围**: Window Functions 覆盖 `ROW_NUMBER`、`RANK`、`DENSE_RANK`；JSON 覆盖 JSON type 与 JSON path 基础查询；GIS 覆盖 `ST_Distance`、`ST_Intersects`、GeoJSON 输入输出。
**验收**: 每个功能都有正例、反例、unsupported boundary fixture；执行证据进入 SQL feature corpus report。

## V312-17：Coverage 与 Disabled-Test Debt Close-out

**优先级**: P0
**目标**: 处理 v3.11.0 禁用测试分析、低覆盖 crate 和 flaky test 遗留问题。
**范围**: parser、mysql-server、mysql-client coverage；历史 disabled/API-drift tests；无独立 test target 测试；`test_wal_perf_throughput` flaky。
**验收**: canonical coverage command 固化；per-crate coverage 报告输出；每个 disabled/API-drift/flaky 项都有 restore、rewrite、quarantine 或 retire 决策，并包含 issue、owner、expiry 和 evidence hash。

## V312-18：SF=10、Sysbench 与 Observability Baseline

**优先级**: P1
**目标**: 补齐 v3.11.0 后续计划中的性能和观测性基础数据。
**范围**: TPC-H SF=10、Sysbench OLTP mixed workload、bulk-load benchmark、Prometheus metrics、Slow Query Log。
**验收**: 每个 benchmark 有可复跑脚本、数据集说明、阈值、日志路径、趋势对比；Prometheus scrape 和慢查询日志有 e2e artifact。

## V312-19：SQL Corpus、Architecture Invariant 与 Reviewer Sign-off Gate

**优先级**: P0
**目标**: 把 v3.11.0 release checklist 中仍为 TBD 的 SQL corpus、架构 invariant 和双 reviewer 签核变成 v3.12 RC/GA 阻断 gate。
**范围**: `test_sql_corpus.sh` all targets、R2.1-R2.8 invariant scripts、reviewer sign-off template。
**验收**: SQL corpus all-target report、R2.1-R2.8 输出、2 名独立 reviewer 签核均包含 command output、timestamp、source agent、source run、evidence hash 和 output location。

## V312-20：v3.6-v3.10 Cross-version Backlog Disposition

**优先级**: P0
**目标**: 建立 v3.6.0 到 v3.10.0 规划任务、综合测试和历史遗留问题总账，防止旧债务在 v3.12 计划中失踪。
**范围**: v3.6 Beta PENDING、coverage/test compile、DML 双路径；v3.7 SHOW/auth/prepared/transaction routing；v3.8 frozen/backlog；v3.9 real SOAK/ignored tests；v3.10 SQLancer/test-runner/test-registry/E2E/anti-fabrication/coverage baseline。
**验收**: `historical-backlog-disposition.yml` 中每项状态只能是 `closed`、`superseded`、`carried`、`deferred` 或 `retired`；所有 `carried` 项必须关联 V312 issue、owner、expiry、evidence hash。

## V312-21：MySQL Compatibility 与 SQL Surface Backlog

**优先级**: P1
**目标**: 复核并补齐 v3.7-v3.10 历史文档中记录的 MySQL 兼容和 SQL surface 缺口。
**范围**: `SHOW TABLES`/metadata、empty-password auth edge、prepared statements、ALTER TABLE RENAME/MODIFY/ADD/DROP、TIMESTAMP、connection pool、stored procedure tokens、column-level permissions、ROLLUP/CUBE/REPLACE/RANK、advanced aggregates。
**验收**: 对 GMP/生产路径相关子集给出 fixture PASS；非目标项必须输出 explicit unsupported 或 deferred decision，不得在 release note 中无边界宣称支持。

## V312-22：Execution Architecture 与 Optimizer Debt Close-out

**优先级**: P1
**目标**: 复核 v3.6-v3.10 的执行路径统一、Parallel/SIMD 主路径和 Q4 优化债务。
**范围**: DML 是否仍绕过 PhysicalPlan/LocalExecutor；VTU/Parallel/SIMD 是否为主路径能力；Hash Semi Join、Anti Join、subquery decorrelation、CBO/histogram 是否需要进入 v3.12 或延期。
**验收**: 执行路径 invariant 脚本有输出；Q4/相关子查询 benchmark 有 baseline；未完成优化不得支撑性能声明。

## V312-23：Storage、Index 与 WAL Tooling Backlog

**优先级**: P1
**目标**: 复核 v3.8-v3.10 遗留的存储、索引和 WAL 工具项，补齐与 GMP 生产路径相关的可靠性验证。
**范围**: WAL checkpoint optimization、wal-verification 工具重设计、复合索引、索引统计、page checksum、partial write/torn page、vector SQL surface 与 index rebuild。
**验收**: 每项有测试、脚本或延期理由；GMP/RAG 必需的 storage invariant 必须进入 RC gate。

## V312-24：Test Infrastructure Activation

**优先级**: P0
**目标**: 将 v3.10.0 中记录为骨架或未充分使用的测试基础设施激活，或有证据地退休。
**范围**: `crates/sqlancer`、`crates/test-runner`、`crates/test-registry`、E2E shell scripts、anti-fabrication known broken test binaries、SQL corpus/SQLLogicTest runner integration。
**验收**: 每个工具可以运行并产生 artifact；如退休或延期，必须说明原因、替代 gate、owner 和 expiry。已知 broken test binaries 不得继续靠 WARN-only 掩盖。

## 附录：英文原文

> 本附录保留本文件改写前的英文原文，便于追溯历史语义；当前正式阅读与执行口径以上方中文正文为准。

# SQLRustGo v3.12.0 Issues Plan

> **Version**: v3.12.0
> **Status**: PLANNED
> **Date**: 2026-08-09

This file breaks the v3.12.0 plan into issue-sized work packages. Every item must produce execution evidence before it can be marked complete.

## V312-01: Prior-Release Blocker Disposition

**Priority**: P0

**Goal**: Ensure v3.12.0 does not inherit hidden v3.11.0 production weak points.

**Scope**:
- Reconcile v3.11.0 G3 coverage.
- Reconcile v3.11.0 G4 TPC-H SF=1.
- Decide v3.11.0 168h SOAK status.
- Reconcile `debt-registry.yaml` against v3.11.0 truth audit, especially F-25/F-26 and extension crate states.

**Exit evidence**:
- Updated blocker disposition report.
- Stage gate output.
- Evidence hashes for any PASS claim.

## V312-02: GMP Schema v3.12

**Priority**: P0

**Goal**: Define SQLRustGo-managed GMP tables for documents, chunks, embeddings, audit logs, and relations.

**Acceptance**:
- Schema creation is idempotent.
- Document version history is preserved.
- Audit rows include previous hash and event hash.
- Relation rows can model SOP, clause, CAPA, deviation, role, and equipment relationships.

## V312-03: GMP Corpus Ingestion

**Priority**: P0

**Goal**: Import `~/gmp-platform/gmp-md` into SQLRustGo-managed GMP tables.

**Acceptance**:
- Full corpus ingestion report includes document count, chunk count, relation count, embedding count, skipped files, and failures.
- Re-ingestion is idempotent.
- Modified source files create new document versions.
- Source path and source hash are preserved.

## V312-04: Embedding Provider and Vector Persistence

**Priority**: P0

**Goal**: Replace required Chroma runtime dependency for the GMP production path with SQLRustGo-managed embedding storage and vector index rebuild.

**Acceptance**:
- Embeddings are stored in SQLRustGo tables.
- Fixed fixture can rebuild a Flat or HNSW index.
- Model name, dimension, vector hash, and chunk id are recorded.
- Embedding provider abstraction supports BGE-M3/Ollama-compatible flows.

## V312-05: Hybrid Retrieval

**Priority**: P0

**Goal**: Implement GMP internal-audit retrieval using SQL filters, keyword score, vector similarity, graph relation boost, and RRF fusion.

**Acceptance**:
- Every result includes source path, document id, version, chunk id, chunk hash, score components, and citation text.
- Results can be filtered by document type, chapter, status, effective date, and relation type.
- Fixed audit question fixture produces deterministic results.

## V312-06: SQL-Backed GMP Graph Projection

**Priority**: P0

**Goal**: Provide graph navigation for GMP evidence without claiming a general graph database.

**Acceptance**:
- Nodes and edges are stored in SQLRustGo tables.
- Supports neighbors and depth-limited paths up to depth 3.
- Supports relation filters.
- Does not require archived `graph` crate as unvalidated production dependency.

## V312-07: Compliance and Data Integrity Controls

**Priority**: P0

**Goal**: Implement ALCOA+ and GMP-relevant controls for internal-audit retrieval.

**Acceptance**:
- Audit hash-chain tamper tests fail closed.
- Role-based access tests cover import, approve, search, export, and audit review.
- Electronic-signature hooks exist for approval and controlled export.
- Compliance matrix maps tests to controls.

## V312-08: Backup and Restore

**Priority**: P0

**Goal**: Verify backup/restore of SQL, GMP documents, embeddings, graph projection, and audit chain.

**Acceptance**:
- Restored counts match source counts.
- Restored hashes match source hashes.
- Vector index can rebuild after restore.
- Audit chain verifies after restore.

## V312-09: Mixed Workload SOAK

**Priority**: P0

**Goal**: Run a 168h workload representative of `~/gmp-platform`.

**Workload**:
- SQL reads/writes.
- GMP corpus import and incremental update.
- Hybrid retrieval.
- Graph traversal.
- Audit export.
- Backup/restore smoke.

**Acceptance**:
- 168h complete.
- 0 crash.
- No audit-chain break.
- No unclassified data loss.

## V312-10: Documentation and Operations

**Priority**: P1

**Goal**: Provide production operation docs for GMP internal-audit retrieval.

**Acceptance**:
- User operations guide exists.
- Compliance matrix signed off.
- Migration guide from SQLite/Chroma/PostgreSQL prototypes exists.
- GA gate report contains only evidence-backed PASS claims.

## V312-11: SQLite SQLLogicTest Oracle Gate

**Priority**: P0

**Goal**: Finish the SQLite automatic testing framework planned in v3.10.0 and left non-blocking/TBD in v3.11.0.

**Background**:
- `docs/releases/v3.10.0/TESTING_SYSTEM_BETA_REPORT.md` proposed SQLLogicTest against SQLite's official corpus as the P0 oracle path.
- `docs/releases/v3.10.0/plans/V310_ISSUES_PLAN.md` V310-14 recorded a runner implementation but a blocked official-suite download.
- `crates/sqlrustgo_sqllogictest` exists and currently has 22 local `.test` files.
- `docs/releases/v3.11.0/RELEASE_GATE_CHECKLIST.md` still had `sqllogictest runner all targets` as TBD.

**Scope**:
- Normalize the canonical runner path to `crates/sqlrustgo_sqllogictest`.
- Build and run the existing smoke corpus.
- Create a reproducible SQLite official SQLLogicTest corpus acquisition/cache plan.
- Add a manifest with upstream snapshot, file count, hash list, skipped files, and exclusion reasons.
- Add or plan `scripts/gate/check_sqllogictest_v312.sh`.
- Save outputs under `docs/releases/v3.12.0/sqllogictest-baseline/` and `docs/releases/v3.12.0/logs/`.

**Acceptance**:
- `cargo build -p sqlrustgo_sqllogictest` succeeds.
- `cargo run -p sqlrustgo_sqllogictest -- --test-dir crates/sqlrustgo_sqllogictest/testdata` produces a smoke report.
- The selected SLT corpus has PASS/FAIL/SKIP classification.
- Every skip/fail group has an issue, owner, expiry, and rationale.
- v3.12.0 GA cannot pass while this item is still TBD.

## V312-12: TPC-H SF=1 Correctness Close-Out

**Priority**: P0

**Goal**: Turn v3.11.0's TPC-H SF=1 22/22可运行性证据 into correctness evidence.

**Scope**:
- Re-run all 22 SF=1 queries from the same fixture.
- Capture SQLRustGo row counts and sorted result SHA256.
- Capture SQLite/PostgreSQL/MySQL or MariaDB reference row counts and SHA256 where supported.
- Explain every zero-row query with data, not prose-only reasoning.
- Store per-query artifacts and summary report.

**Acceptance**:
- 22/22 query artifacts exist.
- No unexplained checksum mismatch.
- No unexplained zero-row result.
- `TPCH_SF1_VERIFICATION_REPORT.md` no longer depends on a future PG SHA256 task for its core correctness claim.

## V312-13: MySQL Wire Protocol and LOAD DATA Hardening

**Priority**: P0

**Goal**: Close the v3.11.0 production-readiness gap around MySQL compatibility and data import.

**Scope**:
- Add wire e2e coverage for COM_QUERY, COM_STMT_PREPARE, COM_STMT_EXECUTE, COM_STMT_CLOSE, error packets, reset connection, TLS, and compression boundaries.
- Add `LOAD DATA LOCAL INFILE` or clearly-scoped alternative bulk import tests.
- Measure SF=1 and, where feasible, SF=10 import time, memory peak, row counts, and hashes.
- Fail closed on silent truncation, type conversion drift, or out-of-memory behavior.

**Acceptance**:
- Wire protocol e2e report exists.
- LOAD DATA/bulk-import report exists with count/hash verification.
- mysql-server and mysql-client coverage trend improves or has explicit non-blocking rationale.

## V312-14: Crash Recovery and Upgrade/Downgrade Verification

**Priority**: P0

**Goal**: Add the recovery evidence needed before SQLRustGo can be promoted beyond controlled production.

**Scope**:
- Run kill -9 / restart tests against mixed read/write workloads.
- Verify WAL replay, dirty page recovery, and audit-chain continuity.
- Back up and restore SQL data, GMP documents, embeddings, graph projection, and audit chain.
- Run v3.10.0/v3.11.0 fixture upgrade to v3.12.0 and rollback/downgrade where supported.
- Store count/hash reports for every recovery and upgrade path.

**Acceptance**:
- Recovery report shows count/hash equality after restart.
- Restore report shows count/hash equality in a clean data directory.
- Upgrade report shows old data readable under v3.12.0.
- Rollback limitations are explicitly documented if full downgrade is not supported.
