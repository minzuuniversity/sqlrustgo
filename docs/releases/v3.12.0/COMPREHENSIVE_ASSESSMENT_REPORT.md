# SQLRustGo v3.12.0 综合评估报告

> **版本**: v3.12.0
> **阶段**: ALPHA
> **评估日期**: 2026-08-14
> **当前本地分支**: `codex/v312-partial-remediation-plan`
> **当前本地 commit**: `50e5d121b0ff73bcf818dc2e8c6cdbafc72a5817`
> **发布定位**: GMP 内审检索数据库 + v3.11.0 弱项硬化 + MySQL-style 基础兼容收口
> **证据等级**: DerivedDoc；本报告基于本地文档、源码静态核查和已有 evidence 报告编写，未重新执行完整 cargo workspace、TPC-H、Sysbench、SQLLogicTest、coverage 或安全扫描

## 0. Provenance

| 字段 | 值 |
|---|---|
| source_agent | Codex |
| source_run | v312-comprehensive-assessment-20260814 |
| timestamp | 2026-08-14T22:20:00+08:00 |
| evidence_hash | local-git:50e5d121b0ff73bcf818dc2e8c6cdbafc72a5817 |
| input_refs | [`../v3.11.0/COMPREHENSIVE_ASSESSMENT_REPORT.md`](../v3.11.0/COMPREHENSIVE_ASSESSMENT_REPORT.md), [`STAGE.yaml`](STAGE.yaml), [`README.md`](README.md), [`FEATURE_CHECKLIST.md`](FEATURE_CHECKLIST.md), [`MYSQL_COMPAT_STATUS.md`](MYSQL_COMPAT_STATUS.md), [`PARTIAL_FEATURE_REMEDIATION_ISSUE_PLAN.md`](PARTIAL_FEATURE_REMEDIATION_ISSUE_PLAN.md), [`COMPREHENSIVE_TEST_FRAMEWORK_AND_COVERAGE_BASELINE.md`](COMPREHENSIVE_TEST_FRAMEWORK_AND_COVERAGE_BASELINE.md), [`v312_verification_report.md`](v312_verification_report.md), [`v312-11-round14-status.md`](v312-11-round14-status.md), [`evidence/mysql_compat/V312-21-VERIFICATION.md`](evidence/mysql_compat/V312-21-VERIFICATION.md), [`evidence/V312-OPEN-ISSUE-REVIEW.md`](evidence/V312-OPEN-ISSUE-REVIEW.md) |
| limitation | 本报告不把历史文档 PASS 声明单独作为当前 PASS 证据；凡缺少当前实跑日志的项目均标为待验证、PARTIAL、DEFERRED 或 OPEN |

## 1. 总体结论

**v3.12.0 目前处于 ALPHA 阶段，不是 GA，也不能宣称已经达到完整 MySQL 5.7 替代水平。**

从现有证据看，v3.12.0 相比 v3.11.0 的方向是正确的：它已经把 GMP 文档、chunk、embedding、audit、hybrid retrieval、SQL-backed graph projection、SQLLogicTest smoke、MySQL surface disposition、分层覆盖率、PARTIAL 功能治理纳入版本范围。但当前仍有多个 GA blocker：TPC-H correctness、SQLLogicTest 16 文件 pass rate、MySQL wire/prepared/TLS/compression、LOAD DATA/SF=10、WAL/crash/backup/restore、Sysbench write path、procedure/trigger、部分 SQL 兼容功能和低覆盖 crate。

因此，v3.12.0 当前可定位为：

| 维度 | 当前结论 |
|---|---|
| MySQL 版本水平 | 语法和功能覆盖接近“MySQL 5.7 子集 + 教学型数据库 + 部分 MySQL 兼容层”；不能宣称完整 MySQL 5.7 |
| 功能成熟度 | GMP 专用能力较强；通用 SQL/MySQL 兼容仍是 PARTIAL/受控 |
| 性能成熟度 | 有 TPC-H、Sysbench、bulk-load、coverage/perf 框架和部分 evidence；还没有形成生产级性能 SLA |
| 稳定性成熟度 | v3.11 有强 SOAK 证据；v3.12 需要重新跑 mixed SOAK、crash recovery、backup/restore、upgrade |
| GMP 生产可行性 | 可作为受控试点/灰度数据库；GA 前必须关闭 GMP 权限、审计链、备份恢复、检索质量和向量索引重建门禁 |
| MySQL 教学可行性 | 可用于 parser、executor、存储、索引、事务、wire protocol、governance 教学；若作为 MySQL 兼容教学平台，需要补足 SHOW/information_schema、procedure/trigger、prepared、事务恢复和 SQLLogicTest |

