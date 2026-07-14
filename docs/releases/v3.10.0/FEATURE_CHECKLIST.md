# v3.10.0 Feature Checklist

> **Version**: v3.10.0
> **Status**: BETA stage (2026-07-13)
> **Owner**: @yinglichina8848 / Claude Code
> **Last update**: 2026-07-13 (Phase 0 implementation per DeepSeek feedback)
> **Related**: V310_ISSUES_PLAN.md, V310_DEVELOPMENT_PLAN.md, TEST_PLAN.md

This document enumerates **all features** in v3.10.0 (new + existing from v3.9.0),
with their test coverage status, source location, and issue tracker. It serves
as the authoritative reference for BETA feature gating and for `check_beta_gate.sh`
to verify feature-test pairing.

---

## 1. Status Legend

| Symbol | Meaning |
|--------|---------|
| ✅ **PASS** | 全部 tests PASS, feature 完整可用 |
| ⚠️ **PARTIAL** | 部分功能已实现 / 部分 tests PASS, 已知未完成项 |
| 🚧 **WIP** | 在 v3.10.0 实施中 (V310-0X 子任务) |
| ❌ **BROKEN** | feature 标记废弃 / tests 都 fail |
| 🚫 **DEFERRED** | 推迟到 v3.11.0+ |

---

## 2. v3.10.0 核心目标 (P0)

| ID | 特性 | V310 子任务 | 估时 | 状态 | 验证 test | Issue |
|----|------|-----------|------|------|-----------|-------|
| **C-1** | DML 完整性 (INSERT/UPDATE/DELETE 子查询) | V310-01 (a-e) | 80h | 🚧 WIP | `tests/dml_integration_test.rs` (5 tests) | #3312 |
| **C-2** | UNION/INTERSECT/EXCEPT 集合操作 | V310-02 (a-c) | 45h | 🚧 WIP | `tests/union_set_operations_test.rs` (3 tests) | #3312 |
| **C-3** | ACID 事务正确性 (ROLLBACK MVCC) | V310-03 | 80h | ✅ PASS | `tests/savepoint_test.rs`, `tests/sem1_savepoint_test.rs`, `tests/mvcc_transaction_test.rs` | #3314 |
| **C-4** | ALTER TABLE 完整性 (RENAME/MODIFY) | V310-04 | 20h | ⚠️ PARTIAL | `tests/alter_table_test.rs` | #3747 |
| **C-5** | 真实崩溃恢复 (kill -9) + 24h SOAK | V310-05 | 80h | ✅ PASS | `tests/integration/issue_3257_wal_fallback_test.rs`, `tests/process_kill_crash_test.rs` | #3772, #3769 |
| **C-6** | CLI binary 统一 (sqlrustgo-cli) | V310-CLI plan Phase 1+2 | 40h | ✅ DONE | `tests/sqlrustgo_cli_soak_e2e_test.rs` | n/a |
| **C-7** | ARCH-2 双路径消除 (mysql-server/bench-cli) | V310-CLI plan Phase 3 | 24h | ✅ DONE | `tests/cross_path_consistency_test.rs` | n/a |
| **C-8** | Wired-SOAK DDL 修复 (PR1) | V310-06 | 40h | 🚧 WIP | `tests/integration/issue_3257_wal_fallback_test.rs` | #3722 |
| **C-9** | Catalog 4 层重构 (PR2) | V310-07 | 80h | 🚧 WIP | `tests/cbo_integration_test.rs` | #3723 |
| **C-10** | DDL 执行路径实现 (PR3) | V310-08 | 80h | 🚧 WIP | `tests/ddl_e2e_test.rs`, `tests/describe_table_test.rs` | #3724 |
| **C-11** | Wire 协议握手修复 (PR4) | V310-09 | 60h | 🚧 WIP | `tests/wire_protocol_smoke.rs`, `tests/mysql_wire_protocol_test.rs` | #3725 |
| **C-12** | 其他 ignore 测试 + 跨版本债 | V310-12 | 60h | 🚧 WIP | (all 49 `#[ignore]` in TEST_PLAN §4) | #3707, #3146, #3136 |

---

## 3. SQL 92 子集覆盖

