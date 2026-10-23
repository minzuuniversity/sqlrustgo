# v4.1.0 — FEATURE_CHECKLIST

> **基线**：`develop/v4.1.0` @ `2406559ac09ecf821566d11bf4a3cc1ce4f4dbb8`
> **provenance:** generated_at=2026-10-23, branch=docs/ga-rc-artifacts-batch32,
> commit=`2406559ac09ecf821566d11bf4a3cc1ce4f4dbb8`, machine=Mac-mini-7.local (arm64, macOS 27.0.1),
> identity=openclaw-mcode, source_repo=openclaw/sqlrustgo,
> policy=Anti-Fabrication-Policy-v1.0
> **gate_policy_eval_id**：`v410-feature-checklist-001`
>
> **阶段**：`ALPHA`（`docs/releases/v4.1.0/STAGE.yaml` → `current_stage: "ALPHA"`）
> **本文件是**：`docs/governance/STAGE_CONFIG.yaml` → `stages.BETA.required_files`
> 与 `stages.BETA.doc_artifacts` 要求的产物。本文件**补齐该产物位**，不推进阶段。
> **生成方式**：由 `openclaw-mcode` 自动生成；晋级审批由人类架构师执行。

---

## 0. 这份文件是什么，不是什么

**是什么**：v4.1.0 的功能项**清单与状态索引** —— 每一项功能都指向仓库里
**真实存在**的 crate / 源文件 / 测试文件 / 门禁脚本 / 证据文件，并给出该功能在
v4.1.0 当前树里的状态。

**不是什么**：

1. **不是** GA 声明。v4.1.0 当前 `current_stage: "ALPHA"`，
   `STAGE.yaml` 的 `current_stage must be GA` 与 `CHANGELOG.md` 的
   `must contain the GA entry` 两项 `required_file_expectations` **仍未满足**；
   GA 段的 `POST_GA_PLAN.md` 也仍缺（`scripts/gate/check_release_artifacts.sh`
   实测见 §8.1）。
2. **不是**门禁通过证明。§2 总表里的「实现位置」列是**代码位置**，不是门禁判定；
   「对应门禁」列是**该功能在阶段框架里由哪个门禁覆盖**，不是该门禁已 PASS。
   本轮**实跑**的门禁只有 3 个（§8.1），其余一律 `NOT RUN`（§8.2）。
3. **不是**旧版本清单的搬运。v3.10.0 的 `FEATURE_CHECKLIST.md` 里有一批功能名
   在本仓库的 crate 里**找不到对应实现**（例如 `sqlrustgo-agentsql`、
   `sqlrustgo-qmd-bridge`、`sqlrustgo-unified-query`、`sqlrustgo-unified-storage`
   这 4 个 crate 目录在本仓库**不存在**）。本文件只写本仓库**当前树上能查证**的功能项。
4. **不代替签署**。§9 的签署栏留空。

**怎么复核**：每一行的「实现位置」都是仓库相对路径，可用 `ls` / `grep` 直接验证；
「证据」列指向 `docs/releases/v4.1.0/evidence/` 下**已存在**的文件或仓库内测试文件。
凡本文件写「未实现 / unknown / NOT RUN」的，都附了**查证过的路径清单**。

---

## 1. 状态图例

| 状态 | 含义 | 判定依据 |
|---|---|---|
| ✅ **Implemented** | 实现存在于所引路径，且仓库内有测试文件或证据文档可指 | 路径存在 + 测试/证据文件存在（**不等于**本轮实跑通过） |
| ⚠️ **Partial** | 部分实现 / 部分通过，仓库文档已明确记录未完成项 | 引用文档明写 partial / 部分 |
| 🚫 **Deferred** | 明确推迟到 v4.1.0 或更晚版本 | `ISSUES_PLAN.md` / `ROADMAP.md` / `FEATURE_MATRIX.md` 明写 DEFERRED |
| 🔴 **Known Gap** | 已登记的缺口，有可复现证据 | `STAGE.yaml` → `known_gaps` / `open_issues` / `DEFECTS_AND_ISSUES.md` |
| ❌ **Not Implemented** | 查证过路径后确认不存在 | 列出查过的路径 |

> **口径说明**：本文件的「Implemented」是**代码存在性 + 可指测试/证据**的判断，
> **不是**「该功能已通过 GA 级验证」。仓库自己的纪律是：**绿色门禁 ≠ GA 级声明**
> （`FEATURE_MATRIX.md` §3.13：`CHANGELOG.md` 里 8h SOAK 的证据地位已被 IAIA-410
> 审计下调）。凡本文件未实跑的门禁，写 `NOT RUN`。

---

## 2. 功能总表（v4.1.0 核心目标）

