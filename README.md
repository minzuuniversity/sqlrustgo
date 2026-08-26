# SQLRustGo

> **更新日期**: 2026-08-26
> **当前开发版**: v3.12.0 (RC: 2026-08-26, post PR #4483)
> **当前开发目标**: 面向 GMP 内审检索系统的 SQLRustGo 数据库、内部向量检索、SQL-backed graph projection 和可审计 evidence bundle；v3.12.0 已通过 RC gate（12/12 crash recovery + 11/11 promotion_to_RC），下一步 GA 治理闭环（168h mixed SOAK）。
> **阶段治理锁**: v3.13 follow-up 已冻结，不能替代 v3.12 Beta/RC/GA；见 [v3.12 阶段治理纠偏报告](docs/releases/v3.12.0/STAGE_GOVERNANCE_REMEDIATION_2026-08-18.md)。

  <img src="https://img.shields.io/badge/v3.11.0-GA-blue?style=flat-square" alt="v3.11.0 GA">
  <img src="https://img.shields.io/badge/v3.12.0-RC-orange?style=flat-square" alt="v3.12.0 RC">
  <img src="https://img.shields.io/badge/license-MIT-green?style=flat-square" alt="License">
</p>

当前仓库的发布口径是：

- **v3.11.0 GA**: 可作为简单生产环境或受控场景的候选数据库版本，但不能宣称为完整 MySQL 5.7 替代品。
- **v3.12.0 (RC)**: 面向 GMP 内审检索数据库、内部向量检索和 SQL-backed graph projection 的受控版本；2026-08-26 完成 BETA→RC 转段（[RC_GATE_REPORT.md](docs/releases/v3.12.0/RC_GATE_REPORT.md)），12/12 crash recovery 测试 PASS + 11/11 promotion_to_RC_requires PASS + B8 thresholds_override 13/13 PASS；下一步进入 GA 治理闭环（168h mixed SOAK）。已知 partial 项（Q17/Q20 全 SF=1 TIMEOUT）已在 [V312-58-SF1-COMPLETION-STATUS.md](docs/releases/v3.12.0/evidence/V312-58-SF1-COMPLETION-STATUS.md) 文档化为 v313-deferred，不阻塞 RC。
- **v3.13.0 (冻结 follow-up)**: 只能承接经用户批准延期的事项或后续规划；v3.13 PR 合并不自动关闭 V312 scope。
- **v4.0.0 方向**: 才适合规划"通用向量数据库 / 通用图数据库 / 更广义生产替代"的产品目标。
---

## 目录

- [当前真实状态](#当前真实状态)
- [快速开始](#快速开始)
- [架构概览](#架构概览)
- [功能矩阵](#功能矩阵)
- [TPC-H 与性能基准](#tpc-h-与性能基准)
- [GMP / RAG / Vector / Graph](#gmp--rag--vector--graph)
- [测试与质量门禁](#测试与质量门禁)
- [文档资源](#文档资源)
- [历史版本](#历史版本)
- [贡献指南](#贡献指南)
- [许可证](#许可证)

## 当前真实状态

| 项 | 当前状态 | 证据 / 说明 |
|---|---|---|
| v3.11.0 阶段 | GA | [v3.11 综合评估](docs/releases/v3.11.0/COMPREHENSIVE_ASSESSMENT_REPORT.md)、[v3.11 STAGE](docs/releases/v3.11.0/STAGE.yaml) |
| v3.11.0 生产边界 | 受控/简单生产候选 | 不等同完整 MySQL 5.7 替代；TPC-H correctness、LOAD DATA、recovery、upgrade 等仍需 v3.12 补强 |
| v3.12.0 阶段 | RC (2026-08-26) | [v3.12 STAGE](docs/releases/v3.12.0/STAGE.yaml) 记录 DRAFT (pre-2026-08-12) -> ALPHA (2026-08-12) -> BETA (2026-08-19) -> **RC (2026-08-26)**；[RC_GATE_REPORT.md](docs/releases/v3.12.0/RC_GATE_REPORT.md) 12/12 crash recovery + 11/11 promotion_to_RC + B8 13/13 全 PASS |
| v3.12.0 产品目标 | GMP 内审检索数据库 | [v3.12 README](docs/releases/v3.12.0/README.md)、[GMP 合规矩阵](docs/releases/v3.12.0/GMP_COMPLIANCE_MATRIX.md) |
| TPC-H SF=1 | **DONE-with-boundary**（22/22 可运行 + 16/22 row-count MATCH；Q8 #4274 + Q16 #4278 FIXED in v3.12 working-tree；剩余 6 zero-row 由 #4272 治理（issue #4273/#4275-#4280 已收口，无 v3.13 defer）） | [v3.11 TPC-H 报告](docs/releases/v3.11.0/TPCH_SF1_22_22_PASS_REPORT.md)、[v3.12 TPC-H correctness](docs/releases/v3.12.0/evidence/tpch/V312-12-TPCH-CORRECTNESS.md)、[V312-48-Q8](docs/releases/v3.12.0/evidence/tpch/V312-48-Q8-VERIFICATION.md)、[V312-48 子 issue #4272-#4280 收口](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4221) |
| TPC-H SF=10 | PARTIAL / 有整改 issue | harness 存在；3/8 SF=10 表已 parity match，剩余大表受 FileStorage 写放大/吞吐瓶颈阻塞；见 [#4020 evidence](docs/releases/v3.12.0/evidence/issue-4020/4020_evidence.md)、[#4217](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4217) |
| Bulk-load SF=10 | PARTIAL / OPEN | 不是 schema creation 阶段失败的旧状态；当前是 3/8 表 match，5/8 大表未完成；见 [#4020](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4020)、[#4217](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4217) |
| MySQL wire + LOAD DATA hardening | PARTIAL / 有整改 issue | [V312-13 报告](docs/releases/v3.12.0/evidence/wire_load_data/V312-13-REPORT.md) failed_steps=0，但完整 MySQL 5.7 兼容、prepared statement sysbench 路径、SF=10 full bulk-load 仍需 [#4223](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4223)、[#4211](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4211)、[#4020](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4020) |
| Sysbench | PARTIAL / 有整改 issue | read_only baseline 已捕获；write/read_write 因行级锁/隔离问题失败，prepared statement 兼容另有缺口；见 [#4210](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4210)、[#4211](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4211) |
| Prometheus metrics | endpoint 已实现并 E2E scrape | [#4021 evidence](docs/releases/v3.12.0/evidence/issue-4021/4021_evidence.md) |
| Slow query log | 已实现并测试 | [#4022 evidence](docs/releases/v3.12.0/evidence/issue-4022/4022_evidence.md) |
| SQLLogicTest | smoke gate 25/25 PASS；full official corpus 未声明完成 | [SQLLogicTest smoke report](docs/releases/v3.12.0/evidence/sqllogictest/smoke-report.md)、[v3.12 Scope Table](docs/releases/v3.12.0/SCOPE_TABLE_v3.12.md)、[#4224](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4224) |
| 覆盖率 | 分层统计中；不再用单一 workspace 口径宣传全达标 | [综合测试框架与覆盖率基线](docs/releases/v3.12.0/COMPREHENSIVE_TEST_FRAMEWORK_AND_COVERAGE_BASELINE.md) |

## 快速开始

```bash
# 克隆
git clone http://192.168.0.252:3000/openclaw/sqlrustgo.git
cd sqlrustgo

# 构建
cargo build --all-features

# 运行测试
cargo test --all-features

# 格式和 lint
cargo fmt --check --all
cargo clippy --all-features -- -D warnings

# MySQL 风格服务端
cargo run --bin sqlrustgo-mysql-server -- serve --host 127.0.0.1 --port 3307
```

> 注意：不同历史文档中曾经出现 `sqlrustgo`、`sqlrustgo-sql-cli` 等旧入口。自 v3.8 起，主要运行入口逐步收敛到 `sqlrustgo-mysql-server` 及 workspace crates。

## 架构概览

```text
SQL / MySQL Wire / Admin
        |
Parser -> Planner -> Optimizer -> Executor
        |                  |
        |                  +-- TPC-H / SQLLogicTest / MySQL compat paths
        |
Storage / Catalog / Transaction / WAL / MVCC
        |
GMP schema / chunks / versions / audit log / relations / embeddings
        |
Hybrid retrieval / Vector retrieval / SQL-backed graph projection / RAG evidence bundle
```

## 功能矩阵

状态说明：

- `DONE`: 有合并代码和测试/报告证据。
- `PARTIAL`: 主路径或 smoke 可用，但生产级闭环、边界测试或性能证据不足。
- `PARTIAL / 有整改 issue`: 仅允许作为 Alpha/Beta 过渡状态；若属于 v3.12 初始生产边界，GA 前必须按 [整改计划](docs/releases/v3.12.0/PARTIAL_FEATURE_REMEDIATION_ISSUE_PLAN.md) 关闭或降级。
- `DEFERRED`: 已有 owner/expiry/follow-up 或明确规划，当前版本不宣称完成。
- `OPEN`: 当前仍阻塞或未达到关闭标准。
- `UNSUPPORTED`: 明确不支持或不作为当前版本目标。

| 能力 | v3.11.0 GA | v3.12.0 当前 | 证据边界 |
|---|---:|---:|---|
| SQL 基础 DDL/DML | DONE | DONE / 持续硬化 | CREATE/INSERT/SELECT/UPDATE/DELETE 主路径可用，corner cases 由 SQLLogicTest 继续覆盖 |
| SQL-92 SELECT / JOIN / GROUP BY | DONE | DONE / 持续硬化 | TPC-H 和 SQL corpus 仍暴露 planner/semantic gap |
| CTE | DONE | DONE | 包括 CTE materialization 改进；递归和复杂兼容仍需按测试声明 |
| 窗口函数 — 核心 12 函数 + 默认 frame | DONE / 受控 | DONE / 受控 | `crates/executor/src/window_executor.rs` 29 单测 PASS；integration 17/21 PASS；详见 [scope 决策](docs/releases/v3.12.0/sql-feature-corpus/window_json_gis_scope.md) §2 |
| 窗口函数 — 显式 ROWS/RANGE BETWEEN / EXCLUDE / NULLS FIRST/LAST 语法 | DEFERRED | DEFERRED → v3.13 | integration 4/21 parser FAIL（Issue #4228 to open） |
| JSON 读路径 (JSON_EXTRACT / JSON_VALUE / JSON_VALID / JSON_TYPE / JSON_KEYS / JSON() / `->` / `->>`) | DONE / 受控 | DONE / 受控 | `crates/executor/tests/json_eval_fn_test.rs` 10/12 PASS；详见 [scope 决策](docs/releases/v3.12.0/sql-feature-corpus/window_json_gis_scope.md) §3 |
| JSON 写路径 / JSON 列类型 / JSON_TABLE / JSON_MERGE | DEFERRED | DEFERRED → v3.13 | Issue #4229 to open |
| GIS (ST_Within / ST_Distance / ST_Contains / ST_Intersects 在 Value::Point + WKT 字面量) | DEFERRED | DEFERRED → v3.13 | `sqlrustgo_gis` 14 单测 PASS；无 spatial column / index / WKT I/O；Issue #4230 to open |
| Optimizer / CBO / Hash Join | DONE | DONE / 持续硬化 | #3909 已通过 PR #4087 close-out；性能债仍按后续 issue 跟踪 |
| WAL / MVCC — crash recovery（kill mid-tx, WAL replay uncommitted tx, incomplete-tx 检测, 8 scenarios 过程杀进程） | PARTIAL | DONE / 受控 | V312-14 gate 5/5 PASS；`process_kill_crash_test` 8/8 PASS（含 Round-3 FAIL 的 `test_kill_mid_insert_update_uncommitted` + `test_mixed_workload_recovery_report`）；详见 [V312-14-RECHECK](docs/releases/v3.12.0/evidence/crash_recovery/V312-14-CRASH-RECOVERY-RECHECK.md) §2 |
| WAL / MVCC — backup/restore API（SHA-256 校验, manifest verify, round-trip, corrupted data/WAL detection） | PARTIAL | DONE / 受控 | `backup_restore_test` 51/51 PASS at HEAD 0f497bbef8；Round-3 API drift 已修复；详见 [V312-14-RECHECK](docs/releases/v3.12.0/evidence/crash_recovery/V312-14-CRASH-RECOVERY-RECHECK.md) §3 |
| WAL / MVCC — v3.10/v3.11 → v3.12 upgrade + rollback fixture（row count / hash / 4-hop preservation） | PARTIAL | DONE / 受控 | `check_upgrade_v310_v311.sh` 11/11 + `upgrade_v310_v311_test` 4/4 + `upgrade_test` 50/50 + `int2_cross_version_upgrade_test` 20/20 + `v380_to_v390_full_upgrade_test` 18/18 + `upgrade_chain_v3_6_to_v3_9_test` 6/6 = 109/109 PASS；详见 [V312-14-RECHECK](docs/releases/v3.12.0/evidence/crash_recovery/V312-14-CRASH-RECOVERY-RECHECK.md) §4 |
| WAL / MVCC — SF=10 TPC-H 全表 bulk-load 后 crash + WAL replay 大 fixture 行为 | N/A | DEFERRED → v3.13 | V312-13 仅覆盖 SF=1 + SF=10 {region,nation,supplier} bulk-load；lineitem/customer/orders 大 fixture 上 crash-recovery + WAL replay 路径未压测；Issue #4239 to open |
| B+Tree / Hash index | DONE | DONE | 索引能力进入主路径；性能趋势需按具体 workload 阅读 |
| Clustered Index / AHI / Change Buffer / Double Write Buffer | DONE | DONE | v3.11 重点功能；仍建议配合 crash/fault injection 继续验证 |
| MySQL wire protocol — COM_QUERY / COM_STMT_PREPARE/EXECUTE/CLOSE / error packet / reset / TLS handshake / compression primitive | PARTIAL | DONE / 受控 | `crates/tools/src/ephemeral.rs` ephemeral server + 22 v312_13_typed_wrappers / 22 mysql_wire_protocol / wire_smoke_mysql_cli 集成测试 PASS；V312-13 gate 10/10 PASS；详见 [V312-50](docs/releases/v3.12.0/evidence/wire_load_data/V312-50-REPORT.md) §2 |
| MySQL wire protocol — `COM_RESET_CONNECTION` 在 libmysqlclient 路径下退化为 `Unknown command` warning | N/A | DONE-with-boundary | test 显式接受 `Ok(())` 或 `Unknown command`；非正确性要求，仅 libmysqlclient 优化提示 |
| Prepared Statement — Sysbench libmysqlclient (PR #4229 修复 `lenenc_int(0x0c)` + non-SELECT `extract_table_name` + INT→LONGLONG) | PARTIAL | DONE | 4/4 sysbench OLTP workloads (oltp_read_only / oltp_insert / oltp_write_only / oltp_read_write) PASS, 0 ignored errors；不再需要 `--db-ps-mode=disable`；证据 `docs/releases/v3.12.0/evidence/issue-4211/20260814T_after_fix2/` |
| LOAD DATA — SF=1 smoke + full + SF=10 region/nation/supplier smoke | PARTIAL | DONE | `v312_13_load_data_sf1_test` + `v312_13_load_data_sf10_test` PASS；V312-13 step 06.5/07/08 PASS；fixtures via `dbgen -s 10 -f -T {r,n,s}` |
| LOAD DATA — SF=10 lineitem/customer/orders/part/partsupp 全量 bulk-load 生产路径 | N/A | DEFERRED → v3.13 | Issue #4217 chunked bulk-load 已关闭，但 SF=10 全表 bulk-load 尚未作为 gate；follow-up issue to open |
| TLS / Compression | PARTIAL | DONE / 受控 | V312-13 step 09 (`force_tls_server_implemented`) + step 10 (`compress_primitives_working`) PASS；rustls + flate2 集成；不宣称 TLS 1.3 全部 cipher suite |
| Sysbench OLTP (oltp_read_only / oltp_insert / oltp_write_only / oltp_read_write) | N/A | DONE / 受控 | 4/4 PASS at develop HEAD post PR #4229；`mysql_compat/SURFACE_DISPOSITION.md` 12/20 libmysqlclient 表面 PASS |
| Prometheus `/metrics` | N/A | DONE / 有限制 | endpoint 和 live scrape 已验证；query counter hot path 仍有 observability debt |
| Slow query log | N/A | DONE / 有限制 | 单元和集成测试通过；未在真实 TPC-H SF=10 长查询上捕获日志 |
| SQLLogicTest smoke baseline (curated 25 .test 文件覆盖 sqlrustgo_simple/duckdb_samples/duckdb_full/root) | N/A | DONE / 受控 | `scripts/gate/check_sqllogictest_v312.sh` 实跑；25/25 PASS, 100% pass rate；`sqlite-corpus-manifest.json::corpus_stats` + `evidence_hash` 校验通过；详见 [V312-51](docs/releases/v3.12.0/evidence/sqllogictest/V312-51-REPORT.md) §2 |
| SQLLogicTest 排除注册表 (16 项历史缺陷 + Round-9 5-class 分类) | N/A | DONE / 受控 | 16/16 已关闭（PR #4074/#4073/#4069/#4082/#4055/#4065/#4066 + commit 7a315826fb）；每项含 id / file / root_cause / follow_up_issue / owner / v3.13_expiry / close_boundary / closed_by_commit；详见 §4 |
| SQLLogicTest — 完整 SQLite 官方 corpus (≈700 files / 6 MB) 集成 + sqlite3 参考输出对比 | N/A | DEFERRED → v3.13 | 当前 25 文件是 curated 子集；完整 corpus 未 vendor；Issue #4238 to open |
| 覆盖率治理 | PASS with follow-up | PARTIAL / blocker | v3.12 采用 per-crate 分层口径；低覆盖 crate 和 SEM-4 gap 由 [#3943](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3943) 收口 |
| GMP schema / version / chunk / relation (CRUD 路径) | N/A | DONE | `sqlrustgo-gmp --lib` 154 tests PASS；`acl.rs` 5 角色 × 12 ops 矩阵编译校验可证 |
| GMP 审计链 — CRUD on gmp_documents (CREATE/UPDATE/DELETE) | N/A | DONE | SHA-256 `event_hash → previous_hash`；3 hash-chain + 2 event-hash tests PASS；详见 [V312-53](docs/releases/v3.12.0/evidence/gmp_compliance/V312-53-REPORT.md) §3 |
| GMP 审计链 — 合规操作 (IMPORT/EXPORT/APPROVE/REVIEW/BACKUP/RESTORE) | N/A | DEFERRED → v3.13 | `AuditAction` 枚举仅 Create/Update/Delete；`import_document` / `bulk_import` / `create_backup` / `restore_backup` 未调用 `record_audit_log`；Issue #4231 to open |
| GMP 篡改检测 — 集成测试 (mutate-then-verify) | N/A | DEFERRED → v3.13 | `test_hash_chain_tamper_detection` 仅验证链完整，未 mutate storage（注释自承）；Issue #4232 to open |
| GMP 嵌入/图投影 篡改检测 | N/A | DEFERRED → v3.13 | `chunk_embeddings` / graph 表无 `previous_hash` / `event_hash` 列；Issue #4233 to open |
| ACL 5 角色 × 12 ops 矩阵全枚举测试 | N/A | DEFERRED → v3.13 | 12 个 spot-check ACL tests PASS（含 `test_permission_guard_fail_closed`）；5×12=60 cell 全枚举程序化测试未做；Issue #4234 to open |
| GMP Hybrid Retrieval | N/A | DONE / 受控 | RRF、filter、citation tests；目标是 GMP 内审检索，不是通用搜索引擎 |
| RAG Evidence Bundle | N/A | DONE / 受控 | citation/evidence_hash/answer envelope tests；需结合 GMP fixture 做质量评估 |
| Internal Vector Retrieval — 嵌入 + Flat 索引 + 混合检索 (vector_score / keyword_score / graph_boost / rrf_score + citation_text + chunk_hash) | PARTIAL | DONE / 受控 | `HashEmbeddingModel` 确定性；`vector_hash` SHA-256；`FlatIndex::build/search`；15 单测 PASS（vector_index 3 + vector_search 4 + retrieval 8）；详见 [V312-52](docs/releases/v3.12.0/evidence/vector_retrieval/V312-52-REPORT.md) §2-3 |
| Internal Vector Retrieval — 固定 GMP audit question fixture 与确定性 top-k | N/A | DEFERRED → v3.13 | 无 ≥5 docs 种子 + 已知 query + 断言 (doc_id, similarity, chunk_hash) 顺序的测试；Issue #4236 to open |
| Internal Vector Retrieval — `rebuild_flat_index` 持久化索引 + 重建前后稳定 (count/hash/top-k) | N/A | DEFERRED → v3.13 | `rebuild_flat_index` 仅写 metadata，`let _index = FlatIndex::build(...)` 被丢弃（compiler 警告）；Issue #4235 to open |
| Internal Vector Retrieval — dimension drift / empty index / model-name fail-closed | N/A | DEFERRED → v3.13 | `upsert_embedding` 不校验 dimension；`vector_search` 对空索引返回 `Ok(vec![])` 而非错误；Issue #4237 to open |
| SQL-backed Graph Projection | N/A | DONE / 受控 | BFS 子图、EvidenceBundle、GraphStats；不宣称通用图数据库 |
| Row-Level Security / Column Privileges | DONE | DONE / 持续硬化 | v3.11 主路径能力；GMP 权限矩阵仍需 v3.12 生产路径验证 |
| 存储过程 | DONE / 受控基础功能 | DONE / 受控基础功能 | V312-55 PR [#4259](http://192.168.0.252:3000/openclaw/sqlrustgo/pulls/4259) + [#4262](http://192.168.0.252:3000/openclaw/sqlrustgo/pulls/4262) + [#4264](http://192.168.0.252:3000/openclaw/sqlrustgo/pulls/4264): `CREATE/DROP/SHOW PROCEDURE` + `CALL` + `IN` 参数 + 过程内确定性 SQL 通过 gate, OUT/INOUT/DEFINER/Dynamic SQL 显式 defer v3.13, 证据: [V312-55-VERIFICATION.md](docs/releases/v3.12.0/evidence/procedure_trigger/V312-55-VERIFICATION.md) |
| 触发器 | DONE / 受控基础功能 | DONE / 受控基础功能 | V312-55 PR [#4262](http://192.168.0.252:3000/openclaw/sqlrustgo/pulls/4262) + [#4264](http://192.168.0.252:3000/openclaw/sqlrustgo/pulls/4264): `BEFORE/AFTER INSERT/UPDATE/DELETE` row trigger + NEW/OLD 上下文 + 事务一致性 + 递归限制 + 权限模型 通过 gate, DEFINER 显式 defer v3.13, 证据: [V312-55-VERIFICATION.md](docs/releases/v3.12.0/evidence/procedure_trigger/V312-55-VERIFICATION.md) |
| 通用复制 / 分布式 | UNSUPPORTED | UNSUPPORTED | 不作为 v3.12 当前目标 |
| 完整 MySQL 5.7 替代 | PARTIAL | OPEN / 非当前声明 | 需要 SQLLogicTest、TPC-H correctness、wire、LOAD DATA、recovery、upgrade 等全部闭环；[#4220](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4220) 要求 README 不得把悬空 PARTIAL 当生产能力 |
| 通用向量数据库 / 图数据库 | UNSUPPORTED | DEFERRED | v4.0.0 方向，不是 v3.12 对外声明 |

## TPC-H 与性能基准

### 口径说明

TPC-H 的 `22/22 completed` 表示 22 个 query 都跑完且没有 OOM/panic；它不自动等于跨引擎结果完全正确。若要宣称 correctness，需要 row count、canonical SHA256 和外部 oracle 对比。

### 历史与当前 TPC-H 状态

| 版本 / 场景 | 状态 | 可声明内容 | 不可声明内容 |
|---|---|---|---|
| v3.9.0 SF=0.1 | DONE / 历史基准 | SF=0.1 22 query 历史性能基准存在 | 不能外推为 SF=1/SF=10 生产能力 |
| v3.10.0 并行执行优化 | DONE / 历史优化 | 部分 TPC-H query 在大数据上有并行和 fast-load 优化记录 | 不能宣称所有 query 线性加速 |
| v3.11.0 SF=1 | DONE with correctness follow-up | 22/22 completed，519.15s，0 OOM，0 panic | 不能宣称 PostgreSQL/MySQL SHA256 零差异 |
| v3.12.0 SF=1 close-out | 受控 / PARTIAL→DONE | row count baseline + 8 zero-row per-query binding manifest 已闭环 (22/22 可运行, 16 行结果, Q8 #4274 + Q16 #4278 在 v3.12 working-tree FIXED 无 v3.13 defer, 6 zero-row 由 #4272 治理) | cross-engine SHA256 闭环 由 [#4221](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4221) → 子 issue [#4272](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4272) + [#4273](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4273)~[#4280](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4280) 收口 |
| v3.12.0 SF=10 harness | PARTIAL / blocker | harness 可运行；当前不是完整 60M lineitem 生产证据 | 不能宣称真实 SF=10 全量 parity；[#4020](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4020)、[#4217](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4217) 继续整改 |
| v3.12.0 Bulk-load SF=10 | PARTIAL / OPEN | runner/gate/evidence 记录 3/8 表 match，5/8 大表未完成；见 [#4020](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4020)、[#4217](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4217) | 不能宣称 8 表真实 bulk-load 完成 |

### v3.11.0 SF=1 关键数据

| 指标 | 结果 |
|---|---:|
| 数据规模 | lineitem 6,001,215 行，8 表约 8.66M 行 |
| 加载路径 | BINT mmap fixture，不是 LOAD DATA |
| 执行结果 | 22/22 query completed |
| 总耗时 | 519.15s |
| 稳定性 | 0 OOM / 0 panic |
| 结果边界 | 6 个 zero-row query 仍需外部 oracle correctness 验证（Q8 #4274 + Q16 #4278 已 FIXED in v3.12 working-tree，剩 6 zero-row 由 #4272 治理） |

### v3.12.0 SF=10 当前边界

[#4018 evidence](docs/releases/v3.12.0/evidence/issue-4018/4018_evidence.md) 明确说明：

- SF=10 harness 和 gate 基础设施已交付。
- 最新证据中 3/8 TPC-H SF=10 表已完成 `parity=match`。
- 剩余大表受 FileStorage 全表重序列化导致的吞吐瓶颈阻塞，需 #4217 继续整改。
- 还没有 8/8 表完整 row-count/hash parity，因此不能声明 SF=10 生产完成。

[#4020 evidence](docs/releases/v3.12.0/evidence/issue-4020/4020_evidence.md) 明确说明：

- Bulk-load runner 存在。
- schema creation 失败是旧阶段问题，最新状态已推进到 3/8 表 `parity=match`。
- 5/8 大表仍未完成，不能宣称 8 表真实 bulk-load 完成。

因此 README 不再把 SF=10 写成完成状态。

## GMP / RAG / Vector / Graph

v3.12.0 的 GMP 方向是“受控内审检索系统数据库”，不是通用数据库产品宣传。

| 模块 | 当前状态 | 证据 |
|---|---:|---|
| GMP schema / version / chunk / relation / audit | DONE | [V312-02](docs/releases/v3.12.0/v312-02-gmp-schema-report.md) |
| Idempotent GMP markdown ingestion | DONE | [V312-03](docs/releases/v3.12.0/v312-03-gmp-ingestion-report.md) |
| Embedding provider | DONE / 受控 | [V312-04](docs/releases/v3.12.0/v312-04-embedding-provider-report.md) |
| Hybrid retrieval | DONE / 受控 | [V312-05](docs/releases/v3.12.0/v312-05-hybrid-retrieval-report.md) |
| SQL-backed graph projection | DONE / 受控 | [V312-06](docs/releases/v3.12.0/v312-06-graph-projection-report.md) |
| RAG evidence bundle | DONE / 受控 | [V312-07](docs/releases/v3.12.0/v312-07-rag-evidence-bundle-report.md) |
| GMP compliance audit controls | 受控 / 子项已闭环 | [GMP 合规矩阵](docs/releases/v3.12.0/GMP_COMPLIANCE_MATRIX.md)、[V312-08](docs/releases/v3.12.0/v312-08-compliance-audit-report.md)、[V312-53](docs/releases/v3.12.0/evidence/gmp_compliance/V312-53-REPORT.md)；子项拆分见 README 行 141-145 (CRUD audit DONE；合规操作/篡改检测/5×12 矩阵 显式 DEFERRED → v3.13)，由 [#4226](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4226) 收口 |

允许的产品声明：SQLRustGo v3.12 支持受控 GMP 内审检索工作负载中的关系存储、chunk、embedding、audit trail、evidence relation、hybrid retrieval 和 SQL-backed graph projection。

禁止的产品声明：v3.12 是通用独立向量数据库、通用图数据库或完整 MySQL 5.7 替代品。

## 测试与质量门禁

v3.12.0 采用分层测试体系，避免把慢测试、性能测试、coverage 和 SOAK 混成一个不可维护的总门禁。

| 层级 | 类型 | 当前目标 |
|---|---|---|
| L0 | 单元测试 | 受影响 crate 和核心 crate 必须 PASS |
| L1 | 集成测试 | SQL / storage / transaction / GMP 主路径可复跑 |
| L2 | E2E / wire / SQLLogicTest smoke | 失败必须 issue-linked，不允许静默 ignore |
| L3 | per-crate coverage | 统一命令、统一报告；低覆盖必须 owner/expiry/issue |
| L4 | 性能测试 | TPC-H、Sysbench、bulk-load、RAG/vector 单独产出趋势和 artifact |
| L5 | SOAK / crash / recovery | Beta/RC/GA 前单独验收，不作为每 PR 门禁 |

关键文档：

- [v3.12 TEST_PLAN](docs/releases/v3.12.0/TEST_PLAN.md)
- [综合测试框架与覆盖率基线](docs/releases/v3.12.0/COMPREHENSIVE_TEST_FRAMEWORK_AND_COVERAGE_BASELINE.md)
- [v3.12 Scope Table](docs/releases/v3.12.0/SCOPE_TABLE_v3.12.md)
- [SQLLogicTest gate 报告](docs/releases/v3.12.0/sqllogictest-oracle-gate-report.md)

## 文档资源

| 文档 | 说明 |
|---|---|
| [CHANGELOG](CHANGELOG.md) | 版本变更历史 |
| [RELEASE_NOTES](RELEASE_NOTES.md) | 发行说明索引 |
| [v3.11 综合评估](docs/releases/v3.11.0/COMPREHENSIVE_ASSESSMENT_REPORT.md) | v3.11 GA 可信度、边界和遗漏测试 |
| [v3.11 TPC-H SF=1 报告](docs/releases/v3.11.0/TPCH_SF1_22_22_PASS_REPORT.md) | SF=1 22/22 可运行性证据 |
| [v3.12 README](docs/releases/v3.12.0/README.md) | v3.12 产品契约 |
| [v3.12 DEVELOPMENT_PLAN](docs/releases/v3.12.0/DEVELOPMENT_PLAN.md) | v3.12 开发计划 |
| [v3.12 FEATURE_CHECKLIST](docs/releases/v3.12.0/FEATURE_CHECKLIST.md) | v3.12 功能清单 |
| [v3.12 PARTIAL 功能整改 Issue 计划](docs/releases/v3.12.0/PARTIAL_FEATURE_REMEDIATION_ISSUE_PLAN.md) | README 中 PARTIAL/OPEN 功能的整改归属、issue 和关闭边界 |
| [v3.12 STAGE](docs/releases/v3.12.0/STAGE.yaml) | v3.12 阶段 SSOT |
| [governance](docs/governance/) | 真实性、门禁、Issue 关闭和多 AI 协作规范 |

## 历史版本

| 版本 | 阶段 / 定位 | 说明 |
|---|---|---|
| v3.12.0 | ALPHA | GMP 内审检索数据库 + v3.11 弱项硬化 |
| v3.11.0 | GA | 简单生产/受控场景候选；TPC-H SF=1 可运行性突破；仍有 correctness 和生产边界 |
| v3.10.0 | GA / 历史 | 并行执行器、fast-load、MySQL 5.7 替代方向推进；部分测试增强未成为阻断门禁 |
| v3.9.0 | GA / 历史 | TPC-H SF=0.1、长稳、治理真实性修复的重要版本 |
| v3.8.0 | GA / 历史 | canonical server 入口、WAL/recovery/文档治理演进 |
| v3.7.0 | GA / 历史 | truthfulness framework 和覆盖率争议治理成形 |
| v3.6.0 | GA / 历史 | 早期覆盖率和 Beta/GA 口径漂移需以后续版本纠正阅读 |

## 贡献指南

```bash
# 格式
cargo fmt --check --all

# 构建
cargo build --all-features

# 测试
cargo test --all-features

# Clippy
cargo clippy --all-features -- -D warnings

# 文档链接
bash scripts/gate/check_docs_links.sh
```

提交文档或关闭 Issue 前，请遵循：

- [ADR-001 Truthfulness Framework](docs/governance/adr/ADR-001-truthfulness-framework.md)
- [Anti-Fabrication Policy](docs/governance/ANTI_FABRICATION_POLICY.md)
- [Issue Closing Verification](docs/governance/ISSUE_CLOSING_VERIFICATION.md)
- [Document Correction Rules](docs/governance/DOC_CHECK_CORRECTION_RULES.md)
- [ADR-008 Test Claim Transparency](docs/governance/adr/ADR-008-test-claim-transparency.md)
- [ADR-014 Multi-AI Coordination](docs/governance/adr/ADR-014-multi-ai-coordination.md)

## 许可证

MIT License，详见 [LICENSE](LICENSE)。
