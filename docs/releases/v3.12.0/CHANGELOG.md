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
| v3.12.0 | TBD | DRAFT | GMP 内审检索数据库版本，附带 v3.11.0 弱项补强 |

## v3.12.0 启动切片 (2026-08-09)

agent: minimax 在本地 fresh checkout (`develop/v3.12.0` @ ed89db05ab, base `9e157ed61b`) 上完成 Track C (MySQL 兼容) 的启动切片；openspec 4/4 artifacts 全部 valid；evidence 已落地。

### 仓库恢复

- 本地仓库从损坏状态恢复：`develop/v3.12.0` 与 `origin/develop/v3.12.0` 同步；4591 个 0-byte 文件清理；pack (164k objects) 完整保留。
- 本次启动前的清理目标是：删除历史本地 dev 分支 (损坏后已不存在)，拉取新基线 (完成)。
- 详见 commit log: 4 commits, latest `<HEAD sha>`.

### 认领与计划 (Gitea 252)

| Issue | Track | 标题 | 状态 | 分支 |
|-------|-------|------|------|------|
| [#3900](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3900) | C | [V312-13] MySQL Wire + LOAD DATA Hardening | OPEN, assigned openclaw | `develop/v3.12.0` |
| [#3906](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3906) | C/B | [V312-19] SQL Corpus、Architecture Invariant 与 Reviewer Sign-off Gate | OPEN, assigned openclaw | `develop/v3.12.0` |
| [#3908](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3908) | C | [V312-21] MySQL Compatibility 与 SQL Surface Backlog | OPEN, assigned openclaw | `develop/v3.12.0` |

每个 issue 已留下 claim comment 含 source_run、evidence_hash 口径和 openspec 链接。

### Openspec 变更 (openspec/changes/)

| Change | Issue | Status | Artifacts |
|--------|-------|--------|-----------|
| `v312-13-mysql-wire-load-data-hardening` | #3900 | valid, 4/4 | proposal, design, tasks, 5 specs (binary-prepared-statement-roundtrip MOD, wire-protocol-execution MOD, mysql-wire-stmt-reset-tls-compression, mysql-wire-error-packet-contract, load-data-sf1-sf10-memory-cap) |
| `v312-19-sql-corpus-arch-invariant-reviewer-gate` | #3906 | valid, 4/4 | proposal, design, tasks, 3 specs (sql-corpus-all-targets-report, arch-invariant-r2-unified-report, reviewer-signoff-template) |
| `v312-21-mysql-compat-sql-surface-backlog` | #3908 | valid, 4/4 | proposal, design, tasks, 2 specs (mysql-compat-surface-disposition, mysql-compat-fixture-suite) |

### 已落地 evidence (docs/releases/v3.12.0/evidence/)

| 路径 | 来源 | 内容 |
|------|------|------|
| `wire_load_data/V312-13-REPORT.md` | `check_v312_13_wire_load_data.sh` | 10 步 evidence 表 (5 PASS, 4 deferred, 1 pre-existing fail from `check_load_data_infile.sh`) |
| `arch_invariants/R2_INVARIANTS_REPORT.md` | `check_r2_invariants.sh` | R2.1-R2.4 真实结果 (2 pass / 2 fail), R2.5-R2.8 honest-gap stub |
| `mysql_compat/SURFACE_DISPOSITION.md` | `check_v312_21_mysql_compat.sh` | 10 v3.7-v3.10 历史 surface 的 decision 表 (PASS/unsupported/deferred) |
| `sql_corpus/ALL_TARGETS_REPORT.md` | `corpus_manifest.yaml` + 初始 seed | 6 corpus target 的状态骨架 (runtime runner 待 v312-19 tasks §1.1-1.3 落地) |

### 测试结果

| 测试目标 | 通过 | 失败 |
|----------|------|------|
| `cargo test --test v312_13_typed_wrappers_test` | 22 | 0 |
| `cargo test --test mysql_wire_protocol_test` (regression) | 28 | 0 |
| `cargo test -p sqlrustgo-mysql-server --test e2e_wire_protocol` | 46 | 0 |
| `cargo test -p sqlrustgo-mysql-server --test prepared_stmt_params_test` | 8 | 0 |
| `cargo check --workspace` | OK | 0 (3 pre-existing warnings) |

### Gate scripts

| 脚本 | 用途 | 首次运行 |
|------|------|----------|
| `scripts/gate/check_v312_13_wire_load_data.sh` | V312-13 evidence | 5/5 typed-wrapper+regression PASS, 4 deferred, 1 pre-existing fail |
| `scripts/gate/check_v312_19_release_gates.sh` | RC/GA 阻断 gate | PASS (artifacts fresh) |
| `scripts/gate/check_r2_invariants.sh` | R2.1-R2.8 driver | 2/4 real pass, 2/4 real fail, 4/4 honest-gap stub |
| `scripts/gate/check_v312_21_mysql_compat.sh` | 10-surface disposition | 10/10 rows seeded |
| `scripts/gate/assert_reviewer_signoff.sh` | dual-reviewer signoff | 3/3 unit cases pass (missing/valid/same-reviewer) |

### 未完成 / 后续工作

- V312-13 §9-10: LOAD DATA SF=1 / SF=10 fixture (依赖 server-side batch loader 增强)
- V312-13 §7-8: TLS / compression (依赖 ephemeral harness 加密支持)
- V312-19 §1.1-1.3: corpus_manifest.yaml runtime runner (当前为 SSOT 文件 + 初始 ALL_TARGETS_REPORT)
- V312-21 §2.1-2.2: 真正的 compat fixture runner (当前 disposition 行为为静态 seed)
- 三项 issue 的 PR 提交需 1 名 reviewer approval (per `BRANCH_GOVERNANCE.md` v1.0 §4.1)

## 附录：英文原文

> 本附录保留本文件改写前的英文原文，便于追溯历史语义；当前正式阅读与执行口径以上方中文正文为准。

# Changelog -- SQLRustGo v3.12.0

> **Status**: DRAFT
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
| v3.12.0 | TBD | DRAFT | GMP internal-audit retrieval database |
