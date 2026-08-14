# SQLRustGo v3.12.0 开发计划

> **provenance:** generated_by=v3.12.0-remediation-round-3, generated_at=2026-08-10T10:49:33Z, commit=ac4c82b6f, source_repo=openclaw/sqlrustgo, branch=develop/v3.12.0, policy=Anti-Fabrication-Policy-v1.0

> **版本**: v3.12.0
> **状态**: 规划中
> **日期**: 2026-08-09
> **产品目标**: 面向 `~/gmp-platform` 的 GMP 内审检索数据库
> **真实性规则**: 本计划只定义未来工作和退出证据，不声明任何 v3.12.0 gate 已通过。

## 1. 规划证据

| 来源 | 证据 | 用途 |
|---|---|---|
| SQLRustGo 本地 checkout | `develop/v3.12.0` at `9e157ed61b68a2a7c61ebae18cd0a6c15ceb79b1`，基于 `develop/v3.11.0` at `6ab0da723e871d28b80dbd15ecf31852cd07c779` | 252/250 Gitea 同步后的 v3.12 Draft 规划基线 |
| GMP-Platform 250 checkout | `develop/v1.4.0` at `c0f366d59e7d` | 集成分析基线 |
| GMP-Platform 集成计划 | commit `556a5f104900` | 跨项目实施方向 |
| SQLRustGo roadmap plan | `docs/plans/2026-08-08-sqlrustgo-v312-v400-gmp-rag-graph-plan.md` | v3.12/v4.0 范围拆分 |
| v3.11.0 综合评估 | `docs/releases/v3.11.0/COMPREHENSIVE_ASSESSMENT_REPORT.md` | GA 证据边界、弱项补强、遗漏测试清单 |
| v3.10.0 测试体系报告 | `docs/releases/v3.10.0/TESTING_SYSTEM_BETA_REPORT.md` | SQLLogicTest / SQLite 官方语料计划，Issue #3373 |
| v3.10.0 issues plan | `docs/releases/v3.10.0/plans/V310_ISSUES_PLAN.md` | V310-14 SQLLogicTest 集成状态和下载缺口 |
| 现有 SLT runner | `crates/sqlrustgo_sqllogictest` | runner 存在；当前 v3.12 smoke gate 覆盖 25 个非 `_unsupported` 本地 `.test` 文件，历史 22 文件口径已过时 |

本文档 claim metadata：

| 字段 | 值 |
|---|---|
| source_agent | Codex |
| source_run | `codex-sqlrustgo-v312-plan-2026-08-09` |
| timestamp | `2026-08-09 13:35:00 CST` |
| evidence_hash | `local-git:9e157ed61b68a2a7c61ebae18cd0a6c15ceb79b1`；实际 gate 必须生成自己的 evidence hash |
| conflict_resolution | SQLRustGo 本地文档是 release SSOT；GMP-Platform 发现作为集成输入 |

## 2. 驱动 v3.12.0 的当前发现

v3.12.0 的必要性来自两方面：SQLRustGo v3.11.0 已具备有价值的数据库基础，但 GMP-Platform 的生产路径还需要更收敛、更可审计的数据库契约，才能安全地把 SQLRustGo 作为主数据库、向量存储和图谱投影存储。

| 发现 | 对 v3.12.0 的影响 |
|---|---|
| GMP-Platform 已依赖 `sqlrustgo-storage`、`sqlrustgo-types`、`sqlrustgo-vector`、`sqlrustgo-graph` | v3.12 必须稳定 GMP-facing kernel contract，避免 ad hoc crate usage |
| `gmp-eval` 仍有独立 hybrid search path | v3.12 必须提供 CLI/API/eval 共用的可审计 query contract |
| LIKE fallback 仍参与评估行为，且禁用不可靠 | v3.12 必须把 LIKE 限为 diagnostic-only，并增加 no-LIKE production gate |
| Vector retrieval 有 empty-index 和 dimension-drift 风险 | v3.12 必须强制 model name、dimension、vector hash、rebuildable index invariants |
| Graph capability 未进入主评估/检索路径 | v3.12 应交付 SQL-backed GMP evidence graph projection，而不是通用图数据库声明 |
| RAG 输出需要更强 citation 和 audit evidence | v3.12 必须让每个检索结果和生成回答都带 evidence bundle |
| v3.11.0 GA 评估发现 G3/G4 证据边界风险 | v3.12 必须关闭 TPC-H correctness、coverage methodology、wire protocol、LOAD DATA、recovery、upgrade/downgrade 缺口 |
| v3.10.0 已规划 SQLite SQLLogicTest，v3.11.0 仍未成为阻断 gate | v3.12 必须把 SQLLogicTest/SQLite oracle testing 提升为 P0 gate |
| gate 检查路径在 `crates/sqllogictest` 与 `crates/sqlrustgo_sqllogictest` 之间漂移 | v3.12 必须统一 runner path 和 package name |
| v3.11.0 综合评估列出 `CREATE SEQUENCE` executor gap | v3.12 必须完成 sequence execution、并覆盖并发、事务、WAL/recovery 场景 |
| v3.11.0 综合评估把 Window/GIS/JSON 列为 v3.12 后续增强 | v3.12 必须按受控 SQL 功能交付，避免把 planned feature 写成已完成能力 |
| v3.11.0 禁用测试分析发现 API 漂移和无 test target 测试 | v3.12 必须建立 disabled/API-drift test debt close-out，恢复、重写或有证据地退休测试 |
| v3.11.0 release checklist 中 SQL corpus、架构 invariant、reviewer sign-off 仍有 TBD | v3.12 RC/GA 必须把这些内容变成可执行 gate 和签核证据 |
| v3.11.0 后续计划记录 TPC-H SF=10、Sysbench、Prometheus、Slow Query Log 仍未落地 | v3.12 必须补齐 performance/observability baseline，不把 SF=1 单点结果外推到生产 |
| v3.6.0 Beta checklist 与 legacy analysis 记录 build/test/clippy/coverage/TPC-H/security/SQL corpus 多项 PENDING | v3.12 必须建立跨版本遗留项总账，逐项标记 closed、superseded、carried 或 deferred |
| v3.6.0/v3.7.0 记录 DML 双执行路径、AST routing、TransactionManager、SHOW、auth、prepared statement 测试缺口 | v3.12 必须复核这些兼容性/架构债务是否已由后续版本真实关闭，未关闭项进入兼容性回归 |
| v3.8.0 frozen list 记录 Parallel/SIMD、sysbench、索引统计、Vector SQL、Recursive CTE、高级函数、CBO、advanced aggregates、wal-verification 等冻结项 | v3.12 必须给出执行范围：GMP 路径相关项进入实现或测试，非 GMP 路径进入有 owner/expiry 的延期清单 |
| v3.9.0 文档记录真实 24h/72h/168h SOAK、ignored long tests、tx_wal autocommit 语义仍需真实硬件/语义复核 | v3.12 mixed SOAK 与 test-debt gate 必须覆盖真实 wall-clock、ignored category 和 tx_wal 契约复核 |
| v3.10.0 RC/测试体系文档记录 SQLancer/test-runner/test-registry 骨架、E2E scripts TBD、anti-fabrication known broken binaries、coverage baseline 缺口 | v3.12 必须把测试基础设施从骨架提升为可运行 gate，或把未使用组件移入有证据的 retired/deferred 状态 |

