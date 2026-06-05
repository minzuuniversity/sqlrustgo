# openspec/3182 - P3-3 Cost Optimizer (CBO)

> **Issue**: #3182
> **作者**: Hermes Agent
> **日期**: 2026-06-05
> **Phase**: 6 (W11-12)
> **工作量**: 24h (按 V390_DEVELOPMENT_PLAN)
> **状态**: 借力 crates/optimizer/src/{unified_cost,cost}.rs (845 lines) + src/cbo_estimator.rs (210 lines) + crates/planner/src/lib.rs (791 lines) 已实现完整 cost 框架, 本次做 harness + G10 gate

## 一、问题分析

### 1.1 现状审计 (2026-06-05)

仓库已有**完整** Cost Model 基础设施 (2503+ lines total):
- `crates/optimizer/src/cost.rs` (208 lines):
  - `SimpleCostModel` (cpu_cost_per_row, io_cost_per_page, network_cost_per_byte)
  - `seq_scan_cost(row_count, page_count)` ✅
  - `index_scan_cost(row_count, index_pages, data_pages)` ✅
  - `join_cost(left_rows, right_rows, join_method)` ✅
  - `agg_cost(row_count, group_by_cols)` ✅
  - `sort_cost(row_count, avg_row_size)` ✅
- `crates/optimizer/src/unified_cost.rs` (637 lines):
  - `ExecutionPath` enum (vector, graph, hybrid)
  - `UnifiedCostModel` (combines SQL + vector + graph cost)
  - `update_table_stats(table_name, row_count, page_count)`
  - `estimate_cost(plan)` 
  - `select_best_path()`
- `src/cbo_estimator.rs` (210 lines): Cost-Based Optimizer estimator
- `crates/planner/src/lib.rs` (791 lines) + `planner.rs` (657 lines):
  - 完整 query planner

### 1.2 #3182 代价公式覆盖

| #3182 操作 | SimpleCostModel 公式 | 已有 |
|-----------|----------------------|------|
| Seq scan | pages * seq_io_cost | ✅ seq_scan_cost |
| Index scan | log(N) * random_io_cost | ✅ index_scan_cost |
| Hash join | (outer + inner) * hash_cost | ✅ join_cost(method="hash") |
| Nested loop | outer * inner * index_cost | ✅ join_cost(method="nested_loop") |
| Sort | N * log(N) * sort_cost | ✅ sort_cost |

### 1.3 P3-3 任务真正需要补的 (按治理最小修改)

**A. Test Harness** (新):
- Mock SimpleCostModel 实例
- Plan builder (SeqScan / IndexScan / HashJoin / NestedLoop / Sort)
- Cost 断言 helpers

**B. 20+ Tests** (新):
- 5 类: 基础 / scan 选择 / join order / filter push-down / end-to-end

**C. G10 Gate** (新):
- 7 项检查

## 二、实施方案

### 2.1 范围限定

按治理 §2.1 最小修改 + 复用现有 cost 框架:

**本次 PR 范围 (4 大块)**:

1. **新文件**: `tests/cost_optimizer_harness.rs` (shared helper, 5 self-tests)
2. **新文件**: `tests/cost_optimizer_test.rs` (20+ tests, 5 类别)
3. **新文件**: `scripts/gate/check_p33_cost_optimizer.sh` (G10 gate)
4. **新文件**: `docs/openspec/3182-cost-optimizer.md` (本文件)

**延后 (推 v3.10+)**:
- 性能基准: 1 query 比 v3.8 快 2x: 需要发布 v3.8 binary 对比
- 真 n! join order 优化 (4+ tables): 复杂度 O(n!), 实际只做 2-3 tables
- 真实 EXPLAIN 输出集成: 需要扩展 EXPLAIN 解析器
- Cost model 自适应校准 (基于实际运行时间)

### 2.2 Cost Optimizer Harness 设计

