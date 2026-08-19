# SQLRustGo v3.12.0 测试计划

> **版本**: v3.12.0
> **状态**: 规划中
> **日期**: 2026-08-09
> **目标**: GMP 内审检索生产门禁

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
| V312-G17 | Window/GIS/JSON 受控功能 | ROW_NUMBER/RANK/DENSE_RANK、JSON path、ST_Distance/ST_Intersects/GeoJSON fixtures | 支持范围内全 PASS；超出范围有明确错误和文档 |
| V312-G18 | 覆盖率与禁用测试债务 | canonical coverage command、disabled/API-drift manifest、flaky test quarantine | parser/mysql-server/mysql-client ≥80% 或有 issue-linked exception；无静默禁用测试 |
| V312-G19 | 性能与观测性 baseline | TPC-H SF=10、Sysbench OLTP、bulk-load benchmark、Prometheus/Slow Query Log e2e | 有可复跑脚本、阈值、日志和趋势对比 |
| V312-G20 | SQL corpus 与发布签核 | `test_sql_corpus.sh` all targets、R2.1-R2.8 invariant、2 reviewer sign-off | SQL corpus/架构 invariant 有输出；签核附 evidence hash |
| V312-G21 | v3.6-v3.10 历史债务总账 | historical backlog disposition + current verification sampling | 每个历史任务为 closed/superseded/carried/deferred，且 carried 项有 v3.12 issue |
| V312-G22 | MySQL 兼容与 SQL surface 回归 | SHOW、auth、prepared statements、ALTER、TIMESTAMP、连接池、函数、列级权限 fixtures | GMP/生产路径相关项 PASS；非目标项有 explicit unsupported/deferred 证据 |
| V312-G23 | 执行架构与优化器 invariant | DML PhysicalPlan/VTU/Parallel/SIMD route checks、Q4 semi/anti join/decorrelation benchmark | 无已知 bypass；优化器未完成项不得写成性能能力 |
| V312-G24 | Storage/Index/WAL tooling 回归 | WAL checkpoint、wal-verification、composite index、index stats、checksum/torn-write tests | storage invariant 有输出；未完成项有 issue/owner/expiry |
| V312-G25 | 测试基础设施激活 | SQLancer、test-runner、test-registry、E2E shell scripts、anti-fabrication binaries | 工具可运行并产出报告，或 documented retired/deferred |
| V312-G26 | 存储过程和触发器基础生产子集 | `CREATE/DROP/SHOW PROCEDURE`、`CALL`、`IN` 参数、确定性 SQL 执行、BEFORE/AFTER row trigger、`NEW/OLD`、事务/WAL/恢复、递归、权限、SQLLogicTest/E2E | `bash scripts/gate/check_v312_procedure_trigger_gate.sh` 退出 0；所有失败/跳过项必须 issue-linked，不能以文档声明代替实测 |
| V312-G27 | 4.0 前功能整改与 MySQL 教学能力补强 | Metadata/SHOW/information_schema、SQL 教学 corpus、多 oracle、transaction/crash recovery 教学实验、prepared/wire 教学实验、Optimizer/EXPLAIN、VIEW/CTE/MERGE、Partition/FullText | V312-56A~56D 是 Beta 准入前 blocker；V312-56E~56H 必须在 Beta 阶段完成或显式降级；`V312-56-VERIFICATION.md` 必须包含实跑命令、exit code、输出摘要和 evidence hash |

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
| Procedure/Trigger fixture | procedure lifecycle SQL、CALL/DML SQL、trigger row semantics、WAL recovery、recursion、privilege denial、SQLLogicTest corpus | V312-G26 存储过程/触发器基础生产子集 |
| Teaching/V400 remediation fixture | metadata/show SQL、teaching SQL corpus、多 oracle manifest、transaction/recovery lab、wire/prepared packet trace、EXPLAIN/plan fixture、VIEW/CTE/MERGE disposition、Partition/FullText decision | V312-G27 4.0 前功能整改与 MySQL 教学能力补强 |

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
- 存储过程或触发器仍只停留在 parser/catalog/create 成功，缺少 `DROP/SHOW PROCEDURE`、`CALL` 真实 DML、`NEW/OLD` 断言、事务/WAL/恢复、递归限制、权限正反例或 SQLLogicTest/E2E gate，却在 README/release note 中写成 `DONE`。
- Metadata/SHOW/information_schema、SQL 教学 corpus、transaction/crash recovery lab、prepared/wire lab 未形成 issue-linked Beta 准入证据，却宣称 v3.12 已适合 MySQL 教学场景。
- VIEW/CTE/MERGE、Partition/FullText 只有 parser/storage 局部实现，却没有 disposition、门禁和文档同步。