## 3. v3.12.0 契约

在 gate 产生执行证据后，允许的产品声明为：

> SQLRustGo v3.12.0 支持受控 GMP 内审检索工作负载，使用 SQLRustGo 管理关系存储、内部向量检索、SQL-backed graph projection 和可审计 evidence bundle。

禁止的 v3.12.0 声明：

- 通用 MySQL 5.7 替代品。
- 通用独立向量数据库。
- 通用图数据库。
- 未处理 v3.11.0 弱项就宣称生产可用。
- 没有 command output、timestamp、source agent、source run、evidence hash 和 output location 就宣称 PASS、GA 或 compliance。

## 4. 架构范围

v3.12.0 应暴露一个 GMP kernel，让 GMP-Platform 无需复制检索逻辑即可调用。

```text
GMP-Platform CLI/API/Eval
        |
        v
SQLRustGo GMP Kernel Contract
        |
        +-- relational document/chunk/version/audit tables
        +-- keyword retrieval with exact match and tokenizer controls
        +-- vector persistence and rebuildable vector index
        +-- SQL-backed relation graph projection
        +-- RRF fusion and evidence bundle generation
        +-- backup/restore and audit-chain verification
```

SQLRustGo 负责 storage correctness、indexing contracts、query evidence 和 gate scripts。GMP-Platform 负责业务流程、UI/API orchestration、文档审查过程和领域 prompt。

## 4.1 v3.11.0 弱项补强范围

v3.12.0 有两条同等 P0 主线：

| 主线 | 目的 | 退出边界 |
|---|---|---|
| Track A：v3.11.0 生产硬化 | 关闭或显式承接 TPC-H correctness、coverage drift、wire protocol、LOAD DATA、crash recovery、backup/restore、upgrade/downgrade、dependency audit | 不再隐藏任何 v3.11.0 P0 blocker |
| Track B：GMP/RAG/Graph kernel | 为 `~/gmp-platform` 交付受控 GMP 内审检索数据库契约 | GMP-Platform 可以用 SQLRustGo-backed retrieval 产出 evidence bundle |

两条主线不能互相替代。GMP 功能不能成为跳过 SQL correctness gate 的理由；SQL correctness gate 也不能被 GMP-only fixtures 替代。

## 4.2 SQLite SQLLogicTest 恢复集成

基于 SQLite 的自动测试计划重新纳入 v3.12.0 P0 质量门禁。

| 项 | 当前证据 | v3.12.0 动作 |
|---|---|---|
| 历史计划 | `docs/releases/v3.10.0/TESTING_SYSTEM_BETA_REPORT.md` 描述使用 SQLite 官方 `.test` 文件，约 623 文件 / 590 万 case | 作为 V312-11 继承 |
| Issue 计划 | `docs/releases/v3.10.0/plans/V310_ISSUES_PLAN.md` V310-14 记录 runner 已实现，但 SQLite 官方 suite 下载受阻 | 完成 testdata 获取，或定义可复现 mirror/cache |
| 现有 runner | `crates/sqlrustgo_sqllogictest` 存在；当前 smoke gate 覆盖 25 个非 `_unsupported` 本地 `.test` 文件 | 纳入 package build/run gate |
| v3.11.0 缺口 | `docs/releases/v3.11.0/RELEASE_GATE_CHECKLIST.md` 中 sqllogictest runner all targets 仍是 TBD | 从 TBD 提升为 v3.12 Alpha/Beta/RC/GA 阶段阈值 |
| 路径漂移 | `scripts/gate/check_beta_gate.sh` 同时检查 `crates/sqllogictest` 和 `crates/sqlrustgo_sqllogictest` | 统一到 `crates/sqlrustgo_sqllogictest` 或记录 rename |

