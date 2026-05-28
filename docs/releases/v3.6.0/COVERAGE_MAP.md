# Operator → Test File Coverage Map

> **版本**: 1.0  
> **状态**: Active  
> **分支**: `develop/v3.6.0`  
> **维护人**: hermes-agent  
> **创建日期**: 2026-05-29  

---

## 一、概述

本文档是 Operator-level Coverage 的 SSOT，定义了每个执行算子与对应测试文件的映射关系。

---

## 二、Operator → Source Files

### 2.1 Scan Operators

| Operator | Source File | Description |
|----------|-------------|-------------|
| SeqScan | `executor/src/scan.rs` | Sequential table scan |
| IndexScan | `executor/src/index_scan.rs` | Index-based scan |
| IndexOnlyScan | `executor/src/index_scan.rs` | Index-only scan (no heap fetch) |

### 2.2 Filter Operators

| Operator | Source File | Description |
|----------|-------------|-------------|
| Selection | `executor/src/filter.rs` | Row-level filter predicate |
| Filter | `executor/src/filter.rs` | Filter with expression evaluation |

### 2.3 Join Operators

| Operator | Source File | Description |
|----------|-------------|-------------|
| HashJoin | `executor/src/parallel_executor.rs` | Hash-based join |
| NestedLoop | `executor/src/parallel_executor.rs` | Nested loop join |
| MergeJoin | `executor/src/merge.rs` | Sort-merge join |

### 2.4 Aggregation Operators

| Operator | Source File | Description |
|----------|-------------|-------------|
| HashAgg | `executor/src/parallel_executor.rs` | Hash-based aggregation |
| SortAgg | `executor/src/merge.rs` | Sort-based aggregation |
| SimpleAgg | `executor/src/parallel_executor.rs` | Simple aggregation |

### 2.5 Sort/Limit Operators

| Operator | Source File | Description |
|----------|-------------|-------------|
| Sort | `executor/src/merge.rs` | External sort |
| Limit | `executor/src/local_executor.rs` | Limit result rows |
| TopN | `executor/src/local_executor.rs` | Top N rows |

### 2.6 Write Operators

| Operator | Source File | Description |
|----------|-------------|-------------|
| Insert | `executor/src/local_executor.rs` | Insert rows |
| Update | `executor/src/local_executor.rs` | Update rows |
| Delete | `executor/src/local_executor.rs` | Delete rows |

---

## 三、Operator → Test Files

### 3.1 Scan Operators

| Operator | Primary Test Files | Additional Coverage |
|----------|-------------------|-------------------|
| SeqScan | `test_seq_scan.rs` | `filter_tests.rs`, `coverage_tests.rs` |
| IndexScan | `index_scan_predicate_test.rs` | `join_tests.rs`, `coverage_exec_engine_test.rs` |
| IndexOnlyScan | `index_scan_predicate_test.rs` | `coverage_exec_engine_test.rs` |

### 3.2 Filter Operators

| Operator | Primary Test Files | Additional Coverage |
|----------|-------------------|-------------------|
| Selection | `test_filter.rs`, `filter_tests.rs` | `coverage_tests.rs` |
| Filter | `filter_tests.rs` | `test_seq_scan.rs` |

### 3.3 Join Operators

| Operator | Primary Test Files | Additional Coverage |
|----------|-------------------|-------------------|
| HashJoin | `test_join.rs`, `hash_join_types_test.rs`, `hash_join_left_null_test.rs` | `full_outer_join_test.rs` |
| NestedLoop | `join_tests.rs` | `test_join.rs` |
| MergeJoin | `merge_test.rs` | `join_tests.rs` |

### 3.4 Aggregation Operators

| Operator | Primary Test Files | Additional Coverage |
|----------|-------------------|-------------------|
| HashAgg | `test_aggregate.rs`, `aggregate_tests.rs` | `aggregate_functions_test.rs` |
| SortAgg | `aggregate_tests.rs` | `test_aggregate.rs` |
| SimpleAgg | `aggregate_functions_test.rs` | `aggregate_tests.rs` |

### 3.5 Sort/Limit Operators

| Operator | Primary Test Files | Additional Coverage |
|----------|-------------------|-------------------|
| Sort | `merge_test.rs` | `test_limit.rs` |
| Limit | `test_limit.rs` | `pipeline_tests.rs` |
| TopN | `test_limit.rs` | `patch_limit_edge_tests.rs` |

### 3.6 Write Operators

| Operator | Primary Test Files | Additional Coverage |
|----------|-------------------|-------------------|
| Insert | `dml_execution_tests.rs` | `ddl_execution_test.rs` |
| Update | `update_execution_test.rs` | `multi_table_update_test.rs`, `dml_execution_tests.rs` |
| Delete | `delete_execution_test.rs` | `dml_execution_tests.rs` |

---

## 四、Test File → Operators Coverage

| Test File | Operators Covered | Lines | Target |
|-----------|------------------|-------|--------|
| `test_seq_scan.rs` | SeqScan | ~500 | 90% |
| `index_scan_predicate_test.rs` | IndexScan, IndexOnlyScan | ~400 | 90% |
| `filter_tests.rs` | Selection, Filter | ~350 | 85% |
| `test_filter.rs` | Selection | ~200 | 85% |
| `join_tests.rs` | NestedLoop, MergeJoin | ~600 | 80% |
| `test_join.rs` | HashJoin | ~500 | 80% |
| `hash_join_types_test.rs` | HashJoin | ~300 | 80% |
| `hash_join_left_null_test.rs` | HashJoin (NULL semantics) | ~250 | 80% |
| `full_outer_join_test.rs` | HashJoin (Full Outer) | ~200 | 80% |
| `merge_test.rs` | MergeJoin, Sort | ~450 | 85% |
| `test_aggregate.rs` | HashAgg, SortAgg | ~400 | 85% |
| `aggregate_tests.rs` | HashAgg, SortAgg, SimpleAgg | ~350 | 85% |
| `aggregate_functions_test.rs` | SimpleAgg | ~300 | 85% |
| `test_limit.rs` | Limit, TopN | ~250 | 85% |
| `dml_execution_tests.rs` | Insert | ~300 | 80% |
| `update_execution_test.rs` | Update | ~300 | 80% |
| `delete_execution_test.rs` | Delete | ~300 | 80% |
| `multi_table_update_test.rs` | Update | ~200 | 80% |

---

## 五、Coverage Gaps

### 5.1 Missing Test Coverage

| Operator | Gap | Severity | Issue |
|----------|-----|----------|-------|
| IndexOnlyScan | No dedicated IndexOnlyScan tests | Medium | Need index-only scan specific tests |
| NestedLoop | No parameterized NestedLoop tests | Low | join_tests.rs has partial coverage |
| TopN | Limited TopN edge case tests | Medium | test_limit.rs has partial coverage |

### 5.2 Known Weaknesses

| Test File | Issue | Impact |
|-----------|-------|--------|
| `hash_join_left_null_test.rs` | NULL semantics edge cases | Low |
| `patch_limit_edge_tests.rs` | Boundary condition coverage | Medium |
| `patch_error_path_coverage.rs` | Error path gaps | Low |

---

## 六、维护规则

1. **新增 Operator**: 必须在此文档添加条目
2. **修改测试映射**: 必须同步更新
3. **发现缺口**: 创建 Issue 并标记 labels: `coverage`, `operator`
