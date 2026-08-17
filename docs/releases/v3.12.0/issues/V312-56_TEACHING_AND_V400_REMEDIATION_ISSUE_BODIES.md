# V312-56 4.0 前功能整改与 MySQL 教学能力补强 Issue 正文

> 本文件用于在 252 Gitea 创建 V312-56 总控和子任务 issue。关闭任何 issue 前必须满足 `docs/governance/ISSUE_CLOSING_VERIFICATION.md`，不得以“文档声称完成”替代 PR 合并和实跑门禁。
>
> **阶段要求:** V312-56A~56D 是 v3.12 Beta 准入前 blocker；V312-56E~56H 必须在 Beta 阶段完成或降级为有 owner/expiry/关闭边界的 deferred，不得拖到 RC 才首次定义。

## Issue Mapping

| Work Item | Gitea Issue | URL |
|---|---|---|
| V312-56 | #4250 | http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4250 |
| V312-56A | #4251 | http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4251 |
| V312-56B | #4252 | http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4252 |
| V312-56C | #4253 | http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4253 |
| V312-56D | #4254 | http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4254 |
| V312-56E | #4255 | http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4255 |
| V312-56F | #4256 | http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4256 |
| V312-56G | #4257 | http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4257 |
| V312-56H | #4258 | http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4258 |

## 总控 Issue: V312-56 4.0 前功能整改与 MySQL 教学能力补强

**Title:** `[v3.12.0][P0][V312-56] 4.0 前功能整改与 MySQL 教学能力补强总控`

**Labels:** `v3.12.0`, `P0`, `feature`, `testing`, `mysql-compat`, `teaching`, `v4.0.0-prep`

### 背景

`docs/releases/v3.12.0/COMPREHENSIVE_ASSESSMENT_REPORT.md` 指出：除存储过程/触发器外，SQLRustGo 还存在一批“有代码/有测试/有文档，但未形成生产或教学闭环”的功能，包括 information_schema/SHOW metadata、ALTER 完整族、Window/JSON/GIS、VIEW/CTE/MERGE、Partition/FullText、prepared/wire 教学实验、transaction/recovery 教学实验、SQLLogicTest 教学 corpus。

这些功能若不在 v3.12 Beta 前定义清楚，会继续造成 README/compat 文档与真实能力不一致，也会影响 4.0.0 生产路线和 MySQL 数据库教学场景。

### 范围

- V312-56A / #4251 Metadata/SHOW/information_schema 教学与兼容闭环
- V312-56B / #4252 SQL 教学 corpus + SQLite/PostgreSQL/MySQL oracle
- V312-56C / #4253 Transaction/crash recovery 教学实验
- V312-56D / #4254 Prepared statement / wire protocol 教学实验
- V312-56E / #4255 Optimizer/EXPLAIN 教学实验
- V312-56F / #4256 VIEW/CTE/MERGE disposition 与门禁
- V312-56G / #4257 Partition/FullText disposition 与 GMP keyword retrieval 决策
- V312-56H / #4258 Beta gate/docs/release evidence 集成

### 总控关闭条件

- V312-56A~56H 全部有 PR 合并到 `develop/v3.12.0`，或有明确的 `DEFERRED`/`UNSUPPORTED` 决策、owner、expiry、关闭边界。
- `docs/releases/v3.12.0/ISSUES_PLAN.md`、`TEST_PLAN.md`、`PARTIAL_FEATURE_REMEDIATION_ISSUE_PLAN.md`、`COMPREHENSIVE_ASSESSMENT_REPORT.md` 与实际状态一致。
- Beta gate 或 Beta 前检查脚本能验证 V312-56A~56D 的完成/降级状态。
- 生成 `docs/releases/v3.12.0/evidence/teaching_v400/V312-56-VERIFICATION.md`，包含 branch、commit、PR、merge commit、命令、exit code、输出摘要、evidence hash、不支持范围。

## V312-56A Metadata/SHOW/information_schema 教学与兼容闭环

**Title:** `[v3.12.0][P0][V312-56A] Metadata/SHOW/information_schema 教学与兼容闭环`

**Labels:** `v3.12.0`, `P0`, `mysql-compat`, `metadata`, `teaching`

### 目标

把 information_schema、SHOW CREATE TABLE、SHOW INDEX、SHOW COLUMNS、DESCRIBE 的真实能力整理为 MySQL 教学和基础客户端可用的受控子集。

### 验收条件