2026-08-09 当前本地基线：

| 检查 | 结果 | v3.12.0 后续要求 |
|---|---|---|
| `cargo build -p sqlrustgo_sqllogictest` | 已由 `scripts/gate/check_sqllogictest_v312.sh` 实跑 PASS | 可作为 smoke gate build evidence；warning-free/clippy-clean 仍以 `cargo clippy --all-features -- -D warnings` 为准 |
| `cargo run -p sqlrustgo_sqllogictest -- --test-dir crates/sqlrustgo_sqllogictest/testdata` | smoke gate 当前 25/25 文件通过，通过率 100.0%；历史 6/16、27.3% 基线已 superseded | 可作为 Beta smoke evidence；不得外推为完整 SQLite official corpus PASS |
| SQLite 官方 SQLLogicTest corpus | full official corpus 尚未集成到当前 gate system；当前是本地 smoke corpus | RC/GA 增加 curated/official corpus manifest、cache/mirror procedure、selected-target definition 和 exclusion registry |

## 4.3 v3.11.0 未完成项承接清单

以下内容来自 v3.11.0 综合评估、发布门禁清单、禁用测试分析和 post-GA 计划。它们在 v3.12.0 中按工作包承接；除非产生执行证据，否则不得写成已完成。

| 类别 | v3.11.0 遗留内容 | v3.12.0 承接方式 |
|---|---|---|
| SQL correctness | TPC-H SF=1 仍需跨引擎 row-count/SHA256；zero-row query 需要逐项解释 | V312-12，作为 P0 correctness close-out |
| Wire/import | MySQL wire protocol 严格路径和 `LOAD DATA`/bulk import 未形成完整 artifact | V312-13，补 COM_QUERY/COM_STMT/error/reset/TLS/compression 与 SF=1/SF=10 导入证据 |
| Recovery/migration | crash recovery、backup/restore、upgrade/downgrade 证据不足 | V312-14，补 kill -9、WAL replay、restore checksum、v3.10/v3.11 upgrade/rollback fixture |
| SQL feature | `CREATE SEQUENCE` executor 未完成；Window/GIS/JSON 仍是计划能力 | V312-15 和 V312-16，按功能测试与 SQL corpus 逐项关闭 |
| 测试债务 | disabled/API-drift tests、无 test target 的历史测试、`test_wal_perf_throughput` flaky | V312-17，建立 test debt manifest 并修复或有证据地退休 |
| 覆盖率 | parser、mysql-server、mysql-client 低于 80%；coverage 口径存在漂移 | V312-17，统一 canonical coverage command，并按 crate 输出报告 |
| 性能/观测性 | TPC-H SF=10、Sysbench OLTP、Prometheus metrics、Slow Query Log、导入性能基线未完成 | V312-18，形成可复跑 benchmark 和观测性 gate |
| SQL corpus/governance | `test_sql_corpus.sh` all targets、架构 invariant 脚本、双 reviewer 签核仍有 TBD | V312-19，纳入 RC/GA 发布签核 gate |
| GMP/RAG/Vector/Graph | GMP fixture、ALCOA+ evidence bundle、vector recall、graph projection consistency、permission matrix 不足 | V312-02 至 V312-08，作为 GMP 内审检索主线交付 |

## 4.4 v3.6.0-v3.10.0 遗留规划承接清单

以下清单只表示“历史文档曾规划或标记未完成”，不表示当前代码仍然失败。v3.12.0 的职责是重新验证、关闭或显式延期，避免旧版本债务在文档中失踪。

| 来源版本 | 历史未闭环项 | v3.12.0 处置 |
|---|---|---|
| v3.6.0 | Beta B1-B8 曾全部 PENDING；Parser/Executor/mysql-server tests2 覆盖率和编译错误延续；门禁条件通过语义不清 | V312-17/V312-20：覆盖率、测试编译、gate 证据口径和跨版本债务总账复核 |
| v3.6.0 | `ExecutionEngine` 与 `LocalExecutor` 双轨，DML 直接走 storage，未统一 PhysicalPlan 路径 | V312-22：执行路径 invariant 与 DML/VTU/Parallel 路由复核 |
| v3.7.0 | `SHOW TABLES`、BEGIN/COMMIT/ROLLBACK routing、ROLLBACK 语义、empty-password auth、prepared statements 测试不足、列元数据泛化 | V312-21：MySQL 兼容性回归；已关闭项需实跑证明，未关闭项进入 issue |
| v3.8.0 | Frozen: SIMD/Parallel 主路径、sysbench、索引统计、Vector SQL、Recursive CTE、ROLLUP/CUBE/REPLACE/RANK、CBO/histogram、STDDEV/VARIANCE/MEDIAN/GROUP_CONCAT、wal-verification、mysqladmin、Cypher | V312-20/V312-21/V312-22/V312-23：按 GMP 相关性分类为 P0/P1/P2/deferred |
| v3.8.0 | Open issues: WAL checkpoint、复合索引、索引统计、TIMESTAMP+连接池、ALTER TABLE、stored procedure tokens | V312-21/V312-23：兼容性和存储/WAL backlog 复核 |
| v3.9.0 | 真实 24h/72h/168h wall-clock SOAK 曾记录 incomplete/blocked；ignored long tests 与 tx_wal autocommit 契约需专用环境 | V312-10/V312-17/V312-18：真实 SOAK、ignored test registry、tx_wal semantic reconciliation |
| v3.10.0 | SQLancer、test-runner、test-registry 骨架未充分使用；SQLLogicTest/SQLite oracle 计划未成为阻断 gate | V312-11/V312-24：oracle/differential testing 与测试编排基础设施 gate |
| v3.10.0 | Coverage baseline、anti-fabrication known broken binaries、E2E shell scripts、gate self verification/sign-off 曾有 TBD/WARN/FAIL | V312-17/V312-19/V312-24：恢复 test binary compile、E2E scripts、meta-gate 和签核 |
| v3.10.0 | SEM-3/SEM-4、F-03 GIS、F-30 SEQUENCE、F-36 列级权限、isolated extension crates、Q4 Hash Semi Join/Anti Join、subquery decorrelation | V312-15/V312-16/V312-21/V312-22：纳入 sequence、GIS/JSON/window、权限和优化器 backlog |