| ID | 功能项 | 状态 | 实现位置 | 对应门禁 | 证据 / 测试 |
|---|---|---|---|---|---|
| F-01 | SQL 解析（含块注释 `/* */`） | ✅ Implemented | `crates/parser/src/lexer.rs:78`（`skip_whitespace` 的 `/* ... */` 分支）、`crates/parser/src/parser.rs`、`crates/parser/src/token.rs` | `check_arch_invariants.sh` | `crates/parser/tests/wp_a_legacy.rs:73`、`:353-364`（`test_4708_block_comments` 回归见证）；`evidence/BATCH28_SEM4_AND_GATE_INTEGRITY_2026-10-23.md` §7 |
| F-02 | 物理计划构建（Planner） | ✅ Implemented | `crates/planner/src/lib.rs`、`crates/planner/src/physical_plan.rs` | `check_int3_single_expr.sh`（G3） | `crates/planner/tests/`；`ARCHITECTURE_ROUTE_DECISION_5193.md` |
| F-03 | 成本优化器（CBO） | ✅ Implemented | `crates/optimizer/src/lib.rs` | `check_p33_cost_optimizer.sh` | `crates/optimizer/tests/`；`tests/integration/optimizer_stats_test.rs` |
| F-04 | DDL 执行（CREATE/ALTER/DROP） | ⚠️ Partial | `crates/executor/src/ddl_executor.rs`、`crates/catalog/src/lib.rs` | `check_g16_ddl_syntax.sh` | WP-C 6 条 `NOT STARTED`（§4）；`ISSUES_PLAN.md` §4.1 |
| F-05 | DML 执行（INSERT/UPDATE/DELETE） | ✅ Implemented | `crates/executor/src/mutation_compiler.rs`、`crates/executor/src/update_compiler.rs`、`crates/executor/src/local_executor_dml.rs` | `check_v312_dml_integrity.sh` | `crates/executor/tests/`；`tests/integration/dml/` |
| F-06 | 主键点查快路径 | ✅ Implemented | `crates/storage/src/wal_storage.rs`（`scan_pk` 转发）、`crates/storage/src/file_storage.rs` | `check_storage_index_wal.sh` | `crates/storage/tests/wal_scan_pk_forwarding_5168.rs`；`evidence/SOAK_8H_AND_CAPACITY_2026-10-09.md` §4.4 |
| F-07 | UPDATE/DELETE `WHERE` 快路径 | ✅ Implemented | `src/engine_dml.rs:919`（`pk_point_lookup`，根包 `sqlrustgo`）、`crates/storage/src/file_storage.rs` | `check_storage_index_wal.sh` | `tests/integration/dml/dml_pk_fast_path_5177.rs` |
| F-08 | MVCC 事务 / 行版本链 | ✅ Implemented | `crates/transaction/src/mvcc.rs`、`crates/transaction/src/version_chain.rs`、`crates/storage/src/mvcc.rs` | `check_sem1_savepoint.sh`（G5） | `crates/transaction/tests/`；`tests/mvcc_reader_tx_5105.rs` |
| F-09 | SAVEPOINT / ROLLBACK TO SAVEPOINT | ✅ Implemented | `crates/transaction/src/savepoint.rs` | `check_sem1_savepoint.sh`（G5） | `crates/transaction/tests/` |
| F-10 | 死锁检测 | ✅ Implemented | `crates/transaction/src/deadlock.rs` | `check_sem1_savepoint.sh`（G5） | `crates/transaction/tests/` |
| F-11 | 序列化隔离（SSI） | ✅ Implemented | `crates/transaction/src/ssi.rs` | `check_sem1_savepoint.sh`（G5） | `crates/transaction/tests/` |
| F-12 | 分布式事务协调（DTC） | ✅ Implemented | `crates/transaction/src/dtc.rs`、`crates/transaction/src/coordinator.rs` | `check_backup_restore.sh`（G6） | `crates/transaction/tests/` |
| F-13 | 值类型系统 / 表达式求值 | ✅ Implemented | `crates/types/src/value.rs`、`crates/executor/src/expr/` | `check_int3_single_expr.sh`（G3） | `crates/types/tests/`；`crates/executor/tests/` |
| F-14 | MySQL 5.7 线协议（COM_QUERY / PREPARE / EXECUTE / CLOSE / PING / QUIT / INIT_DB / RESET_CONNECTION） | ✅ Implemented | `crates/mysql-server/src/lib.rs:980-988` | `check_v312_21_mysql_compat.sh` | `crates/mysql-server/tests/`；`tests/integration/mysql_server_e2e_test.rs` |
| F-15 | `mysql_native_password` 认证 | ✅ Implemented | `crates/mysql-server/src/lib.rs:970`、`:1152`、`:1177` | `check_v312_21_mysql_compat.sh` | `crates/mysql-server/src/lib.rs:1265-1304`（单元测试） |
| F-16 | TLS 传输 | ✅ Implemented | `crates/mysql-server/src/lib.rs:2095-2164`（`TlsStream`，基于 `rustls`） | `check_v312_21_mysql_compat.sh` | `evidence/TLS_READ_SPIN_HANG_5099_2026-10-09.md` |
| F-17 | LOAD DATA LOCAL INFILE | ✅ Implemented | `crates/mysql-server/src/load_data.rs:18`（`parse_tbl_line`）、`:79`（`bulk_insert`） | `check_load_data_infile.sh` | `crates/mysql-server/src/load_data.rs:170-269`（单元测试）；`evidence/LOCAL_INFILE_CLIENT_GAP_2026-10-04.txt` |
| F-18 | 磁盘持久化存储（FileStorage + WAL） | ✅ Implemented | `crates/storage/src/file_storage.rs`、`crates/storage/src/wal_storage.rs` | `check_storage_index_wal.sh` | `crates/storage/tests/`；`evidence/WAL_RECOVERY_SCALE_P0_08_2026-10-23.md` |
| F-19 | WAL 崩溃恢复（去重索引） | ✅ Implemented | `crates/storage/src/recovery_engine.rs`（`RecoveryRowIndex`） | `check_p12_crash_test.sh`（G8） | `crates/storage/tests/wal_recovery_dedup_p0_08.rs`（5/5 PASS） |
| F-20 | B+Tree 索引 | ✅ Implemented | `crates/storage/src/bplus_tree/`、`crates/storage/src/index_registry.rs` | `check_storage_index_wal.sh` | `crates/storage/tests/`；`tests/integration/page_test.rs` |
| F-21 | Buffer Pool | ✅ Implemented | `crates/storage/src/buffer_pool.rs` | `check_storage_index_wal.sh` | `crates/storage/tests/`；commit `1bc78df7b8`（2026-10-15） |
| F-22 | 自适应哈希索引（AHI） | ✅ Implemented | `crates/storage/src/adaptive_hash_index.rs` | `check_p34_parallel_executor.sh` | `tests/integration/storage/adaptive_hash_main_path_test.rs`、`tests/integration/sql/adaptive_hash_index_test.rs`、`tests/integration/sql/explain_ahi_promoted_test.rs` |
| F-23 | 聚簇表 | ✅ Implemented | `crates/storage/src/clustered_table.rs` | `check_storage_index_wal.sh` | `tests/clustered_wal_bypass_r1_5228.rs`；commit `f03c81d047`（2026-10-16，#5228/#5233/#5234） |
| F-24 | Change Buffer | ✅ Implemented | `crates/storage/src/change_buffer.rs` | `check_storage_index_wal.sh` | `tests/change_buffer_main_path_test.rs` |
| F-25 | Double-Write Buffer | ✅ Implemented | `crates/storage/src/double_write_buffer.rs` | `check_storage_index_wal.sh` | `tests/double_write_main_path_test.rs` |
| F-26 | Checkpoint | ✅ Implemented | `crates/storage/src/checkpoint.rs` | `check_storage_index_wal.sh` | `crates/storage/tests/` |
| F-27 | 半同步复制（Binlog） | ⚠️ Partial | `crates/storage/src/binlog_server.rs`、`crates/storage/src/binlog_client.rs`、`crates/storage/src/binlog_protocol.rs` | `check_backup_restore.sh`（G6） | ⚠️ **SSOT 与代码有出入**：`STAGE.yaml` 记「`binlog_server.rs:238` 丢弃 `HeartbeatAck`」，但本轮实测当前树 `:388` 起**已记录**该 ACK（写入 `acked_lsn` 并回 `AckOk`），`:421` 起的 `BinlogAck`（半同步等待对象）走 `acked_pos`（#4937）。⇒ STAGE.yaml 的行号引用指向**更早的树状态**，需属主裁决是否订正（见 §5.4） |
| F-28 | 备份 / 恢复 | ✅ Implemented | `crates/storage/src/backup.rs`、`crates/storage/src/backup_coordinator.rs`、`crates/tools/src/backup.rs`、`crates/tools/src/backup_restore.rs` | `check_backup_restore.sh`（G6） | `evidence/BACKUP_DATASOURCE_AND_TYPE_4938_2026-10-06.md` |
| F-29 | 并行执行器（intra-query） | ✅ Implemented | `crates/executor/src/parallel_executor.rs` | `check_int2_no_orphan.sh`（G2） | `tests/integration/int2/` |
| F-30 | 并行 GROUP BY / Hash Join | ✅ Implemented | `crates/executor/src/parallel_group_by.rs`、`crates/executor/src/parallel_hash_join.rs` | `check_p34_parallel_executor.sh` | `crates/executor/tests/` |
| F-31 | SIMD 批量求值 | ✅ Implemented | `crates/executor/src/simd_eval.rs`、`crates/executor/src/vec_simd.rs` | `check_p35_simd.sh` | `crates/executor/tests/` |
| F-32 | 向量化 / Pipeline 执行 | ✅ Implemented | `crates/executor/src/vector_executor.rs`、`crates/executor/src/pipeline_executor.rs`、`crates/executor/src/record_batch.rs` | `check_p34_parallel_executor.sh` | `crates/executor/tests/` |
| F-33 | 窗口函数 | ✅ Implemented | `crates/executor/src/window_executor.rs` | `check_g_correctness_v390.sh` | `crates/executor/tests/` |
| F-34 | 触发器 | ✅ Implemented | `crates/executor/src/trigger.rs`、`crates/executor/src/trigger_eval/` | `check_v312_procedure_trigger_gate.sh` | commit `144e5df982`（2026-10-15，#5349） |
| F-35 | 存储过程 | ✅ Implemented | `crates/executor/src/stored_proc.rs` | `check_v312_procedure_trigger_gate.sh` | commit `bf2bed26d3`（2026-10-15，#5346 SHOW PROCEDURE STATUS） |
| F-36 | EXPLAIN 输出 | ✅ Implemented | `crates/executor/src/explain.rs` | `check_plan_integrity_v310.sh` | commit `1ce91dd9e8` / `843c2c314a`（2026-10-15，EXPLAIN `est_rows`/`est_cost`/`pages_visited`） |
| F-37 | 查询缓存 | ✅ Implemented | `crates/executor/src/query_cache.rs`、`crates/executor/src/query_cache_config.rs` | `check_p33_cost_optimizer.sh` | `crates/executor/tests/` |
| F-38 | 索引扫描执行 | ✅ Implemented | `crates/executor/src/index_scan.rs` | `check_storage_index_wal.sh` | `crates/executor/tests/` |
| F-39 | Prometheus 指标 | ✅ Implemented | `crates/telemetry/src/prometheus.rs`、`crates/mysql-server/src/metrics_endpoint.rs`、`crates/server/src/metrics_endpoint.rs`、`crates/common/src/metrics_aggregator.rs` | `check_prometheus_metrics.sh`、`check_4021_prometheus_metrics.sh` | `tests/unit/prometheus_test.rs`、`tests/e2e/monitoring_test.rs` |
| F-40 | 慢查询日志 | ✅ Implemented | `crates/query-stats/src/slow_query_log.rs` | `check_4022_slow_query_log.sh` | `crates/mysql-server/tests/slow_query_log.rs` |
| F-41 | INFORMATION_SCHEMA | ✅ Implemented | `crates/information-schema/src/lib.rs`、`src/system_tables.rs` | `check_v312_21_mysql_compat.sh` | `tests/integration/sql/information_schema_test.rs` |
| F-42 | 审计日志 / 不可变哈希链 | ✅ Implemented | `crates/gmp/src/audit.rs`（GMP 审计，tamper-evident SHA-256 链）、`crates/security/src/audit.rs`（通用审计日志）、`crates/security/src/audit_chain.rs`（V400-07 ALCOA+ 审计链，9 项属性）、`crates/executor/src/sql_log.rs` | `check_p21_audit_log.sh`、`check_p23_hash_chain.sh`（G10） | `tests/integration/stress/audit_log_test.rs`、`tests/integration/sql/hash_chain_test.rs` |
| F-43 | 时间旅行查询 | ✅ Implemented | `crates/transaction/src/mvcc.rs`、`crates/transaction/src/version_chain.rs` | `check_p22_time_travel.sh`（G10） | `tests/integration/sql/time_travel_test.rs` |
| F-44 | 防火墙 / 会话管理 / 加密 | ✅ Implemented | `crates/security/src/firewall.rs`、`crates/security/src/session.rs`、`crates/security/src/encryption.rs`、`crates/security/src/tls.rs` | `check_security.sh`、`check_attack_surface.sh` | `crates/security/src/firewall_tests.rs` |
| F-45 | 逻辑备份 / 物理备份 / mysqldump | ✅ Implemented | `crates/tools/src/mysqldump.rs`、`crates/tools/src/physical_backup.rs`、`crates/tools/src/backup.rs` | `check_backup_restore.sh`（G6） | `crates/tools/tests/` |
| F-46 | 升级工具链 | ✅ Implemented | `crates/tools/src/upgrade.rs` | `check_p14_upgrade_test.sh`（G9） | `crates/tools/tests/` |
| F-47 | 配置热加载 / 日志轮转 | ✅ Implemented | `crates/tools/src/config_hot_reload.rs`、`crates/tools/src/log_rotation.rs` | `check_no_log_tbl_json.sh` | `crates/tools/tests/` |
| F-48 | 5 远端同步与漂移检测 | ✅ Implemented | `scripts/sync/5remotes_sync.sh`、`scripts/sync/5remotes_drift_check.sh`、`scripts/sync/README.md` | `check_main_freshness.sh`、`check_drift_not_pass.sh` | `evidence/SYNC_TOOLS_STRICT_5520_2026-10-23.md`、`evidence/SYNC_SCRIPT_5380_2026-10-20.md` |
| F-49 | SQL 语料回归门禁 | ✅ Implemented | `crates/sql-corpus/src/lib.rs` | `check_sql_corpus_gate.sh` | `scripts/gate/corpus_manifest.yaml` |
| F-50 | TPC-H 基线比对 | ✅ Implemented（工具侧） | `scripts/gate/check_g1_tpch_baseline.sh`、`scripts/gate/tpch_baseline_hash.py` | `check_g1_tpch_baseline.sh`（G1） | `evidence/TPCH_FIXTURE_5379_2026-10-20.md`；**但 SF=1 判定 not recorded（§5.2）** |

