# SQLRustGo v3.11.0 开发计划 — 23 任务详细分解

> **Last update**: 2026-07-18
> **创建日期**: 2026-07-13
> **创建人**: openclaw
> **分支**: `develop/v3.11.0` (待从 `develop/v3.10.0` 创建)
> **父文档**: [`V311_VERSION_PLAN.md`](V311_VERSION_PLAN.md)
> **目标**: 23 任务 / ~1156h / 4 阶段
> **债务基线**: v3.10.0 LEGACY_DEBT_CLOSURE_TRACKING_REPORT (23 OPEN 项)

---

## 0. 总体任务清单

| 编号 | 任务 | 工作量 | 优先级 | 阶段 |
| --- | --- | --- | --- | --- |
| V311-01 | F-23 Clustered Index 主路径集成 | 80h | P0 | ALPHA-BETA |
| V311-02 | F-24 Adaptive Hash Index 主路径集成 | 60h | P0 | ALPHA-BETA |
| V311-03 | F-25 Change Buffer 主路径集成 | 40h | P0 | ALPHA-BETA |
| V311-04 | F-26 Double-Write Buffer 主路径集成 | 50h | P0 | ALPHA-BETA |
| V311-05 | F-29 Row-Level Security 主路径集成 | 40h | P1 | BETA |
| V311-06 | F-31 Performance Schema instrumentation hooks | 30h | P1 | BETA |
| V311-07 | F-32 MySQL Admin (sqlrustgo-admin ↔ mysql-server 集成) | 30h | P1 | BETA |
| V311-08 | F-35 Password Rotation 主路径集成 | 20h | P1 | BETA |
| V311-09 | F-36 列级权限实现 | 40h | P0 | ALPHA-BETA |
| V311-10 | F-30 CREATE SEQUENCE 实现 | 20h | P1 | BETA |
| V311-11 | F-03 GIS 空间数据类型 (POINT + WITHIN) | 80h | P1 | BETA |
| V311-12 | F-27 Table Compression (LZ4/zstd) | 50h | P1 | BETA |
| V311-13 | SEM-3 ALTER TABLE RENAME/MODIFY 完整 | 20h | P0 | ALPHA |
| V311-14 | SEM-4 覆盖率 ≥85% | 60h | P0 | BETA-RC |
| V311-15 | Q4 相关子查询 Hash Semi Join 算子 | 80h | P0 | ALPHA-BETA |
| V311-16 | 子查询去相关 (decorrelation) optimizer pass | 60h | P1 | BETA |
| V311-17 | Hash Anti Join 算子 (NOT EXISTS / NOT IN) | 40h | P1 | BETA |
| V311-18 | CTE 物化 (WITH ... AS (SELECT) 物化) | 30h | P1 | BETA |
| V311-19 | Extension Crate 决策实施 (5 删 + 3 归档 + 1 集成) | 84h | P1 | ALPHA-BETA |
| V311-20 | TPC-H SF=1.0 baseline (#3431) | 80h | P0 | BETA-RC |
| V311-21 | 168h SOAK v3.11.0 | (Hermes 协作) | P1 | RC |
| V311-22 | 文档架构整理 (合并 5 个 plans → 3 个 plans) | 12h | P2 | ALPHA |
| **合计** | 23 任务 | **~1156h** | — | — |

> 注: Q4 优化 4 项 (V311-15~18) 总 210h 是 v3.10.0 实测发现的核心瓶颈；F-XX 集成 9 项总 400h 是债务清零主体。

---

## 1. ALPHA 阶段任务 (P0，~3 周)

### V311-01: F-23 Clustered Index 主路径集成

**背景**: `tests/clustered_index_test.rs` 7/7 PASS，但生产路径 0 集成。`ARCHITECTURE_DEBT_ANALYSIS.md` §5.1 已规划 ~800 行重构。

**目标**: 让 `CREATE TABLE` 支持 `ENGINE=InnoDB CLUSTERED`，行存储在 B+ Tree 叶子节点

**实施步骤**:
1. (16h) 设计 ClusteredIndex schema (heap tuple 在叶子节点)
2. (24h) 实现 `ClusteredIndex::insert/delete/scan` API
3. (16h) 集成到 `ExecutionEngine::execute_create_table` 和 DML executor
4. (16h) 测试: 主路径 INSERT/SELECT/UPDATE/DELETE 走 ClusteredIndex
5. (8h) 性能对比: ClusteredIndex vs HeapTable

**验收**:
- `cargo test cluster_index_main_path_test` PASS
- `docs/releases/v3.11.0/perf/CLUSTERED_INDEX_PERF.md` 性能对比
- `docs/governance/debt/debt-registry.yaml`: F-23 state VERIFIED → CLOSED

### V311-09: F-36 列级权限实现

**背景**: F-36 列级权限 0 代码，是 MySQL 5.7 替代关键缺失。

**目标**: 实现 `GRANT SELECT(col1, col2) ON table TO user` 语法

**实施步骤**:
1. (8h) Parser: `GRANT ... ON (col_list)` 语法
2. (8h) AuthManager: 列权限 catalog (HashMap<TableName, Vec<ColName>>)
3. (16h) Executor: `SELECT` 时检查列权限
4. (4h) Wire protocol: 错误信息规范化
5. (4h) 测试: 8 个 unit + 4 个 integration

**验收**:
- `cargo test column_privilege_test` 12/12 PASS
- MySQL client `mysql -u user -p -e "SELECT col1 FROM t"` 行为正确
- F-36 state DEFERRED → CLOSED

### V311-13: SEM-3 ALTER TABLE RENAME/MODIFY 完整

**背景**: v3.10.0 已加 MODIFY 关键字到 lexer (PR #3773)，但 executor 仅 stub。

**目标**: 实现 RENAME TABLE / RENAME COLUMN / MODIFY COLUMN 实际行为

**实施步骤**:
1. (4h) Parser: ALTER TABLE 语法扩展（已有基础）
2. (8h) Executor: rename_table 实际修改 catalog
3. (4h) Executor: rename_column / modify_column 实际修改 schema
4. (4h) 测试: 6 个 ignore 测试 unignore

**验收**:
- C-4b / C-4c / C-4d 全部 PASS
- SEM-3 state IN_PROGRESS → CLOSED

### V311-15: Q4 相关子查询 Hash Semi Join 算子 (核心)

**背景**: Issue #3792 实测：Q4 占总执行时间 96% (450K orders × 3M lineitem = 1.35 万亿次 naive 比较)

**目标**: 实现 Hash Semi Join，将 Q4 性能从 14.5 分钟降到 < 5 分钟

**实施步骤**:
1. (16h) 设计 Hash Semi Join API (`HashSemiJoin::new(build_keys, probe_keys, build_side, probe_side)`)
2. (24h) 实现 HashTable 构建（probe phase 用 bloom filter 优化）
3. (16h) 集成到 VolcanoExecutor：识别 `WHERE EXISTS (SELECT ...)` 模式
4. (16h) 性能测试：TPC-H Q4 @ SF=1 / SF=3 实测
5. (8h) 回归测试：所有 EXISTS / NOT EXISTS 查询不走 Semi Join 时不退化

**验收**:
- TPC-H Q4 @ SF=3: **< 5 分钟**（v3.10.0 = 14.5 分钟，加速 ≥ 2.9x）
- `cargo test hash_semi_join_test` 10/10 PASS
- 现有 Q1/Q3/Q5 等非 EXISTS 查询不退化

### V311-19 (subset): Extension Crate 删除 (5 项, ~32h)

**背景**: 11 个 extension crate SCOPE_DEFERRED，需产品决策。v3.11.0 决定 5 删 + 3 归档。

**实施步骤** (删除/归档 5 项):
1. (8h) 删除 `distributed` (8K LOC, 0 集成)
2. (4h) 删除 `agentsql` (4.4K LOC, 0 集成)
3. (2h) 删除 `qmd-bridge` (700 LOC, 0 调用方)
4. (2h) 删除 `evidence-graph` (600 LOC, gate tool 独立)
5. (4h) 删除 `unified-query` + `unified-storage` (依赖 graph/vector)
6. (8h) 归档 `gmp` / `rag` / `graph` 到 `archive/v3.11/`（不参与 build）
7. (4h) 更新 `docs/governance/debt/debt-registry.yaml`: 5 SCOPE_DEFERRED → DELETED

**验收**:
- `cargo build --workspace` 不包含 8 个删除/归档 crate
- `docs/governance/debt/debt-registry.yaml`: 5 项 state SCOPE_DEFERRED → DELETED
- `docs/releases/v3.11.0/EXTENSION_CRATES_ARCHIVE.md`: 8 项归档说明

### V311-22: 文档架构整理

**目标**: 统一文档架构，删除 v3.10.0 多余 plans 文件

**实施步骤**:
1. (2h) 删除 `plans/V310_VERSION_PLAN.md` (内容已合并到 v3.11 VERSION_PLAN)
2. (2h) 删除 `plans/V310_DEVELOPMENT_PLAN.md` (内容已合并到 V311_DEVELOPMENT_PLAN)
3. (2h) 删除 `plans/V310_ISSUES_PLAN.md` (内容已合并到 V311_DEBT_CLOSURE_PLAN)
4. (2h) 删除 `plans/V310_10_COVERAGE_PLAN.md` (合并到 V311-14)
5. (2h) 删除 `plans/V310_CLI_BINARY_PLAN.md` (合并到 V311-07)
6. (2h) 创建 `plans/INDEX.md` (v3.11.0) 列出 4 个 plans

**验收**:
- `docs/releases/v3.11.0/plans/` 仅含 4 个文件：VERSION_PLAN / DEVELOPMENT_PLAN / DEBT_CLOSURE_PLAN / DOCS_RESTRUCTURE_PLAN
- `ISOLATED_MODULES.md` (root) 更新为 v3.11.0

---

## 2. BETA 阶段任务 (P0+P1，~4 周)

### V311-02: F-24 Adaptive Hash Index 主路径集成

**背景**: `tests/adaptive_hash_index_test.rs` 7/7 PASS，但 0 主路径集成。

**目标**: 监控热点 page，自动建 hash index 加速等值查询

**实施步骤**:
1. (12h) 设计 AdaptiveHashIndex 监控器（LRU + 热度统计）
2. (24h) 实现 hash index 自动构建逻辑
3. (16h) 集成到 BTreeIndex::scan 路径（先查 AHI，miss 则走 B+ Tree）
4. (8h) 测试 + 性能对比

**验收**: `cargo test adaptive_hash_main_path_test` PASS, F-24 state VERIFIED → CLOSED

### V311-03: F-25 Change Buffer 主路径集成

**背景**: `tests/change_buffer_test.rs` 5/5 PASS，但 0 主路径集成。

**目标**: 非唯一二级索引的变更延迟 merge

**实施步骤**:
1. (8h) 设计 Change Buffer 数据结构（per-page pending ops）
2. (16h) 实现 INSERT/UPDATE/DELETE 写 Change Buffer
3. (12h) 实现 page 读取时合并 Change Buffer
4. (4h) 测试

**验收**: `cargo test change_buffer_main_path_test` PASS, F-25 state VERIFIED → CLOSED

### V311-04: F-26 Double-Write Buffer 主路径集成

**背景**: `tests/double_write_buffer_test.rs` 6/6 PASS，但 fsync 未实际接。

**目标**: 真实 fsync 实现 partial page write 保护

**实施步骤**:
1. (12h) 实现 `FileStorage::write_page` 时同时写 doublewrite 区
2. (16h) 实现 crash recovery 时从 doublewrite 恢复
3. (16h) 测试: kill -9 后 recovery 正确
4. (6h) 性能测试

**验收**: T-20 crash recovery 仍 PASS, F-26 state VERIFIED → CLOSED

### V311-05: F-29 Row-Level Security 主路径集成

**背景**: `tests/row_level_security_test.rs` 6/6 PASS，但 0 主路径集成。

**目标**: 实现 `CREATE POLICY` + DML 时检查 row-level predicate

**实施步骤**:
1. (12h) Parser: CREATE POLICY 语法
2. (12h) Catalog: Policy storage
3. (12h) Executor: DML 时注入 row filter
4. (4h) 测试

**验收**: `cargo test row_level_security_main_path_test` PASS, F-29 state VERIFIED → CLOSED

### V311-06: F-31 Performance Schema instrumentation hooks

**目标**: 在执行引擎关键路径插入 instrumentation，让 Performance Schema 可观测

**实施步骤**:
1. (12h) 定义 `InstrumentationHook` trait
2. (12h) 在 VolcanoExecutor 各算子插入 hook
3. (6h) 测试

**验收**: `cargo test performance_schema_instrument_test` PASS, F-31 state VERIFIED → CLOSED

### V311-07: F-32 MySQL Admin 集成

**背景**: `sqlrustgo-admin` binary 已存在 (`crates/admin/`)，但与 mysql-server 解耦。

**目标**: 让 `mysqladmin` 命令通过 mysql wire protocol 连接 sqlrustgo-mysql-server

**实施步骤**:
1. (12h) mysql-client 复用: `crates/admin/src/backup.rs` 走 mysql-client
2. (8h) `sqlrustgo-admin status` 命令实现
3. (6h) 测试: end-to-end mysqladmin → mysql-server

**验收**: `cargo test mysqladmin_e2e_test` PASS, F-32 state PARTIAL → CLOSED

### V311-08: F-35 Password Rotation 主路径集成

**目标**: AuthManager 支持 password expiration + rotation policy

**实施步骤**:
1. (8h) Catalog: password expiration timestamp
2. (8h) Executor: 连接时检查过期
3. (4h) 测试

**验收**: `cargo test password_rotation_main_path_test` PASS, F-35 state VERIFIED → CLOSED

### V311-10: F-30 CREATE SEQUENCE 实现

**目标**: 实现 `CREATE SEQUENCE seq START 1 INCREMENT 1; SELECT nextval('seq')`

**实施步骤**:
1. (8h) Parser: CREATE SEQUENCE 语法
2. (8h) Catalog: Sequence storage
3. (4h) Executor: nextval / currval 函数

**验收**: `cargo test sequence_test` PASS, F-30 state DEFERRED → CLOSED

### V311-11: F-03 GIS 空间数据类型 (POINT + WITHIN)

**目标**: 实现 POINT 类型 + ST_WITHIN 函数（最小化实现）

**实施步骤**:
1. (16h) Type: Point (x, y f64)
2. (16h) Parser: Point literal / column type
3. (24h) Executor: ST_WITHIN 函数
4. (16h) Index: R-Tree（可选）
5. (8h) 测试

**验收**: `cargo test gis_basic_test` PASS, F-03 state DEFERRED → CLOSED

### V311-12: F-27 Table Compression (LZ4/zstd)

**背景**: v3.10.0 仅 RLE，缺 LZ4/zstd 真实压缩

**目标**: 让 `CREATE TABLE ... COMPRESSION='LZ4'` 实际生效

**实施步骤**:
1. (8h) 集成 `lz4_flex` / `zstd` crate
2. (24h) FileStorage::write_page 压缩
3. (12h) FileStorage::read_page 解压
4. (6h) 测试 + 性能对比

**验收**: `cargo test compression_lz4_test` PASS, F-27 state VERIFIED → CLOSED

### V311-14: SEM-4 覆盖率 ≥85%

**目标**: 三 crate 覆盖率 ≥85% (executor / parser / storage)

**实施步骤**:
1. (16h) 分析 v3.10.0 coverage gap（哪些 module < 80%）
2. (24h) 为核心算子 (Hash Join, Aggregation, Sort) 补 unit test
3. (16h) 修正 cargo llvm-cov 测量方法统一（per-crate, 见 COVERAGE_TESTING_METHODOLOGY.md）
4. (4h) CI 集成

**验收**: `cargo llvm-cov test -p <crate> --no-fail-fast` 3 crate 均 ≥85%（per-crate, 不是 `--workspace`）

### V311-16: 子查询去相关 (decorrelation) optimizer pass

**目标**: 实现 optimizer pass，将相关子查询重写为 JOIN

**实施步骤**:
1. (16h) 设计: 检测相关子查询模式
2. (24h) 实现: 重写为 Semi Join / Inner Join
3. (12h) 测试 + 回归

**验收**: TPC-H Q2, Q4, Q17, Q20, Q21 性能不退化

### V311-17: Hash Anti Join 算子 (NOT EXISTS / NOT IN)

**目标**: 实现 Hash Anti Join 算子

**实施步骤**:
1. (12h) 设计 HashAntiJoin API
2. (16h) 实现 + bloom filter 优化
3. (8h) VolcanoExecutor 集成
4. (4h) 测试

**验收**: TPC-H NOT EXISTS 查询性能提升

### V311-18: CTE 物化

**目标**: WITH ... AS (SELECT) 物化中间结果，避免重复计算

**实施步骤**:
1. (8h) 设计 CTE materialization flag
2. (16h) 实现: 第一次执行缓存结果
3. (6h) 测试

**验收**: `cargo test cte_materialize_test` PASS

### V311-19 (subset): F-32 Admin 与 mysql-server 集成

**目标**: sqlrustgo-admin 与 mysql-server 通过 wire protocol 通信

**实施步骤**:
1. (12h) crates/admin 引入 mysql-client
2. (12h) backup/restore 走 mysql wire protocol
3. (6h) e2e 测试

**验收**: admin 命令可通过 mysql client 连接到 server 执行

---

## 3. RC 阶段任务 (~3 周)

### V311-20: TPC-H SF=1.0 baseline (#3423)

**目标**: TPC-H 22/22 @ SF=1 全部 PASS

**实施步骤**:
1. (24h) 生成 TPC-H SF=1 fixture (~1.1 GB)
2. (24h) 运行 22 queries，定位 fail/regression
3. (24h) 修复发现的问题
4. (8h) 文档化结果

**验收**: `cargo run --release --bin tpch_runner -- --sf 1 --queries 22` 22/22 PASS

### V311-21: 168h SOAK v3.11.0 (#3648, Hermes 协作)

**目标**: 168 小时连续运行 0 error

**验收**: Issue #3266 (168h SOAK) v3.11.0 PASS

### V311-14 验证: 覆盖率 ≥85% CI

**目标**: CI gate `check_coverage.sh` 验证 3 crate 均 ≥85%（使用 per-crate `cargo llvm-cov test -p <crate>`）

---

## 4. 阶段工时汇总

| 阶段 | 任务数 | 工时 | 占比 |
| --- | --- | --- | --- |
| ALPHA | 6 | 280h | 26% |
| BETA | 13 | 530h | 48% |
| RC | 3 | 280h | 26% |
| **合计** | **22** | **1090h** | **100%** |

> 注: 实际投入约 1236h（含 buffer），约 1.5 人月 × 1 月

---

## 5. 与 v3.10.0 V310-12 (跨版本债) 的关系

v3.10.0 V310-12 (跨版本债) 中提到的 M-5/M-6/H-2 三项:

- **M-5 INT-2 `parallel_degree`**: v3.10.0 已通过 PR #3767 闭环 ✅
- **M-6 INT-3 stored_proc expr 重构**: v3.10.0 INT-3 已闭环 ✅
- **H-2 (ARCH-3)**: v3.10.0 ARCH-3 95% 闭环 → v3.11.0 RC 阶段完成 100%

故 v3.11.0 不再继承 V310-12 任何债务。

---

## 6. 参考资料

- `docs/governance/debt/debt-registry.yaml` — SSOT（v3.11.0 必达 0 OPEN）
- `docs/releases/v3.10.0/LEGACY_DEBT_CLOSURE_TRACKING_REPORT.md` — 23 OPEN 项基线
- `docs/releases/v3.10.0/ARCHITECTURE_DEBT_ANALYSIS.md` — F-XX 集成设计
- `docs/releases/v3.10.0/PARALLEL_EXECUTOR_OPTIMIZATION.md` — Q4 性能瓶颈
- `docs/releases/v3.10.0/perf/PERFORMANCE_BASELINE.md` — 实测 baseline

---

*Created: 2026-07-13 (DRAFT stage init)*

### V311-23 (NEW from v3.10.0 SOAK): PERF-5 High-concurrency INSERT fix

| Field | Value |
| --- | --- |
| **Issue** | #3434 (Gitea 250) |
| **Debt** | `docs/governance/debt/debt-registry.yaml#perf/PERF-5` |
| **Discovered** | 2026-07-15 06:32 UTC during v3.10.0 168h SOAK |
| **Priority** | P1 |
| **Estimated** | 60h |

**Problem**: v3.10.0 server has high-concurrency INSERT failure:
- TPC-H Q1/Q6/Q12/Q14 rotation: WORKING (Q1 279ms @ 107K lineitem)
- OLTP SELECT/UPDATE/COUNT: 100% success
- OLTP INSERT INTO customer: 0% success (ERROR 2013 "Lost connection to server during query")
- Customer count remains at 1,124 throughout 1h of OLTP workload

**Server is stable** (10h+ uptime, 1.9GB RSS, 35% CPU, no crashes).
This is a connection management / lock contention issue under high concurrency.

**Tasks**:
1. (16h) Investigate root cause - reproduce locally with 4+ concurrent OLTP threads
2. (24h) Fix connection pool / lock contention - may require MVCC visibility fix
3. (12h) Add regression test in `tests/stress/concurrent_insert_test.rs`
4. (8h) Update SOAK orchestrator to include high-concurrency INSERT test

**Workaround (immediate)**: Reduce OLTP threads to 1, or increase `--max-connections` to 500+

**Acceptance**:
- 4-thread concurrent OLTP for 1 hour: 0% INSERT errors
- Customer count increases by ~2400 rows (80 inserts/min × 60 min × 4 threads / 8 expected per cycle)
- Server stays stable, no memory growth, no FD leak
