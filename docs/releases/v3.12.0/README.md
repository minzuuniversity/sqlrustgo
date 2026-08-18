# SQLRustGo v3.12.0

> **provenance:** generated_by=v3.12.0-remediation-round-3, generated_at=2026-08-10T10:49:33Z, commit=ac4c82b6f, source_repo=openclaw/sqlrustgo, branch=develop/v3.12.0, policy=Anti-Fabrication-Policy-v1.0

> **状态**: 规划中
> **产品目标**: 面向 `~/gmp-platform` 的 GMP 内审检索数据库
> **规划日期**: 2026-08-09
> **当前整改口径更新**: 2026-08-18

v3.12.0 被规划为 SQLRustGo 第一个明确面向 GMP 内审检索工作负载的版本。它使用 SQLRustGo 作为受监管文档存储、chunk、embedding、audit trail、evidence relation、hybrid retrieval 和 SQL-backed graph projection 的数据库基础。

本版本不得描述为通用向量数据库或通用图数据库；这些是 v4.0.0 的目标。

2026-08-09 规划更新：v3.12.0 同时承担 v3.11.0 GA 弱项补强职责。进入 GA 前，必须关闭或显式重门禁 TPC-H correctness、coverage methodology、MySQL wire protocol、LOAD DATA/bulk import、crash recovery、backup/restore、upgrade/downgrade、dependency audit refresh，以及 v3.10.0 已规划但 v3.11.0 没有成为阻断 gate 的 SQLite SQLLogicTest oracle gate。

2026-08-14 整改更新：README 与历史开发计划中的 `PARTIAL` 不能作为 v3.12 初始生产能力声明。属于 v3.12 生产边界的 `PARTIAL` 必须绑定到 [PARTIAL 功能整改 Issue 计划](PARTIAL_FEATURE_REMEDIATION_ISSUE_PLAN.md)，在 GA 前关闭为 `DONE / 受控`，或降级为 `DEFERRED` / `UNSUPPORTED` 并说明不属于 v3.12 初始生产边界。

2026-08-18 阶段治理纠偏：v3.12.0 仍处于 `ALPHA`，不得通过把 V312 open issue 批量改成 v3.13 follow-up 来绕过 Beta/RC/GA。详见 [v3.12.0 阶段治理纠偏报告](STAGE_GOVERNANCE_REMEDIATION_2026-08-18.md)。`develop/v3.13.0` 只能作为冻结 follow-up 分支，不能作为关闭 V312 issue 的默认证据来源。

## 发布契约

允许的 v3.12.0 声明：

> SQLRustGo v3.12.0 支持受控 GMP 内审检索工作负载，使用 SQLRustGo 管理关系存储、内部向量检索、SQL-backed graph projection 和可审计 evidence bundle。

禁止的声明：

- 通用独立向量数据库。
- 通用图数据库。
- 未处理 v3.11.0 弱项就宣称生产发布。
- 没有 168h mixed SOAK 证据就宣称生产发布。
- 没有 SQLLogicTest、TPC-H correctness、wire protocol、LOAD DATA、recovery 和 upgrade 证据就宣称广义 MySQL 5.7 替代。

## 关键文档

| 文档 | 用途 |
|---|---|
| `STAGE.yaml` | 阶段 SSOT 和 promotion criteria |
| `STAGE_GOVERNANCE_REMEDIATION_2026-08-18.md` | v3.12/v3.13 阶段漂移纠偏、Issue 分级和 Beta/RC 边界 |
| `COMPREHENSIVE_ASSESSMENT_REPORT.md` | v3.12.0 综合评估：MySQL 水平、功能/性能/稳定性、GMP 生产可行性、教学场景增强项 |
| `DEVELOPMENT_PLAN.md` | 实施计划和 GMP-Platform 集成工作包 |
| `VERSION_PLAN.md` | 产品范围和工作包 |
| `TEST_PLAN.md` | gate 和测试矩阵 |
| `ISSUES_PLAN.md` | V312-01 到 V312-54 的任务拆分和 follow-up |
| `PARTIAL_FEATURE_REMEDIATION_ISSUE_PLAN.md` | README 中 PARTIAL/OPEN 功能的整改归属、issue 和关闭边界 |
| `GMP_COMPLIANCE_MATRIX.md` | GMP/ALCOA+ 合规控制映射 |
| `fixtures/gmp_audit_questions.yml` | 检索质量 fixture seed |

## 对 v3.11.0 的依赖

v3.12.0 从 v3.11.0 GA 评估状态出发。进入 v3.12.0 GA 前，必须关闭或显式承接以下 v3.11.0 弱项：G3 覆盖率口径、G4 TPC-H SF=1 correctness、168h SOAK 适用范围、sqllogictest gate drift、MySQL wire protocol 缺口、LOAD DATA/bulk import、crash recovery、backup/restore、upgrade/downgrade、dependency audit refresh 和 debt-registry drift。

## 新增硬化门禁

