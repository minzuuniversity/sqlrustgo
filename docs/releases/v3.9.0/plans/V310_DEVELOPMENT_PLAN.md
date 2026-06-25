# SQLRustGo v3.10.0 开发计划 — 功能 backlog + 测试要求

> **基于**: v3.9.0 `#[ignore]` 审计（2026-06-25）
> **分支**: `develop/v3.9.0` @ `ff77472bf`
> **目的**: 将 v3.9.0 中未解决的 44 个 `#[ignore]` 测试整理为 v3.10.0 功能需求

---

## 0. 总览

| 类别 | 数量 | 说明 |
|------|------|------|
| F-1: DML 增强 | 7 | INSERT SELECT / 子查询 UPDATE/DELETE / 多表 DML |
| F-2: UNION 扩展 | 3 | INTERSECT / EXCEPT / UNION ORDER BY+LIMIT |
| F-3: Cypher 图查询 | 5 | CREATE / MERGE / OPTIONAL MATCH / 无向边 |
| F-4: 事务增强 | 2 | MemoryStorage ROLLBACK / 事务边界 |
| F-5: 性能基准 | 17 | QPS/Sysbench/TPC-H 基准（保留手动运行） |
| F-6: 长时稳定性 | 5 | Soak 5m-30m / 72h smoke（Z6G4 阻塞） |
| F-7: Manual Oracle | 1 | SHA256 oracle 生成 |
| **合计** | **40** | 另有 2 个辅助 ignore + 2 个 TPCH 引擎测试 |

---

## F-1: DML 增强（7 个 ignore）

**来源**: `tests/dml_integration_test.rs`

### F-1a: INSERT ... SELECT（2 个 ignore → 修复）

| 行 | 测试 | 问题 |
|----|------|------|
| 101 | `insert_select_copies_rows` | INSERT ... SELECT → 0 rows in MemoryStorage |
| 120 | `insert_select_with_type_coercion` | 同上 |

**根因**: MemoryStorage 的 INSERT SELECT 路径没有正确执行数据复制。
**修复要求**: executor 实现正确的 INSERT SELECT — 从源表读取行，插入目标表。
**验证**: `cargo test insert_select_copies_rows insert_select_with_type_coercion` → PASS

### F-1b: UPDATE ... SET col = (SELECT ...)（1 个 ignore → 修复）

| 行 | 测试 | 问题 |
|----|------|------|
| 220 | `update_with_subquery_in_set` | UpdateStatement has no sub-select support |

**根因**: `UpdateStatement` 的 SET 子句只支持直接值，不支持子查询表达式。
**修复要求**: 扩展 SET 子句解析，支持 `UPDATE t SET col = (SELECT ...)`。
**验证**: `cargo test update_with_subquery_in_set` → PASS

### F-1c: Multi-table UPDATE（1 个 ignore → 修复）

| 行 | 测试 | 问题 |
|----|------|------|
| 236 | `update_multiple_tables` | UpdateStatement is single-table only |

**根因**: parser/executor 只处理单表 UPDATE。
**修复要求**: 支持 `UPDATE t1, t2 SET t1.col = ... WHERE ...` 语法和执行路径。
**验证**: `cargo test update_multiple_tables` → PASS

### F-1d: DELETE ... WHERE col IN (SELECT ...)（1 个 ignore → 修复）

| 行 | 测试 | 问题 |
|----|------|------|
| 306 | `delete_with_subquery_in_where` | DeleteStatement has no sub-select support |

**根因**: DELETE WHERE 子句不支持 IN (SELECT ...) 形式。
**修复要求**: 扩展 WHERE 解析，支持相关子查询和 IN (SELECT ...) 形式。
**验证**: `cargo test delete_with_subquery_in_where` → PASS

### F-1e: Multi-table DELETE（1 个 ignore → 修复）

| 行 | 测试 | 问题 |
|----|------|------|
| 322 | `delete_multiple_tables` | DeleteStatement is single-table only |