## 5. 工作包

| ID | 工作包 | 优先级 | 退出证据 |
|---|---|---|---|
| V312-01 | prior-version blocker disposition | P0 | blocker report、STAGE.yaml 更新、SQLLogicTest disposition table |
| V312-02 | GMP kernel contract | P0 | serialization/error/evidence field contract tests；GMP adapter test |
| V312-03 | GMP schema and corpus ingestion | P0 | full corpus ingestion report；re-ingestion idempotency；versioning test |
| V312-04 | keyword retrieval without production LIKE fallback | P0 | deterministic top-k fixture；no-LIKE gate；query trace |
| V312-05 | vector persistence and rebuildable index | P0 | dimension mismatch fail-closed；index rebuild；vector active RRF channel |
| V312-06 | SQL-backed GMP evidence graph | P0 | node/edge count；depth-limited path；graph boost trace |
| V312-07 | hybrid retrieval and RAG evidence bundle | P0 | audit question fixture；每个结果都有 citation bundle；missing-citation fail-closed |
| V312-08 | compliance、audit trail、access control | P0 | audit hash chain、ACL、tamper tests |
| V312-09 | backup/restore and upgrade path | P0 | restore equality、upgrade/downgrade report |
| V312-10 | mixed workload SOAK | P0 | 168h SQL + ingestion + retrieval + audit report |
| V312-11 | SQLite SQLLogicTest oracle gate | P0 | build/run output、corpus manifest、baseline report、gate integration |
| V312-12 | TPC-H SF=1 correctness close-out | P0 | row-count/SHA256 跨引擎报告 |
| V312-13 | MySQL wire + LOAD DATA hardening | P0 | wire e2e、LOAD DATA row/hash/memory 证据 |
| V312-14 | crash recovery and upgrade/downgrade verification | P0 | WAL replay、backup/restore、upgrade、rollback evidence |
| V312-15 | CREATE SEQUENCE executor close-out | P0 | sequence DDL/DML、并发、事务、WAL/recovery 测试报告 |
| V312-16 | Window/GIS/JSON controlled SQL feature delivery | P1 | ROW_NUMBER/RANK/DENSE_RANK、JSON path、GIS distance/intersects/GeoJSON fixtures |
| V312-17 | coverage and disabled-test debt close-out | P0 | parser/mysql-server/mysql-client ≥80% 或带 issue 的未达标说明；disabled/API-drift manifest |
| V312-18 | SF=10、Sysbench 与观测性 baseline | P1 | TPC-H SF=10、Sysbench OLTP、Prometheus metrics、Slow Query Log、bulk-load benchmark |
| V312-19 | SQL corpus、架构 invariant 与 reviewer sign-off gate | P0 | SQL corpus all-target report、R2.1-R2.8 invariant output、2 名 reviewer 签核 |
| V312-20 | v3.6-v3.10 cross-version backlog disposition | P0 | historical-backlog-disposition.yml，所有旧任务 marked closed/superseded/carried/deferred |
| V312-21 | MySQL compatibility and SQL-surface backlog | P1 | SHOW/auth/prepared/ALTER/TIMESTAMP/connection-pool/functions/permissions 回归报告 |
| V312-22 | execution architecture and optimizer debt close-out | P1 | DML PhysicalPlan invariant、Parallel/SIMD route evidence、Hash Semi/Anti Join 或 decorrelation disposition |
| V312-23 | storage/index/WAL tooling backlog | P1 | WAL checkpoint、wal-verification、composite index、index statistics、checksum/torn-write disposition |
| V312-24 | test infrastructure activation | P0 | SQLancer/test-runner/test-registry/E2E scripts 可运行或 retired/deferred evidence |

## 6. 发布里程碑

| 阶段 | 目标 | 必需证据 |
|---|---|---|
| Alpha | schema、代表性 ingestion、sqllogictest runner build | build/fmt/clippy、schema tests、SLT runner smoke |
| Beta | keyword/vector/graph/hybrid retrieval、sqllogictest smoke corpus | deterministic retrieval、evidence bundle、SLT report |
| RC | compliance、backup/restore、TPC-H correctness、wire/LOAD DATA/recovery、coverage/test-debt close-out、历史债务总账 | audit/ACL、cross-engine、wire、LOAD DATA、recovery logs、disabled-test manifest、SQL corpus report、v3.6-v3.10 disposition |
| GA | GMP internal-audit production | 168h mixed SOAK、GA gate report、无未解决 v3.6-v3.11 carried P0、2 名 reviewer 签核 |