## 2. v3.12 达到了 MySQL 的哪个版本水平

### 2.1 简短回答

**v3.12 当前不能称为 MySQL 5.7 兼容数据库。更准确的说法是：它具备 MySQL 5.7 风格的一部分 SQL surface、wire protocol smoke、DDL/DML 子集和若干高级能力，但整体仍处于“受控 MySQL 子集”水平。**

如果用 MySQL 版本做类比：

| 类比维度 | 评估 |
|---|---|
| SQL 基础 DDL/DML | 接近早期 MySQL 子集：CREATE/INSERT/SELECT/UPDATE/DELETE、部分 ALTER、REPLACE、SHOW/DESCRIBE 可用 |
| MySQL 5.7 常用 SQL | PARTIAL：ROLLUP/CUBE、JSON 函数、窗口函数、ALTER 族、SHOW CREATE TABLE 等有局部实现或证据，但文档与门禁仍需对齐 |
| MySQL 5.7 wire/client | PARTIAL：COM_QUERY/部分 COM_STMT/LOAD DATA smoke 有进展；prepared roundtrip、TLS、compression、reset、error path 仍需收口 |
| MySQL 5.7 运维生态 | 不足：information_schema、SHOW CREATE、权限细节、连接池、timestamp 行为、auth edge、backup/restore 工具链仍不完整 |
| MySQL 5.7 生产替代 | 未达到：TPC-H correctness、SQLLogicTest、crash recovery、upgrade、Sysbench write path、LOAD DATA/SF=10 未完全闭环 |

### 2.2 已经具备的 MySQL-style 能力

| 能力 | 当前证据 |
|---|---|
| ALTER TABLE ADD/DROP/MODIFY/RENAME | `evidence/mysql_compat/V312-21-VERIFICATION.md` 将 alter_add/drop/modify/rename 归为 PASS；源码 `src/engine_ddl.rs` 也有对应执行分支 |
| SHOW TABLES / DESCRIBE / SHOW CREATE TABLE | `MYSQL_COMPAT_STATUS.md` 记录 SHOW/DESCRIBE 支持；`src/engine_ddl.rs` 已有 `execute_show_create_table` |
| REPLACE INTO | 集成测试 `tests/integration/dml/replace_test.rs` 存在；复杂语义仍需区分 unsupported/deferred |
| ROLLUP / CUBE | `src/engine_select.rs` 有 WITH ROLLUP/CUBE 执行逻辑，`tests/integration/sql/rollup_cube_test.rs` 有集成测试；但 MySQL compat 文档仍有 deferred 残留，需要整改 |
| JSON/GIS 函数 | `crates/executor/src/expr/mod.rs` 有 JSON_EXTRACT/JSON_VALID/JSON_TYPE 和 ST_WITHIN/ST_DISTANCE/ST_CONTAINS/ST_INTERSECTS；需要限定 SQL 类型、索引和错误语义 |
| CREATE/DROP/ALTER SEQUENCE | `STAGE.yaml` 和 `v312_verification_report.md` 记录 V312-15；源码主路径有 Create/Drop/Alter Sequence 分发 |
| 存储过程/触发器 | 不是“完全没有”；已有 parser、catalog、CALL、CREATE TRIGGER、DML hook 局部路径；但 V312-55 明确为 P0 整改，不得宣称完成 |

### 2.3 尚未达到 MySQL 5.7 的关键差距