**根因**: parser/executor 只处理单表 DELETE。
**修复要求**: 支持 `DELETE t1, t2 FROM t1 JOIN t2 ON ... WHERE ...` 语法。
**验证**: `cargo test delete_multiple_tables` → PASS

### F-1: DML 增强 — 测试矩阵

| 测试 | 现状 | 修复后验证 |
|------|------|-----------|
| `insert_select_copies_rows` | FAIL: 0 rows | 3 rows inserted |
| `insert_select_with_type_coercion` | FAIL: 0 rows | rows with correct types |
| `update_with_subquery_in_set` | FAIL: Null | Integer(42) |
| `update_multiple_tables` | FAIL: ParseError | rows updated |
| `delete_with_subquery_in_where` | FAIL: 0 rows | 1 row deleted |
| `delete_multiple_tables` | FAIL: ParseError | rows deleted |

---

## F-2: UNION 扩展（3 个 ignore）

**来源**: `tests/union_set_operations_test.rs`

### F-2a: INTERSECT（1 个 ignore → 修复）

| 行 | 测试 | 问题 |
|----|------|------|
| 257 | `intersect_returns_common_rows` | no Statement::Intersect variant |

**根因**: `Statement` 枚举缺少 `Intersect` 变体，parser 遇到 `INTERSECT` 报解析错误。
**修复要求**:
1. 添加 `Statement::Intersect` 枚举变体
2. Parser 支持 `SELECT ... INTERSECT SELECT ...` 语法
3. Executor 实现集合交语义（去重）
**验证**: `cargo test intersect_returns_common_rows` → rows.len() == 2

### F-2b: EXCEPT（1 个 ignore → 修复）

| 行 | 测试 | 问题 |
|----|------|------|
| 276 | `except_returns_left_minus_right` | no Statement::Except variant |

**根因**: `Statement` 枚举缺少 `Except` 变体。
**修复要求**:
1. 添加 `Statement::Except` 枚举变体
2. Parser 支持 `SELECT ... EXCEPT SELECT ...` 语法
3. Executor 实现集合差语义（去重）
**验证**: `cargo test except_returns_left_minus_right` → rows.len() == 1

### F-2c: UNION ORDER BY/LIMIT（1 个 ignore → 修复）

| 行 | 测试 | 问题 |
|----|------|------|
| 299 | `order_by_after_top_level_union` | UnionStatement lacks order_by/limit fields |

**根因**: `UnionStatement` 没有 `order_by` 和 `limit` 字段。
**修复要求**:
1. 扩展 `UnionStatement` 结构体添加 `order_by: Vec<OrderByExpr>` 和 `limit: Option<u64>`
2. Parser 支持 `SELECT ... UNION ... ORDER BY col LIMIT n` 语法
3. Executor 在 UNION 结果上应用排序和 LIMIT
**验证**: `cargo test order_by_after_top_level_union` → rows.len() == 3

### F-2: UNION 扩展 — 测试矩阵

| 测试 | 现状 | 修复后验证 |
|------|------|-----------|
| `intersect_returns_common_rows` | FAIL: 3 rows (全部返回) | 2 rows (交集) |
| `except_returns_left_minus_right` | FAIL: 3 rows (全部返回) | 1 row (差集) |
| `order_by_after_top_level_union` | FAIL: 6 rows (无排序) | 3 rows (排序+limit) |

---

## F-3: Cypher 图查询（5 个 ignore）

**来源**: `tests/graph_cypher_integration_test.rs`

### F-3a: Cypher CREATE 关键字

| 行 | 测试 | 问题 |
|----|------|------|
| 466 | `test_cypher_create_node_keyword` | CREATE keyword not supported in Cypher |

**修复要求**: Parser 支持 `CREATE` 子句（`CREATE (n:Label {prop: val})`），Executor 实现节点创建。

### F-3b: Cypher MERGE 关键字

| 行 | 测试 | 问题 |
|----|------|------|
| 476 | `test_cypher_merge_keyword` | MERGE keyword not supported |