> **表读法**：「实现位置」列全部是本仓库**当前树上存在**的路径，可用 `ls` 直接验证。
> 「对应门禁」列是**阶段框架里覆盖该功能的门禁脚本**（`scripts/gate/*.sh`，全部存在），
> **不代表该门禁本轮已跑过**。本轮实跑情况见 §8。

---

## 3. 按能力域分节

### 3.1 DDL

| 功能项 | 状态 | 实现位置 | 备注 |
|---|---|---|---|
| CREATE TABLE / DROP TABLE | ✅ Implemented | `crates/executor/src/ddl_executor.rs`、`crates/catalog/src/lib.rs` | — |
| ALTER TABLE ADD / DROP COLUMN | ✅ Implemented | `crates/executor/src/ddl_executor.rs` | — |
| ALTER TABLE RENAME COLUMN | ✅ Implemented | `src/engine_ddl.rs:1361`（`AlterTableOperation::RenameColumn`）、`crates/parser/src/parser.rs:470`、`crates/storage/src/file_storage.rs:7222` | WP-F / #4848。⚠️ `ISSUES_PLAN.md` §4.3 仍记 `NOT STARTED` —— **与当前代码有出入**（§5.4） |
| `sqlite_master` / `sqlite_sequence` / `sqlite_temp_master` | ✅ Implemented | `src/engine_select.rs:520`（合成 `sqlite_master`）、`:642`（`query_sqlite_sequence`，注释标 #4682）、`src/system_tables.rs:62`、`:229` | WP-C / #4682。⚠️ `ISSUES_PLAN.md` §4.1 仍记 `NOT STARTED` —— **与当前代码有出入**（§5.4） |
| CREATE PROCEDURE 的 DDL 落地 | ✅ Implemented | `src/execution_engine_methods.rs:1289`（`execute_create_procedure`）→ `crates/catalog/src/catalog.rs:234`（`add_stored_procedure`）；类型定义 `crates/catalog/src/stored_proc.rs:7` | WP-C / #4652。⚠️ `ISSUES_PLAN.md` §4.1 仍记「接受但不存储 / `NOT STARTED`」—— **与当前代码有出入**（§5.4）。⚠️ SQLite CLI 模式另走 **OR-downgrade**（`crates/sqlrustgo-cli/src/sqlite_mode.rs:325`） |
| CREATE FUNCTION 的 DDL 落地 | ❌ **Not Implemented** | 查证路径：`src/execution_engine_methods.rs:1449`（`execute_create_function`）、`crates/catalog/src/catalog.rs`（**无** `add_*function` 写入 API）、`src/execution_engine_methods.rs` 的 `execute_drop_function` 报 "function '...' not found" | WP-C / #4652 的**函数**侧仍成立 |
| SQLite AUTOINCREMENT 语义 | ⚠️ Partial | `src/engine_select.rs:642`（按有 AUTOINCREMENT 列的表合成 `sqlite_sequence`）、`src/engine_dml.rs:505`（自增列选取按 MySQL InnoDB 约定：单列） | WP-C / #4672。**查证结论**：生产代码中**没有** `#4672` 的实现标记（`grep "#4672"` 在 `src/` 与 `crates/` 的非测试代码中零命中）；`sqlite_sequence` 是通用合成而非该 issue 的专门修复 ⇒ 记 Partial，**不记 Implemented** |
| 复杂 DROP INDEX / function index / partial index | ⚠️ Partial | `src/execution_engine_methods.rs:926`（DROP INDEX 已接入存储层，标注 #4669）；**function index / partial index**：查证路径 `crates/storage/src/index_registry.rs`、`crates/parser/src/parser.rs` —— `PartialIndex` / `function_index` 零命中 | WP-C / #4669 的**索引形态**部分仍缺 |
| CHECK 多条件约束 | ✅ Implemented | `crates/storage/src/engine.rs:60`（`evaluate_check_constraint` → `evaluate_check_constraint_ast`，按 AST 求值，AND/OR 走 `Expression` 分支）、`src/engine_dml.rs:727-729`（INSERT 时逐条校验） | WP-C / #4709。⚠️ `ISSUES_PLAN.md` §4.1 的「静默接受非法 row」与当前代码**不符**（约束**已被强制**）—— §5.4 登记该出入 |
| UPSERT（`INSERT ... ON CONFLICT`） | ⚠️ Partial | `src/engine_dml.rs:648`（SQLite/Postgres `ON CONFLICT DO NOTHING`，标注 #4642） | SQLite CLI 模式另走 **OR-downgrade**（`crates/sqlrustgo-cli/src/sqlite_mode.rs:275`，标注 #4703） |
| trigger-column 语法 | ❌ **Not Implemented** | 查证路径：`crates/parser/src/parser.rs`、`crates/parser/src/token.rs`、`crates/executor/src/trigger.rs` —— `TriggerColumn` / `trigger_column` 在 parser 中**零命中**（运行期的 `SET NEW.col` 由 commit `144e5df982` / #5349 覆盖，但**语法层**未见对应节点） | WP-C / #4703 的 trigger-column 侧仍缺 |
| ALTER COLUMN DEFAULT | ✅ Implemented | `crates/executor/src/ddl_executor.rs` | `evidence/ALTER_COLUMN_DEFAULT_BATCH13_2026-10-22.md` |

### 3.2 DML

| 功能项 | 状态 | 实现位置 | 备注 |
|---|---|---|---|
| INSERT（单行 / 多行 / INSERT IGNORE） | ✅ Implemented | `crates/executor/src/mutation_compiler.rs` | `crates/executor/tests/` |
| INSERT ... SELECT / CTAS | ✅ Implemented | `crates/executor/src/mutation_compiler.rs`、`crates/executor/src/ast_adapter.rs` | commit `7db4b96f46`（2026-10-14，#5268 CTAS `SELECT *` 限定名） |
| UPDATE ... SET | ✅ Implemented | `crates/executor/src/update_compiler.rs` | `tests/multi_table_update_5117.rs`；`evidence/MULTI_TABLE_UPDATE_5117_2026-10-08.md` |
| DELETE（含无 WHERE 全表删除） | ✅ Implemented | `crates/executor/src/mutation_compiler.rs` | commit `593fb8775d`（2026-10-13，#5285 debug_assert panic） |
| 多表 UPDATE / 多表 DELETE | ✅ Implemented | `crates/executor/src/update_compiler.rs`、`crates/executor/src/local_executor_dml.rs` | `evidence/MULTI_TABLE_DML_DB_5106_2026-10-10.md` |
| 二级索引维护（写入路径） | ✅ Implemented | `crates/storage/src/index_registry.rs` | commit `1cb784dd5f`（2026-10-13，#5303 静默丢行） |
| 行身份（RowId）保真 | ✅ Implemented | `crates/storage/src/mvcc.rs`、`crates/transaction/src/version_chain.rs` | commit `48f62d1906`（2026-10-14，#5281） |
| `NATURAL JOIN` / 多列 `USING` | ✅ Implemented | `src/engine_select.rs:5202`、`:5360`（标注 V313-109 / #4668）、`crates/parser/src/parser.rs:816` | WP-D / #4668。⚠️ `ISSUES_PLAN.md` §4.2 仍记 `NOT STARTED` —— **与当前代码有出入**（§5.4） |
| `> ALL` / `= ANY` 子查询 | ⚠️ Partial | AST 有 `Expression::QuantifiedOp`（`src/execution_engine.rs:1397`、`src/engine_cte.rs:207`）；求值走 `src/engine_select.rs:1921` 的 `pre_evaluate_quantified_subquery`（#4687 Step 1.6，覆盖**非相关**情形）。**相关**情形走保守 fallback —— `src/engine_select.rs:1833` 明写 #4656 的专属 plumbing **已被回退**（会遮蔽相关子查询分支） | WP-D / #4656：**非相关**已覆盖，**相关**保守降级 ⇒ 记 Partial |
| `LEFT JOIN USING` | ✅ Implemented | `src/engine_select.rs:5158`（`USING(col1, col2, ...)` → 逐列解析为 `t1.col_i = t2.col_i`）、`:5561`（投影掉重复的右列）；标注 V312-73 / #4649 | WP-D / #4649。⚠️ `ISSUES_PLAN.md` §4.2 仍记「退化为笛卡尔积 / `NOT STARTED`」—— **与当前代码有出入**（§5.4） |
| 相关标量子查询 | ✅ Implemented | `src/engine_select.rs:750`（`SELECT (SELECT b.val FROM b WHERE b.id = a.id)`，标注 V312-75 / #4636）、`:2980` | WP-D / #4636。⚠️ `ISSUES_PLAN.md` §4.2 仍记 `NOT STARTED` —— **与当前代码有出入**（§5.4） |
| O(log N) DELETE 评估 | ⚠️ Partial（仅评估） | `crates/storage/src/file_storage.rs` | F-10：`WriteState` 到不了主键 B+Tree；「按索引定位再 `Vec::remove`」方案已记为**否决**（破坏多行删除的 ROLLBACK），见 `FEATURE_MATRIX.md` F-10 |
| `CHAR(n)` PAD SPACE 比较语义 | ✅ Implemented | `src/engine_select_pk.rs:94`（`is_pad_space_type`）、`:101`、`:112`（PAD SPACE 与 `scan_pk` 严格相等的交互，标注 #4846）、`src/engine_dml.rs:905` | WP-G / #4846。⚠️ `ISSUES_PLAN.md` §4.4 仍记 `NOT STARTED` —— **与当前代码有出入**（§5.4） |

### 3.3 事务与并发