| 差距 | 当前状态 | 影响 |
|---|---|---|
| SQLLogicTest 全量或精选语义门禁 | smoke gate PASS，但 16 文件 runner 仍 6/16 | 不能证明 SQL 语义兼容 |
| TPC-H correctness | v3.11 22/22 可运行；v3.12 要求 row-count/SHA256；zero-row 风险仍需关闭 | 不能证明分析 SQL 正确性 |
| COM_STMT / prepared statement | 有基本路径，`prepared_stmt_roundtrip` deferred | 阻塞 sysbench 默认路径和很多 MySQL 客户端 |
| TLS / compression / reset / error edge | 有 smoke 或 typed wrapper，生产客户端路径未闭环 | 阻塞生产客户端兼容声明 |
| information_schema / SHOW CREATE / metadata | 有 crate 或局部函数，但 SQL/wire 生态不完整 | 影响 ORM、教学工具和运维工具 |
| TIMESTAMP 默认值、zero value、timezone | deferred | 影响 MySQL 应用迁移 |
| 权限细节 | RLS/column privilege 有基础，但 MySQL-style column-level permissions 被标过 unsupported/deferred | GMP 生产和教学都需硬化 |
| 存储过程/触发器 | V312-55 P0 | 影响 MySQL 教学完整性和审计自动化 |
| crash recovery / backup / upgrade | v3.12 仍需当前 HEAD 实跑 | 影响生产可恢复性 |

## 3. 功能评估

### 3.1 GMP/RAG/Graph 专用能力

v3.12 在 GMP 专用方向已经形成较清晰的产品主线：

| 功能 | 当前状态 | 评估 |
|---|---|---|
| GMP document/chunk/embedding/audit schema | `FEATURE_CHECKLIST.md` 标为 DONE | 可作为 GMP 内审检索数据底座候选 |
| Idempotent GMP markdown ingestion | DONE | 适合进入 representative corpus 试点 |
| Hybrid retrieval | DONE | 需继续用固定 GMP 问题集验证召回率、引用质量和稳定性 |
| RAG evidence bundle | DONE | 符合 GMP 内审“可追溯回答”的方向 |
| SQL-backed graph projection | DONE / 受控 | 可用于 evidence navigation，不等同通用图数据库 |
| Audit hash-chain | DONE | 必须继续补 tamper 正反例和 backup/restore 后一致性 |
| Internal vector retrieval | PARTIAL / blocker | 需要 index rebuild、dimension/hash、empty-index、质量 fixture 门禁 |

**评估**: 如果 GMP-Platform 是受控部署、数据规模可管理、具备回滚方案，v3.12 可以进入集成试点。但要用于 GMP 生产环境，GA 前必须补齐权限矩阵、审计链、备份恢复、检索质量、向量索引重建和 mixed SOAK 证据。

### 3.2 SQL 与执行引擎

| 能力 | 当前状态 | 需要补强 |
|---|---|---|
| 基础 DDL/DML | 基本可用 | SQLLogicTest selected corpus、错误语义、事务边界 |
| SELECT/JOIN/GROUP BY | v3.11 已有 TPC-H 可运行性突破；v3.12 继续 correctness | TPC-H row-count/SHA256、zero-row 解释、复杂 join 正确性 |
| Window/GIS/JSON | 局部实现 | 明确 v3.12 受控子集或降级 DEFERRED；补 SQL path/e2e |
| MERGE | parser/LocalExecutor 迹象存在，但 `execute()` 主路径返回 unsupported | 不应纳入 v3.12 MySQL 兼容声明；可作为 SQL 标准扩展后续 |
| VIEW/CTE | CTE 已进入主路径；VIEW 当前更像 definition storage | 补 view expansion、权限、更新限制和 SQLLogicTest |
| Procedure/Trigger | V312-55 P0 | CREATE/DROP/SHOW PROCEDURE、CALL、IN 参数、NEW/OLD、WAL/recovery、权限、递归 |

### 3.3 存储、事务和恢复

v3.11 的存储和索引能力已经较强，但 v3.12 的生产评估不能只继承 v3.11 的 GA 结论。当前 v3.12 必须重新验证：

| 项 | 生产要求 |
|---|---|
| WAL / MVCC | DML 主路径、rollback、checkpoint、replay、crash recovery 当前 HEAD 实跑 |
| backup/restore | GMP documents、embeddings、graph relations、audit chain 恢复后 hash 一致 |
| upgrade/downgrade | v3.10/v3.11 到 v3.12 升级和回滚脚本实跑 |
| partition/fulltext | storage 层有结构或测试，但 SQL surface/门禁不足；若不进 v3.12，应明确 DEFERRED |
| torn page / double write | 若仍有 NOT IMPLEMENTED 或待验证项，必须保留 blocker 或风险接受 |