- `information_schema.tables`、`information_schema.columns`、`information_schema.indexes` 至少有 SQL 查询路径或明确 unsupported error。
- `SHOW CREATE TABLE` 输出与 live schema 一致，包含 column type、NULL、primary key、default 的受控字段。
- `SHOW COLUMNS` 不再是空 placeholder；若暂不支持 LIKE pattern，必须 fail-closed 并文档化。
- `SHOW INDEX` 至少返回已注册索引的 index name/table/column/type，或明确降级为 `DEFERRED`。
- MySQL CLI / e2e fixture 覆盖上述正反例。

### 必跑命令

```bash
cargo test --test mysql_server_e2e_test show -- --nocapture
cargo test --test ddl_e2e_test show -- --nocapture
bash scripts/gate/check_v312_21_mysql_compat.sh
```

## V312-56B SQL 教学 corpus + SQLite/PostgreSQL/MySQL oracle

**Title:** `[v3.12.0][P0][V312-56B] SQL 教学 corpus 与多 oracle 对比`

**Labels:** `v3.12.0`, `P0`, `sqllogictest`, `teaching`, `oracle`

### 目标

建立面向教学的 SQL corpus，不把 full SQLite official corpus 的大目标和教学最小闭环混在一起。教学 corpus 必须覆盖 SELECT、JOIN、GROUP、NULL、ORDER、LIMIT、DDL、DML、错误语义和事务基础。

### 验收条件

- 新增 `tests/compat/teaching_sql_v3_12/manifest.yml`，列出每个 SQL 文件的 oracle、期望状态、owner、适用阶段。
- 每个文件至少有 SQLite oracle；MySQL/PostgreSQL oracle 可分级补充。
- 每个 FAIL/SKIP 都有 issue link、owner、expiry、关闭边界。
- `check_sqllogictest_v312.sh` 或新建 gate 能区分 `teaching corpus PASS`、`smoke PASS`、`official corpus NOT CLAIMED`。
- 教学 corpus 不允许 `#[ignore]` 静默通过。

### 必跑命令

```bash
bash scripts/gate/check_sqllogictest_v312.sh
cargo test -p sqlrustgo-sql-corpus --test corpus_test -- --nocapture
bash scripts/gate/check_gate_test_integrity.sh
```

## V312-56C Transaction/crash recovery 教学实验

**Title:** `[v3.12.0][P0][V312-56C] Transaction/crash recovery 教学实验`

**Labels:** `v3.12.0`, `P0`, `transaction`, `recovery`, `teaching`

### 目标

补齐用于教学的事务和恢复实验：BEGIN/COMMIT/ROLLBACK/SAVEPOINT、隔离可见性、kill -9/WAL replay、checkpoint、backup/restore。

### 验收条件

- 新增教学实验文档，能逐步展示事务提交、回滚、savepoint 和可见性。
- crash/recovery fixture 能生成 before/after count/hash。
- backup/restore 后 GMP documents、embeddings、graph、audit chain 有 checksum。
- 所有实验命令可在本地复跑，失败不得被文档描述为 PASS。

### 必跑命令

```bash
bash scripts/gate/check_v312_14_crash_recovery.sh
cargo test --test wal_tx_contract_test -- --nocapture
cargo test --test compatibility_harness -- --nocapture
```

## V312-56D Prepared statement / wire protocol 教学实验

**Title:** `[v3.12.0][P0][V312-56D] Prepared statement 与 wire protocol 教学实验`

**Labels:** `v3.12.0`, `P0`, `mysql-compat`, `wire`, `teaching`

### 目标

把 COM_QUERY、COM_STMT_PREPARE/EXECUTE、text/binary result、error packet、reset、LOAD DATA 的受控路径整理成可教学、可验证的实验。

### 验收条件

- prepared statement roundtrip 有正例和错误参数数量/类型反例。
- wire protocol 实验输出 packet trace 或结构化 summary。
- LOAD DATA 教学 fixture 包含 row-count/hash。
- TLS/compression 若不能在 v3.12 Beta 完成，必须显示为 `DEFERRED`，并有 owner/expiry/关闭边界。

### 必跑命令

```bash
bash scripts/gate/check_v312_13_wire_load_data.sh
bash scripts/gate/check_v312_21_mysql_compat.sh
cargo test -p sqlrustgo-mysql-server --test e2e_wire_protocol -- --nocapture
```

## V312-56E Optimizer/EXPLAIN 教学实验