## 7. 必跑命令

```bash
bash scripts/gate/check_docs_links.sh
bash scripts/gate/check_docs_consistency.sh
cargo fmt --check --all
cargo clippy --all-features -- -D warnings
cargo test --all-features
bash scripts/gate/check_sqllogictest_v312.sh
cargo build -p sqlrustgo_sqllogictest
cargo run -p sqlrustgo_sqllogictest -- --test-dir crates/sqlrustgo_sqllogictest/testdata
```

任何 PASS 声明都必须附 command output、timestamp、source_agent、source_run、evidence_hash 和 output location。

## 附录：英文原文

> 本附录保留本文件改写前的英文原文，便于追溯历史语义；当前正式阅读与执行口径以上方中文正文为准。

# SQLRustGo v3.12.0 Development Plan

> **Version**: v3.12.0
> **Status**: DRAFT
> **Date**: 2026-08-09
> **Product target**: GMP internal-audit retrieval database for `~/gmp-platform`
> **Truthfulness rule**: This plan defines future work and exit evidence. It does not claim any v3.12.0 gate has passed.

## 1. Planning Evidence

| Source | Evidence | Use |
|---|---|---|
| SQLRustGo local checkout | `develop/v3.12.0` at `9e157ed61b68a2a7c61ebae18cd0a6c15ceb79b1`, based on `develop/v3.11.0` at `6ab0da723e871d28b80dbd15ecf31852cd07c779` | v3.12 Draft planning baseline after syncing 252/250 Gitea |
| GMP-Platform 250 checkout | `develop/v1.4.0` at `c0f366d59e7d` | Integration findings baseline |
| GMP-Platform integration plan | commit `556a5f104900` | Cross-project implementation direction |
| SQLRustGo roadmap plan | `docs/plans/2026-08-08-sqlrustgo-v312-v400-gmp-rag-graph-plan.md` | v3.12/v4.0 scope split |
| v3.11.0 comprehensive assessment | `docs/releases/v3.11.0/COMPREHENSIVE_ASSESSMENT_REPORT.md` | GA evidence boundary, weak-point hardening, missing-test list |
| v3.10.0 testing-system report | `docs/releases/v3.10.0/TESTING_SYSTEM_BETA_REPORT.md` | SQLLogicTest / SQLite official corpus plan, Issue #3373 |
| v3.10.0 issues plan | `docs/releases/v3.10.0/plans/V310_ISSUES_PLAN.md` | V310-14 SQLLogicTest integration status and known download gap |
| Existing SLT runner | `crates/sqlrustgo_sqllogictest` | Runner exists; the current v3.12 smoke gate covers 25 non-`_unsupported` local `.test` files, superseding the older 22-file count |

Claim metadata for this document:

| Field | Value |
|---|---|
| source_agent | Codex |
| source_run | `codex-sqlrustgo-v312-plan-2026-08-09` |
| timestamp | `2026-08-09 13:35:00 CST` |
| evidence_hash | `local-git:9e157ed61b68a2a7c61ebae18cd0a6c15ceb79b1`; implemented gates must generate their own evidence hashes |
| conflict_resolution | Local SQLRustGo docs are the release SSOT; GMP-Platform findings are treated as integration inputs |

## 2. Current Findings Driving v3.12.0

v3.12.0 is planned because SQLRustGo v3.11.0 has useful database foundations, but the current GMP-Platform production path still needs a tighter database contract before SQLRustGo can safely become its primary database, vector store, and graph projection store.

| Finding | Impact on v3.12.0 |
|---|---|
| GMP-Platform already depends on `sqlrustgo-storage`, `sqlrustgo-types`, `sqlrustgo-vector`, and `sqlrustgo-graph` | v3.12 must stabilize a supported GMP-facing kernel contract instead of relying on ad hoc crate usage |
| `gmp-eval` uses its own hybrid search path instead of one shared retrieval engine | v3.12 must provide one auditable query contract that GMP CLI, API, and eval can call consistently |
| LIKE fallback is still part of the evaluation behavior, and disabling it is not reliably enforced | v3.12 must make LIKE diagnostic-only and add a no-LIKE production gate |
| Vector retrieval has reported empty-index and dimension-drift risks | v3.12 must enforce model name, dimension, vector hash, and rebuildable index invariants |
| Graph capability exists but is not active in the main evaluation/retrieval path | v3.12 must deliver SQL-backed GMP evidence graph projection, not a general graph database claim |
| RAG output needs stronger citation and audit evidence | v3.12 must make evidence bundles mandatory for every retrieval result and generated answer |
| v3.11.0 GA assessment found G3/G4 evidence-boundary risks | v3.12 must close TPC-H correctness, coverage-methodology, wire protocol, LOAD DATA, recovery, and upgrade/downgrade gaps before any broader production claim |
| v3.10.0 planned SQLLogicTest against SQLite official corpus, and v3.11.0 listed `sqllogictest runner all targets PASS`, but the gate was still TBD | v3.12 must promote SQLLogicTest/SQLite oracle testing to a real P0 gate with command output and baseline artifacts |
| Existing gate checks refer to both `crates/sqllogictest` and `crates/sqlrustgo_sqllogictest` | v3.12 must reconcile the runner path and package name so checks target the actual crate |