## 4. 性能评估

### 4.1 当前可引用的性能信号

| 性能项 | 当前事实边界 |
|---|---|
| v3.11 TPC-H SF=1 | 22/22 completed、519.15s、0 panic、0 OOM；但不能外推为 correctness 零差异 |
| v3.12 TPC-H | 已有 SF=1/SF=10、bulk-load、oracle artifact 目录；仍需以当前 HEAD 形成最终 correctness report |
| Sysbench | read_only 有可用信号；write_only/read_write 和 prepared statement 仍有 blocker |
| LOAD DATA / bulk-load | smoke/SF=1/SF=10 evidence 逐步积累；大表吞吐、内存和 hash parity 仍是主要风险 |
| RAG/vector | 有 GMP fixture 和检索方向；尚未形成生产 SLA，包括 rebuild 时间、top-k 质量、延迟分布 |
| SOAK | v3.11 长稳强；v3.12 需要 SQL + GMP ingest + retrieval + audit + backup/restore mixed SOAK |

### 4.2 性能结论

**v3.12 当前不能给出通用生产性能承诺。** 它可以给出“已建立性能测试框架、已有局部基线、正在补充 TPC-H/Sysbench/bulk-load/RAG/vector evidence”的结论。GA 前应只承诺实测过的 workload，并保存 fixture、commit、命令、日志、row-count/hash 和硬件信息。

## 5. 稳定性与测试质量

v3.12 已经建立了更合理的 5 层测试/覆盖率框架：

| 层级 | 作用 | 当前风险 |
|---|---|---|
| L0 单元测试 | 每 PR 基础质量 | 低覆盖 crate 仍需补测 |
| L1 集成测试 | SQL、事务、GMP、存储主路径 | SQLLogicTest selected corpus 未完成 |
| L2 E2E/wire/SQLLogicTest smoke | 客户端和协议路径 | wire ignored/fail/deferred 需要清零或风险接受 |
| L3 per-crate coverage | 分模块覆盖率治理 | mysql-server、parser、vector、gmp 等仍需补齐 |
| L4 性能测试 | TPC-H/Sysbench/bulk-load/RAG/vector | 不应混入 coverage PASS；需要独立 artifact |
| L5 SOAK/crash/recovery | 生产可恢复性 | v3.12 mixed SOAK 和 crash/restore 未最终闭环 |

覆盖率基线显示，16 个核心/生产相关 crate 中 9 个已达 80% line coverage，7 个未达 80%。特别是 mysql-server、parser、vector、gmp 需要优先治理，因为它们直接影响 MySQL 兼容、SQL 语义、GMP 检索和向量能力。

## 6. 用于 GMP 生产环境是否可行

### 6.1 当前结论

**可以进入受控试点，不建议直接作为无回滚的 GMP 生产主库。**

允许的部署边界：

| 场景 | 结论 |
|---|---|
| GMP 文档导入和内审检索试点 | 可行，前提是数据可重建、结果可人工复核 |
| GMP RAG evidence bundle 生成 | 可行，前提是保留 citation、source path、chunk hash、evidence hash |
| GMP 审计链和权限验证 | 可进入试点，但必须补 ACL 正反例、tamper、backup/restore 后一致性 |
| GMP 生产主库 | 暂不建议，直到 crash/recovery、backup/restore、upgrade、mixed SOAK、权限矩阵全部 PASS |
| GMP 通用向量数据库/图数据库替代 | 不可宣称；v3.12 仅是内部受控 vector retrieval 和 SQL-backed graph projection |

### 6.2 GMP 生产前必须关闭的清单