| 类别 | 特性 | 状态 | 测试 | 备注 |
|------|------|------|------|------|
| **DDL** | CREATE TABLE | ✅ | `tests/ddl_e2e_test.rs` | |
| | DROP TABLE | ✅ | `tests/ddl_e2e_test.rs` | |
| | ALTER TABLE ADD/DROP COLUMN | ✅ | `tests/alter_table_test.rs` | |
| | ALTER TABLE RENAME | 🚧 WIP | `tests/alter_table_test.rs` | V310-04 |
| | ALTER TABLE MODIFY COLUMN | 🚧 WIP | `tests/alter_table_test.rs` | V310-04 (PR #3773 fixed MODIFY lexer) |
| | CREATE INDEX | ✅ | (via storage) | F-12 (clustered index) F-XX 路线图 |
| **DML** | INSERT (single row) | ✅ | (covered) | |
| | INSERT (multi-row) | ✅ | (covered) | |
| | INSERT ... SELECT | 🚧 WIP | `tests/dml_integration_test.rs` | V310-01a |
| | UPDATE ... SET | ✅ | (covered) | |
| | UPDATE ... SET col = (SELECT ...) | 🚧 WIP | `tests/dml_integration_test.rs` | V310-01b |
| | Multi-table UPDATE | 🚧 WIP | `tests/dml_integration_test.rs` | V310-01c |
| | DELETE | ✅ | (covered) | |
| | DELETE ... WHERE col IN (SELECT ...) | 🚧 WIP | `tests/dml_integration_test.rs` | V310-01d |
| | Multi-table DELETE | 🚧 WIP | `tests/dml_integration_test.rs` | V310-01e |
| | REPLACE | ✅ | `tests/replace_test.rs` | |
| **DQL** | SELECT basic | ✅ | (covered) | |
| | WHERE | ✅ | (covered) | |
| | GROUP BY | ✅ | `tests/aggregate_functions_test.rs` | |
| | ORDER BY ASC/DESC | ✅ | `tests/operators/order_by_desc.rs` | |
| | LIMIT/OFFSET | ✅ | `tests/limit_clause_test.rs` | |
| | DISTINCT | ✅ | `tests/distinct_test.rs` | |
| | JOIN (INNER/LEFT/RIGHT/FULL) | ✅ | `tests/operators/multi_join_3_table.rs` | |
| | Subquery (IN/EXISTS/scalar) | ✅ | `tests/operators/exists.rs` | EXISTS correlated 还有 2 个 `#[ignore]` |
| | CTE (WITH) | ✅ | `tests/cte_e2e_test.rs` | |
| | Window Functions | ✅ | `tests/operators/aggregate_*` | |
| | UNION | ⚠️ PARTIAL | `tests/union_set_operations_test.rs` | INTERSECT/EXCEPT `#[ignore]` (3 tests) |
| | INTERSECT | 🚧 WIP | `tests/union_set_operations_test.rs` | V310-02a, 1 test |
| | EXCEPT | 🚧 WIP | `tests/union_set_operations_test.rs` | V310-02b, 1 test |
| | UNION ORDER BY/LIMIT | 🚧 WIP | `tests/union_set_operations_test.rs` | V310-02c, 1 test |
| **聚合** | COUNT/SUM/AVG/MIN/MAX | ✅ | `tests/aggregate_functions_test.rs` | |
| | GROUP BY HAVING | ✅ | (covered) | |
| | ROLLUP/CUBE | 🚧 WIP | `tests/rollup_cube_test.rs` | (1 test, 0 ignored) |
| **表达式** | CASE WHEN | ✅ | `tests/expression_operators_test.rs` | |
| | COALESCE/ISNULL | ✅ | `tests/null_semantics_test.rs` | |
| | CAST | ✅ | `tests/expression_operators_test.rs` | |
| | LIKE/NOT LIKE | ✅ | (covered) | |
| | IN list | ✅ | `tests/in_value_list_test.rs` | |
| | BETWEEN | ✅ | (covered) | |
| **事务** | BEGIN/COMMIT/ROLLBACK | ✅ | `tests/mvcc_transaction_test.rs` | |
| | SAVEPOINT | ✅ | `tests/savepoint_test.rs`, `tests/sem1_savepoint_test.rs` | |
| | ROLLBACK TO SAVEPOINT | ✅ | (covered) | |
| | Isolation levels (READ COMMITTED, etc) | ✅ | `tests/mvcc_transaction_test.rs` | |
| **存储过程** | CREATE PROCEDURE | ✅ | `tests/stored_proc_catalog_test.rs` | INT-3 expr 委托 done |
| | CALL | ✅ | `tests/stored_proc_catalog_test.rs` | |
| **触发器** | CREATE TRIGGER | ✅ | (in `crates/executor/src/trigger.rs`) | |
| **视图** | CREATE VIEW | ✅ | (covered) | |
| **分区** | PARTITION BY | ✅ | `tests/partition_e2e_test.rs` | |

---

## 4. MySQL 5.7 协议兼容

| 协议特性 | 状态 | 测试 | 备注 |
|---------|------|------|------|
| COM_QUERY | ✅ | `tests/wire_protocol_smoke.rs` | |
| COM_STMT_PREPARE | ✅ | `tests/mysql_wire_protocol_test.rs`, `tests/prepared_stmt_test.rs` | |
| COM_STMT_EXECUTE | ✅ | (covered) | |
| COM_STMT_CLOSE | ✅ | (covered) | |
| COM_PING | ✅ | (covered) | |
| COM_QUIT | ✅ | (covered) | |
| COM_INIT_DB | ✅ | (covered) | |
| Auth (mysql_native_password) | ✅ | (covered) | |
| Auth (caching_sha2_password) | ✅ | (covered) | |
| SSL/TLS | ✅ | (covered) | v3.9.0 default |
| Multi-statement | ⚠️ PARTIAL | `tests/multi_statement_test.rs` (1 ignored) | V310-12b (M-6 INT-3) |
| Local Infile | ✅ | `tests/load_local_infile_test.rs` | |
| 错误格式 (4-byte errno + SQL state) | ✅ | (covered) | |
| SHOW TABLES / DESCRIBE | ✅ | `tests/show_tables_test.rs`, `tests/describe_table_test.rs` | |

---

## 5. 性能特性 (TPC-H)

| TPC-H Query | v3.10.0 状态 | 测试 |
|------------|------------|------|
| Q1 (Pricing Summary) | ✅ | `tests/tpch_full_22_test.rs`, `tests/tpch_value_correctness_test.rs` |
| Q2 (Minimum Cost) | ✅ | (covered) |
| Q3 (Shipping Priority) | ✅ | (covered) |
| Q4 (Order Priority) | ✅ | (covered) |
| Q5 (Local Supplier) | ✅ | (covered) |
| Q6 (Forecasting Revenue Change) | ✅ | (covered) |
| Q7 (Volume Shipping) | 🚧 WIP | parser fix in V310-11a, partial 22/22 in SF=0.1 |
| Q8 (National Market Share) | 🚧 WIP | parser fix V310-11a |
| Q9 (Product Type Profit) | ⚠️ PARTIAL | (covered) 但有 perf 退化 (Q9 6.7x speedup in v3.9.0) |
| Q10 (Returned Item) | ✅ | (covered) |
| Q11 (Important Stock) | ✅ | (covered) |
| Q12 (Shipping Modes) | 🚧 WIP | parser fix V310-11a |
| Q13 (Customer Distribution) | ✅ | (covered) |
| Q14 (Promotion Effect) | ✅ | (covered) |
| Q15 (Top Supplier) | ✅ | (covered) |
| Q16 (Parts Supplier) | ✅ | (covered) |
| Q17 (Small Quantity Order) | ✅ | (covered) |
| Q18 (Large Volume Customer) | ✅ | (covered) |
| Q19 (Discounted Revenue) | ✅ | (covered) |
| Q20 (Potential Part Promotion) | ✅ | (covered) |
| Q21 (Suppliers Who Kept Orders) | ✅ | (covered) |
| Q22 (Global Sales Opportunity) | ✅ | (covered) |

**SF=0.1 (600k 行)**: 22/22 ✅ 已 PASS (v3.9.0)
**SF=1 (6M 行)**: 6/10 (Q7/Q8/Q9/Q12 部分 WIP, V310-11 子任务)

---

## 6. 存储 & 索引特性

| 特性 | 状态 | 测试 | 备注 |
|------|------|------|------|
| MemoryStorage (in-memory) | ✅ | (covered) | primary in-memory engine |
| FileStorage (disk-backed) | ✅ | (covered) | WAL + file-based persistence |
| B+Tree Index | ✅ | `tests/buffer_pool_test.rs`, `tests/ci/buffer_pool_test.rs` | |
| WAL (Write-Ahead Log) | ✅ | `tests/wal_integration_test.rs`, `tests/wal_tx_contract_test.rs` | T-19 disk I/O delay done |
| Group Commit | ✅ | (covered) | |
| Checkpointing | ✅ | (covered) | |
| Gap Locking (REPEATABLE READ) | ✅ NEW | `tests/gap_locking_test.rs` | v3.10.0 F-16 closed (PR #3788) |
| Clustered Index | 🚫 DEFERRED | `tests/clustered_index_test.rs` (7 tests, all isolated) | F-23, v3.11+ |
| Adaptive Hash Index | 🚫 DEFERRED | `tests/adaptive_hash_index_test.rs` (7 tests) | F-24, v3.11+ |
| Change Buffer | 🚫 DEFERRED | `tests/change_buffer_test.rs` (5 tests) | F-25, v3.11+ |
| Double-Write Buffer | 🚫 DEFERRED | `tests/double_write_buffer_test.rs` (6 tests) | F-26, v3.11+ |
| Table Compression | 🚫 DEFERRED | `tests/table_compression_test.rs` (8 tests, RLE only) | F-27, v3.11+ |
| Buffer Pool | ✅ | (covered) | |
| LRU Cache | ✅ | `tests/integration_buffer_pool.rs` | |
| Parallel Scan | ✅ NEW | `tests/parallel_scan_test.rs` | v3.10.0 I-12 closed (PR #3767) |
| Crash Recovery (WAL replay) | ✅ | `tests/recovery_scenarios_test.rs` | |
| Recovery Fuzzer | ✅ | `tests/recovery_fuzzer_test.rs` (1 ignored) | |

---

## 7. 执行引擎特性

| 特性 | 状态 | 测试 | 备注 |
|------|------|------|------|
| Sequential Execution | ✅ | (covered) | |
| Vectorized Tuple Update (VTU) | ✅ | `tests/vtu_ir_test.rs`, `tests/vtu_ir_modules_test.rs` | v3.10.0 ARCH-3 closed |
| Parallel Executor (intra-query) | ✅ OPTIMIZED | `tests/parallel_executor_test.rs`, `tests/parallel_semantic_tests.rs` | v3.10.0 I-12 closed (PR #3767), Issue #3792 optimization (PR #3370, #3829) — PARALLEL_MIN_ROWS=2M, 6 项优化, 实测 1M 行聚合 1.27x/join 1.08x |
| Parallel Group BY | ✅ NEW | `tests/parallel_group_by_test.rs` | v3.10.0 |
| Parallel Hash Join | ✅ NEW | `tests/parallel_hash_join_test.rs` | v3.10.0 |
| SIMD Batch Eval | ✅ NEW | `tests/simd_eval_test.rs` | v3.10.0 |
| CBO Cost Model | ✅ | `tests/cbo_integration_test.rs`, `tests/cost_optimizer_test.rs` | |
| FOR UPDATE (Gap Locking) | ✅ NEW | `tests/gap_locking_test.rs` | v3.10.0 F-16 |
| Multi-statement | ⚠️ PARTIAL | `tests/multi_statement_test.rs` (1 ignored) | V310-12b |
| CTE (Recursive) | ⚠️ PARTIAL | `tests/cte_e2e_test.rs` | WITH RECURSIVE 未实现, v3.10 范围外 |

### 7.1 Parallel Executor 优化验证 (Issue #3792)

| 优化项 | 状态 | 实测效果 |
|--------|------|----------|
| PARALLEL_MIN_ROWS=2M (统一) | ✅ | 三处定义一致 |
| 并行触发前置判断 (2x overhead gate) | ✅ | 小数据集自动回退串行 |
| 性能埋点 (tracing) | ✅ | partition_ms / filter_ms / merge_ms |
| Batch-Parallel 8K 行 chunks | ✅ | 调度开销减少 50%+ |
| Rayon 线程数动态配置 | ✅ | 每 query 独立线程数 |
| 自适应并行度选择 | ✅ | 1-8 线程自动调整 |

**实测加速比（4 线程，1M 行 lineitem）：**

| Query | 类型 | 加速比 |
|-------|------|--------|
| Q1 | 聚合 (10 列) | **1.27x** |
| Q3 | 3-way join | **1.08x** |
| Q5 | 6-way join | **1.10x** |

详见 `PARALLEL_EXECUTOR_OPTIMIZATION.md` 和 `perf/PERFORMANCE_BASELINE.md`。

---

## 8. 扩展 crate (extension crates) — 隔离状态

> 这些是 v3.10.0 工作区成员但**不在主路径 (server/client)** 中。按 DeepSeek 反馈, 仍保留作为扩展平台; 但应清楚标注为"extension", 避免与主路径混淆。

| Crate | 状态 | 主路径集成? | 备注 |
|-------|------|------------|------|
| `sqlrustgo-agentsql` | ⚠️ ISLAND | ❌ | "extension crate", F-32 mysqladmin 已迁到 sqlrustgo-admin |
| `sqlrustgo-gmp` | ⚠️ ISLAND | ❌ | evidence_graph, AI extension |
| `sqlrustgo-rag` | ⚠️ ISLAND | ❌ | RAG pipeline |
| `sqlrustgo-distributed` | ⚠️ ISLAND | ❌ | Raft, gRPC, 2PC; 与 MySQL 5.7 替代无关 |
| `sqlrustgo-graph` | ⚠️ ISLAND | ❌ | Cypher graph queries, v3.10 范围外 |
| `sqlrustgo-vector` | ⚠️ PARTIAL | ❌ | vector storage, 集成到 storage; v3.10 不主推 |
| `sqlrustgo-qmd-bridge` | ⚠️ ISLAND | ❌ | markdown→SQL bridge |
| `sqlrustgo-evidence-graph` | ⚠️ ISLAND | ❌ | evidence tracking |
| `sqlrustgo-unified-query` | ⚠️ ISLAND | ❌ | unified query API |
| `sqlrustgo-unified-storage` | ⚠️ ISLAND | ❌ | unified storage layer |
| `sqlrustgo-cache` | ✅ | ❌ (transitively via executor) | LRU cache, used by executor |
| `sqlrustgo-spill` | ⚠️ ISLAND | ❌ | grace hash join, F-XX 路线图 |
| `sqlrustgo-security` | ⚠️ ISLAND | ❌ | session/audit, deprecated (v3.10 不主推) |

> **结论**: 13 个 extension crates 全部不在主路径。v3.10.0 计划中它们是 **out-of-scope**, 走独立 release cycle。

---

## 9. Gate 集成 (与 STAGE_CONFIG.yaml + TEST_PLAN.md §5 对应)

| 阶段 | 必跑 gates | 必跑 features 验证 | 必跑 E2E |
|------|-----------|-------------------|---------|
| **DRAFT** | `check_docs_links.sh`, `cargo build` | `cargo build --all-features` (确保 7 个 bin + 43 个 crate 都能编译) | 无 |
| **ALPHA** | `check_alpha_v3.10.0.sh` | ALPHA 15 checks (Cargo build/test/fmt + A2 arch invariants + A3 required files + A4 branch sanity) | 无 |
| **BETA** | `check_beta_gate.sh` (新), 5 通用 gates, Cargo build/test/fmt/clippy | §3-§7 全部 "✅ PASS" 特性 + 49 `#[ignore]` ≤ 30 + 13 扩展 crate 编译 OK | E2E-01, 02, 04, 07, 08, 09 (6 个) |
| **RC** | BETA 全部 + 4 共享 gates + anti-fab + drift | §3-§7 全部 ✅ + `#[ignore]` ≤ 10 | + E2E-05, 06 |
| **GA** | RC 全部 + 3 治理 gates | §3-§7 全部 ✅ + `#[ignore]` = 0 in `tests/` (其余在 `tests/disabled/`) | + E2E-03, 10 (24h/168h SOAK) |

---

## 10. References

- `docs/governance/STAGE_CONFIG.yaml` — 5 阶段门禁 SSOT
- `docs/releases/v3.10.0/plans/V310_ISSUES_PLAN.md` — 12 个 V310 子 ISSUE 详述
- `docs/releases/v3.10.0/plans/V310_DEVELOPMENT_PLAN.md` — 26 任务 / 500h / 4 阶段
- `docs/releases/v3.10.0/plans/V310_10_COVERAGE_PLAN.md` — 覆盖率 ≥ 80% 计划
- `docs/releases/v3.10.0/plans/V310_CLI_BINARY_PLAN.md` — 二进制收敛
- `docs/releases/v3.10.0/TEST_PLAN.md` — 测试矩阵 + E2E + ignore 计划
- `docs/releases/v3.10.0/V310_TEST_BINARY_GATE_AUDIT_REPORT.md` — 源分析

---

*Last updated: 2026-07-13 by Claude Code (hermes-agent)*
*Phase 0 implementation per DeepSeek review feedback (local://attachment-1)*