**修复要求**: Parser 支持 `MERGE` 子句，Executor 实现"匹配或创建"语义。

### F-3c: Cypher 无向边模式

| 行 | 测试 | 问题 |
|----|------|------|
| 485 | `test_cypher_undirected_relationship_pattern` | undirected `-` pattern not supported |

**修复要求**: Parser 支持无向边 `-`（当前只支持有向边 `->`）。

### F-3d: Cypher OPTIONAL MATCH

| 行 | 测试 | 问题 |
|----|------|------|
| 494 | `test_cypher_optional_match_returns_null_for_missing` | OPTIONAL MATCH not supported |

**修复要求**: Parser 支持 `OPTIONAL MATCH`，Executor 对不匹配部分返回 NULL 值。

### F-3e: Cypher 其他 gap

| 行 | 测试 | 问题 |
|----|------|------|
| 16 | `test_cypher_executor_label_predicate_with_boolean_true` | edge case label predicate |

**说明**: 此测试行 16 是另一种 Cypher 边缘情况，需单独评估。

### F-3: Cypher — 测试矩阵

| 测试 | 现状 | 修复后验证 |
|------|------|-----------|
| `test_cypher_create_node_keyword` | FAIL: CREATE not supported | node created |
| `test_cypher_merge_keyword` | FAIL: MERGE not supported | node matched/created |
| `test_cypher_undirected_relationship_pattern` | FAIL: undirected - | edge created both dirs |
| `test_cypher_optional_match_returns_null_for_missing` | FAIL: OPTIONAL MATCH | NULL for missing |
| `test_cypher_executor_label_predicate_with_boolean_true` | 需评估 | 需评估 |

---

## F-4: 事务增强（5 个 ignore）

**来源**: `tests/dml_integration_test.rs` + `tests/stored_proc_catalog_test.rs`

### F-4a: ROLLBACK 不回滚 DML（2 个 ignore → 修复）

| 行 | 测试 | 问题 |
|----|------|------|
| 352 | `transaction_rollback_undoes_dml` | ROLLBACK does not revert DML rows in MemoryStorage |
| 372 | `transaction_update_then_rollback` | 同上 |

**根因**: MemoryStorage 的 ROLLBACK 路径只回滚 WAL 状态，不回滚实际插入的 DML 行。
**修复要求**: MemoryStorage 需要 MVCC 或类似机制，在 ROLLBACK 时撤销 DML 变更。
**验证**: `cargo test transaction_rollback_undoes_dml transaction_update_then_rollback` → rows count unchanged after rollback

### F-4b: MemoryStorage 事务边界（3 个 ignore → 修复）

| 行 | 测试 | 问题 |
|----|------|------|
| 284 | `test_trigger_executes_update` | MemoryStorage does not support transactions |
| 315 | `test_trigger_executes_delete` | 同上 |
| 346 | `test_trigger_executes_insert` | 同上 |

**根因**: MemoryStorage 没有实现事务边界（begin/commit/rollback），trigger DML 需要事务支持。
**修复要求**: MemoryStorage 实现基本事务支持（begin/commit/rollback），或修改 trigger 评估路径以处理无事务情况。
**验证**: `cargo test test_trigger_executes_update test_trigger_executes_delete test_trigger_executes_insert` → PASS

### F-4: 事务增强 — 测试矩阵

| 测试 | 现状 | 修复后验证 |
|------|------|-----------|
| `transaction_rollback_undoes_dml` | FAIL: 3 rows (未回滚) | 1 row (回滚后) |
| `transaction_update_then_rollback` | FAIL: index out of bounds | correct rows |
| `test_trigger_executes_update` | FAIL | PASS |
| `test_trigger_executes_delete` | FAIL | PASS |
| `test_trigger_executes_insert` | FAIL | PASS |

---

## F-5: 性能基准（17 个 ignore — 保留手动运行）