## 3. v3.12.0 Contract

Allowed v3.12.0 product claim after gates produce execution evidence:

> SQLRustGo v3.12.0 supports controlled GMP internal-audit retrieval workloads with SQLRustGo-managed relational storage, internal vector retrieval, SQL-backed graph projection, and auditable evidence bundles.

Disallowed v3.12.0 claims:

- General-purpose MySQL 5.7 replacement.
- General-purpose standalone vector database.
- General-purpose graph database.
- Production readiness without v3.11.0 weak-point disposition.
- PASS, GA, or compliance claims without command output, timestamp, source agent, source run, evidence hash, and output location.

## 4. Architecture Scope

v3.12.0 should expose a GMP kernel that GMP-Platform can use without duplicating retrieval logic.

```text
GMP-Platform CLI/API/Eval
        |
        v
SQLRustGo GMP Kernel Contract
        |
        +-- relational document/chunk/version/audit tables
        +-- keyword retrieval with exact match and tokenizer controls
        +-- vector persistence and rebuildable vector index
        +-- SQL-backed relation graph projection
        +-- RRF fusion and evidence bundle generation
        +-- backup/restore and audit-chain verification
```

The SQLRustGo side owns storage correctness, indexing contracts, query evidence, and gate scripts. GMP-Platform owns business workflows, UI/API orchestration, document review processes, and domain-specific prompts.

## 4.1 v3.11.0 Weak-Point Hardening Scope

v3.12.0 has two equal P0 tracks:

| Track | Purpose | Exit boundary |
|---|---|---|
| Track A: Production hardening from v3.11.0 | Close or explicitly carry v3.11.0 weak points: TPC-H correctness, coverage drift, wire protocol, LOAD DATA, crash recovery, backup/restore, upgrade/downgrade, dependency audit | No hidden v3.11.0 P0 blocker remains |
| Track B: GMP/RAG/Graph kernel | Deliver the controlled GMP internal-audit retrieval database contract for `~/gmp-platform` | GMP-Platform can run SQLRustGo-backed retrieval with evidence bundles |

The two tracks must not be traded off against each other. GMP features cannot justify skipping SQL correctness gates, and SQL correctness gates cannot be replaced by GMP-only fixtures.

## 4.2 SQLite SQLLogicTest Reinstatement

The SQLite-based automatic testing plan is re-adopted as a v3.12.0 P0 quality gate.

| Item | Current evidence | v3.12.0 action |
|---|---|---|
| Historical plan | `docs/releases/v3.10.0/TESTING_SYSTEM_BETA_REPORT.md` describes SQLLogicTest using SQLite official `.test` files, about 623 files / 5.9M cases | Carry forward as V312-11 |
| Issue plan | `docs/releases/v3.10.0/plans/V310_ISSUES_PLAN.md` V310-14 marks runner implemented but SQLite official suite download blocked | Complete testdata acquisition or define a reproducible mirror/cache |
| Existing runner | `crates/sqlrustgo_sqllogictest` exists; the current smoke gate covers 25 non-`_unsupported` local `.test` files | Make package build/run part of gate |
| v3.11.0 gap | `docs/releases/v3.11.0/RELEASE_GATE_CHECKLIST.md` lists sqllogictest runner all targets as TBD | Promote from TBD to required v3.12 Alpha/Beta/RC/GA staged thresholds |
| Path drift | `scripts/gate/check_beta_gate.sh` checks both `crates/sqllogictest` and `crates/sqlrustgo_sqllogictest` | Normalize checks to `crates/sqlrustgo_sqllogictest` or document the rename |

Current local baseline on 2026-08-09:

| Check | Result | Required v3.12.0 follow-up |
|---|---|---|
| `cargo build -p sqlrustgo_sqllogictest` | Executed through `scripts/gate/check_sqllogictest_v312.sh`; PASS | Keep as smoke build evidence; warning-free/clippy-clean remains governed by `cargo clippy --all-features -- -D warnings` |
| `cargo run -p sqlrustgo_sqllogictest -- --test-dir crates/sqlrustgo_sqllogictest/testdata` | Current smoke gate is 25/25 files passing, 100.0%; the historical 6/16, 27.3% baseline is superseded | Valid Beta smoke evidence; do not extrapolate to full SQLite official corpus PASS |
| SQLite official SQLLogicTest corpus | Full official corpus is not integrated into the current gate system; current scope is local smoke corpus | Add curated/official corpus manifest, cache/mirror procedure, selected-target definition, and exclusion registry for RC/GA |

## 5. Work Packages

### V312-01: Prior-Version Blocker Disposition

**Priority**: P0

**Goal**: Prevent v3.12.0 from inheriting unverified v3.11.0 claims.

**Implementation**:
- Produce a v3.11.0 weak-point disposition report for G3 coverage, G4 TPC-H SF=1, 168h SOAK, sqllogictest gate drift, and debt-registry drift.
- Decide whether each blocker is closed with evidence or carried as an explicit v3.12.0 P0 blocker.
- Ensure release docs do not describe v3.11.0 or v3.12.0 as GA without gate evidence.
- Treat the following as explicit v3.12 hardening inputs: TPC-H zero-row correctness, wire protocol strict path, LOAD DATA, coverage methodology, crash recovery, backup/restore, upgrade/downgrade, dependency audit refresh.