## 5. SQLLogicTest / SQLite Oracle Gate

## 5A. V312-G27：4.0 前功能整改与 MySQL 教学能力补强

V312-G27 是从 `COMPREHENSIVE_ASSESSMENT_REPORT.md` 派生出来的 Beta 前整改门禁，专门处理“类似存储过程和触发器”的功能漂移：代码、测试、文档中存在局部实现，但没有形成生产/教学闭环。

### Beta 准入前 blocker

| 子项 | 要求 | 证据 |
|---|---|---|
| V312-56A Metadata/SHOW/information_schema | information_schema tables/columns/indexes 或明确 unsupported；SHOW CREATE/COLUMNS/INDEX/DESCRIBE 正反例 | MySQL/e2e fixture + compat gate + `V312-56A_METADATA_TEACHING.md` (56A-R1/R2/R4 DONE @ PR #4323/#4349/#4345, 56A-R3 DEFERRED → v3.13+ MySQL admin extensions) |
| V312-56B SQL 教学 corpus | teaching corpus manifest、多 oracle、PASS/FAIL/SKIP、issue-linked exclusions | SQLLogicTest/corpus gate + `V312-56B_CORPUS_TEACHING.md` (PR #4327 @ `5c6e640edb`) |
| V312-56C Transaction/crash recovery lab | BEGIN/COMMIT/ROLLBACK/SAVEPOINT、kill -9/WAL replay、backup/restore count/hash | recovery/compat tests + `V312-56C_TRANSACTION_TEACHING.md` (V312-14 gate PARTIAL,3 FAIL #3965 disclosed) |
| V312-56D Prepared/wire lab | COM_QUERY/COM_STMT/error/reset/LOAD DATA；TLS/compression DONE 或 DEFERRED | wire/load-data gate + `V312-56D_WIRE_TEACHING.md` (TLS client DEFERRED + wire trace DEFERRED) |

### Beta Gate Snapshot (2026-08-19, post-PR #4354 workspace clippy fix)

```
$ bash scripts/gate/check_beta_v3.12.0.sh
PASS: 38/40   WARN: 2   BLOCKERS: 0
✓ All checks pass — ready for BETA promotion.
```

All 12 `promotion_to_BETA_requires` items from `STAGE.yaml` are satisfied.
Full snapshot: `docs/releases/v3.12.0/evidence/v312-56/V312-56-VERIFICATION.md`
(Beta Gate Snapshot section).

### Beta 阶段收口项

| 子项 | 要求 | 证据 |
|---|---|---|
| V312-56E Optimizer/EXPLAIN | EXPLAIN/plan dump、统计信息、hash/semi/anti join 教学 fixture | optimizer/planner tests |
| V312-56F VIEW/CTE/MERGE | VIEW/CTE/MERGE 明确 DONE/DEFERRED/UNSUPPORTED | parser/e2e/docs consistency |
| V312-56G Partition/FullText | Partition/FullText 与 SQL surface/GMP keyword retrieval 决策一致 | storage/e2e/docs consistency |
| V312-56H Gate/docs/evidence | Beta gate 检查 V312-56A~56D；`V312-56-VERIFICATION.md` 归档证据 | beta gate + docs links |

禁止事项：

- 不允许把 `V312-56-VERIFICATION.md` 写成纯文字总结；必须包含实际命令、exit code、输出摘要和 evidence hash。
- 不允许用 `TBD`、`PENDING` 或“后续处理”关闭 V312-56 子任务。
- 不允许把 MERGE、VIEW、Partition、FullText 的 parser/storage 局部实现宣传成 SQL 主路径完成。
- 不允许在 V312-14 gate = PARTIAL (3 FAIL #3965) 的情况下,把 56C transaction/crash recovery lab 宣传为 PASS,需诚实披露 PARTIAL 状态。
- 不允许用 `SUBSTANTIALLY_COMPLETE` / `ACCEPTED-WITH-BINDING-MANIFEST` 关闭标记(per V313-STRICT-CLOSE-STANDARDS §3)。DEFERRED-with-explicit-boundary(带 owner + expiry + close boundary)方可接受。

SQLite 自动测试框架是从 v3.10.0 继承的 P0 项，v3.12.0 必须把它变成可执行门禁。

| 阶段 | 必需 SLT 证据 |
|---|---|
| Alpha | `cargo build -p sqlrustgo_sqllogictest` 成功；runner `--help` 可用 |
| Beta | `crates/sqlrustgo_sqllogictest/testdata` 本地 smoke corpus 可运行并输出报告 |
| RC | curated SQLite-compatible subset 运行，并输出 PASS/FAIL/SKIP 分类和 issue-linked exclusions |
| GA | selected SLT targets 全部通过，或每个 skipped/failed group 都有 issue、owner、expiry、rationale |

必需命令：

```bash
cargo build -p sqlrustgo_sqllogictest
cargo run -p sqlrustgo_sqllogictest -- --test-dir crates/sqlrustgo_sqllogictest/testdata
bash scripts/gate/check_sqllogictest_v312.sh
```

2026-08-09 当前基线：

| 命令 | 观察结果 | 对测试计划的含义 |
|---|---|---|
| `cargo build -p sqlrustgo_sqllogictest` | build 可完成，但依赖 crate 仍有 warning | 只能作为初始 Alpha build evidence，不能作为 clippy/warning-free evidence |
| `cargo run -p sqlrustgo_sqllogictest -- --test-dir crates/sqlrustgo_sqllogictest/testdata` | runner 可完成；6/16 文件通过，通过率 27.3% | v3.12 必须 triage failures、分类 expected incompatibilities，并在 Beta/RC 前提升 smoke gate |
| `bash scripts/gate/check_sqllogictest_v312.sh` | 当前计划基线中脚本尚不存在 | 实现前不能称 SQLLogicTest gate 已集成 |

必需 artifact：

| Artifact | 路径 |
|---|---|
| SLT smoke report | `docs/releases/v3.12.0/sqllogictest-baseline/smoke-report.md` |
| SQLite corpus manifest | `docs/releases/v3.12.0/sqllogictest-baseline/sqlite-corpus-manifest.json` |
| Exclusion registry | `docs/releases/v3.12.0/sqllogictest-baseline/exclusions.yml` |
| Gate output | `docs/releases/v3.12.0/logs/sqllogictest_<commit>_<timestamp>.log` |

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
| Procedure/Trigger close-out | `docs/releases/v3.12.0/evidence/procedure_trigger/V312-55-VERIFICATION.md` 和 `docs/releases/v3.12.0/logs/procedure_trigger_<commit>_<timestamp>.log` |
| Teaching/V400 remediation close-out | `docs/releases/v3.12.0/evidence/teaching_v400/V312-56-VERIFICATION.md` 和 `docs/releases/v3.12.0/logs/teaching_v400_<commit>_<timestamp>.log` |

## 8. 综合测试框架与覆盖率口径

v3.12.0 的覆盖率与综合测试执行口径以
`docs/releases/v3.12.0/COMPREHENSIVE_TEST_FRAMEWORK_AND_COVERAGE_BASELINE.md`
为准。该文档明确区分单元测试、集成测试、E2E、性能测试和 SOAK，不再用单一全
workspace 覆盖率数字替代分模块质量判断。

推荐覆盖率采集命令：

```bash
bash scripts/gate/check_v312_coverage_baseline.sh
```

Alpha / smoke gate 只验证框架配置，不触发 16 个 crate 的全量覆盖率采集：

```bash
bash scripts/gate/check_v312_coverage_baseline.sh --check-config
```

复核已有覆盖率 artifact 时使用：

```bash
bash scripts/gate/check_v312_coverage_baseline.sh --enforce-stage alpha \
  --summary docs/releases/v3.12.0/coverage-baseline/current_<commit>_<timestamp>/summary.json
```

夜间、Beta、RC、GA 才允许不传 `--summary` 直接执行 `--enforce-stage <stage>`，因为这会重新采集所有 tracked crate 的覆盖率。

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
| Beta | Local smoke corpus under `crates/sqlrustgo_sqllogictest/testdata` runs and writes a report |
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
| `cargo build -p sqlrustgo_sqllogictest` | Build completes; warnings are still emitted from dependent crates | Acceptable only as initial Alpha build evidence, not as clippy/warning-free evidence |
| `cargo run -p sqlrustgo_sqllogictest -- --test-dir crates/sqlrustgo_sqllogictest/testdata` | Runner completes with 6/16 files passing and 27.3% pass rate | v3.12.0 must triage failures, classify expected incompatibilities, and raise the smoke gate before Beta/RC |
| `bash scripts/gate/check_sqllogictest_v312.sh` | Script not yet present in the current plan baseline | Must be implemented before the SQLLogicTest gate can be called integrated |

Required artifacts:

| Artifact | Path |
|---|---|
| SLT smoke report | `docs/releases/v3.12.0/sqllogictest-baseline/smoke-report.md` |
| SQLite corpus manifest | `docs/releases/v3.12.0/sqllogictest-baseline/sqlite-corpus-manifest.json` |
| Exclusion registry | `docs/releases/v3.12.0/sqllogictest-baseline/exclusions.yml` |
| Gate output | `docs/releases/v3.12.0/logs/sqllogictest_<commit>_<timestamp>.log` |

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