| 必须项 | 关闭条件 |
|---|---|
| GMP corpus full ingestion | 对 `~/gmp-platform/gmp-md` 生成行数、chunk hash、version manifest |
| Retrieval quality | 固定 GMP 内审问题集 top-k、citation、answer envelope 可复跑 |
| ACL / RLS / column permissions | import/search/export/review/backup/restore 正反例 |
| Audit chain | tamper tests fail closed；备份恢复后 hash-chain 不断裂 |
| Vector index rebuild | empty-index、dimension mismatch、embedding version mismatch fail-closed |
| Backup/restore | documents、embeddings、graph、audit rows 恢复后 checksum 一致 |
| Mixed SOAK | 168h SQL + GMP ingest + retrieval + audit + backup/restore，0 unexplained error |
| Crash recovery | kill -9/WAL replay 后 count/hash 一致 |

## 7. 用于 MySQL 数据库教学场景还需要哪些增强

### 7.1 当前教学价值

v3.12 很适合用于数据库系统课程中的“可阅读、可改造、可验证”教学：

| 教学主题 | 当前适配度 |
|---|---|
| SQL parser/AST | 高 |
| 查询执行器和表达式求值 | 高 |
| 存储引擎、B+Tree、索引、WAL | 中高 |
| 事务、MVCC、recovery | 中，需要补 crash/rollback 实验 |
| 优化器、CBO、join 算法 | 中高 |
| MySQL wire protocol | 中，适合讲协议子集和兼容边界 |
| governance / anti-fabrication / gate | 高，非常适合 AI 协作工程教学 |

### 7.2 教学场景必须补的能力

| 优先级 | 增强项 | 目的 |
|---|---|---|
| P0 | 完整 SQL 教学 corpus | 用 SQLite/PostgreSQL/MySQL oracle 比对 SELECT、JOIN、GROUP、NULL、ORDER、LIMIT、DDL、DML |
| P0 | information_schema / SHOW CREATE / DESCRIBE | 支撑学生用 MySQL 客户端和 ORM 工具观察 schema |
| P0 | transaction lab | BEGIN/COMMIT/ROLLBACK/SAVEPOINT、隔离级别、死锁/冲突演示 |
| P0 | crash recovery lab | kill -9、WAL replay、checkpoint、backup/restore 可复现实验 |
| P1 | stored procedure / trigger lab | CREATE/DROP/SHOW PROCEDURE、CALL、IN 参数、BEFORE/AFTER、NEW/OLD |
| P1 | prepared statement / binary protocol lab | 展示 COM_STMT_PREPARE/EXECUTE 和 text/binary protocol 差异 |
| P1 | optimizer explain lab | EXPLAIN、统计信息、histogram、hash join、semi/anti join |
| P1 | SQLLogicTest integration | 让学生理解数据库兼容测试和 oracle 差异 |
| P2 | benchmark lab | TPC-H/Sysbench/bulk-load 的正确运行方式，避免把局部性能误宣传为生产能力 |

## 8. 4.0.0 前必须增强和整改的类似功能

除存储过程和触发器外，下列“有代码/有测试/有文档，但未形成生产闭环”的功能应在 4.0.0 前完成整改或明确降级：