| 功能项 | 状态 | 实现位置 | 备注 |
|---|---|---|---|
| BEGIN / COMMIT / ROLLBACK | ✅ Implemented | `crates/transaction/src/manager.rs`、`crates/transaction/src/transaction_manager.rs` | `evidence/EXPLICIT_TX_NOT_SILENTLY_COMMITTED_4847_2026-10-07.md` |
| MVCC 行版本链 | ✅ Implemented | `crates/storage/src/mvcc.rs`、`crates/transaction/src/version_chain.rs` | commit `ccf1409397`（2026-10-10，#5277 按行身份而非首列做键） |
| MVCC 垃圾回收 | ✅ Implemented | `crates/storage/src/mvcc_gc.rs` | `crates/storage/tests/` |
| SAVEPOINT / ROLLBACK TO SAVEPOINT | ✅ Implemented | `crates/transaction/src/savepoint.rs` | G5 门禁 `check_sem1_savepoint.sh` |
| 死锁检测 | ✅ Implemented | `crates/transaction/src/deadlock.rs` | `crates/transaction/tests/` |
| SSI 序列化隔离 | ✅ Implemented | `crates/transaction/src/ssi.rs` | `crates/transaction/tests/` |
| 锁管理器 | ✅ Implemented | `crates/transaction/src/lock_manager.rs`、`crates/transaction/src/lock.rs`、`crates/storage/src/lock/` | `tests/blk2_shared_tx_path_test.rs` |
| 分布式事务（DTC） | ✅ Implemented | `crates/transaction/src/dtc.rs`、`crates/transaction/src/participant.rs`、`crates/transaction/src/router.rs` | `crates/transaction/tests/` |
| 按库隔离的会话上下文 | ✅ Implemented | `crates/storage/src/file_storage.rs`（删除共享 `current_db` 状态）、`crates/transaction/src/gid.rs` | commit `0a2a186c7c`（2026-10-11，#5057）；`evidence/SESSION_DB_READPATH_5057_2026-10-07.md` |
| 并发事务丢行（#5099） | ⚠️ **Partial** | `crates/storage/src/file_storage.rs`（事务身份不再走共享槽位） | 丢行率 40.4% → ~1%，但 `ERROR 1062` 与偶发服务器卡死**仍未解决**；`evidence/CONCURRENT_TX_ROLLBACK_DATA_LOSS_5099_2026-10-08.md` |
| TLS 读路径忙循环卡死（#5099） | ✅ Implemented | `crates/mysql-server/src/lib.rs:2095-2164` | 1h SOAK 3602s / 0 FATAL；`evidence/TLS_READ_SPIN_HANG_5099_2026-10-09.md` |

### 3.4 类型与表达式

| 功能项 | 状态 | 实现位置 | 备注 |
|---|---|---|---|
| 标量值类型系统 | ✅ Implemented | `crates/types/src/value.rs` | `crates/types/tests/` |
| `Value::Hash` 与 `PartialEq` 一致性 | ✅ Implemented | `crates/types/src/value.rs` | commit `93e0d92519`（2026-10-23）：原先 `0.0` 与 `-0.0` 相等却哈希不同 |
| 表达式求值 / 函数 | ✅ Implemented | `crates/executor/src/expr/` | `crates/executor/tests/` |
| 隐式类型转换（数字列 vs 字符串字面量） | ✅ Implemented | `crates/executor/src/expr/` | commit `3765dbec33`（2026-10-13，#5300 静默丢行） |
| `GROUP BY` 无聚合时不折叠重复组 | ✅ Implemented | `crates/executor/src/expr/` | commit `6a8878c9b0`（2026-10-13，#5301） |
| WP-B 函数族完整行为（#4721 / #4674 / #4716 / #4676 / #4675 / #4670） | ⚠️ Partial | `crates/executor/src/expr/` | `ISSUES_PLAN.md` §4.5：v4.0.0 仅部分支持，完整行为顺延 |

### 3.5 MySQL Wire 协议

| 协议特性 | 状态 | 实现位置 | 备注 |
|---|---|---|---|
| COM_QUERY | ✅ Implemented | `crates/mysql-server/src/lib.rs:982` | — |
| COM_STMT_PREPARE / EXECUTE / CLOSE | ✅ Implemented | `crates/mysql-server/src/lib.rs:985-987` | `crates/mysql-server/src/lib.rs:2629` 起构造 PREPARE 响应 |
| COM_PING / COM_QUIT / COM_INIT_DB | ✅ Implemented | `crates/mysql-server/src/lib.rs:980-984` | COM_QUIT 不返回 OK 包，直接半关连接（`:2715`） |
| COM_RESET_CONNECTION | ✅ Implemented | `crates/mysql-server/src/lib.rs:988` | — |
| `mysql_native_password` 认证 | ✅ Implemented | `crates/mysql-server/src/lib.rs:970`、`:1177` | 单元测试 `:1265-1304` |
| `caching_sha2_password` 认证 | ❌ **Not Implemented** | 查证路径：`crates/mysql-server/src/lib.rs` 全文（`AUTH_PLUGIN` 常量为单一 `mysql_native_password`，无 caching_sha2 分支） | v3.10.0 清单曾把该项标为已覆盖；本树**无对应实现** |
| TLS | ✅ Implemented | `crates/mysql-server/src/lib.rs:2095-2164` | 基于 `rustls`；`crates/security/src/tls.rs` |
| LOAD DATA LOCAL INFILE | ✅ Implemented | `crates/mysql-server/src/load_data.rs:18`、`:79` | `evidence/LOCAL_INFILE_CLIENT_GAP_2026-10-04.txt` 记客户端侧缺口 |
| 结果集 schema 通道 | ✅ Implemented | `crates/mysql-server/src/result_set.rs` | commit `1cb784dd5f` / `3e3424a426` 系列（#5271 结果集按声明 schema 构造） |
| 多语句（multi-statement） | ⚠️ Partial | `crates/mysql-server/src/lib.rs:1041`（`SERVER_MORE_RESULTS_EXISTS` 状态位，注释记为 #4019/#4020/#4022 multi-query blocker 的修复） | 协议位已实现；但本轮**未**实跑多语句端到端场景，仓库内亦未见专用的多语句 E2E 产物 ⇒ 只记「实现存在 + 无端到端实测」 |
| `SHOW CREATE TABLE` | ✅ Implemented | `crates/mysql-server/src/lib.rs` | commit `37f616d956`（2026-10-14，2 列修复） |
| `SHOW PROCEDURE STATUS` | ✅ Implemented | `crates/executor/src/stored_proc.rs` | commit `bf2bed26d3`（2026-10-15，#5346） |
| `SHOW DATABASES` / `SHOW TABLES` | ✅ Implemented | `crates/information-schema/src/lib.rs` | `evidence/ISSUE_5009_VALUES_AND_SHOW_DATABASES_2026-10-05.txt` |
| 协议层走 engine schema 而非 `show_column_headers` | ✅ Implemented | `crates/mysql-server/src/lib.rs` | commit `ab6fc8af81`（2026-10-15，#5341） |

### 3.6 存储与索引

| 功能项 | 状态 | 实现位置 | 备注 |
|---|---|---|---|
| FileStorage（磁盘持久化） | ✅ Implemented | `crates/storage/src/file_storage.rs` | `crates/storage/tests/` |
| WAL 写入 / 重放 | ✅ Implemented | `crates/storage/src/wal_storage.rs` | `crates/storage/tests/` |
| WAL 恢复去重（每表一次索引） | ✅ Implemented | `crates/storage/src/recovery_engine.rs` | commit `a685661448`（2026-10-23）；5/5 语义测试 `crates/storage/tests/wal_recovery_dedup_p0_08.rs` |
| Checkpoint | ✅ Implemented | `crates/storage/src/checkpoint.rs` | — |
| B+Tree / 二级索引 | ✅ Implemented | `crates/storage/src/bplus_tree/`、`crates/storage/src/index_registry.rs` | — |
| Buffer Pool | ✅ Implemented | `crates/storage/src/buffer_pool.rs` | commit `1bc78df7b8`；`crates/storage/src/buffer_pool_metrics.rs` |
| 聚簇表 | ✅ Implemented | `crates/storage/src/clustered_table.rs` | commit `f03c81d047`（#5228/#5233/#5234） |
| Change Buffer | ✅ Implemented | `crates/storage/src/change_buffer.rs` | `tests/change_buffer_main_path_test.rs` |
| Double-Write Buffer | ✅ Implemented | `crates/storage/src/double_write_buffer.rs` | `tests/double_write_main_path_test.rs` |
| 自适应哈希索引（AHI） | ✅ Implemented | `crates/storage/src/adaptive_hash_index.rs` | commit `e81541e45c`（2026-10-14，AHI 状态机测试） |
| 故障注入框架 | ✅ Implemented | `crates/storage/src/fault_injection.rs` | commit `b4f743fdff`（2026-10-12，新增 `AfterWALAppend` / `AfterIndexUpdate` 故障点） |
| 列存（columnar） | ✅ Implemented | `crates/storage/src/columnar/` | `crates/storage/tests/` |
| 二进制存储格式 | ✅ Implemented | `crates/storage/src/binary_storage.rs`、`binary_storage_v2.rs`、`binary_format.rs`、`bin_index.rs`、`bin_segment.rs` | `tests/integration/bin_storage_*.rs` |
| 索引重建（Index Rebuild） | ❌ **Not Implemented** | 查证路径：`crates/storage/src/index_registry.rs`、`crates/storage/src/bplus_tree/` | `STAGE.yaml` `legacy_chain_audit_2026_09_30.C9_storage_index_wal` 明确登记为缺口 |
| Page Checksum 全覆盖 | ⚠️ Partial | `crates/storage/src/page.rs:179` | 同上登记：`page.rs:179` 处仍有跳过分支 |
| 索引统计自动采集 | ❌ **Not Implemented** | 查证路径：`crates/optimizer/src/lib.rs`、`crates/storage/src/index_registry.rs` | 同上登记为缺口 |
| WAL 校验工具 | ✅ Implemented | `crates/wal-verification/src/lib.rs` | 独立 crate；`STAGE.yaml` 记「WAL Verification Tool GMP path」 |
| 半同步复制（Binlog Server） | ⚠️ Partial | `crates/storage/src/binlog_server.rs` | `:388` 起 `HeartbeatAck` **已记录**；`:421` 起 `BinlogAck` 走 `acked_pos`。与 `STAGE.yaml` 的行号引用有出入，见 §3.3 / §5.4 |

