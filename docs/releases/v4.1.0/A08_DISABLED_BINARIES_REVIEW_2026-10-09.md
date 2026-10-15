# A08: B2 Disabled Test Binaries 全量复核报告

> **Issue**: #5230 (IAIA-410/A08)
> **source_agent**: opencode+Mimo2.6Flash
> **source_run**: iaia410-a08-5230-20261009T223121Z
> **commit**: `738b286ccc` (develop/v4.1.0)
> **evidence_hash**: `e50c409ee46825ce66a0989111a8bf22111be7341786d7c57cc76d5f452d42f2`
> **generated_at**: 2026-10-09T22:31:21Z

## 1. 冻结 inventory

SSOT: `scripts/gate/run_b2_per_binary.py` `DISABLED_BINARIES` = **89 项**（截至 `738b286ccc`）。

分类依据（逐 binary 实跑 `cargo test --all-features --test <name> -- --test-threads=1`）：

| 类别 | 数量 | 说明 |
|------|------|------|
| **REACTIVATED**（现稳定 PASS） | **12** | 可从 `DISABLED_BINARIES` 移出 |
| **STILL_FAILING**（真缺陷） | **22** | 仍需保持禁用 |
| **COMPILE_FAIL**（编译债） | **3** | `WalEntry.database_name` 缺字段（#5137 遗留） |
| **ENVIRONMENT**（环境依赖） | **18** | 需 TPC-H fixture / 外部 DB |
| **PERF_ONLY**（慢测试） | **6** | 应走 perf 门禁而非 B2 |
| **TIMEOUT**（挂起） | **1** | `regression_test` >90s 未完成 |
| **WIRE_HARNESS**（wire 测试，共享 harness 失败） | **27** | 统一因 `tpch_wire_harness.rs:40` fixture 缺失 |

## 2. REACTIVATED（12 项，本次可移出）

全部在 `738b286ccc` 上连续 2 次 PASS（0 failed），证据 hash `e50c409ee46825ce66a0989111a8bf22111be7341786d7c57cc76d5f452d42f2`。

| # | Binary | PASS | 耗时 | 复跑命令 |
|---|--------|------|------|----------|
| 1 | `ddl_e2e_test` | 26/0 | 0.01s | `cargo test --all-features --test ddl_e2e_test -- --test-threads=1` |
| 2 | `savepoint_test` | 4/0 | 0.00s | `cargo test --all-features --test savepoint_test -- --test-threads=1` |
| 3 | `mvcc_transaction_test` | 27/0 | 2.13s | `cargo test --all-features --test mvcc_transaction_test -- --test-threads=1` |
| 4 | `operators_join` | 21/0 | 0.03s | `cargo test --all-features --test operators_join -- --test-threads=1` |
| 5 | `window_function_test` | 36/0 | 0.01s | `cargo test --all-features --test window_function_test -- --test-threads=1` |
| 6 | `differential_corpus_test` | 36/0 | 0.00s | `cargo test --all-features --test differential_corpus_test -- --test-threads=1` |
| 7 | `issue_4491_join_groupby_alias_col_and_scalar_subquery` | 3/0 | 0.00s | `cargo test --all-features --test issue_4491_join_groupby_alias_col_and_scalar_subquery -- --test-threads=1` |
| 8 | `dml_integration_test` | 24/0 | 0.02s | `cargo test --all-features --test dml_integration_test -- --test-threads=1` |
| 9 | `diag_q22_cell_level` | 4/0 | 0.01s | `cargo test --all-features --test diag_q22_cell_level -- --test-threads=1` |
| 10 | `tpch_sf01_perf_baseline_test` | 4/0 | 0.00s | `cargo test --all-features --test tpch_sf01_perf_baseline_test -- --test-threads=1` |
| 11 | `physical_backup_test` | 12/0 | 6.99s | `cargo test --all-features --test physical_backup_test -- --test-threads=1` |
| 12 | `repro_v312_85` | 11/0 | 0.01s | `cargo test --all-features --test repro_v312_85 -- --test-threads=1` |

**总计**: 204 tests, 0 failed。

## 3. COMPILE_FAIL（3 项）

`WalEntry` 缺 `database_name` 字段（`#5137` 引入后未同步更新测试）：

| Binary | 错误 |
|--------|------|
| `tx_wal_contract_tests` | E0063 missing `database_name` (37 处) |
| `recovery_scenarios_test` | E0063 missing `database_name` (9 处) |
| `recovery_fuzzer_test` | E0063 missing `database_name` (34 处) |

**归因**: 编译债，非功能缺陷。修复 = 为 `WalEntry` 初始化补 `database_name: None`。

## 4. STILL_FAILING（22 项，真缺陷）

