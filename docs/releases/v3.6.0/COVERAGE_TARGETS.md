# Per-Operator Coverage Targets

> **版本**: 1.0
> **状态**: Active
> **分支**: `develop/v3.6.0`
> **维护人**: hermes-agent
> **创建日期**: 2026-05-29

---

## 一、概述

本文档定义了每个 Executor Operator 的详细 coverage targets，是 COVERAGE_SSOT 的具体化。

---

## 二、Coverage Targets 汇总

### 2.1 Summary Table

| Operator | Category | Target | Current | Status | File(s) |
|----------|----------|--------|---------|--------|---------|
| SeqScan | Scan | 90% | - | TBD | scan.rs |
| IndexScan | Scan | 90% | - | TBD | index_scan.rs |
| IndexOnlyScan | Scan | 90% | - | TBD | index_scan.rs |
| Selection | Filter | 85% | - | TBD | filter.rs |
| Filter | Filter | 85% | - | TBD | filter.rs |
| HashJoin | Join | 80% | - | TBD | parallel_executor.rs |
| NestedLoop | Join | 80% | - | TBD | parallel_executor.rs |
| MergeJoin | Join | 80% | - | TBD | merge.rs |
| HashAgg | Aggregation | 85% | - | TBD | parallel_executor.rs |
| SortAgg | Aggregation | 85% | - | TBD | merge.rs |
| SimpleAgg | Aggregation | 85% | - | TBD | parallel_executor.rs |
| Sort | Sort/Limit | 85% | - | TBD | merge.rs |
| Limit | Sort/Limit | 85% | - | TBD | local_executor.rs |
| TopN | Sort/Limit | 85% | - | TBD | local_executor.rs |
| Insert | Write | 80% | - | TBD | local_executor.rs |
| Update | Write | 80% | - | TBD | local_executor.rs |
| Delete | Write | 80% | - | TBD | local_executor.rs |

---

## 三、详细 Target 定义

### 3.1 Scan Operators (Target: 90%)

#### SeqScan
- **File**: `executor/src/scan.rs`
- **Target Line Coverage**: 90%
- **Target Branch Coverage**: 85%
- **Test File**: `test_seq_scan.rs`
- **Critical Paths**:
  - Full table scan
  - Filter pushdown
  - Projection
  - Empty result set

#### IndexScan
- **File**: `executor/src/index_scan.rs`
- **Target Line Coverage**: 90%
- **Target Branch Coverage**: 85%
- **Test File**: `index_scan_predicate_test.rs`
- **Critical Paths**:
  - Index range scan
  - Index seek
  - Predicate evaluation
  - Index fetch

#### IndexOnlyScan
- **File**: `executor/src/index_scan.rs`
- **Target Line Coverage**: 90%
- **Target Branch Coverage**: 85%
- **Test File**: `index_scan_predicate_test.rs`
- **Critical Paths**:
  - Index-only access
  - No heap fetch
  - Covering index

---

### 3.2 Filter Operators (Target: 85%)

#### Selection
- **File**: `executor/src/filter.rs`
- **Target Line Coverage**: 85%
- **Target Branch Coverage**: 80%
- **Test File**: `test_filter.rs`, `filter_tests.rs`
- **Critical Paths**:
  - Predicate evaluation
  - NULL handling
  - Short-circuit evaluation

#### Filter
- **File**: `executor/src/filter.rs`
- **Target Line Coverage**: 85%
- **Target Branch Coverage**: 80%
- **Test File**: `filter_tests.rs`
- **Critical Paths**:
  - Expression evaluation
  - Type coercion
  - Error handling

---

### 3.3 Join Operators (Target: 80%)

#### HashJoin
- **File**: `executor/src/parallel_executor.rs`
- **Target Line Coverage**: 80%
- **Target Branch Coverage**: 75%
- **Test File**: `test_join.rs`, `hash_join_types_test.rs`
- **Critical Paths**:
  - Hash table build
  - Probe phase
  - NULL key handling
  - Memory spill