这些任务已经拆分为 V312-56 总控和子任务 issue：[#4250](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4250) 负责总控，[#4251](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4251)-[#4254](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4254) 是 Beta 准入前 blocker，[#4255](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4255)-[#4258](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4258) 是 Beta 阶段收口任务。

| 功能 | 当前风险 | 4.0 前要求 |
|---|---|---|
| information_schema / metadata | 有 crate 和局部 SHOW，但 SQL/wire 生态不完整 | 完成 information_schema.tables/columns/indexes/processlist 基础查询 |
| SHOW CREATE TABLE / SHOW INDEX / SHOW COLUMNS | 局部实现或 placeholder 并存 | 对 MySQL 客户端可用，输出与真实 schema 一致 |
| ALTER TABLE 完整族 | ADD/DROP/MODIFY/RENAME 可用，CHANGE/constraints/index/partition 仍有 gap | v3.12 至少完成生产子集；复杂语法可明确 defer |
| Window functions | parser/executor 局部存在，主 SQL/wire path 不完整 | 完成 ROW_NUMBER/RANK/DENSE_RANK/PARTITION/ORDER 的受控子集 |
| JSON | 函数存在，JSON 类型、路径语义、索引和错误语义未闭环 | 完成 GMP 所需 JSON 文档字段和基础函数 |
| GIS | ST_* 函数存在，空间索引和 MySQL GIS 兼容不足 | 作为受控函数能力，或明确不做通用 GIS |
| REPLACE/UPSERT | 基础测试存在，复杂唯一键/事务/trigger 交互需验证 | 与唯一索引、trigger、WAL 一起门禁 |
| VIEW/CTE | CTE 可用，VIEW 更像 definition storage | 完成 view expansion、权限和只读语义 |
| MERGE | 主 `execute()` 返回 unsupported | 不应宣传；若保留，必须接入主执行路径和测试 |
| Partition / FullText | 存储层结构存在，SQL surface 不完整 | 若不作为 4.0 核心，必须写为 DEFERRED；若用于 GMP 检索，FullText 可做受控关键词检索 |

## 9. 当前风险分级

| 风险 | 严重度 | 处置 |
|---|---:|---|
| 把 v3.12 宣称为完整 MySQL 5.7 替代 | P0 | 禁止，直到 SQLLogicTest/TPC-H/wire/LOAD DATA/recovery/upgrade 全部闭环 |
| 把 GMP 内部 vector retrieval 宣称为通用向量数据库 | P0 | 禁止，v4.0.0 目标 |
| PARTIAL 功能进入 GA release note | P0 | 必须绑定 #4220/#4221-#4227/V312-55 或降级 |
| SQLLogicTest 只 smoke PASS 却宣传 full corpus | P0 | 禁止；16/16 或 selected scope table 才能升级 |
| MySQL compat 文档与源码事实不一致 | P1 | 需要整改 `MYSQL_COMPAT_STATUS.md` 和 gate fixture |
| coverage 数字脱离 test health | P1 | 使用 per-crate 口径，记录 failed/ignored/report-only |
| 性能局部结果外推生产 SLA | P1 | 只声明已实测 workload |

## 10. 进入 Beta/RC/GA 的建议判据

### Beta 前

1. #4220 PARTIAL 总控中每个 README PARTIAL 都有 issue、owner、expiry、关闭边界。
2. SQLLogicTest smoke gate 保持 PASS，16 文件缺口有 approved scope table。
3. GMP schema/ingestion/retrieval/graph/audit 五条主线可复跑。
4. coverage baseline 当前 HEAD 重采集，低覆盖和 report-only failure 均有 issue。
5. MySQL compat 文档与源码/test 事实一致，不再把已实现功能写成 not implemented。
6. V312-56A~56D（#4251-#4254）完成或显式降级，并由 Beta gate 检查；不得只创建 issue。

### RC 前

1. TPC-H SF=1 row-count/SHA256 无 unexplained mismatch。
2. MySQL wire、prepared statement、LOAD DATA、TLS/compression 边界清楚。
3. crash recovery、backup/restore、upgrade/downgrade 当前 HEAD PASS。
4. V312-55 procedure/trigger gate 退出 0，或 README 明确降级且不作 MySQL-style 基础兼容声明。
5. GMP full corpus ingestion、retrieval quality、ACL/audit-chain、vector rebuild 有完整 evidence。

### GA 前

1. 168h mixed SOAK 完成并归档日志/hash。
2. 所有 P0 blocker closed，或有用户批准的风险接受。
3. release note 中无未解释的 PARTIAL/PENDING/TBD。
4. 生产声明只覆盖已实测 workload。
5. GA_GATE_REPORT 记录 commit、命令、exit code、输出摘要、evidence hash。

## 11. 最终判断

**v3.12.0 是一个方向正确、治理约束更强、面向 GMP 场景更清晰的 ALPHA 版本。它已经超过“玩具数据库”阶段，适合继续作为 GMP-Platform 的受控试点数据库和数据库教学平台原型。**

但截至本报告日期，它还不是完整 MySQL 5.7 替代品，也不是可无条件投入 GMP 生产的数据库。v3.12 若要成为“功能比较完备的初始生产版本”，必须把 PARTIAL 功能从文档状态推进到可执行门禁状态：能完成的在 v3.12 做成 `DONE / 受控`，不能完成的明确 `DEFERRED` 或 `UNSUPPORTED`，并且不得进入生产宣传。