| # | Binary | 失败 | 语义 |
|---|--------|------|------|
| 1 | `types_value_test` | 5/8 | 类型系统 |
| 2 | `string_funcs_test` | 13/1 | 字符串函数 |
| 3 | `sequence_test` | 9/2 | 序列 |
| 4 | `upsert_test` | 5/1 | UPSERT |
| 5 | `replace_test` | 0/4 | REPLACE INTO 未实现 (#890) |
| 6 | `union_set_operations_test` | 10/1 | UNION |
| 7 | `show_tables_test` | 26/5 | SHOW TABLES |
| 8 | `show_full_tables_test` | 21/3 | SHOW FULL TABLES |
| 9 | `wal_tx_contract_test` | 36/6 | WAL 事务契约 |
| 10 | `io_delay_fault_test` | 0/3 | IO 延迟注入 |
| 11 | `stored_proc_catalog_test` | 17/5 | 存储过程目录 |
| 12 | `teaching_corpus_oracle_test` | 0/2 | 教学语料 oracle |
| 13 | `oracle_g5_sem1` | 20/1 | SAVEPOINT 嵌套 |
| 14 | `oracle_p34_parallel_executor` | 23/2 | 并行执行器 inventory |
| 15 | `sem1_savepoint_test` | 5/1 | SAVEPOINT dispatch |
| 16 | `repro_3282_orderby_desc` | 3/1 | Q18 ORDER BY DESC |
| 17 | `load_local_infile_test` | 20/1 | LOAD DATA LOCAL |
| 18 | `e2e_canonical_subprocess` | 17/14 | 旧 e2e 子进程 |
| 19 | `mysql_client_e2e_test` | 17/3 | MySQL 客户端 e2e |
| 20 | `l3_canonical_binary` | 16/1 | L3 handshake/auth |
| 21 | `g13_oltp1_concurrent_select_test` | 16/1 | OLTP 并发读 |
| 22 | `v312_71_sqlite_master_test` | 3/1 | sqlite_master |

## 5. ENVIRONMENT / WIRE_HARNESS / PERF（66 项）

### 5.1 TPC-H fixture 依赖（18 项）

需 `/tmp/tpch-sf01` 或 `/tmp/tpch-sf1` fixture：

`diag_q11`, `diag_q11_3way`, `diag_q11_having`, `diag_q11_steps`, `diag_q11_where`, `diag_q12`, `diag_q12_deep`, `diag_q14_full`, `diag_q14_only`, `diag_q14_q16`, `diag_q6_filter`, `diag_q6_where_parsed`, `diag_shipdate_type`, `diag_q7_subset_columns`, `q13_subquery_repro`, `q16_notin_subquery_regression`, `q21_cell_regression_test`, `q2_q17_repro_test`, `eval_22_vs_sf01`, `eval_22_vs_sqlite`

### 5.2 Wire harness 失败（27 项）

共享 `tpch_wire_harness.rs:40` panic（缺 fixture）：

`tpch_22_mysql_cli_wire_test`, `tpch_22_queries_wire_test`, `tpch_bug_regression_test`, `tpch_compliance_test`, `tpch_full_22_test`, `tpch_full_test`, `tpch_gate_test`, `tpch_hash_test`, `tpch_per_query_timeout_test`, `tpch_q8_q21_perf_regression_test`, `tpch_q9_audit`, `tpch_sf01_22_queries_wire_test`, `tpch_sf01_22_vs_3engines`, `tpch_sf01_22_vs_3engines_test`, `tpch_sf01_22_vs_sqlite`, `tpch_sf01_22_vs_sqlite_test`, `tpch_sf01_inprocess_test`, `tpch_sf01_oracle_dump`, `tpch_sf1_gate_contract_test`, `tpch_soak_qps`, `tpch_soak_test`, `tpch_value_correctness_test`, `tpch_value_test_v2`, `tpch_wire_smoke`, `oracle_g1_tpch_sha256`, `v312_13_load_data_sf10_test`, `quick_query`

### 5.3 PERF_ONLY（6 项，应移入 perf 门禁）

| Binary | 原因 |
|--------|------|
| `int2_substance_parallel_test` | 200K INSERT 循环 |
| `parallel_main_path_test` | 600K INSERT × 7 tests |
| `parallel_perf_baseline_test` | 500K INSERT |
| `tpch_soak_qps` | 30s soak |
| `tpch_soak_test` | soak |
| `tpch_q8_q21_perf_regression_test` | perf regression budget |

### 5.4 TIMEOUT（1 项）

`regression_test` — `test_regression_suite` >90s 未完成（含 25 sub-test）。

## 6. 防腐化失效条件

沿用 `b2-off-production-path-registry.md` R1-R4 模式，本次 REACTIVATED 项失效条件：

- **R1**: 若 `cargo test --all-features --test <binary>` 连续 2 次 FAIL，则回滚移出并重开 Issue。
- **R2**: 每次 `WalEntry` 结构变更必须同步修复 3 个 COMPILE_FAIL binary。
- **R3**: TPC-H fixture 依赖项在 fixture 就绪后必须逐项复验。

## 7. 下一步

1. 本次 PR: 从 `DISABLED_BINARIES` 移出 12 项 REACTIVATED。
2. 后续 Issue: 3 项 COMPILE_FAIL（`WalEntry.database_name`）→ 独立 defect。
3. 后续 Issue: 22 项 STILL_FAILING → 按语义分组（类型/字符串/序列/REPLACE/SHOW/WAL/SAVEPOINT/OLTP/LOAD DATA）。
4. 后续 Issue: PERF_ONLY 6 项 → 迁移至 perf 门禁。
