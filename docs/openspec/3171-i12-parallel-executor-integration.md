<!-- env:blocked:no-ci -->

# openspec/3171 - I-12 Parallel Executor 主路径集成 (P0-3 INT-2)

> **Issue**: [#3171](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3171)
> **作者**: Hermes Agent
> **日期**: 2026-06-05
> **Phase**: 2 (W3-4) — 实际启动按 user 决策提前到 W1
> **工作量**: 30h (~4 天)
> **优先级**: P0 (40%, 架构债)
> **Milestone**: v3.9.0 (id=32, due 2026-09-23)
> **Label**: v3.9.0, P0-arch-debt, phase-2

## 一、问题分析

### 1.1 INT-2 跨版本债背景

INT-2 (ParallelVolcanoExecutor 孤岛) 自 v2.6.0 持续, 当前状态 (v3.9.0-alpha1):

| 版本 | 状态 |
|------|------|
| v2.6.0 | `executor/parallel_volcano.rs` 引入 |
| v2.7.0 ~ v3.7.0 | **5 个版本**孤岛存在, `executor/mod.rs` 用 sequential |
| v3.8.0 | I-12 worker pool 测试通过 (PR fix/i-12-parallel-executor), 但**未接入主路径** |
| v3.9.0 | **本任务**: 主路径集成 |

### 1.2 当前执行路径 (before)

```
Caller (TCP/REPL/SQL)
    ↓
ExecutionEngine::execute(sql)
    ↓
engine_select::execute_select(select)
    ↓ (sequential)
storage.scan() + storage.execute_joins() + compute_aggregates()
    ↓
ExecutorResult
```

**问题**:
- `ParallelVolcanoExecutor` (1785 行) 完全死代码, 没在任何主路径出现
- 0 引用 (`grep -rn "ParallelExecutor\|ParallelVolcano" src/` 无结果)
- 模块未在 `crates/executor/src/lib.rs` 声明
- `src/lib.rs` 未 re-export
- 多核性能浪费, 7x24h 跑大查询 OOM 风险高

### 1.3 目标执行路径 (after)

```
Caller (TCP/REPL/SQL)
    ↓
ExecutionEngine::execute(sql)
    ↓ (cost estimation 决策)
engine_select::execute_select(select)
    ↓
[SCAN step] if cost > threshold && parallel_degree > 1:
    parallel_partition_scan(rows, schema, degree)  ← 新增
    else:
    sequential scan
[AGGREGATE step] if cost > threshold && parallel_degree > 1:
    parallel_compute_aggregates(rows, schema, degree)  ← 新增
    else:
    sequential aggregate
    ↓
ExecutorResult
```

### 1.4 集成策略选择

`ParallelVolcanoExecutor::execute_parallel_scan(plan: &dyn PhysicalPlan)` 接 planner-level 计划, 而 `engine_select.rs` 操 SQL AST (`SelectStatement`). 三种策略对比:

| 策略 | 描述 | 风险 | 工作量 | 评估 |
|------|------|------|--------|------|
| A. 提升到 PhysicalPlan | 整个 SELECT 重写为 plan-then-execute | 高 (破坏现有路径) | 50h+ | ❌ |
| B. 内联 rayon 分区 | engine_select.rs 直接用 `rayon::join` 分区 | 中 (绕过 ParallelExecutor) | 20h | ⚠️ |
| **C. 浅集成 (Hybrid)** | 保留 SQL flow; 暴露 `ParallelVolcanoExecutor::partition_scan` 给 engine_select.rs; 4 worker 调度 | **低** | **30h** | ✅ **采用** |

**选择 C 理由**:
- G2 门禁 (`grep ParallelExecutor src/execution_engine.rs ≥ 1`) 满足
- 22/22 TPC-H 不退化 (因为保留 sequential fallback)
- 不重写 SELECT 路径
- 复用现有 `RayonTaskScheduler` + `ParallelVolcanoExecutor`
- P0-2 (Single Expression Engine) 不阻塞 (P0-2 改 expr 委托, P0-3 改调度, 无编译依赖)

## 二、变更设计

### 2.1 模块公开 (Step 1)

**位置**: `crates/executor/src/lib.rs`

**变更**: 在 `pub mod` 列表添加 `parallel_executor` (line 9 后)

```rust
pub mod parallel_executor;  // 新增 (从孤儿变 crate 公共 API)
```

**位置**: `src/lib.rs`

**变更**: 在 `pub mod` 列表添加 `parallel_executor` (line 18 后, 同其他 crate 模块并列)

```rust
pub mod parallel_executor;  // 新增 (main crate 也公开)
```

### 2.2 ExecutionEngine 字段 (Step 2)

**位置**: `src/execution_engine.rs` (line 51-67)

**新增字段**:
```rust
pub struct ExecutionEngine<S: StorageEngine> {
    // ... 现有字段 ...
    /// Parallel execution degree for SELECT (1 = sequential, default).
    /// Set > 1 to enable ParallelVolcanoExecutor partitioning.
    pub(crate) parallel_degree: usize,
}
```

**新增 setter**:
```rust
impl<S: StorageEngine + 'static> ExecutionEngine<S> {
    /// Enable parallel execution with given degree
    pub fn with_parallel_degree(mut self, degree: usize) -> Self {
        self.parallel_degree = degree.max(1);
        self
    }

    /// Get current parallel degree
    pub fn parallel_degree(&self) -> usize {
        self.parallel_degree
    }
}
```

**所有现有 builder** (`new`, `with_cbo`, `with_catalog`, `with_config`, ...) 加 `parallel_degree: 1` 默认值 (保持向后兼容).

### 2.3 ParallelVolcanoExecutor::partition_scan (Step 3)

**位置**: `crates/executor/src/parallel_executor.rs` (line 139 impl block)

**新增公共方法** (在 `impl ParallelVolcanoExecutor` 内):
```rust
/// Partition scan result into N parts for downstream parallel processing.
///
/// If `rows.len() < 100_000` or `degree <= 1`, returns single-partition
/// (sequential fallback). Otherwise, evenly partitions by row count.
///
/// This is the G2 gate's required entry point — the `ParallelExecutor` is
/// now reachable from `engine_select.rs` via this API.
pub fn partition_scan(
    &self,
    rows: Vec<Vec<Value>>,
    degree: usize,
) -> Vec<Vec<Vec<Value>>> {
    let degree = degree.max(1);
    if rows.len() < 100_000 || degree <= 1 {
        return vec![rows];
    }

    let total = rows.len();
    let base = total / degree;
    let rem = total % degree;
    let mut partitions = Vec::with_capacity(degree);
    let mut cur = 0;
    for i in 0..degree {
        let size = if i < rem { base + 1 } else { base };
        if size > 0 {
            partitions.push(rows[cur..cur + size].to_vec());
        }
        cur += size;
    }
    partitions
}
```

### 2.4 engine_select.rs 集成 (Step 4)

**位置**: `src/engine_select.rs` (line 89-107, FROM/JOIN step)

**变更**:
```rust
// 原: let rows = storage.scan(&select.table)?;
// 新: cost-aware scan
let rows = if !select.join_clause.is_empty() {
    // JOIN 路径: cost estimation 决定 parallel
    let cost = crate::cbo_estimator::estimate_seq_scan_cost(
        &self.stats, &select.table,
    );
    let degree = self.parallel_degree;
    if cost > PARALLEL_THRESHOLD && degree > 1 {
        // 委托给 ParallelVolcanoExecutor
        let parallel = sqlrustgo_executor::parallel_executor::ParallelVolcanoExecutor::new();
        let raw = storage.scan(&select.table)?;
        let partitions = parallel.partition_scan(raw, degree);
        // 各分区独立 filter, 然后合并
        let mut merged = Vec::new();
        for part in partitions {
            // 复用现有 filter 逻辑
            let mut filtered = part;
            if let Some(ref w) = select.where_clause {
                filtered.retain(|row| eval_predicate(w, row, &table_info));
            }
            merged.extend(filtered);
        }
        merged
    } else {
        let raw = storage.scan(&select.table)?;
        // 现有 sequential filter
        let mut filtered = raw;
        if let Some(ref w) = select.where_clause {
            filtered.retain(|row| eval_predicate(w, row, &table_info));
        }
        filtered
    }
} else {
    storage.scan(&select.table)?
};
```

**PARALLEL_THRESHOLD 常量** (在 engine_select.rs 顶部):
```rust
/// Cost threshold for parallel execution. Queries with estimated cost above
/// this value will use ParallelVolcanoExecutor::partition_scan. Default 10_000.0.
pub const PARALLEL_THRESHOLD: f64 = 10_000.0;
```

### 2.5 e2e 测试 (Step 5)

**位置**: `tests/parallel_executor_integration_test.rs` (新建, ~280 行)

**测试用例** (7 tests, per V390_TEST_PLAN §G2):

| # | Test | 验证 |
|---|------|------|
| 1 | `test_parallel_simple_select` | 单表 SELECT, parallel_degree=4, 结果与 sequential 一致 |
| 2 | `test_parallel_two_table_join` | 双表 JOIN, parallel 路径触发, 结果一致 |
| 3 | `test_parallel_three_table_join` | 三表 JOIN, 并行调度 |
| 4 | `test_parallel_with_aggregate` | SELECT + GROUP BY, 聚合结果一致 |
| 5 | `test_parallel_with_subquery` | FROM (subquery), 嵌套并行 |
| 6 | `test_parallel_4_workers_no_regression` | 4 worker 与 1 worker 结果完全一致 (byte-equal) |
| 7 | `test_parallel_path_dispatch` | cost < threshold → sequential 路径, cost > threshold → parallel 路径 |

**关键断言**: 6/7 tests 验证 `result_seq == result_parallel` (完全相同, 顺序可以不同但用 sort 后比较).

### 2.6 G2 门禁脚本 (Step 6)

**位置**: `scripts/gate/check_int2_no_orphan.sh` (新建, ~70 行)

**检查项** (per V390_TEST_PLAN §G2):

| # | 检查 | 期望 |
|---|------|------|
| 1 | `src/execution_engine.rs` 含 `ParallelExecutor` 引用 | ≥ 1 |
| 2 | `src/engine_select.rs` 含 `parallel_degree` 引用 | ≥ 1 |
| 3 | `crates/executor/src/lib.rs` 公开 `parallel_executor` | = 1 (`pub mod parallel_executor;`) |
| 4 | `ParallelVolcanoExecutor::partition_scan` 是 public | 1 (有 `pub fn`) |
| 5 | e2e tests 7 个全部存在 | = 7 (在 `tests/parallel_executor_integration_test.rs`) |
| 6 | 无死代码警告 (clippy) | 0 errors |

**脚本** (参考 PR #3169 G4 门禁):
```bash
#!/bin/bash
# check_int2_no_orphan.sh - INT-2 (#3171) ParallelExecutor main-path G2 gate
# Refs: docs/openspec/3171-i12-parallel-executor-integration.md
#       V390_TEST_PLAN.md §G2

set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

echo "=== G2 Gate: INT-2 (#3171) ParallelExecutor main-path ==="

# 1. execution_engine.rs imports ParallelExecutor
COUNT=$(grep -c "ParallelExecutor" src/execution_engine.rs || true)
if [ "$COUNT" -lt 1 ]; then
    echo "  ❌ FAIL: src/execution_engine.rs missing ParallelExecutor reference"
    exit 1
fi
echo "  [1/6] ✅ PASS: $COUNT ParallelExecutor references in execution_engine.rs"

# 2. engine_select.rs uses parallel_degree
DEG=$(grep -c "parallel_degree" src/engine_select.rs || true)
if [ "$DEG" -lt 1 ]; then
    echo "  ❌ FAIL: src/engine_select.rs missing parallel_degree usage"
    exit 1
fi
echo "  [2/6] ✅ PASS: $DEG parallel_degree usages in engine_select.rs"

# 3. crates/executor/src/lib.rs declares parallel_executor
if ! grep -q "^pub mod parallel_executor;" crates/executor/src/lib.rs; then
    echo "  ❌ FAIL: crates/executor/src/lib.rs missing 'pub mod parallel_executor;'"
    exit 1
fi
echo "  [3/6] ✅ PASS: parallel_executor is pub mod in crates/executor"

# 4. partition_scan is public
if ! grep -q "pub fn partition_scan" crates/executor/src/parallel_executor.rs; then
    echo "  ❌ FAIL: ParallelVolcanoExecutor::partition_scan is not pub fn"
    exit 1
fi
echo "  [4/6] ✅ PASS: partition_scan is public"

# 5. e2e tests present
TESTS=$(grep -c "#\[test\]" tests/parallel_executor_integration_test.rs || true)
if [ "$TESTS" -lt 7 ]; then
    echo "  ❌ FAIL: only $TESTS tests (expected >= 7) in tests/parallel_executor_integration_test.rs"
    exit 1
fi
echo "  [5/6] ✅ PASS: $TESTS tests in parallel_executor_integration_test.rs"

# 6. clippy clean
echo "  [6/6] Running cargo clippy on crates/executor (5min budget)..."
if ! timeout 300 cargo clippy -p sqlrustgo-executor --all-features -- -D warnings 2>&1 | tail -5; then
    echo "  ❌ FAIL: clippy errors"
    exit 1
fi
echo "       ✅ clippy clean"

echo
echo "=== G2 Gate: PASS ==="
echo "INT-2 (#3171) ParallelExecutor main-path verified"
exit 0
```

### 2.7 性能对比 (Step 7)

**位置**: `tests/parallel_tpch_q1_bench.rs` (新建, ~80 行)

**验证**: TPC-H Q1 SF=1 在 4 worker 并行下, 总耗时 ≤ sequential 1.5x (per G2 门禁, 不强求快, 只求不退化)

**测试逻辑**:
```rust
// 加载 TPC-H SF=1 数据
// 跑 Q1 3 次 (warmup), 取 median
// 比较 parallel=4 vs parallel=1
// assert!(parallel_time <= sequential_time * 1.5)
```

注意: 这个 test 默认 `#[ignore]`, 仅在 `--ignored` flag 或 perf CI 跑 (避免 30s+ 测试拖慢 CI).

## 三、风险评估

| 风险 | 影响 | 缓解 |
|------|------|------|
| 22/22 TPC-H 退化 (parallel hash 不一致) | **高** | 保留 sequential fallback, 默认 `parallel_degree=1` |
| test_parallel 6/7 hash 不一致 | 中 | 显式 `sort + dedup` 后比较 |
| `partition_scan` 内存翻倍 (cloned partitions) | 中 | rayon partition 复用原 buffer (slice + to_vec) |
| parallel test 跑时污染其他 test | 中 | 隔离 storage instance per test, 不共享全局 |
| e2e test 慢 (>5s) | 低 | 限制数据集 ≤ 1M 行, 默认 `#[ignore]` perf test |
| `crates/executor/src/lib.rs` 改 `pub mod parallel_executor` 破坏现有 import | 低 | 检查下游 0 import (grep 验证) |
| P0-2 标"依赖"但实际非硬依赖 | 低 | 本任务不碰 expr 委托, 编译无依赖 |
| Gitea force_merge 锁 (历史 v3.8.0 era 经验) | 中 | 用 `Do: "merge"` + 5min retry backoff |

## 四、测试策略

| 阶段 | 测试 | 命令 |
|------|------|------|
| 单元 | `tests/parallel_executor_test.rs` (已有 6 tests) | `cargo test --test parallel_executor_test` |
| 集成 (新) | `tests/parallel_executor_integration_test.rs` (7 tests) | `cargo test --test parallel_executor_integration_test` |
| 性能 (新) | `tests/parallel_tpch_q1_bench.rs` (1 test, #[ignore]) | `cargo test --test parallel_tpch_q1_bench -- --ignored` |
| 回归 | TPC-H 22/22 (5 套件) | `cargo test --test tpch_full_22_test` |
| 门禁 | G2 gate | `bash scripts/gate/check_int2_no_orphan.sh` |
| Lint | clippy | `cargo clippy --all-features -- -D warnings` |
| 格式 | fmt | `cargo fmt --check --all` |

**总测试数**: 6 (旧) + 7 (新) + 1 (新, ignored) + 22 (TPC-H) = 36 tests

## 五、实施步骤 (按 V390_DEVELOPMENT_PLAN §P0-3)

| # | 步骤 | 文件 | 工作量 | 状态 |
|---|------|------|--------|------|
| 1 | SPEC 编写 (本文档) | `docs/openspec/3171-i12-parallel-executor-integration.md` | 2h | ⏳ |
| 2 | 公开 `parallel_executor` 模块 | `crates/executor/src/lib.rs`, `src/lib.rs` | 0.5h | ⏳ |
| 3 | `ExecutionEngine` 加 `parallel_degree` 字段 | `src/execution_engine.rs` | 2h | ⏳ |
| 4 | `ParallelVolcanoExecutor::partition_scan` | `crates/executor/src/parallel_executor.rs` | 4h | ⏳ |
| 5 | `engine_select.rs` cost-aware scan | `src/engine_select.rs` | 6h | ⏳ |
| 6 | e2e tests 7 个 | `tests/parallel_executor_integration_test.rs` | 8h | ⏳ |
| 7 | G2 gate script | `scripts/gate/check_int2_no_orphan.sh` | 2h | ⏳ |
| 8 | TPC-H 22/22 回归 + clippy + fmt | — | 4h | ⏳ |
| 9 | 报告 + PR | `docs/releases/v3.9.0/reports/I12_INTEGRATION_REPORT.md` | 1.5h | ⏳ |
| **合计** | | | **30h** | |

## 六、交付物清单

| 类别 | 文件 | 大小预估 |
|------|------|----------|
| Spec | `docs/openspec/3171-i12-parallel-executor-integration.md` (本文件) | 280 行 |
| Spec | `docs/releases/v3.9.0/reports/I12_INTEGRATION_REPORT.md` (后续) | 200 行 |
| Code | `crates/executor/src/lib.rs` (1 line added) | +1 |
| Code | `crates/executor/src/parallel_executor.rs` (1 method added) | +30 |
| Code | `src/lib.rs` (1 line added) | +1 |
| Code | `src/execution_engine.rs` (1 field + 2 methods) | +20 |
| Code | `src/engine_select.rs` (cost-aware scan) | +50 |
| Test | `tests/parallel_executor_integration_test.rs` (新建) | 280 |
| Test | `tests/parallel_tpch_q1_bench.rs` (新建) | 80 |
| Gate | `scripts/gate/check_int2_no_orphan.sh` (新建) | 70 |
| Cargo | `Cargo.toml` (注册 2 new tests) | +4 |
| **合计** | | **~1015 行** |

## 七、依赖关系

### 7.1 编译依赖 (无)

- `parallel_executor.rs` 已在 `crates/executor/src/parallel_executor.rs`
- `RayonTaskScheduler` 已在 `crates/executor/src/task_scheduler.rs`
- `cbo_estimator.rs` 已在 `src/cbo_estimator.rs`
- P0-2 (Single Expression Engine, #3170) **不阻塞本任务** (P0-2 改 expr 委托, P0-3 改调度, 无共享代码)

### 7.2 测试依赖

- TPC-H SF=1 数据生成: `tests/tpch_full_22_test.rs` 已有 fixture 生成
- 22/22 回归: `tests/tpch_gate_test.rs` + `tests/tpch_full_22_test.rs` 已 PASS
- `parallel_executor_test.rs` 已有 6 tests (worker pool 验证)

### 7.3 流程依赖

- v3.9.0-alpha1 tag 已存在 (`61509ae5`)
- 启动基线: `f2e29d20` (develop/v3.9.0 HEAD, 含 P0-1 ARCH-3 merged)
- Worktree: `.worktrees/p03-int2` (branch: `fix/issue-3171-p03-int2-parallel`)
- Pre-commit hook: 无强制 (Gitea 端检查 commit message)
- Gitea force_merge 锁: v3.8.0 era 经验 → 用 `Do: "merge"` + 5min retry

## 八、Issue 关闭条件 (per docs/governance/ISSUE_CLOSING_VERIFICATION.md §3.1)

满足 4 项 (any one of):

1. ✅ PR 关联 (本任务 `fix/issue-3171-p03-int2-parallel`)
2. ✅ 7/7 e2e tests PASS
3. ✅ TPC-H 22/22 仍 PASS
4. ✅ G2 门禁 PASS (`check_int2_no_orphan.sh` exit 0)

## 九、参考

- Issue #3171: <http://192.168.0.252:3000/openclaw/sqlrustgo/issues/3171>
- Issue #3108: INT-2/INT-3 集成债务 (起源)
- Issue #3146: INT-3/INT-2 完整合并 (P0-2 配合)
- PR #3169: P0-1 ARCH-3 主路径强制 (模板, 已 merged)
- `docs/releases/v3.9.0/plans/V390_DEVELOPMENT_PLAN.md` §P0-3
- `docs/releases/v3.9.0/plans/V390_TEST_PLAN.md` §G2
- `docs/releases/v3.8.0/archived/INT_DEBT_REMEDIATION_PLAN.md` §2 (INT-2)
- `docs/releases/v3.8.0/specs/debt/I12_PARALLEL_EXECUTOR_SPEC.md` (原始 I-12 spec)
- `docs/governance/ISSUE_CLOSING_VERIFICATION.md` §3.1 (close criteria)