#### NestedLoop
- **File**: `executor/src/parallel_executor.rs`
- **Target Line Coverage**: 80%
- **Target Branch Coverage**: 75%
- **Test File**: `join_tests.rs`
- **Critical Paths**:
  - Outer loop
  - Inner loop
  - Join predicate evaluation

#### MergeJoin
- **File**: `executor/src/merge.rs`
- **Target Line Coverage**: 80%
- **Target Branch Coverage**: 75%
- **Test File**: `merge_test.rs`
- **Critical Paths**:
  - Sorted inputs
  - Equality join
  - Duplicate handling

---

### 3.4 Aggregation Operators (Target: 85%)

#### HashAgg
- **File**: `executor/src/parallel_executor.rs`
- **Target Line Coverage**: 85%
- **Target Branch Coverage**: 80%
- **Test File**: `test_aggregate.rs`, `aggregate_tests.rs`
- **Critical Paths**:
  - Group by hash
  - Aggregation functions
  - NULL handling

#### SortAgg
- **File**: `executor/src/merge.rs`
- **Target Line Coverage**: 85%
- **Target Branch Coverage**: 80%
- **Test File**: `aggregate_tests.rs`
- **Critical Paths**:
  - Sorted input
  - Group transition
  - Aggregate computation

#### SimpleAgg
- **File**: `executor/src/parallel_executor.rs`
- **Target Line Coverage**: 85%
- **Target Branch Coverage**: 80%
- **Test File**: `aggregate_functions_test.rs`
- **Critical Paths**:
  - No group by
  - Single-row result

---

### 3.5 Sort/Limit Operators (Target: 85%)

#### Sort
- **File**: `executor/src/merge.rs`
- **Target Line Coverage**: 85%
- **Target Branch Coverage**: 80%
- **Test File**: `merge_test.rs`
- **Critical Paths**:
  - External merge sort
  - Spill to disk
  - Multi-way merge

#### Limit
- **File**: `executor/src/local_executor.rs`
- **Target Line Coverage**: 85%
- **Target Branch Coverage**: 80%
- **Test File**: `test_limit.rs`
- **Critical Paths**:
  - Early termination
  - Offset handling
  - Zero limit

#### TopN
- **File**: `executor/src/local_executor.rs`
- **Target Line Coverage**: 85%
- **Target Branch Coverage**: 80%
- **Test File**: `test_limit.rs`
- **Critical Paths**:
  - Top N heap
  - Order by evaluation
  - Boundary handling

---

### 3.6 Write Operators (Target: 80%)

#### Insert
- **File**: `executor/src/local_executor.rs`
- **Target Line Coverage**: 80%
- **Target Branch Coverage**: 75%
- **Test File**: `dml_execution_tests.rs`
- **Critical Paths**:
  - Row insertion
  - Constraint checking
  - Auto-commit

#### Update
- **File**: `executor/src/local_executor.rs`
- **Target Line Coverage**: 80%
- **Target Branch Coverage**: 75%
- **Test File**: `update_execution_test.rs`
- **Critical Paths**:
  - Row update
  - Index update
  - Concurrent update

#### Delete
- **File**: `executor/src/local_executor.rs`
- **Target Line Coverage**: 80%
- **Target Branch Coverage**: 75%
- **Test File**: `delete_execution_test.rs`
- **Critical Paths**:
  - Row deletion
  - Cascade delete
  - Index cleanup

---

## 四、Status Criteria

| Status | Criteria | Action |
|--------|----------|--------|
| PASS | Coverage >= Target | None |
| WARNING | Target - 5% <= Coverage < Target | Review needed |
| FAIL | Coverage < Target - 5% | PR blocking |

---

## 五、Measurement Schedule

- **Per-PR**: Run `check_operator_coverage.sh` on modified files
- **Daily**: Full operator coverage report
- **Release**: Gate requirement - all operators must be PASS or WARNING