**Exit evidence**:
- Blocker report with command outputs and evidence hashes.
- Updated `STAGE.yaml` references.
- No hidden P0 blocker in `docs/releases/v3.12.0/STAGE.yaml`.
- SQLLogicTest disposition table: runner exists, testdata count, smoke pass, official corpus status.

### V312-02: GMP Kernel Contract

**Priority**: P0

**Goal**: Provide one supported SQLRustGo-facing contract for GMP-Platform.

**Implementation**:
- Define stable request/response structs for ingestion, retrieval, citation bundles, audit events, and graph traversal.
- Keep the contract narrow enough for v3.12.0: GMP internal-audit retrieval only.
- Return score components for keyword, vector, graph relation boost, and final RRF rank.

**Exit evidence**:
- Contract tests for serialization, error handling, and required evidence fields.
- GMP-Platform adapter test showing CLI/API/eval can call the same query path.

### V312-03: GMP Schema and Corpus Ingestion

**Priority**: P0

**Goal**: Store GMP documents, chunks, versions, embeddings, audit rows, and relations in SQLRustGo-managed data.

**Implementation**:
- Add idempotent schema creation for document metadata, chunk text, document versions, embedding metadata, audit events, and relation edges.
- Import `~/gmp-platform/gmp-md` with deterministic document and chunk IDs.
- Preserve source path, source hash, document version, effective status, and chunk hash.
- Classify skipped files and failures; unclassified ingestion failures block promotion.

**Exit evidence**:
- Full corpus ingestion report with document, chunk, embedding, relation, skipped, and failed counts.
- Re-ingestion test proving unchanged files do not create duplicate active records.
- Modified-file test proving a new document version is created without overwriting the original.

### V312-04: Keyword Retrieval Without Production LIKE Fallback

**Priority**: P0

**Goal**: Replace LIKE-driven scoring in the production retrieval path with auditable keyword retrieval.

**Implementation**:
- Define tokenizer and exact-match behavior for Chinese GMP terms, SOP numbers, clause IDs, deviation IDs, CAPA IDs, equipment IDs, and role names.
- Make LIKE fallback a diagnostic mode only.
- Add a production gate that fails when the GMP production query profile uses LIKE fallback scoring.

**Exit evidence**:
- Fixed keyword fixture with deterministic top-k results.
- No-LIKE production gate output.
- Query trace showing keyword terms, matched chunks, and score contribution.

### V312-05: Vector Persistence and Rebuildable Index

**Priority**: P0

**Goal**: Make vector retrieval reliable enough for GMP RAG retrieval.

**Implementation**:
- Persist embedding model, dimension, provider, chunk ID, vector hash, created timestamp, and active flag.
- Enforce dimension mismatch as a hard error.
- Support the GMP default embedding profile as `bge-m3`/1024d unless the deployment config explicitly selects another validated profile.
- Rebuild Flat or HNSW index from SQLRustGo-managed vectors.
- Treat an empty vector index as a gate failure for GMP production profile.

**Exit evidence**:
- Dimension mismatch test fails closed.
- Index rebuild test reproduces vector count and top-k fixture.
- GMP retrieval fixture proves vector is an active RRF channel, not only a fallback.

### V312-06: SQL-Backed GMP Evidence Graph

**Priority**: P0

**Goal**: Support GMP evidence navigation without claiming v3.12.0 is a general graph database.

**Implementation**:
- Store nodes and edges for documents, clauses, chunks, SOPs, deviations, CAPA items, equipment, roles, and audit findings.
- Support neighbors and depth-limited path queries up to depth 3.
- Support relation filters and evidence bundle output for every returned path.
- Keep Neo4j and the archived graph crate out of the required v3.12.0 production path unless separately revalidated.

**Exit evidence**:
- Node/edge count tests from representative corpus.
- Depth-limited path tests with deterministic results.
- Retrieval trace showing graph relation boost as an independent signal.

### V312-07: Hybrid Retrieval and RAG Evidence Bundle

**Priority**: P0

**Goal**: Provide the retrieval foundation GMP-Platform needs for internal-audit RAG.

**Implementation**:
- Fuse keyword, vector, and graph scores through traceable RRF.
- Return source path, document ID, version, chunk ID, chunk hash, citation text, score components, and access-control decision for every result.
- Provide a RAG answer context envelope that contains only cited chunks and carries evidence hashes forward.
- Ensure generated answers cannot hide missing citations.

**Exit evidence**:
- GMP audit question fixture report.
- Every returned answer/result has a citation bundle.
- Missing-citation test fails closed.

### V312-08: Compliance, Audit Trail, and Access Control

**Priority**: P0

**Goal**: Map SQLRustGo behavior to GMP/ALCOA+ controls for internal-audit retrieval.

**Implementation**:
- Add tamper-evident audit hash chain for import, approve, search, export, backup, restore, and evidence review events.
- Add role-based checks for import, approve, search, export, and audit review.
- Add electronic-signature hooks for controlled approval and export events.
- Update the GMP compliance matrix only when tests exist.

**Exit evidence**:
- Audit-chain tamper test fails closed.
- ACL denial and allow tests.
- Compliance matrix rows reference test evidence rather than design intent.

### V312-09: Backup, Restore, and Mixed SOAK

**Priority**: P0

**Goal**: Prove the GMP/RAG/graph store can survive routine operations.

**Implementation**:
- Back up relational data, document/chunk text, embedding metadata, vector payloads, graph projection, and audit chain.
- Restore into a clean SQLRustGo data directory and verify counts and hashes.
- Run a mixed workload that includes SQL reads/writes, ingestion, retrieval, graph traversal, audit export, and backup/restore smoke.