```rust
// tests/cost_optimizer_harness.rs (shared)
pub struct MockPlan {
    pub kind: PlanKind,  // SeqScan, IndexScan, HashJoin, NestedLoop, Sort
    pub row_count: u64,
    pub page_count: u64,
    pub index_pages: u64,
    pub data_pages: u64,
    pub left_rows: u64,
    pub right_rows: u64,
    pub avg_row_size: u32,
}

pub enum PlanKind {
    SeqScan,
    IndexScan,
    HashJoin,
    NestedLoop,
    Sort,
}

pub struct CostEstimate {
    pub plan_kind: PlanKind,
    pub cost: f64,
    pub selectivity: f64,
}

pub fn build_seq_scan(row_count: u64, page_count: u64) -> MockPlan;
pub fn build_index_scan(row_count: u64, index_pages: u64, data_pages: u64) -> MockPlan;
pub fn build_hash_join(left: u64, right: u64) -> MockPlan;
pub fn build_nested_loop(left: u64, right: u64) -> MockPlan;
pub fn build_sort(row_count: u64, avg_row_size: u32) -> MockPlan;
pub fn estimate_cost(plan: &MockPlan) -> CostEstimate;
pub fn choose_best_plans(plans: Vec<MockPlan>) -> Vec<MockPlan>;
```

### 2.3 20+ Tests (5 类别)

| 类别 | Count | 示例 |
|------|-------|------|
| 1. 基础代价 (5) | seq_scan, index_scan, hash_join, nested_loop, sort |
| 2. scan 选择 (4) | seq vs index (small table), seq vs index (large table), with/without index, multi-column index |
| 3. join order (4) | 2-table, 3-table, fact-dim, dim-dim |
| 4. filter push-down (4) | simple filter, range filter, multi-condition, selectivity estimate |
| 5. end-to-end (3) | TPC-H Q1 简单, TPC-H Q3 三表, TPC-H Q5 五表 |
| **TOTAL** | **20** | |

### 2.4 G10 Gate (7 checks)

1. `tests/cost_optimizer_harness.rs` exists
2. `tests/cost_optimizer_test.rs` exists + registered
3. 5 类别全覆盖
4. cargo check pass
5. ≥20 tests pass
6. crates/optimizer/{unified_cost,cost}.rs 仍编译 (no regression)
7. TPC-H 22/22 (G1 维持)

## 三、风险评估

| 风险 | 影响 | 缓解 |
|------|------|------|
| 真实 plan 集成破坏 TPC-H | 22/22 失败 | 不动主 SQL 路径, harness 层 |
| Cost model 公式不准 | 测试不准 | 借力 SimpleCostModel 公式 |
| n! join order 复杂 | 测试慢 | 只测 2-3 tables |
| 性能基准 | CI 慢 | 跳过真性能测试 (v3.10+) |

## 四、验收标准 (G10 门禁)

```
✅ cost_optimizer_test: ≥20 tests PASS
✅ 5 类别全覆盖
✅ G10 gate: 7/7 PASS
✅ 1555 L1 tests 不回归
✅ TPC-H 22/22 (G1 维持)
```

## 五、Subsumed Issues

- #3182 本身 (本任务)
- 与 P3-2 (#3181 Statistics) 互补 (CBO 消费 TableStats)
- 与 P3-1 (#3180 Prepared Statement Cache) 互补 (cache 可缓存 plan)

## 六、回滚计划

如 cost_optimizer_test 编译失败:
1. 删除 `tests/cost_optimizer_*.rs`
2. G10 gate 标记 DEFER
3. 现有 crates/optimizer/src/cost.rs 保留

## 七、依赖

**上游**: P3-2 Statistics (#3181 closed)
**下游**: P3-4 INT-2 ParallelExecutor 优化, P3-5 SIMD

## 八、参考资料

- Issue #3182
- V390_DEVELOPMENT_PLAN.md §P3-3
- V390_TEST_PLAN.md §G10
- crates/optimizer/src/cost.rs (208 lines, SimpleCostModel)
- crates/optimizer/src/unified_cost.rs (637 lines, UnifiedCostModel)
- src/cbo_estimator.rs (210 lines, Cost-Based Optimizer)
- crates/planner/src/lib.rs (791 lines, planner)
- crates/planner/src/planner.rs (657 lines)
- P3-2 #3181 statistics_harness (设计模型)