### 3.7 执行引擎

| 功能项 | 状态 | 实现位置 | 备注 |
|---|---|---|---|
| 顺序执行 | ✅ Implemented | `crates/executor/src/executor.rs`、`crates/executor/src/scan.rs` | `crates/executor/tests/` |
| 并行执行器 | ✅ Implemented | `crates/executor/src/parallel_executor.rs` | G2 门禁 `check_int2_no_orphan.sh` |
| 并行 GROUP BY / Hash Join | ✅ Implemented | `crates/executor/src/parallel_group_by.rs`、`crates/executor/src/parallel_hash_join.rs` | — |
| SIMD 求值 | ✅ Implemented | `crates/executor/src/simd_eval.rs`、`crates/executor/src/vec_simd.rs` | — |
| 向量化执行 | ✅ Implemented | `crates/executor/src/vector_executor.rs`、`crates/executor/src/vector_scan.rs`、`crates/executor/src/vectorization.rs` | — |
| Pipeline 执行 | ✅ Implemented | `crates/executor/src/pipeline_executor.rs`、`crates/executor/src/pipeline_trace.rs` | — |
| 窗口函数 | ✅ Implemented | `crates/executor/src/window_executor.rs` | — |
| 算子画像 / 埋点 | ✅ Implemented | `crates/executor/src/operator_profile.rs`、`crates/executor/src/instrumentation.rs`、`crates/executor/src/executor_metrics.rs` | — |
| 取消（Cancellation） | ✅ Implemented | `crates/executor/src/cancellation_token.rs` | — |
| 计划完整性校验 | ✅ Implemented | `crates/executor/src/explain.rs`、`crates/planner/src/physical_plan.rs` | `check_plan_integrity_v310.sh` |
| `PhysicalPlan::execute()` 接线（#5193 F3） | ✅ Implemented | `crates/planner/src/physical_plan.rs`、`ARCHITECTURE_ROUTE_DECISION_5193.md` | 路线裁决 = 选项 C（PR #5207）；`FEATURE_MATRIX.md` F-12 记 CLOSED |
| 生产路径登记册（off-production-path） | ✅ Implemented | `docs/releases/v4.1.0/b2-off-production-path-registry.md`、`scripts/gate/check_off_production_path_registry.sh` | — |
| 事务性执行包装 | ✅ Implemented | `crates/executor/src/transactional_executor.rs` | — |
| SQL 日志 / 归一化 | ✅ Implemented | `crates/executor/src/sql_log.rs`、`crates/executor/src/sql_normalizer.rs` | — |

### 3.8 可观测性与运维

| 功能项 | 状态 | 实现位置 | 备注 |
|---|---|---|---|
| Prometheus 指标 | ✅ Implemented | `crates/telemetry/src/prometheus.rs`、`crates/mysql-server/src/metrics_endpoint.rs` | `check_prometheus_metrics.sh` |
| 慢查询日志 | ✅ Implemented | `crates/query-stats/src/slow_query_log.rs` | `check_4022_slow_query_log.sh` |
| 审计日志 / 哈希链 | ✅ Implemented | `crates/gmp/src/audit.rs`、`crates/security/src/audit.rs`、`crates/security/src/audit_chain.rs`、`crates/executor/src/sql_log.rs` | G10 三个门禁：audit log / time travel / hash chain（门禁脚本实测目标是 `crates/gmp/src/audit.rs` 与 `crates/executor/src/sql_log.rs`） |
| 时间旅行查询 | ✅ Implemented | `crates/transaction/src/mvcc.rs`、`crates/transaction/src/version_chain.rs` | `check_p22_time_travel.sh` 的实测目标即这两个文件 |
| 告警 | ✅ Implemented | `crates/security/src/alert.rs` | `crates/security/src/alert_tests.rs` |
| INFORMATION_SCHEMA | ✅ Implemented | `crates/information-schema/src/lib.rs` | — |
| 备份 / 恢复 / mysqldump | ✅ Implemented | `crates/tools/src/{backup.rs,backup_restore.rs,mysqldump.rs,physical_backup.rs}` | G6 门禁 |
| 高可用（HA）切换 | ✅ Implemented | `crates/tools/src/ha.rs`、`crates/storage/src/failover_manager.rs` | `crates/tools/tests/` |
| 升级工具 | ✅ Implemented | `crates/tools/src/upgrade.rs` | G9 门禁 |
| 配置热加载 / 日志轮转 | ✅ Implemented | `crates/tools/src/config_hot_reload.rs`、`crates/tools/src/log_rotation.rs` | — |
| 远程告警 / 取消 | ✅ Implemented | `crates/security/src/cancel.rs` | — |
| 测试清单诚实性治理 | ✅ Implemented | `scripts/gate/check_test_manifest_honesty.sh`、`scripts/gate/check_test_inventory.sh`、`scripts/gate/check_test_target_execution.sh` | `evidence/TEST_MANIFEST_HONESTY_5449_2026-10-21.md` 等 |
| `#[ignore]` 债务治理 | ✅ Implemented | `scripts/gate/check_anti_ignore_gate.sh`、`tests/baseline/ignore_registry.json` | 2026-10-23 实测树内重数 == 登记数；见 §8.1 |

### 3.9 扩展 crate

见 §6（隔离状态专节）。

---

## 4. Deferred / WP-C..G 项（v4.0.0 推迟到 v4.1.0）

> 来源：`docs/releases/v4.1.0/ISSUES_PLAN.md` §4.1–§4.4（权威标题取自
> `docs/releases/v4.0.0/LEGACY_ISSUES.md` §3）+ `docs/releases/v4.1.0/ROADMAP.md` §1 M-8
> + `FEATURE_MATRIX.md` P-02。
>
> ⚠️ **本表的两列必须分开读**：「`ISSUES_PLAN.md` 状态」列是**该文档 2026-09-30 的记载**；
> 「当前树实测」列是**本轮在 `2406559ac0` 上查代码得到的结果**。
> **二者有 9 条不一致**（见 §5.4）—— 本文件按**实测**记功能状态，
> `ISSUES_PLAN.md` 的原文状态原样并列，**不替属主裁决该订正哪一边**。

| WP 组 | Issue | 标题（`ISSUES_PLAN.md` §4 原文） | `ISSUES_PLAN.md` 状态 | 当前树实测（2026-10-23） | 实现位置 |
|---|---|---|---|---|---|
| WP-C（DDL/完整性，6 条） | #4682 | `sqlite_master` / `sqlite_sequence` / `sqlite_temp_master` 缺失 | `NOT STARTED` | ✅ **已实现** | `src/engine_select.rs:520`、`:642`、`src/system_tables.rs:62`、`:229` |
| | #4652 | CREATE PROCEDURE/FUNCTION 接受但不存储（DDL fake-success） | `NOT STARTED` | ⚠️ **部分已实现**（PROCEDURE 有写入路径；FUNCTION 无） | `src/execution_engine_methods.rs:1289` → `crates/catalog/src/catalog.rs:234` |
| | #4672 | SQLite AUTOINCREMENT 仍未生效 | `NOT STARTED` | ⚠️ **Partial / unknown**（`sqlite_sequence` 为通用合成；非测试代码中 `#4672` 零命中） | `src/engine_select.rs:642`、`src/engine_dml.rs:505` |
| | #4669 | 复杂 DROP INDEX / function index / partial index | `NOT STARTED` | ⚠️ **Partial**（DROP INDEX 已接线；function / partial index 零命中） | `src/execution_engine_methods.rs:926` |
| | #4709 | CHECK multi-condition 静默接受非法 row | `NOT STARTED` | ✅ **已实现**（约束在 INSERT 时被强制） | `crates/storage/src/engine.rs:60`、`src/engine_dml.rs:727-729` |
| | #4703 | UPSERT / trigger-column syntax failures | `NOT STARTED` | ⚠️ **部分已实现**（UPSERT 有；trigger-column 语法零命中） | `src/engine_dml.rs:648`；`crates/sqlrustgo-cli/src/sqlite_mode.rs:275` |
| WP-D（Join/Subquery，4 条） | #4668 | NATURAL JOIN / multi-column USING 错误 | `NOT STARTED` | ✅ **已实现** | `src/engine_select.rs:5202`、`crates/parser/src/parser.rs:816` |
| | #4656 | `> ALL` / `= ANY` 子query 错误 | `NOT STARTED` | ⚠️ **Partial**（非相关已覆盖；相关情形保守降级） | `src/engine_select.rs:1921`、`:1833` |
| | #4649 | LEFT JOIN USING 退化为笛卡尔积 | `NOT STARTED` | ✅ **已实现** | `src/engine_select.rs:5158`、`:5561` |
| | #4636 | Correlated scalar subquery 失败 | `NOT STARTED` | ✅ **已实现** | `src/engine_select.rs:750`、`:2980` |
| WP-F（Schema Migration，1 条） | #4848 | ALTER TABLE RENAME COLUMN | `NOT STARTED` | ✅ **已实现**（**关键路径项**，v4.0.0 GA 声明边界） | `src/engine_ddl.rs:1361`、`crates/storage/src/file_storage.rs:7222` |
| WP-G（Type/Comparison，1 条） | #4846 | CHAR(n) PAD SPACE semantics | `NOT STARTED` | ✅ **已实现**（**关键路径项**，v4.0.0 GA 声明边界） | `src/engine_select_pk.rs:94-120`、`src/engine_dml.rs:905` |

**两条口径必须同时读**（`FEATURE_MATRIX.md` M-8 与 P-02 的分歧，本文件不替属主裁决）：

| 口径 | 说法 | 来源 |
|---|---|---|
| A | 「WP-C/D/F/G merged via #5041/#5043/#5045/#5047」 | `STAGE.yaml` `last_transition.reason`；`ROADMAP.md` M-8 的新 SSOT 读法 |
| B | 「SCOPED / MIGRATED to v4.1.0 — **implementation NOT STARTED**」 | `FEATURE_MATRIX.md` P-02；`ISSUES_PLAN.md` §4.1–§4.4 的 Status 列 |