**Title:** `[v3.12.0][P1][V312-56E] Optimizer/EXPLAIN 教学实验`

**Labels:** `v3.12.0`, `P1`, `optimizer`, `teaching`

### 目标

补齐用于教学的 EXPLAIN、统计信息、histogram、hash join、semi/anti join 和简单 CBO 选择实验。

### 验收条件

- EXPLAIN 或等价 plan dump 可稳定输出 join type、estimated rows、chosen access path。
- 至少 5 个教学 SQL fixture 展示不同计划选择。
- 统计信息更新前后计划变化可复现。
- 不得把 heuristic plan 误写成成本模型严格正确。

### 必跑命令

```bash
cargo test -p sqlrustgo-optimizer --all-features -- --nocapture
cargo test -p sqlrustgo-planner --all-features -- --nocapture
```

## V312-56F VIEW/CTE/MERGE disposition 与门禁

**Title:** `[v3.12.0][P1][V312-56F] VIEW/CTE/MERGE disposition 与门禁`

**Labels:** `v3.12.0`, `P1`, `sql-surface`, `teaching`, `v4.0.0-prep`

### 目标

明确 VIEW、CTE、MERGE 在 v3.12、4.0.0、教学场景中的真实边界。MERGE 当前主 `execute()` 路径返回 unsupported，不得宣传。

### 验收条件

- CTE 教学 fixture 覆盖普通 CTE、递归 CTE 当前边界、错误语义。
- VIEW 若只保存 definition，必须标为 PARTIAL/DEFERRED；若进入教学场景，必须实现 view expansion 正反例。
- MERGE 必须选择：接入主执行路径并测试，或明确 `UNSUPPORTED/DEFERRED`。
- README、MYSQL_COMPAT_STATUS、COMPREHENSIVE_ASSESSMENT_REPORT 状态一致。

### 必跑命令

```bash
cargo test --test parser_e2e_test view -- --nocapture
cargo test --test merge_e2e_test -- --nocapture
bash scripts/gate/check_docs_consistency.sh
```

## V312-56G Partition/FullText disposition 与 GMP keyword retrieval 决策

**Title:** `[v3.12.0][P1][V312-56G] Partition/FullText disposition 与 GMP keyword retrieval 决策`

**Labels:** `v3.12.0`, `P1`, `storage`, `fulltext`, `partition`, `gmp`

### 目标

明确 Partition 和 FullText 是 4.0.0 生产能力、v3.12 受控教学/GMP keyword retrieval 能力，还是延期项。

### 验收条件

- PartitionInfo/storage-level tests 与 SQL `ALTER TABLE SET PARTITIONED BY` unsupported 状态不再冲突。
- FullTextIndex/storage-level tests 与 SQL MATCH/AGAINST 或 GMP keyword retrieval 需求有明确关系。
- 若 GMP 需要关键词检索，则定义受控 FullText 子集和 fixture。
- 若不进入 v3.12，则 README 和 release docs 明确 `DEFERRED`，不得保留模糊 PARTIAL。

### 必跑命令

```bash
cargo test -p sqlrustgo-storage fulltext -- --nocapture
cargo test --test partition_e2e_test -- --nocapture
bash scripts/gate/check_docs_consistency.sh
```

## V312-56H Beta gate/docs/release evidence 集成

**Title:** `[v3.12.0][P0][V312-56H] Beta gate、文档和 release evidence 集成`

**Labels:** `v3.12.0`, `P0`, `gate`, `docs`, `release`

### 目标

把 V312-56A~56G 的完成/延期状态接入 Beta 前检查和 release 文档，避免报告写了计划但门禁不执行。

### 验收条件

- 新增或更新 gate，检查 V312-56A~56D 在 Beta 准入前已完成或明确降级。
- `TEST_PLAN.md` 包含 V312-G27 教学与 4.0 前功能整改门禁。
- `STAGE.yaml` promotion_to_BETA_requires 包含 V312-56A~56D。
- `PARTIAL_FEATURE_REMEDIATION_ISSUE_PLAN.md` 和 `ISSUES_PLAN.md` 同步 issue mapping。
- `V312-56-VERIFICATION.md` 归档所有命令、exit code、输出摘要和 evidence hash。

### 必跑命令

```bash
bash scripts/gate/check_docs_links.sh entry
bash scripts/gate/check_docs_consistency.sh
bash scripts/gate/check_beta_v3.12.0.sh
```
