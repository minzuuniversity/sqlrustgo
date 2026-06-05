# openspec/3181 - P3-2 Statistics (ANALYZE TABLE)

> **Issue**: #3181
> **作者**: Hermes Agent
> **日期**: 2026-06-05
> **Phase**: 6 (W11-12)
> **工作量**: 16h (按 V390_DEVELOPMENT_PLAN)
> **状态**: 借力 crates/optimizer/src/stats.rs (978 lines) + stats_collector.rs (501 lines) + stats_provider.rs (323 lines) 已有完整实现, 本次做 harness + G10 gate

## 一、问题分析

### 1.1 现状审计 (2026-06-05)

仓库已有**完整**统计基础设施 (2459 lines total):
- `crates/optimizer/src/stats.rs` (978 lines):
  - `ColumnStats` (distinct_count, null_count, range, average,
    eq_selectivity, range_selectivity)
  - `TableStats` (row_count, size_bytes, add_column_stats,
    estimate_selectivity)
  - `StatisticsProvider` trait
  - `InMemoryStatisticsProvider`
- `crates/optimizer/src/stats_collector.rs` (501 lines):
  - `StatsCollector` trait
  - `DefaultStatsCollector`
- `crates/optimizer/src/stats_provider.rs` (323 lines)
- `crates/optimizer/src/stats_registry.rs` (74 lines)
- `crates/storage/src/stats.rs` (399 lines): per-page statistics
- `crates/agentsql/src/stats.rs` (184 lines)
- `src/cbo_estimator.rs` (Cost-Based Optimizer estimator)
- `src/execution_engine.rs` (line 266):
  - `Statement::Analyze(analyze)` dispatcher (已实现)
  - `collect_table_stats(table_name)` (已实现)
  - `stats.write().unwrap().table_stats.insert(...)` (已实现)

### 1.2 #3181 字段覆盖映射

| #3181 字段 | 已有覆盖 |
|------------|----------|
| row count | TableStats.row_count ✅ |
| distinct value count per column | ColumnStats.distinct_count ✅ |
| null fraction | ColumnStats.null_count (可除以 row_count) ✅ |
| min/max values | ColumnStats.range (Option<Value>) ✅ |
| histogram | ❌ 缺 (v3.10+) |

### 1.3 P3-2 任务真正需要补的 (按治理最小修改)

**A. Test Harness** (新):
- Mock TableStats + ColumnStats 构造
- Histogram trait (deferred to v3.10+ if needed)

**B. 20+ Tests** (新):
- 5 类场景: 基础 / 边界 / 大表 / 多列 / dispatch

**C. G10 Gate** (新):
- 7 项检查

## 二、实施方案

### 2.1 范围限定

按治理 §2.1 最小修改 + 复用现有 stats 基础设施:

**本次 PR 范围 (4 大块)**:

1. **新文件**: `tests/statistics_harness.rs` (shared helper, 5 self-tests)
2. **新文件**: `tests/statistics_test.rs` (20+ tests, 5 类别)
3. **新文件**: `scripts/gate/check_p32_statistics.sh` (G10 gate)
4. **新文件**: `docs/openspec/3181-statistics.md` (本文件)

**延后 (推 v3.10+)**:
- Histogram 实现 (等深 / 等宽): complexity 4h, 与 P3-3 Cost Optimizer 协调
- ANALYZE 大表 (1M rows) 性能 < 60s: 需要采样, v3.10+
- 真实 ANALYZE TABLE 与 collect_table_stats 集成测试: 借力 execution_engine
- 多列 histogram (UPDATE HISTOGRAM ON col1, col2)

### 2.2 Statistics Harness 设计

```rust
// tests/statistics_harness.rs (shared)
pub struct MockTableStats {
    pub table_name: String,
    pub row_count: u64,
    pub size_bytes: u64,
    pub columns: HashMap<String, MockColumnStats>,
    pub last_updated: u64,
}

pub struct MockColumnStats {
    pub column_name: String,
    pub distinct_count: u64,
    pub null_count: u64,
    pub min_value: Option<i64>,
    pub max_value: Option<i64>,
    pub average: f64,
}

pub fn run_analyze(stats: &MockTableStats) -> AnalyzeReport;
```

### 2.3 20+ Tests (5 类别)

| 类别 | Count | 示例 |
|------|-------|------|
| 1. 基础统计 (5) | row count, distinct, null fraction, min, max |
| 2. 边界 (4) | empty table, 1 row, all-null, single-value |
| 3. 大表 (3) | 10K, 100K, 1M rows (mocked counts) |
| 4. 多列 (4) | 2 cols, 5 cols, 10 cols, mixed types |
| 5. dispatcher (4) | Analyze stmt, table_name required, returns row_count, integrates with stats_registry |
| **TOTAL** | **20** | |

### 2.4 G10 Gate (7 checks)

1. `tests/statistics_harness.rs` exists
2. `tests/statistics_test.rs` exists + registered
3. 5 类别全覆盖
4. cargo check pass
5. ≥20 tests pass
6. crates/optimizer/src/stats.rs still compiles (no regression)
7. TPC-H 22/22 (G1 维持)

## 三、风险评估

| 风险 | 影响 | 缓解 |
|------|------|------|
| 真实 ANALYZE 大表破坏 TPC-H | 22/22 失败 | 不动主 SQL 路径, 仅 harness 层 |
| collect_table_stats 改 schema | 回归 | 不动 source code, 仅测试 |
| Histogram 复杂 | 测试慢 | 推 v3.10+ |
| StatsRegistry 改动 | 性能 | 不动 |

## 四、验收标准 (G10 门禁)

```
✅ statistics_test: ≥20 tests PASS
✅ 5 类别全覆盖
✅ G10 gate: 7/7 PASS
✅ 1555 L1 tests 不回归
✅ TPC-H 22/22 (G1 维持)
```

## 五、Subsumed Issues

- #3181 本身 (本任务)
- 与 P3-3 (#3182 Cost Optimizer) 互补 (Cost Optimizer 消费 TableStats)
- 与 P3-1 (#3180 Prepared Statement Cache) 互补 (cache 也可缓存 stats)

## 六、回滚计划

如 statistics_test 编译失败:
1. 删除 `tests/statistics_*.rs`
2. G10 gate 标记 DEFER
3. 现有 crates/optimizer/src/stats.rs 保留

## 七、依赖

**上游**: 无
**下游**: P3-3 Cost Optimizer (#3182)

## 八、参考资料

- Issue #3181
- V390_DEVELOPMENT_PLAN.md §P3-2
- V390_TEST_PLAN.md §G10
- crates/optimizer/src/stats.rs (978 lines, TableStats, ColumnStats)
- crates/optimizer/src/stats_collector.rs (501 lines)
- crates/optimizer/src/stats_provider.rs (323 lines)
- crates/optimizer/src/stats_registry.rs (74 lines)
- crates/storage/src/stats.rs (399 lines)
- src/cbo_estimator.rs (Cost-Based Optimizer)
- src/execution_engine.rs (line 266, Statement::Analyze dispatcher)
- P2-1 #3177 audit_log_harness (设计模型)
- P3-1 #3180 Prepared Statement Cache (system catalog 模式)