口径 A 的 4 个 PR **实际交付的是回归测试矩阵**（把占位测试换成真断言），不是功能实现：
`2f96229f62`（WP-C，28 个占位测试 → 真断言）、`7b8953a1b0`（WP-D，29 个占位测试）、
`54e5a5fa83`（WP-F，8 个占位测试）、`e94613a7b0`（WP-G）。

> ⚠️ **第三条读法（本文件实测得出）**：上表「当前树实测」列显示，**12 条里有 9 条
> 在当前树上已经有实现**（#4682 / #4652（部分）/ #4709 / #4668 / #4649 / #4636 /
> #4848 / #4846，另 #4656、#4669、#4672、#4703 为部分）。也就是说，口径 B 的
> 「implementation NOT STARTED」在**当前树**上**已不准确**；口径 A 的「merged」
> 指的是**回归测试矩阵**合入，而非功能合入。⇒ **三条口径都不能单独当作事实**，
> 逐条状态以上表的「当前树实测」列为准。**这 9 条不一致需要属主裁决并订正
> `ISSUES_PLAN.md` §4**（§5.4 登记）。本文件只记录，不代为销账。

**WP-B / WP-E / WP-H 继承项**（`ISSUES_PLAN.md` §4.5–§4.7）：
WP-B 6 条（#4721 / #4674 / #4716 / #4676 / #4675 / #4670，Partial）、
WP-E（#4847 Partial via V400-05；#4626 未开始）、WP-H #4639。
v4.1.0 backlog 合计 **24** 条（本轮**未**逐条实测 WP-B/E/H 的代码状态）。

---

## 5. Known Gap 清单

### 5.1 V400-09 — 168h 多模型 SOAK：**NOT MET**

| 项 | 内容 |
|---|---|
| 判据 | `alpha_to_beta` / `rc_to_ga`：「V400-09 168h SOAK FINAL_REPORT exists OR explicit deferral recorded」 |
| 状态 | **NOT MET**（`STAGE.yaml` → `known_gaps.V400_09_soak.status`） |
| 核验日期 | 2026-10-07 |
| 实测结论 | (1) #4944 checkpoint 记录的服务进程 PID 1957614 已不在进程表；(2) 全盘搜索无 `STABILITY_REPORT_*.md` / `metrics.csv` / `*soak*run[34]*`；(3) `docs/releases/v4.0.0/V400_09_168H_SOAK_FINAL_REPORT.md` 日期为 2026-09-30，记录的是第一轮（2026-09-19 kickoff）的失败，其正文自述 "the 168h SOAK process never ran to completion" |
| 裁决状态 | `decision: "OPEN — 是否重跑 168h SOAK 属架构师裁决，本 SSOT 不代为决定。"` |
| 影响 | `STAGE.yaml` → `claims_boundaries.prohibited_pre_ga`：**GA 之前不得宣称 168h 多模型 SOAK 结果** |
| 证据 | `docs/releases/v4.1.0/STAGE.yaml` → `known_gaps.V400_09_soak`；`docs/releases/v4.0.0/V400_09_168H_SOAK_FINAL_REPORT.md` |

### 5.2 TPC-H SF=1 22/22：**not recorded**

| 项 | 内容 |
|---|---|
| 判据 | `stages.GA.required_gates` → `scripts/gate/check_g1_tpch_baseline.sh`（G1，`stage_min: ALPHA`） |
| 状态 | **not recorded** —— `docs/releases/v4.1.0/EVALUATION_REPORT.md` §7.1 第 378 行逐字记载："TPC-H SF=1 22/22 verdict for v4.1.0 \| **not recorded**" |
| 原因 | `evidence/TPCH_FIXTURE_5379_2026-10-20.md` 只覆盖 fixture 自包含性（#5379），该轮明确把 SF=1 排除在范围外（#5387） |
| 已知空缺 | G1 门禁所需 SF=1 数据集未就绪 ⇒ 本轮 `NOT RUN`（**NOT RUN ≠ PASS**） |
| 证据 | `docs/releases/v4.1.0/EVALUATION_REPORT.md` §7.1；`docs/releases/v4.1.0/evidence/TPCH_FIXTURE_5379_2026-10-20.md` |

### 5.3 Q20 oracle（#4540）：**NOT MET**（`STAGE.yaml` → `open_issues` 剩余的唯一一条）

| 项 | 内容 |
|---|---|
| 判据 | `STAGE.yaml` → `open_issues`（2026-10-23 复核后剩 1 条） |
| 状态 | **OPEN** |
| 实测结论 | `sqlrustgo/q20.tsv` oracle 缺失（22 个文件缺 1）；`CROSS_ENGINE_HASH_CHECK.json` 记 `present:false`；这与 `docs/releases/v3.12.0/evidence/v312-59/GA5_TPCH_SF1_REPORT.md:40` 的 "Q20 PASS" **直接矛盾**。当前树里两个 oracle 产物都不存在：`docs/releases/v3.12.0/evidence/tpch/cross_engine_sf1/` 只有 `build_cross_engine_summary.py` |
| 证据 | `docs/releases/v4.1.0/DEFECTS_AND_ISSUES.md` P1-02；`docs/releases/v3.12.0/evidence/v312-59/GA5_TPCH_SF1_REPORT.md` |

### 5.4 其余登记缺口（原文摘录 + 本轮实测出入登记）

| ID | 状态 | 内容摘要 | 来源 |
|---|---|---|---|
| P0-03 | **REVOKED**（状态，非缺陷） | v4.0.0 的 GA 判定被撤销：`docs/releases/v4.0.0/STAGE.yaml` → `ga_promotion_status.verdict = "REVOKED 2026-09-30 — v4.0.0 never legitimately reached GA"`。**但 v4.0.0 代码确已发布**：2026-09-19，tag `v4.0.0-final` @ `54571eeca0` → `07178c9d66`，5 远端 `main` 均含该 commit | `STAGE.yaml` → `resolved_issues_2026_10_23` 首条 |
| P0-04 | **OPEN** | 5 远端 `develop/v4.1.0` 漂移。2026-10-23 实测：`main` 在 5 远端**全部**为 `515ab236d0bf`（一致）；`develop/v4.1.0` 则 gitea252 = `2406559ac0`、github = `2406559ac0`、gitcode = `cc075f264fdd`、gitee = `cc075f264fdd`、gitea250(origin) = `878e1d62c3a5` ⇒ **未收敛** | `DEFECTS_AND_ISSUES.md` P0-04；本轮 `git ls-remote` 实测 |
| WP-C..G 的 12 条 backlog 状态 | **SSOT 与代码有出入（12 条中 9 条不一致）** | `ISSUES_PLAN.md` §4.1–§4.4 把 12 条全部记为 `NOT STARTED`；本轮在 `2406559ac0` 上逐条查生产代码，**9 条已有实现**（#4682 / #4652（PROCEDURE 侧）/ #4709 / #4668 / #4649 / #4636 / #4848 / #4846；#4656、#4669、#4672、#4703 为部分）。逐条对照见 **§4 的「当前树实测」列**。⇒ **`ISSUES_PLAN.md` §4 需要属主裁决并订正**；本文件只记录，不代为销账 | 本轮实测（`grep` + 源码路径）；`ISSUES_PLAN.md` §4.1–§4.4 |
| 半同步 `HeartbeatAck` | Partial（**SSOT 与代码有出入**） | `STAGE.yaml` → `hard_serial_edges` 记「#4936 → #4937（主库丢弃 `HeartbeatAck`，半同步无可等待对象）」，并指 `crates/storage/src/binlog_server.rs:238`。本轮实测当前树 `:388` 起的 `HeartbeatAck` 分支**已记录** ACK（写入 `acked_lsn`、回 `AckOk`），`:421` 起的 `BinlogAck` 走 `acked_pos`（注释明确标注 #4937）⇒ **STAGE.yaml 的行号引用指向更早的树状态**。本文件不替属主裁决，仅登记该出入 | `STAGE.yaml`；本轮实测 `crates/storage/src/binlog_server.rs` |
| WAL 恢复与 listener 的先后关系 | Partial | 恢复在 listener 接受连接之前完成 ⇒ 重启是硬启动中断；P0-08 修复**未改变**该结构性结论 | `STAGE.yaml` → `resolved_issues_2026_10_23`（P0-08 条目「Not claimed」） |
| 5 远端 `develop/v4.1.0` 收敛 | 未完成 | 见 P0-04 | — |

### 5.5 缺陷与 open_issues 现状（2026-10-23）

| 级别 | 原始（2026-10-01） | 仍未销账（2026-10-23） | 主要类别 |
|---|---|---|---|
| **P0** | 8 | **2** | v4.0.0 GA 撤销（`REVOKED`，是状态不是缺陷）、5 远端漂移 |
| **P1** | 7 | **6** | 覆盖率 60.58%（已销账，计数为历史）、Q20 oracle 缺产物、WP-A RE-OPENED + 20 条 backlog、crate 状态描述错误、G13 门禁红 |
| **P2** | 7 | 7 | 门禁假阳性、Q20 decorrelate 零调用点、大文件无门禁、v3.12 失联、未核实遗留、运营债 |
| **合计** | **22** | **15** | |

来源：`docs/releases/v4.1.0/DEFECTS_AND_ISSUES.md` §0。
`STAGE.yaml` → `open_issues` 复核后**剩 1 条**（P1-02 / #4540 Q20 oracle）。

---

## 6. 扩展 crate 的隔离状态

> **查证方法**：读 `Cargo.toml` 的 `members` 与 `default-members`。
> **实测结论**：`Cargo.toml` 有 `members`（**44 项** = **42 个** `crates/*` +
> `tools/compat-runner` + `tools/xtask`），**没有 `default-members` 键、也没有 `exclude` 键**
> ⇒ **默认构建路径包含全部 44 个 member**。
> `crates/` 下共有 **43 个 crate 目录**，**唯一不在 `members` 里的是 `crates/soak-client`**
> —— 该目录**只有 `crates/soak-client/src/main.rs` 一个文件，没有 `Cargo.toml`** ⇒
> 它**不是一个 crate**，不参与任何 cargo 构建。