| Gate | 目的 |
|---|---|
| SQLite SQLLogicTest | 把已有 `crates/sqlrustgo_sqllogictest` runner 和 SQLite corpus 计划变成真实 gate |
| TPC-H correctness | 用 row-count 和 SHA256 关闭 zero-row / checksum 风险 |
| MySQL wire protocol | 覆盖 COM_QUERY、COM_STMT、error packet、reset、TLS/compression |
| LOAD DATA / bulk import | 验证导入行数、hash、内存和耗时 |
| Crash recovery / upgrade | 验证 kill -9、WAL replay、backup/restore、v3.10/v3.11 到 v3.12 升级和回滚 |

## PARTIAL 功能治理

v3.12.0 的目标是“功能比较完备的初始生产版本”，不是把大量半成品功能堆进 release note。因此：

- `PARTIAL` 只能是 Alpha/Beta 过渡状态，不能进入 GA 产品声明。
- 属于 GMP 内审检索、MySQL-style 基础兼容、TPC-H/SQL correctness、recovery、coverage、bulk-load、Sysbench 的 `PARTIAL` 是 GA blocker。
- 非 v3.12 初始生产边界的能力，例如通用向量数据库、通用图数据库、存储过程、复制/分布式、未限定的 Window/GIS/JSON，必须改为 `DEFERRED` 或 `UNSUPPORTED`。
- Window/GIS/JSON 三类 SQL 功能的 v3.12 边界已写入 [`sql-feature-corpus/window_json_gis_scope.md`](./sql-feature-corpus/window_json_gis_scope.md)：Window → `DEFERRED-3.13`，JSON → `DONE-subset`（`JSON_EXTRACT/JSON_VALUE/JSON_UNQUOTE` + `->`/`->>` 操作符），GIS → `DONE-subset`（`ST_DISTANCE/ST_WITHIN/ST_CONTAINS/ST_INTERSECTS`，仅 2D Point/WKT-bbox）。
- #4220 是 PARTIAL 功能整改总控；#4221-#4227 是本轮新增的缺口 issue；已有 #3943、#4020、#4210、#4211、#4217、#4218、#4219 继续作为对应整改证据入口。

## 附录：英文原文

> 本附录保留本文件改写前的英文原文，便于追溯历史语义；当前正式阅读与执行口径以上方中文正文为准。

# SQLRustGo v3.12.0

> **Status**: DRAFT
> **Product target**: GMP internal-audit retrieval database for `~/gmp-platform`
> **Planning date**: 2026-08-09

v3.12.0 is planned as the first SQLRustGo release focused on GMP internal-audit retrieval workloads. It uses SQLRustGo as the database foundation for regulated document storage, chunking, embeddings, audit trails, evidence relations, hybrid retrieval, and SQL-backed graph projection.

This release must not be described as a general-purpose vector database or graph database. Those are v4.0.0 goals.

2026-08-09 planning update: v3.12.0 also becomes the hardening release for v3.11.0 GA weak points. It must close or explicitly re-gate TPC-H correctness, coverage methodology, MySQL wire protocol, LOAD DATA/bulk import, crash recovery, backup/restore, upgrade/downgrade, dependency audit refresh, and the SQLite SQLLogicTest oracle gate that was planned in v3.10.0 but remained non-blocking in v3.11.0.

## Release Contract

Allowed v3.12.0 claim:

> SQLRustGo v3.12.0 supports controlled GMP internal-audit retrieval workloads with SQLRustGo-managed relational storage, internal vector retrieval, SQL-backed graph projection, and auditable evidence bundles.

Disallowed claims:

- General-purpose standalone vector database.
- General-purpose graph database.
- Production release without v3.11.0 weak-point disposition.
- Production release without 168h mixed SOAK evidence.
- Broad MySQL 5.7 replacement without SQLLogicTest, TPC-H correctness, wire protocol, LOAD DATA, recovery, and upgrade evidence.

## Key Documents

| Document | Purpose |
|---|---|
| `STAGE.yaml` | Stage SSOT and promotion criteria |
| `DEVELOPMENT_PLAN.md` | Implementation plan and GMP-Platform integration work packages |
| `VERSION_PLAN.md` | Product scope and work packages |
| `TEST_PLAN.md` | Gate and test matrix |
| `ISSUES_PLAN.md` | V312-01 through V312-14 task breakdown |
| `GMP_COMPLIANCE_MATRIX.md` | GMP/ALCOA+ compliance control mapping |
| `fixtures/gmp_audit_questions.yml` | Retrieval quality fixture seed |

## Dependency

v3.12.0 is planned from the v3.11.0 GA assessment state. Before v3.12.0 GA, the project must close or explicitly carry v3.11.0 weak points: G3 coverage口径, G4 TPC-H SF=1 correctness, 168h SOAK scope, sqllogictest gate drift, MySQL wire protocol gaps, LOAD DATA/bulk import, crash recovery, backup/restore, upgrade/downgrade, dependency audit refresh, and debt-registry drift.

## Added Hardening Gates

| Gate | Purpose |
|---|---|
| SQLite SQLLogicTest | Turn the existing `crates/sqlrustgo_sqllogictest` runner and SQLite corpus plan into a real gate |
| TPC-H correctness | Add row-count and SHA256 cross-engine checks beyond 22/22可运行性 |
| MySQL wire + LOAD DATA | Harden protocol and data import paths needed by MySQL-style workloads |
| Recovery + upgrade | Prove WAL replay, backup/restore, and v3.10/v3.11 to v3.12 upgrade behavior |
