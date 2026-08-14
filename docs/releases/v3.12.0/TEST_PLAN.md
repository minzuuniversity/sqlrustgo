# SQLRustGo v3.12.0 测试计划

> **版本**: v3.12.0
> **状态**: 规划中
> **日期**: 2026-08-09
> **目标**: GMP 内审检索生产门禁
> **最近事实修正**: 2026-08-14，SQLLogicTest smoke gate 已在
> `develop/v3.12.0` commit `b6aede7996acc6a040bb847012e834e726bc4c03`
> 实跑 PASS；旧的 6/16、27.3% 基线仅作为历史记录。

## 1. 测试矩阵

| Gate | 领域 | 方法 | 阈值 |
|---|---|---|---|
| V312-G1 | 核心 SQL 构建与测试 | `cargo build` / `cargo test` / `cargo fmt` / `cargo clippy` | 退出码 0，0 warning |
| V312-G2 | v3.11 继承阻断项 | stage、debt、TPC-H、coverage、sqllogictest 证据 | 无未解决隐藏 P0 |
| V312-G3 | GMP schema | schema 单元测试和集成测试 | document/chunk/embedding/audit/relation 表有效 |
| V312-G4 | 语料导入 | 完整导入 `~/gmp-platform/gmp-md` | 0 个未分类失败 |
| V312-G5 | 向量检索 | 固定检索 fixture | top-k 结果确定，索引可重建 |
| V312-G6 | 混合检索 | GMP 内审问题集 | 每个结果都有引用证据包 |
| V312-G7 | 图谱投影 | 节点、边、路径测试 | 计数和按关系过滤的路径结果确定 |
| V312-G8 | 合规控制 | ACL、电子签名 hook、审计链 | 篡改测试 fail closed |
| V312-G9 | 备份恢复 | 恢复完整 GMP/RAG/投影存储 | hash 和 count 相等 |
| V312-G10 | 混合长稳 | 168h SQL + 导入 + 检索 + 审计 | 0 crash，audit-chain 不断裂 |
| V312-G11 | SQLite SQLLogicTest 判定门禁 | `sqlrustgo_sqllogictest` runner + SQLite 官方/缓存语料 | 选定目标通过，或每个排除项都关联 issue |
| V312-G12 | TPC-H 正确性收口 | SF=1 SQLRustGo 与 SQLite/PostgreSQL/MySQL 对比 row-count + SHA256 | 无无法解释的零行或 checksum 不一致 |
| V312-G13 | MySQL wire protocol 硬化 | COM_QUERY、COM_STMT、error、reset、TLS、compression E2E | 产生确定性的通过/失败 artifact |
| V312-G14 | LOAD DATA / 批量导入 | SF=1/SF=10 导入 benchmark + memory cap | 无 OOM；row-count/hash 相等 |
| V312-G15 | 崩溃恢复与升级 | kill -9、WAL replay、backup/restore、v3.10->v3.12 upgrade/downgrade | 恢复后 count/hash 相等 |
| V312-G16 | CREATE SEQUENCE executor | DDL、NEXTVAL、default expression、并发、事务回滚、WAL/recovery tests | 语义与持久化结果确定；不再标为 executor gap |
| V312-G17 | JSON/GIS 受控功能（Window → DEFERRED-3.13，见 `sql-feature-corpus/window_json_gis_scope.md`） | JSON_EXTRACT/JSON_VALUE/JSON_UNQUOTE 正反例；ST_Distance/ST_Within/ST_Contains/ST_Intersects 2D-Point 正反例 | JSON/GIS 支持范围内全 PASS，超出范围 fail with explicit unsupported error；Window 不测，3.13 跟进 |
| V312-G18 | 覆盖率与禁用测试债务 | canonical coverage command、disabled/API-drift manifest、flaky test quarantine | parser/mysql-server/mysql-client ≥80% 或有 issue-linked exception；无静默禁用测试 |
| V312-G19 | 性能与观测性 baseline | TPC-H SF=10、Sysbench OLTP、bulk-load benchmark、Prometheus/Slow Query Log e2e | 有可复跑脚本、阈值、日志和趋势对比 |
| V312-G20 | SQL corpus 与发布签核 | `test_sql_corpus.sh` all targets、R2.1-R2.8 invariant、2 reviewer sign-off | SQL corpus/架构 invariant 有输出；签核附 evidence hash |
| V312-G21 | v3.6-v3.10 历史债务总账 | historical backlog disposition + current verification sampling | 每个历史任务为 closed/superseded/carried/deferred，且 carried 项有 v3.12 issue |
| V312-G22 | MySQL 兼容与 SQL surface 回归 | SHOW、auth、prepared statements、ALTER、TIMESTAMP、连接池、函数、列级权限 fixtures | GMP/生产路径相关项 PASS；非目标项有 explicit unsupported/deferred 证据 |
| V312-G23 | 执行架构与优化器 invariant | DML PhysicalPlan/VTU/Parallel/SIMD route checks、Q4 semi/anti join/decorrelation benchmark | 无已知 bypass；优化器未完成项不得写成性能能力 |
| V312-G24 | Storage/Index/WAL tooling 回归 | WAL checkpoint、wal-verification、composite index、index stats、checksum/torn-write tests | storage invariant 有输出；未完成项有 issue/owner/expiry |
| V312-G25 | 测试基础设施激活 | SQLancer、test-runner、test-registry、E2E shell scripts、anti-fabrication binaries | 工具可运行并产出报告，或 documented retired/deferred |
| V312-G26 | 4.0 前功能整改门禁 | `PARTIAL_FEATURE_REMEDIATION_ISSUE_PLAN.md` 中每个 `PARTIAL` 都有整改 issue；issue mapping 与 `ISSUES_PLAN.md` 一致；READMED 中的 `PARTIAL` 状态与整改计划对齐 | 0 个未归属 `PARTIAL`；mapping diff=0；README 状态列与 plan 同步 |
| V312-G27 | V312-56 教学能力与 4.0 前功能整改门禁 | V312-56A~56H 子任务在 Beta 准入前已完成或明确降级；`tests/compat/teaching_sql_v3_12/` 28+ SQL fixtures + manifest.yml；SHOW INDEX/SHOW COLUMNS LIKE glob matcher 实跑 PASS；prepared/* + 6 crash tests + 3 tx tests + 5 EXPLAIN fixtures | V312-56A/B/C/D 实跑全 PASS；E/F/G disposition 已写入 `MYSQL_COMPAT_STATUS.md`；H Beta gate integration (B6_V312_56_TEACHING_CORPUS + B6_V312_56_EXPLAIN_FIXTURES) PASS |

## 2. 必需测试资产

| 资产 | 来源 | 用途 |
|---|---|---|
| 代表性 GMP 语料 | `~/gmp-platform/gmp-md` | 导入和检索 |
| 内审问题集 | `docs/releases/v3.12.0/fixtures/gmp_audit_questions.yml` | 检索质量 |
| Embedding fixture | 固定模型/提供方生成 | 确定性向量测试 |
| 图关系 fixture | SOP/CAPA/deviation 样本生成 | 图谱投影 |
| 篡改 fixture | 合成的修改后审计事件 | 审计链 fail closed |
| SQLLogicTest smoke 语料 | `crates/sqlrustgo_sqllogictest/testdata` | runner build/run 和本地兼容门禁 |
| SQLite 官方 SQLLogicTest 语料 | SQLite upstream snapshot 或可复现本地 mirror/cache | 广覆盖 SQL oracle testing |
| TPC-H SF=1 fixture | dbgen/BINT fixture + row-count manifest | 正确性和性能收口 |
| Wire protocol fixture | prepared statement、error packet、reset、TLS/compression scripts | MySQL 兼容性硬化 |
| 恢复 fixture | v3.10/v3.11/v3.12 WAL/data snapshots | 崩溃恢复和升级验证 |
| Sequence fixture | sequence DDL/default/NEXTVAL/concurrency/recovery SQL | `CREATE SEQUENCE` executor close-out |
| SQL feature corpus | window、JSON、GIS 正反例 SQL | 受控 SQL feature delivery |
| Disabled-test manifest | v3.11 禁用测试分析和 API-drift 清单 | 恢复、重写或有证据退休历史测试 |
| SF=10/Sysbench fixture | TPC-H SF=10 数据集、Sysbench schema/workload、bulk-load scripts | 性能基线和容量边界 |
| Observability fixture | Prometheus scrape、slow query threshold、error packet/retry traces | metrics/logging e2e |
| SQL corpus/invariant fixture | SQL corpus all-target scripts、R2.1-R2.8 invariant scripts、review sign-off template | RC/GA governance gate |
| Historical backlog manifest | v3.6.0-v3.10.0 plan/gate/test/debt 摘要 | 跨版本遗留项处置 |
| MySQL compatibility corpus | SHOW/auth/prepared/ALTER/TIMESTAMP/functions/permissions SQL and wire scripts | 兼容性回归 |
| Execution architecture probes | DML route grep、VTU/Parallel/SIMD invocation traces、Q4 optimizer benchmark | 执行路径和优化器复核 |
| Storage/WAL fixture | checkpoint、wal-verification、composite-index、index-statistics、torn-write/checksum cases | 存储可靠性 |
| Test infrastructure fixture | SQLancer seeds、test-runner config、test-registry manifest、E2E shell scripts | 测试基础设施激活 |

## 3. 证据要求

每个 PASS claim 必须包含：command、timestamp、source agent、source run、evidence hash、output location 和 PASS/FAIL boundary。

## 4. 拒绝规则

只要出现以下任一情况，v3.12.0 不得进入 GA：

- 静默忽略 v3.11.0 G3/G4/SOAK blocker。
- SQLLogicTest 仍只是文档/TBD 项，没有集成到真实 gate。
- SLT runner 路径仍在 `crates/sqllogictest` 与 `crates/sqlrustgo_sqllogictest` 之间漂移。
- SQLite 官方/缓存 corpus 没有 manifest、hash、file count 或 exclusion policy。
- GMP corpus import 有 unclassified failures。
- search result 缺少 source path、version、chunk hash 或 citation text。
- audit hash-chain tamper test 不会 fail。
- vector index 不能从 SQLRustGo-managed data 重建。
- graph projection 依赖未重新验证的 archived `graph` crate。
- 168h mixed SOAK 未完成，或没有证据却写成 PASS。
- TPC-H SF=1 zero-row 或 checksum mismatch 只用文字解释，没有 cross-engine output。
- `LOAD DATA`、backup/restore 声明缺少 count/hash evidence。
- `CREATE SEQUENCE` 仍停留在 AST/parser 层，executor/recovery 没有实跑证据。
- Window/GIS/JSON 写入 release note，但没有限定支持范围、错误边界和 fixture 输出。
- 历史禁用测试、API 漂移测试或 flaky 测试没有 manifest，却在 release 中写成“测试体系完整”。
- TPC-H SF=10、Sysbench、Prometheus 或 Slow Query Log 只有计划，没有可复跑 artifact。
- SQL corpus、架构 invariant 或 reviewer sign-off 仍是 TBD。
- v3.6-v3.10 历史遗留项没有总账，或总账中存在 `unknown`/`TBD` 状态。
- SHOW/auth/prepared/ALTER/TIMESTAMP/函数/权限等兼容性项被 release note 宣称支持，但没有对应 fixture。
- DML/VTU/Parallel/SIMD 执行路径仍依赖人工 grep 解释，没有脚本化 invariant 输出。
- SQLancer、test-runner、test-registry 或 E2E scripts 仍是骨架，却被计入测试体系能力。

## 5. SQLLogicTest / SQLite Oracle Gate

SQLite 自动测试框架是从 v3.10.0 继承的 P0 项，v3.12.0 必须把它变成可执行门禁。

| 阶段 | 必需 SLT 证据 |
|---|---|
| Alpha | `cargo build -p sqlrustgo_sqllogictest` 成功；runner `--help` 可用 |
| Beta | `crates/sqlrustgo_sqllogictest/testdata` 本地 smoke corpus 通过 `scripts/gate/check_sqllogictest_v312.sh`，manifest 显示 `pass_files == total_files`、`fail_files == 0`、open exclusions = 0 |
| RC | curated SQLite-compatible subset 运行，并输出 PASS/FAIL/SKIP 分类和 issue-linked exclusions |
| GA | selected SLT targets 全部通过，或每个 skipped/failed group 都有 issue、owner、expiry、rationale |

必需命令：

```bash
cargo build -p sqlrustgo_sqllogictest
cargo run -p sqlrustgo_sqllogictest -- --test-dir crates/sqlrustgo_sqllogictest/testdata
bash scripts/gate/check_sqllogictest_v312.sh
```

2026-08-14 当前 smoke 基线：

| 命令 | 观察结果 | 对测试计划的含义 |
|---|---|---|
| `cargo build -p sqlrustgo_sqllogictest` | 已由 `check_sqllogictest_v312.sh` 实跑，PASS | 可作为 smoke gate 的 build evidence；warning-free 仍以 clippy gate 为准 |
| `cargo run -p sqlrustgo_sqllogictest -- --test-dir crates/sqlrustgo_sqllogictest/testdata` | gate 实跑 clean；25/25 文件通过，通过率 100.0% | 本地 smoke corpus 达到 Beta smoke 要求 |
| `bash scripts/gate/check_sqllogictest_v312.sh` | exit 0；5 PASS / 0 FAIL；open exclusions = 0，closed historical exclusions = 16 | v3.12 smoke gate 已集成；full SQLite official/curated corpus 仍是 RC/GA 扩展项 |

必需 artifact：

| Artifact | 路径 |
|---|---|
| SLT smoke report | `docs/releases/v3.12.0/evidence/sqllogictest/smoke-report.md` |
| SQLite corpus manifest | `docs/releases/v3.12.0/evidence/sqllogictest/sqlite-corpus-manifest.json` |
| Exclusion registry | `docs/releases/v3.12.0/evidence/sqllogictest/exclusions.yml` |
| Gate output | `docs/releases/v3.12.0/logs/sqllogictest_<commit>_<timestamp>.log` |

Beta gate 不得只检查上述报告文件存在。必须实跑
`scripts/gate/check_sqllogictest_v312.sh`，并读取 manifest/exclusions 确认
当前 checkout 与报告一致。旧报告 `sqllogictest-oracle-gate-report.md` 仅保留
作为入口说明，不再作为唯一 evidence。

## 6. v3.11 弱项回归计划

| v3.11.0 评估弱项 | v3.12 回归测试 |
|---|---|
| TPC-H zero-row query correctness | 对 22 个 SF=1 query 做 cross-engine row-count 和 SHA256 compare |
| G3 coverage 口径漂移 | 单一 canonical coverage command，保存输出，不混用 PASS claim |
| MySQL wire protocol 低覆盖 | COM_QUERY、COM_STMT_PREPARE/EXECUTE/CLOSE、error packet、reset、TLS/compression E2E |
| LOAD DATA 未验证 | SF=1/SF=10 import、row counts、hashes、memory/time cap |
| Crash recovery 证据不足 | kill -9、WAL replay、dirty page recovery、backup/restore checksum |
| Upgrade/downgrade 证据不足 | v3.10.0/v3.11.0 fixture upgrade to v3.12.0 和 rollback verification |
| GMP/RAG/Vector/Graph 生产缺口 | GMP corpus、retrieval quality、vector rebuild、graph projection、ACL、audit-chain gates |
| `CREATE SEQUENCE` executor gap | DDL、NEXTVAL/default expression、并发、rollback、WAL replay/recovery |
| Window/GIS/JSON 仍是计划能力 | 对支持子集建立正反例 fixture，超出范围必须 fail with explicit unsupported error |
| disabled/API-drift tests | 逐文件恢复、重写或带 issue/owner/expiry 退休；不得再用静默 `cfg(any())` 隐藏 |
| parser/mysql-server/mysql-client 覆盖率不足 | canonical coverage command；per-crate 报告；未达标项必须进入 issue 计划 |
| TPC-H SF=10 / Sysbench / Observability 未落地 | 可复跑 benchmark、慢查询日志、Prometheus scrape 和趋势比较 |
| SQL corpus / invariant / reviewer sign-off TBD | all-target SQL corpus report、R2.1-R2.8 scripts output、2 reviewer evidence hash |
| v3.6 Beta PENDING 与覆盖率/测试编译延续问题 | historical disposition + current build/test/coverage sampling，不得沿用历史 PASS claim |
| v3.7 SHOW/auth/prepared/transaction routing 限制 | MySQL wire and SQL compatibility fixture，确认 closed 或 carried |
| v3.8 frozen/backlog 功能 | 按 GMP 相关性分层；Vector SQL、index stats、WAL tooling、advanced SQL functions 有明确范围 |
| v3.9 real SOAK and ignored long tests | wall-clock SOAK、ignored test registry、tx_wal autocommit reconciliation |
| v3.10 SQLancer/test-runner/test-registry/E2E scripts | 工具运行报告、manifest、anti-fabrication test binary compile close-out |

## 7. v3.11 历史遗留项专项测试

| 专项 | 必须输出 |
|---|---|
| Sequence executor close-out | `docs/releases/v3.12.0/logs/sequence_executor_<commit>_<timestamp>.log` 和 row/hash/recovery summary |
| Disabled/API-drift close-out | `docs/releases/v3.12.0/test-debt/disabled-test-disposition.yml`，每项含 action、owner、expiry、evidence |
| Coverage close-out | `docs/releases/v3.12.0/coverage/per-crate-coverage_<commit>_<timestamp>.md` |
| SQL feature controlled delivery | `docs/releases/v3.12.0/sql-feature-corpus/window_json_gis_report.md` |
| Performance/observability baseline | `docs/releases/v3.12.0/perf/sf10_sysbench_observability_<commit>_<timestamp>.md` |
| SQL corpus and invariant gate | `docs/releases/v3.12.0/gates/sql_corpus_invariant_signoff_<commit>_<timestamp>.md` |
| v3.6-v3.10 backlog disposition | `docs/releases/v3.12.0/historical-backlog/v36_v310_disposition_<commit>_<timestamp>.yml` |
| MySQL compatibility backlog | `docs/releases/v3.12.0/compat/mysql57_surface_regression_<commit>_<timestamp>.md` |
| Execution architecture backlog | `docs/releases/v3.12.0/architecture/execution_optimizer_invariants_<commit>_<timestamp>.md` |
| Storage/index/WAL backlog | `docs/releases/v3.12.0/storage/storage_wal_index_backlog_<commit>_<timestamp>.md` |
| Test infrastructure backlog | `docs/releases/v3.12.0/test-infra/sqlancer_runner_registry_e2e_<commit>_<timestamp>.md` |

## 8. 综合测试框架与覆盖率口径

v3.12.0 的覆盖率与综合测试执行口径以
`docs/releases/v3.12.0/COMPREHENSIVE_TEST_FRAMEWORK_AND_COVERAGE_BASELINE.md`
为准。该文档明确区分单元测试、集成测试、E2E、性能测试和 SOAK，不再用单一全
workspace 覆盖率数字替代分模块质量判断。

推荐覆盖率采集命令：

```bash
bash scripts/gate/check_v312_coverage_baseline.sh
```

## 附录：英文原文

> 本附录保留本文件改写前的英文原文，便于追溯历史语义；当前正式阅读与执行口径以上方中文正文为准。

# SQLRustGo v3.12.0 Test Plan

> **Version**: v3.12.0
> **Status**: DRAFT
> **Date**: 2026-08-09
> **Target**: GMP internal-audit retrieval production gate

## 1. Test Matrix

| Gate | Area | Method | Threshold |
|---|---|---|---|
| V312-G1 | Core SQL build/test | cargo build/test/fmt/clippy | exit 0, 0 warnings |
| V312-G2 | v3.11 carried blockers | stage + debt + TPC-H + coverage + sqllogictest evidence | no unresolved hidden P0 |
| V312-G3 | GMP schema | schema unit/integration tests | document/chunk/embedding/audit/relation tables valid |
| V312-G4 | Corpus ingestion | full `~/gmp-platform/gmp-md` import | 0 unclassified failures |
| V312-G5 | Vector retrieval | fixed retrieval fixture | deterministic top-k and rebuildable index |
| V312-G6 | Hybrid retrieval | GMP internal-audit question set | citation bundle on every result |
| V312-G7 | Graph projection | node/edge/path tests | deterministic counts and relation-filtered paths |
| V312-G8 | Compliance controls | ACL, e-signature hook, audit chain | tamper tests fail closed |
| V312-G9 | Backup/restore | restore full GMP/RAG/projection store | hash and count equality |
| V312-G10 | Mixed SOAK | 168h SQL + ingest + retrieval + audit | 0 crash, no audit-chain break |
| V312-G11 | SQLite SQLLogicTest oracle | `sqlrustgo_sqllogictest` runner + SQLite official/cached corpus | selected targets PASS or issue-linked exclusion |
| V312-G12 | TPC-H correctness close-out | SF=1 SQLRustGo vs SQLite/PostgreSQL/MySQL row-count + SHA256 | no unexplained zero-row or checksum mismatch |
| V312-G13 | MySQL wire protocol hardening | COM_QUERY/COM_STMT/error/reset/TLS/compression e2e | deterministic pass/fail artifact |
| V312-G14 | LOAD DATA / bulk import | SF=1/SF=10 import benchmark + memory cap | no OOM; row-count/hash equality |
| V312-G15 | Crash recovery and upgrade | kill -9, WAL replay, backup/restore, v3.10->v3.12 upgrade/downgrade | count/hash equality after recovery |
| V312-G16 | CREATE SEQUENCE executor | DDL, NEXTVAL, default expression, concurrency, tx rollback, WAL/recovery tests | semantics + persistence deterministic; no longer an executor gap |
| V312-G17 | JSON/GIS controlled features (Window → DEFERRED-3.13, see `sql-feature-corpus/window_json_gis_scope.md`) | JSON_EXTRACT/JSON_VALUE/JSON_UNQUOTE +/-; ST_Distance/ST_Within/ST_Contains/ST_Intersects 2D-Point +/- | JSON/GIS in-scope all PASS; out-of-scope fail with explicit unsupported error; Window not tested, tracked in 3.13 |
| V312-G18 | Coverage and disabled-test debt | canonical coverage command, disabled/API-drift manifest, flaky test quarantine | parser/mysql-server/mysql-client ≥80% or issue-linked exception; no silent disabled tests |
| V312-G19 | Performance and observability baseline | TPC-H SF=10, Sysbench OLTP, bulk-load benchmark, Prometheus/Slow Query Log e2e | re-runnable scripts, thresholds, logs and trend comparisons |
| V312-G20 | SQL corpus and release sign-off | `test_sql_corpus.sh` all targets, R2.1-R2.8 invariant, 2-reviewer sign-off | SQL corpus/architecture invariant output; sign-off with evidence hash |
| V312-G21 | v3.6-v3.10 historical debt ledger | historical backlog disposition + current verification sampling | each item closed/superseded/carried/deferred, carried items have v3.12 issue |
| V312-G22 | MySQL compat and SQL surface regression | SHOW, auth, prepared statements, ALTER, TIMESTAMP, connection pool, functions, column-level privilege fixtures | GMP/production-path items PASS; non-target items have explicit unsupported/deferred evidence |
| V312-G23 | Execution architecture and optimizer invariant | DML PhysicalPlan/VTU/Parallel/SIMD route checks, Q4 semi/anti join/decorrelation benchmark | no known bypass; optimizer WIP must not be claimed as performance capability |
| V312-G24 | Storage/Index/WAL tooling regression | WAL checkpoint, wal-verification, composite index, index stats, checksum/torn-write tests | storage invariant output; WIP items issue/owner/expiry-linked |
| V312-G25 | Test infrastructure activation | SQLancer, test-runner, test-registry, E2E shell scripts, anti-fabrication binaries | tools run and produce reports, or documented retired/deferred |
| V312-G26 | Pre-4.0 PARTIAL remediation gate | every `PARTIAL` in `PARTIAL_FEATURE_REMEDIATION_ISSUE_PLAN.md` has an owner issue; issue mapping matches `ISSUES_PLAN.md`; README `PARTIAL` column aligned | 0 unattributed `PARTIAL`; mapping diff=0; README column matches plan |
| V312-G27 | V312-56 teaching capability + pre-4.0 remediation gate | V312-56A~56H sub-tasks completed or explicitly demoted before Beta entry; `tests/compat/teaching_sql_v3_12/` 28+ SQL fixtures + manifest.yml; SHOW INDEX/SHOW COLUMNS LIKE glob matcher live PASS; prepared/* + 6 crash tests + 3 tx tests + 5 EXPLAIN fixtures | V312-56A/B/C/D live PASS; E/F/G disposition recorded in `MYSQL_COMPAT_STATUS.md`; H Beta gate integration (B6_V312_56_TEACHING_CORPUS + B6_V312_56_EXPLAIN_FIXTURES) PASS |

## 2. Required Test Assets

| Asset | Source | Use |
|---|---|---|
| 代表性 GMP 语料 | `~/gmp-platform/gmp-md` | 导入和检索 |
| Internal-audit question set | new `docs/releases/v3.12.0/fixtures/gmp_audit_questions.yml` | retrieval quality |
| Embedding fixture | generated from fixed model/provider | deterministic vector tests |
| Graph relation fixture | generated from SOP/CAPA/deviation samples | graph projection |
| 篡改 fixture | 合成的修改后审计事件 | 审计链 fail closed |
| SQLLogicTest smoke corpus | `crates/sqlrustgo_sqllogictest/testdata` | runner build/run and local compatibility gate |
| SQLite official SQLLogicTest corpus | SQLite upstream snapshot or reproducible local mirror/cache | broad SQL oracle testing |
| TPC-H SF=1 fixture | dbgen/BINT fixture with row-count manifest | correctness and performance close-out |
| Wire protocol fixture | prepared statement, error packet, reset, TLS/compression scripts | MySQL compatibility hardening |
| Recovery fixture | WAL/data snapshots across v3.10/v3.11/v3.12 | crash recovery and upgrade verification |

## 3. Evidence Requirements

Every PASS claim must include:

- command
- timestamp
- source agent
- source run
- evidence hash
- output location
- PASS/FAIL boundary

## 4. Rejection Rules

v3.12.0 must not enter GA if any of these are true:

- v3.11.0 G3/G4/SOAK blockers are silently ignored.
- SQLLogicTest remains only a document/TBD item and is not integrated into a real gate.
- The SLT runner path remains ambiguous between `crates/sqllogictest` and `crates/sqlrustgo_sqllogictest`.
- SQLite official/cached corpus acquisition has no manifest, hash, file count, or exclusion policy.
- GMP corpus import has unclassified failures.
- Search result lacks source path, version, chunk hash, or citation text.
- Audit hash-chain tamper test does not fail.
- Vector index cannot rebuild from SQLRustGo-managed data.
- Graph projection requires archived `graph` crate as an unvalidated production dependency.
- 168h mixed SOAK is not completed or is described as PASS without evidence.
- TPC-H SF=1 zero-row or checksum mismatch is explained only by text, without cross-engine output.
- `LOAD DATA` or backup/restore claims are made without count/hash evidence.

## 5. SQLLogicTest / SQLite Oracle Gate

The SQLite automatic testing framework is a carried-forward P0 item from v3.10.0. It must become executable in v3.12.0.

| Stage | Required SLT evidence |
|---|---|
| Alpha | `cargo build -p sqlrustgo_sqllogictest` succeeds; runner `--help` prints usable options |
| Beta | Local smoke corpus under `crates/sqlrustgo_sqllogictest/testdata` passes `scripts/gate/check_sqllogictest_v312.sh`; manifest shows `pass_files == total_files`, `fail_files == 0`, and open exclusions = 0 |
| RC | Curated SQLite-compatible subset runs with PASS/FAIL/SKIP classification and issue-linked exclusions |
| GA | All selected SLT targets pass, or every skipped/failed group has an issue, owner, expiry, and rationale |

Required commands:

```bash
cargo build -p sqlrustgo_sqllogictest
cargo run -p sqlrustgo_sqllogictest -- --test-dir crates/sqlrustgo_sqllogictest/testdata
bash scripts/gate/check_sqllogictest_v312.sh
```

Current baseline captured on 2026-08-09:

| Command | Observed result | Test-plan implication |
|---|---|---|
| `cargo build -p sqlrustgo_sqllogictest` | Executed through `check_sqllogictest_v312.sh`; PASS | Valid smoke build evidence; warning-free status is still governed by clippy |
| `cargo run -p sqlrustgo_sqllogictest -- --test-dir crates/sqlrustgo_sqllogictest/testdata` | Gate run is clean; 25/25 files pass, pass rate 100.0% | Local smoke corpus meets the Beta smoke requirement |
| `bash scripts/gate/check_sqllogictest_v312.sh` | exit 0; 5 PASS / 0 FAIL; open exclusions = 0 and closed historical exclusions = 16 | Smoke gate is integrated; full SQLite official/curated corpus remains RC/GA scope |

Required artifacts:

| Artifact | Path |
|---|---|
| SLT smoke report | `docs/releases/v3.12.0/evidence/sqllogictest/smoke-report.md` |
| SQLite corpus manifest | `docs/releases/v3.12.0/evidence/sqllogictest/sqlite-corpus-manifest.json` |
| Exclusion registry | `docs/releases/v3.12.0/evidence/sqllogictest/exclusions.yml` |
| Gate output | `docs/releases/v3.12.0/logs/sqllogictest_<commit>_<timestamp>.log` |

The earlier 6/16, 27.3% baseline is superseded by the current
`check_sqllogictest_v312.sh` evidence at commit `b6aede7996`. The Beta gate
must execute the gate and validate the manifest/exclusions; a report file
existing on disk is not sufficient evidence.

## 6. v3.11 Weak-Point Regression Plan

| Weak point from v3.11.0 assessment | v3.12 regression test |
|---|---|
| TPC-H zero-row query correctness | Cross-engine row-count and SHA256 compare for all 22 SF=1 queries |
| G3 coverage口径漂移 | Single canonical coverage command, stored output, no mixed PASS claims |
| MySQL wire protocol低覆盖 | COM_QUERY, COM_STMT_PREPARE/EXECUTE/CLOSE, error packet, reset, TLS/compression e2e |
| LOAD DATA未验证 | SF=1/SF=10 import, row counts, hashes, memory/time cap |
| Crash recovery证据不足 | kill -9, WAL replay, dirty page recovery, backup/restore checksum |
| Upgrade/downgrade证据不足 | v3.10.0/v3.11.0 fixture upgrade to v3.12.0 and rollback verification |
| GMP/RAG/Vector/Graph生产缺口 | GMP corpus, retrieval quality, vector rebuild, graph projection, ACL and audit-chain gates |