**因此需要区分两件事**：「是否在默认构建路径」与「是否在生产执行路径」是**两个不同的轴**。
下表的「主路径」列按 `STAGE.yaml` → `legacy_chain_audit_2026_09_30.C7_extension_crates`
的记载 + `docs/releases/v4.1.0/b2-off-production-path-registry.md` 的登记口径给出。

| crate 目录 | package 名 | 在默认构建路径? | 在生产执行路径? | 状态 | 来源 |
|---|---|---|---|---|---|
| `crates/gmp` | `sqlrustgo-gmp` | ✅ 是 | ❌ 否 | ⚠️ **SCOPE_DEFERRED**（产品决策待定） | `STAGE.yaml` C7；`scripts/gate/check_gmp_consumer.sh`；`evidence/gmp_consumer_gate_20261005.md` |
| `crates/rag` | `sqlrustgo-rag` | ✅ 是 | ❌ 否 | ⚠️ **SCOPE_DEFERRED** | `STAGE.yaml` C7 |
| `crates/graph` | `sqlrustgo-graph` | ✅ 是 | ❌ 否 | 🚫 **ARCHIVED** | `STAGE.yaml` C7 |
| `crates/admin` | `sqlrustgo-admin` | ✅ 是 | ❌ 否 | ⚠️ **PARTIAL**（与 F-32 `mysqladmin` 的记载矛盾） | `STAGE.yaml` C7 |
| `crates/vector` | `sqlrustgo-vector` | ✅ 是 | ❌ 否 | ⚠️ **SCOPE_INTERNAL** | `STAGE.yaml` C7；`evidence/VECTOR_COSINE_5384_2026-10-22.md`（#5384 修 cosine 恒为 0） |
| `crates/distributed` | `sqlrustgo-distributed` | ✅ 是 | ❌ 否 | ⚠️ 扩展（Raft / gRPC / 2PC，与 MySQL 5.7 替代无关） | `STAGE.yaml` C7；`crates/transaction/src/dtc.rs` 是与之相关的独立实现 |
| `crates/evidence-graph` | `evidence-graph` | ✅ 是 | ❌ 否 | 扩展（证据追踪） | `STAGE.yaml` C7；commit `fb109aa170`（2026-10-16） |
| `crates/graph-cli` | `graph-cli` | ✅ 是 | ❌ 否 | 扩展（Cypher CLI） | `STAGE.yaml` C7 |
| `crates/gis` | `sqlrustgo_gis` | ✅ 是 | ❌ 否 | 扩展（GIS） | `STAGE.yaml` C7；`crates/gis/src/lib.rs` |
| `crates/shard-router` | `sqlrustgo-shard-router` | ✅ 是 | ❌ 否 | 扩展 | `STAGE.yaml` C7 |
| `crates/spill` | `sqlrustgo-spill` | ✅ 是 | ❌ 否 | 扩展（grace hash join） | `STAGE.yaml` C7 |
| `crates/cache` | `sqlrustgo-cache` | ✅ 是 | ⚠️ 经 executor 间接使用 | 扩展 | `STAGE.yaml` C7 |
| `crates/sql-corpus` | `sqlrustgo-sql-corpus` | ✅ 是 | ❌ 否 | 门禁支撑（`check_sql_corpus_gate.sh`） | `scripts/gate/corpus_manifest.yaml` |
| `crates/sqlancer` | `sqlancer` | ✅ 是 | ❌ 否 | 模糊测试支撑 | `crates/sqlancer/src/lib.rs` |
| `crates/wal-verification` | `sqlrustgo-wal-verification` | ✅ 是 | ❌ 否 | 工具（`STAGE.yaml` 记「WAL Verification Tool GMP path」） | `STAGE.yaml` C9 |
| `crates/test-registry` / `test-runner` / `test-results` / `test-reporter` | 同名 | ✅ 是 | ❌ 否 | 门禁支撑 | `scripts/gate/check_test_inventory.sh` 等 |
| `crates/transaction-stress` | `transaction-stress` | ✅ 是 | ❌ 否 | 压测支撑 | `crates/transaction-stress/src/lib.rs` |
| `crates/cli` | `sqlrustgo-soak` | ✅ 是 | ❌ 否 | 压测支撑（注意 package 名与目录名不同） | `crates/cli/src/lib.rs` |
| `crates/soak-client` | **无 `Cargo.toml`** | ❌ **否** | ❌ 否 | **不是 crate**（该目录下仅 `crates/soak-client/src/main.rs` 一个文件），不参与构建 | 本轮实测 `ls crates/soak-client/` |

**结论**：

1. **默认构建路径**上**没有**被排除的 crate（`Cargo.toml` 无 `default-members` /
   `exclude`）；唯一不在 `members` 的目录 `crates/soak-client` 连 `Cargo.toml` 都没有。
2. **生产执行路径**上，上表 19 行里的 **18 个 crate 全部不在**（第 19 行
   `crates/soak-client` 本就不是 crate）——依据是
   `docs/releases/v4.1.0/b2-off-production-path-registry.md`（B2「不在生产执行路径」
   排除清单，SSOT = `scripts/gate/run_b2_per_binary.py` 的 `OFF_PRODUCTION_PATH_BINARIES`）
   与 `STAGE.yaml` C7 的逐条状态记载。
3. ⚠️ **一处口径冲突（本文件不替属主裁决）**：`STAGE.yaml` C7 把 `admin` 记为
   **PARTIAL** 且注明「contradicts F-32」；`FEATURE_MATRIX.md` §1 与 §3 均未复述该条。

---

## 7. Gate 集成（功能项 → 真实存在的门禁脚本）

> 全部脚本路径均已核实存在于 `scripts/gate/`（该目录共 183 个 `.sh`）。
> **「必跑阶段」列来自 `docs/governance/STAGE_CONFIG.yaml` 的 `stage_min` 累计规则**：
> ALPHA = G1+G4；BETA = +G2/G3/G5/G6；RC = +G8/G9/G10；GA = +G16。

| G | 关注点 | 门禁脚本 | 覆盖的功能项 | 必跑阶段 | 本轮状态 |
|---|---|---|---|---|---|
| G1 | TPC-H 22/22 基线（回归） | `scripts/gate/check_g1_tpch_baseline.sh` | F-50 | ALPHA 起累计 | **NOT RUN**（SF=1 数据未就绪，§5.2） |
| G2 | INT-2 ParallelExecutor 集成 | `scripts/gate/check_int2_no_orphan.sh` | F-29 | BETA 起累计 | NOT RUN |
| G3 | INT-3 Single Expression | `scripts/gate/check_int3_single_expr.sh` | F-02、F-13 | BETA 起累计 | NOT RUN |
| G4 | ARCH-3 VtuGuard 主路径 | `scripts/gate/check_arch3_no_bypass.sh` | F-01、F-02 | ALPHA 起累计 | NOT RUN |
| G5 | SEM-1 Savepoint MVCC | `scripts/gate/check_sem1_savepoint.sh` | F-08、F-09、F-10、F-11 | BETA 起累计 | NOT RUN |
| G6 | Backup/Restore 100+ 场景 | `scripts/gate/check_backup_restore.sh` | F-12、F-27、F-28、F-45 | BETA 起累计 | NOT RUN |
| G7 | 24h SOAK（压缩时间） | `scripts/gate/check_p13_soak_test.sh` | 稳定性 | RC 起（非阻断） | **NOT RUN**（且 168h SOAK 判定 NOT MET，§5.1） |
| G8 | Crash Matrix 100+ 场景 | `scripts/gate/check_p12_crash_test.sh` | F-19 | RC 起累计 | NOT RUN |
| G9 | Upgrade Test 50+ 路径 | `scripts/gate/check_p14_upgrade_test.sh` | F-46 | RC 起累计 | NOT RUN |
| G10 | Audit Log + Time Travel + Hash Chain | `scripts/gate/check_p21_audit_log.sh`、`check_p22_time_travel.sh`、`check_p23_hash_chain.sh` | F-42、F-43 | RC 起累计 | NOT RUN |
| G11 | QPS/TPS 基准 | `scripts/gate/check_g11_qps.sh` | 性能 | RC 起（非阻断） | NOT RUN（需基准机） |
| G12 | Sysbench OLTP | `scripts/gate/check_g12_sysbench.sh` | 性能 | RC 起（非阻断） | NOT RUN（需 `sysbench` 外部工具） |
| G13 | 24h+ 稳定性（真实） | `scripts/gate/check_g13_stability.sh` | 稳定性 | GA 起（非阻断） | NOT RUN（需 Z6G4 主机 + 报告） |
| G14 | 真实崩溃测试（进程级） | `scripts/gate/check_g14_real_crash.sh` | F-19 | RC 起（非阻断） | NOT RUN（需专用主机） |
| G15 | 性能报告汇总 | `scripts/gate/check_g15_perf_report.sh` | 性能 | GA 起（非阻断） | NOT RUN（需报告产物） |
| G16 | 兼容性 | `scripts/gate/check_g16_compatibility.sh` | 升级路径 | GA 起累计 | NOT RUN |

**功能专项门禁**（不在 G 编号内，但存在且被阶段清单引用）：