**说明**: 这些测试有严格的时间阈值断言，适合在专用性能环境中手动运行，不适合默认 CI。

### F-5a: QPS 基准（10 个 ignore）

**来源**: `tests/qps_benchmark_test.rs:70,95,124,153,185,213,238,285,349,375`
**要求**: 在 release 模式 + 专用机器上运行 `cargo test --release --test qps_benchmark_test -- --ignored`
**验证**: 每秒查询数 > 基准值（按硬件环境设定）

### F-5b: v3.8.0 性能基线（6 个 ignore）

**来源**: `tests/bench_v380_point_agg.rs:41,91,137,184,232,275`
**要求**: 与 v3.8.0 对比，性能不退化
**验证**: `cargo test --release --test bench_v380_point_agg -- --ignored --release`

### F-5c: 批量插入性能（3 个 ignore）

**来源**: `tests/perf_eng_batched_insert_test.rs:52,92` + 1 个未列出
**要求**: 1000 行 < 1s，10000 行 < 10s（release 模式）
**验证**: `cargo test --release --test perf_eng_batched_insert_test -- --ignored`

---

## F-6: 长时稳定性测试（5 个 ignore — Z6G4 阻塞）

### F-6a: TPC-H Soak（4 个 ignore）

**来源**: `tests/tpch_soak_test.rs:70,78,86,94`
**要求**: `test_soak_5m/10m/20m/30m` — 已验证 5m-30m 全部 PASS
**状态**: 代码正常，需 Z6G4 环境运行 72h/168h

### F-6b: 72h 稳定性（1 个 ignore）

**来源**: `tests/long_run_stability_72h_test.rs:6`
**要求**: `long_run_stability_72h_smoke` — 5 秒 smoke 已 PASS
**阻塞**: Z6G4（192.168.0.252）网络不可达（2026-06-25 仍无解）

---

## F-7: Manual Oracle（1 个 ignore）

**来源**: `tests/oracle_g1_tpch_sha256.rs:138`
**要求**: `cargo test --test oracle_g1_tpch_sha256 -- --ignored --generate-baseline`
**说明**: SHA256 oracle 生成，手动运行

---

## 其他（4 个 ignore）

### tpch_wire_smoke_sf（8 个 ignore）

**来源**: `tests/tpch_wire_smoke_sf.rs:18,34,37,39,280,310,312,315`
**说明**: 这些是辅助函数定义（`// !` 注释行），不是独立测试，数量不计入功能 backlog。

### tpch_sf1_22_vs_3engines_test（2 个 ignore）

**来源**: `tests/tpch_sf1_22_vs_3engines_test.rs:8,146`
**说明**: 需单独评估引擎一致性测试。

### recovery_fuzzer / crash_monkey（各 1 个）

**来源**: `tests/recovery_fuzzer_test.rs:584`，`tests/crash_monkey_test.rs:252`
**说明**: 50k/100k 迭代测试，手动 `--ignored` 运行，无 bug。

---

## 开发优先级建议

| 优先级 | 任务 | 工作量估计 | 风险 |
|--------|------|-----------|------|
| P1 | F-4a: ROLLBACK 回滚 | 中 | 中（需 MVCC） |
| P1 | F-4b: MemoryStorage 事务 | 中 | 中 |
| P1 | F-1a: INSERT SELECT | 中 | 低 |
| P2 | F-1b-e: 其他 DML 增强 | 中-高 | 中 |
| P2 | F-2: UNION 扩展 | 中 | 中（parser + executor） |
| P3 | F-3: Cypher 增强 | 高 | 高（parser + executor） |
| — | F-5/F-6/F-7 | 手动运行 | — |

---

## v3.10.0 配套文档

- `IGNORE_REGISTRY_2026-06-25.md` — 完整 ignore 清单（含历史）
- `V390_DEVELOPMENT_PLAN.md` — v3.9.0 架构债/可靠性任务
- `LONG_STABILITY_TESTS_ANALYSIS.md` — 长时测试详细分析