**Exit evidence**:
- Restore report with count and hash equality.
- Vector index rebuild after restore.
- 168h mixed SOAK report before any GA claim.

### V312-10: GMP-Platform Integration Verification

**Priority**: P0

**Goal**: Ensure SQLRustGo v3.12.0 is useful to GMP-Platform as deployed, not only internally tested.

**Implementation**:
- Provide a local integration profile for `~/gmp-platform` that uses SQLRustGo for storage, vector retrieval, and graph projection.
- Route GMP-Platform eval through the shared SQLRustGo-backed query contract.
- Ensure `--no-like-fallback` produces a real no-LIKE execution path.
- Produce cross-repository verification output without merging unsupported production claims into either repo.

**Exit evidence**:
- GMP-Platform eval report using SQLRustGo-backed retrieval.
- Trace proving keyword, vector, and graph channels were active or explicitly disabled by config.
- No-LIKE fallback enforcement output.

### V312-11: SQLite SQLLogicTest Oracle Gate

**Priority**: P0

**Goal**: Restore and complete the SQLite automatic testing plan that was introduced in v3.10.0 and left non-blocking in v3.11.0.

**Implementation**:
- Normalize the canonical runner path to `crates/sqlrustgo_sqllogictest` and package name `sqlrustgo_sqllogictest`.
- Preserve the current 25 non-`_unsupported` local `.test` files as the smoke corpus, and update the manifest whenever the selected smoke set changes.
- Add a reproducible acquisition path for the SQLite official SQLLogicTest corpus, or a checked-in manifest that records the exact upstream snapshot, source URL, file count, hash list, excluded tests, and exclusion reasons.
- Define staged thresholds:
  - Alpha: runner builds and `--help` works.
  - Beta: smoke corpus runs and produces a machine-readable report.
  - RC: curated SQLite-compatible subset runs with deterministic pass/fail/skip classification.
  - GA: all selected v3.12 SLT targets pass or have documented, issue-linked exclusions.
- Save reports under `docs/releases/v3.12.0/evidence/sqllogictest/`, with execution logs under `docs/releases/v3.12.0/logs/`.
- Integrate the gate into `scripts/gate/check_beta_gate.sh` successor logic or a dedicated `scripts/gate/check_sqllogictest_v312.sh`.

**Exit evidence**:
- `cargo build -p sqlrustgo_sqllogictest` output.
- `cargo run -p sqlrustgo_sqllogictest -- --test-dir crates/sqlrustgo_sqllogictest/testdata` output for the smoke corpus.
- Testdata manifest with file count, upstream snapshot, skipped features, and evidence hash.
- Gate script output showing PASS/FAIL with exit code.

## 6. Milestones

| Milestone | Scope | Exit criteria |
|---|---|---|
| Alpha | Blocker disposition, schema, ingestion skeleton, kernel contract | Build/fmt/clippy evidence plus schema and contract tests |
| Beta | Keyword, vector, graph, hybrid retrieval, sqllogictest smoke corpus | Fixed fixtures produce deterministic retrieval and evidence bundles; SLT smoke report exists |
| RC | Compliance, backup/restore, integration verification, curated SQLite SLT subset | Audit, ACL, restore, no-LIKE, GMP-Platform eval, and SLT subset evidence exists |
| GA | Controlled GMP internal-audit production readiness | All v3.12.0 gates pass with evidence, 168h mixed SOAK complete, no unresolved P0 blocker, SLT selected targets pass or are issue-linked exclusions |

## 7. Test and Gate Commands

The exact package names may be adjusted during implementation, but every command in a release gate must execute real checks and save logs under `docs/releases/v3.12.0/logs/`.

```bash
cargo fmt --all -- --check
cargo clippy --all-features -- -D warnings
cargo test --all-features
bash scripts/gate/check_docs_links.sh
bash scripts/gate/check_docs_consistency.sh
bash scripts/gate/check_gmp_v312.sh
bash scripts/gate/check_tpch_sf1.sh --sf1-dir "$SF1_DIR"
bash scripts/gate/check_sqllogictest_v312.sh
cargo build -p sqlrustgo_sqllogictest
cargo run -p sqlrustgo_sqllogictest -- --test-dir crates/sqlrustgo_sqllogictest/testdata
```

Planned GMP-specific checks:

```bash
cargo test --all-features gmp_schema_contract
cargo test --all-features gmp_ingestion_contract
cargo test --all-features gmp_keyword_no_like_contract
cargo test --all-features gmp_vector_dimension_contract
cargo test --all-features gmp_graph_projection_contract
cargo test --all-features gmp_audit_chain_contract
```

Planned cross-repository verification from GMP-Platform:

```bash
cargo run --release -p gmp-eval -- \
  --limit 408 \
  --no-like-fallback \
  --output reports/sqlrustgo-v312-eval.json
```

## 8. v4.0.0 Deferrals

These items are intentionally not v3.12.0 goals:

- General MySQL 5.7 replacement claim.
- Public standalone vector database product contract.
- Public standalone graph database product contract.
- Distributed HA or multi-tenant service guarantees.
- General Cypher compatibility.
- GMP-independent enterprise RAG platform claim.

v4.0.0 should promote the validated v3.12.0 GMP internals into first-class database, vector database, and graph database contracts only after v3.12.0 evidence exists.