| 脚本 | 覆盖 | 出现阶段 |
|---|---|---|
| `scripts/gate/check_coverage_v312.sh` | 覆盖率（L1_8 8-crate） | ALPHA 起累计 |
| `scripts/gate/check_anti_fabrication.sh` | 反编造 | BETA 起累计 |
| `scripts/gate/check_anti_ignore_gate.sh` | `#[ignore]` 债务 | ALPHA 起累计 |
| `scripts/gate/check_sql_corpus_gate.sh` | SQL 语料 | ALPHA 起累计 |
| `scripts/gate/check_storage_index_wal.sh` | 存储 / 索引 / WAL（F-18…F-26） | 仓库内专项门禁 |
| `scripts/gate/check_int_debt.sh` / `check_cross_version_debt.sh` / `check_arch_sem_debt.sh` | 三条债务台账 | BETA 起累计 |
| `scripts/gate/check_integration_gate.sh` | 集成 | RC 起累计 |
| `scripts/gate/check_full_gate_verification.sh` | 全门禁接线自检 | RC 起累计 |
| `scripts/gate/check_release_artifacts.sh` | **本文件是它的产物** | ALPHA 起累计 |
| `scripts/gate/check_evidence_binding.sh` | 文档证据绑定 | 仓库内专项门禁 |
| `scripts/gate/check_main_freshness.sh` / `check_drift_not_pass.sh` | 远端收敛 / 漂移 | F-48 |

> ⚠️ **G 编号是别名，不是判据**（`STAGE_CONFIG.yaml` #5437 的原话）。GA 的权威判据是
> `stages.GA.required_gates` 脚本路径清单（**31 项 = 27 个门禁脚本 + 4 条 cargo 命令**），
> **不是**「G1-G16 全 PASS」。两者相关但不是同一个集合。

---

## 8. 本轮实跑的边界

### 8.1 实测并引用的门禁（2026-10-23，Mac-mini-7.local / arm64 / macOS 27.0.1）

| 门禁 | 实测输出摘录 | 结论 |
|---|---|---|
| `scripts/gate/check_release_artifacts.sh --version v4.1.0` | 补齐前：`stage: ALPHA`；ALPHA 段 `required_file` ×3 + `doc_artifact` ×4 全 `OK`；`BETA: 5 present, 2 missing`（MISS `FEATURE_CHECKLIST.md` 的 required_file 与 doc_artifact）；`GA: 5 present, 6 missing`（MISS `GA_RELEASE_TIMELINE.md` ×2 等）；`RESULT: PASS`；`EXIT=0`。<br>**补齐后（本文件落盘后复跑）**：`BETA: 7 present, 0 missing`；`GA: 10 present, 1 missing`（仅 `POST_GA_PLAN.md`）；`RESULT: PASS`；`EXIT=0` | **PASS**（ALPHA 段）。本批次两份产物把 BETA 段的 2 个 MISS 与 GA 段的 `GA_RELEASE_TIMELINE.md` MISS 清零 |
| `scripts/gate/check_evidence_binding.sh v4.1.0 <out_dir>` | `PASS=137, WARN=123, FAIL=0, UNVERIFIED=0`；`✅ Evidence binding check PASSED`；`EXIT=0` | **PASS**。⚠️ 该门禁枚举发布目录下的文档集合，**PASS 计数随文档集合变化**（本轮两次实测：补齐前 `PASS=127`、补齐后 `PASS=137`，WARN 恒为 123、FAIL 与 UNVERIFIED 恒为 0）。本行是**补齐后**的实测值 |
| `scripts/gate/check_anti_fabrication.sh` | 见 §8.1 末尾说明 | **PASS**（`ERRORS=0, WARNINGS=0`） |

> ⚠️ **`check_evidence_binding.sh` 的用法**：脚本签名是
> `<version> <out_dir>`（两个参数，缺一即 `exit 2`）。本轮以临时目录作 `out_dir` 运行，
> 报告写在 `out_dir` 内，**未在仓库内留下任何文件**。

### 8.2 本轮**未**实跑的门禁（NOT RUN ≠ PASS）

**GA 阶段 `required_gates` 的 27 个门禁脚本 + 4 条 cargo 命令里，本轮只跑了 3 个**
（见 §8.1）。其余全部 `NOT RUN`。原因分类：

| 原因 | 涉及 |
|---|---|
| 本轮范围之外（批次 32 只交付两份文档） | `check_rc_ga_gate.sh`、`check_arch_invariants.sh`、`check_arch3_no_bypass.sh`、`check_int2_no_orphan.sh`、`check_int3_single_expr.sh`、`check_sem1_savepoint.sh`、`check_backup_restore.sh`、`check_p12_crash_test.sh`、`check_p14_upgrade_test.sh`、`check_p21_audit_log.sh`、`check_p22_time_travel.sh`、`check_p23_hash_chain.sh`、`check_integration_gate.sh`、`check_arch_sem_debt.sh`、`check_cross_version_debt.sh`、`check_int_debt.sh`、`check_full_gate_verification.sh`、`check_coverage_v312.sh`、`check_sql_corpus_gate.sh`、`check_g16_compatibility.sh`、`check_drift_not_pass.sh`、`check_architecture_freeze.sh`、`check_gate_self_verification.sh`、`check_gate_test_integrity.sh` |
| 前置数据/硬件/外部工具未就绪 | `check_g1_tpch_baseline.sh`（SF=1 数据未就绪，§5.2）、`check_p13_soak_test.sh`、`check_g11_qps.sh`、`check_g12_sysbench.sh`、`check_g13_stability.sh`、`check_g14_real_crash.sh`、`check_g15_perf_report.sh` |
| cargo 命令（`cargo build --all-features` / `cargo test --all-features` / `cargo fmt --check` / `cargo clippy --all-features -- -D warnings`） | 本轮**未**在整仓范围实跑，故不记 PASS |

### 8.3 覆盖率数据（引用而非本轮实测）

| crate | L1_8 行覆盖率 | 对 RC(80) | 对 GA(85) |
|---|---|---|---|
| parser | 86.46 % | ✅ | ✅ |
| planner | 88.91 % | ✅ | ✅ |
| executor | 82.74 % | ✅ | ❌（单独看 < 85） |
| transaction | 93.29 % | ✅ | ✅ |
| storage | 80.61 % | ✅ | ❌（单独看 < 85） |
| catalog | 87.33 % | ✅ | ✅ |
| optimizer | 87.85 % | ✅ | ✅ |
| types | 95.28 % | ✅ | ✅ |
| **8-crate 平均** | **87.80 %** | ✅ | ✅ |
| mysql-server | 87.89 % | ✅ | ✅ |

来源：`docs/releases/v4.1.0/evidence/BATCH28_SEM4_AND_GATE_INTEGRITY_2026-10-23.md` §1.3
（8-crate 完整实跑）与 `docs/releases/v4.1.0/evidence/COV_01_02_COVERAGE_GATE_2026-10-07.md`
（mysql-server）。

门槛：`STAGE_CONFIG.yaml` → `thresholds`：`COVERAGE_MIN_ALPHA: 50` /
`COVERAGE_MIN_BETA: 75` / `COVERAGE_MIN_RC: 80` / `COVERAGE_MIN_GA: 85`。

> ⚠️ **两个口径并存，引用时必须注明**：① `STAGE.yaml` → `rc_to_ga.exit_criteria` 写
> 「Coverage >= 80%」；② `STAGE_CONFIG` → `COVERAGE_MIN_GA` 是 **85**，
> 而 `STAGE.yaml` 自己的 `coverage_target_for_ga` 明写「85%（STAGE_CONFIG
> COVERAGE_MIN_GA）；80% 是 COVERAGE_MIN_RC」。
> ⇒ **GA 门槛按 85 判**。是否改 `STAGE.yaml` 的措辞，是属主的裁决事项。
> ⚠️ 另有第三个数字 **60.58 %** 记在 `DEFECTS_AND_ISSUES.md` P1-01（2026-10-01 实测），
> 2026-10-23 实测**不复现**（parser 实测 86.46 %）—— 该条已销账，但数字仍在文档里。

---

## 9. 签署

| 角色 | 姓名 | 日期 | 结论 |
|---|---|---|---|
| 提交人（`openclaw-mcode`） | openclaw-mcode | 2026-10-23 | 清单已生成；全部功能项指向真实路径；NOT MET / NOT RUN 均已注明原因 |
| BETA 评审（codeowner） | ______ | ______ | ______ |
| BETA 评审（governance） | ______ | ______ | ______ |
| 人类架构师 | ______ | ______ | ______ |

> 本文件由 `openclaw-mcode` 自动生成，晋级审批由人类架构师执行。
> `docs/governance/STAGE_CONFIG.yaml` → `transitions.ALPHA_to_BETA.requires_human_approval: true`
> ⇒ **本文件不构成 BETA 晋级批准**，签署栏留空由人填写。

---

## 10. 参考资料

- `docs/governance/STAGE_CONFIG.yaml` —— 阶段判据的 SSOT（阈值 / required_files / required_gates / G 别名 / 跃迁需人批）
- `docs/releases/v4.1.0/STAGE.yaml` —— v4.1.0 per-version SSOT（`current_stage` / `exit_criteria` / `known_gaps` / `open_issues` / `ga_carrier_branch`）
- `docs/releases/v4.1.0/FEATURE_MATRIX.md` —— v4.1.0 相对 v4.0.0 的权威变更矩阵与证据指针
- `docs/releases/v4.1.0/DEFECTS_AND_ISSUES.md` —— 缺陷目录与销账状态
- `docs/releases/v4.1.0/EVALUATION_REPORT.md` —— §7.1「not recorded」清单
- `docs/releases/v4.1.0/ISSUES_PLAN.md` —— §4 WP-A..G backlog（24 条）与 §4.7 汇总
- `docs/releases/v4.1.0/ROADMAP.md` —— §1 里程碑 M-1..M-10
- `docs/releases/v4.1.0/CHANGELOG.md` / `RELEASE_NOTES.md` —— 变更与发布口径
- `docs/releases/v4.1.0/b2-off-production-path-registry.md` —— 生产路径排除清单
- `docs/releases/v4.1.0/evidence/` —— 81 份一手证据
- `docs/releases/v4.0.0/STAGE.yaml` / `GA_GATE_REPORT.md` / `GA_RELEASE_TIMELINE.md` —— 继承基线与 GA 撤销判定
- `docs/releases/v3.10.0/FEATURE_CHECKLIST.md` —— 格式先例（状态图例口径）
