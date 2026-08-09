# Design: MEDIAN 聚合函数实现

## 1. 架构概览

MEDIAN 是一种有序聚合函数，需要在计算前对数据进行排序。与 SUM/AVG/COUNT 不同，MEDIAN 无法通过单遍扫描增量计算。

```
输入列: [3, 1, 4, 1, 5, 9, 2, 6]
排序后: [1, 1, 2, 3, 4, 5, 6, 9]
中位数: (3 + 4) / 2 = 3.5  (偶数个元素，取平均值)
```

## 2. 当前状态分析

### 2.1 Parser 层

`AggregateFunction` 枚举（`crates/planner/src/lib.rs`）当前定义：
```rust
pub enum AggregateFunction {
    Count,
    Sum,
    Avg,
    Min,
    Max,
    // 缺失: Median
}
```

### 2.2 Executor 层

`crates/executor/src/expr/mod.rs` 的 `eval_fn` 分发逻辑：
- 已有 STDDEV_POP/VAR_POP/VAR_SAMP 实现
- 已有 GROUP_CONCAT 实现
- MEDIAN 关键字未被识别，直接返回 NULL

### 2.3 聚合框架

`crates/executor/src/parallel_group_by.rs` 的 `AggregateCall`：
```rust
match agg.func {
    AggregateFunction::Count => self.update_count(...),
    AggregateFunction::Sum => self.update_sum(...),
    AggregateFunction::Avg => self.update_avg(...),
    AggregateFunction::Min => self.update_min(...),
    AggregateFunction::Max => self.update_max(...),
    // 缺失: Median
}
```

## 3. 实现方案

### 3.1 AggregateFunction 枚举扩展

```rust
// crates/planner/src/lib.rs
#[derive(Debug, Clone, PartialEq)]
pub enum AggregateFunction {
    Count,
    Sum,
    Avg,
    Min,
    Max,
    Median,  // 新增
}
```

Display 实现添加：
```rust
AggregateFunction::Median => "MEDIAN",
```

### 3.2 eval_fn 分发逻辑

```rust
// crates/executor/src/expr/mod.rs
"COUNT" => count_aggregate(args),
"SUM" => sum_aggregate(args),
"AVG" => avg_aggregate(args),
"MIN" => min_aggregate(args),
"MAX" => max_aggregate(args),
"STDDEV" | "STDDEV_POP" => stddev_variance(args, true),
"VAR_POP" | "VAR" => variance_value(args, true),
"VAR_SAMP" => variance_value(args, false),
"GROUP_CONCAT" => group_concat(args),
"MEDIAN" => median_aggregate(args),  // 新增
```

### 3.3 median_aggregate 实现

```rust
fn median_aggregate(args: &[Value]) -> Value {
    let mut values: Vec<f64> = Vec::new();
    for v in args {
        match v {
            Value::Integer(i) => values.push(*i as f64),
            Value::Float(f) => values.push(*f),
            Value::Null => continue,
            _ => return Value::Null,
        }
    }

    if values.is_empty() {
        return Value::Null;
    }

    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let len = values.len();
    if len % 2 == 0 {
        // 偶数元素：中间两值平均
        let mid = len / 2;
        Value::Float((values[mid - 1] + values[mid]) / 2.0)
    } else {
        // 奇数元素：中间值
        Value::Float(values[len / 2])
    }
}
```

### 3.4 并行化感知

MEDIAN 作为非增量聚合函数，在并行执行场景下需要特殊处理：

| 方案 | 描述 | 权衡 |
|------|------|------|
| 单分区优先 | 强制单分区计算 MEDIAN | 无法利用并行化，适合小数据集 |
| 两阶段 | 第一阶段计算分位数边界，第二阶段合并 | 复杂，但可并行 |

当前实现优先覆盖单节点场景（`--executor-parallelism=1` 或强制单分区），并行 MEDIAN 留待后续迭代。

### 3.5 类型处理

| 输入类型 | 输出类型 | 说明 |
|----------|----------|------|
| 整数列 | Float | 中位数可能为小数 |
| 浮点列 | Float | 直接返回 |
| NULL | Null | 跳过 NULL 值 |
| 混合类型 | Null | 不支持返回错误 |

## 4. 关键决策

| 决策点 | 选项 | 选择 | 理由 |
|--------|------|------|------|
| 偶数元素策略 | 中间低值 / 中间高值 / 平均 | 平均 | MySQL 兼容 |
| 空集合策略 | NULL / 0 | NULL | SQL 标准行为 |
| NULL 处理 | 跳过 / 计入 | 跳过 | MySQL MEDIAN 行为 |
| 并行支持 | 支持 / 不支持 | 暂不支持 | 单节点优先 |

## 5. Fixture 设计

### `tests/compat/mysql_v3_13/median_basic.sql`

```sql
-- name: median_basic
-- expect: PASS

CREATE TABLE t (val INT);
INSERT INTO t VALUES (1), (2), (3), (4), (5);

-- 奇数元素：中位数为 3
SELECT MEDIAN(val) FROM t;

-- 偶数元素：平均中间两值
INSERT INTO t VALUES (6);
SELECT MEDIAN(val) FROM t;

-- 全 NULL
INSERT INTO t VALUES (NULL), (NULL);
SELECT MEDIAN(val) FROM t;

DROP TABLE t;
```

### `tests/compat/mysql_v3_13/median_grouped.sql`

```sql
-- name: median_grouped
-- expect: PASS

CREATE TABLE t (cat TEXT, val INT);
INSERT INTO t VALUES ('A', 1), ('A', 3), ('A', 5), ('B', 2), ('B', 4);

-- 分组中位数
SELECT cat, MEDIAN(val) FROM t GROUP BY cat ORDER BY cat;

DROP TABLE t;
```

## 6. 验证方式

```bash
# 运行 MEDIAN 相关测试
cargo test -p sqlrustgo-executor median

# 运行兼容测试
./scripts/gate/run_compat_tests.sh

# 验证偶数/奇数中位数正确性
cargo test -p sqlrustgo-executor -- test_median_*
```

## 7. 失败模式

- 输入全为 NULL：返回 `NULL`
- 空表：返回 `NULL`
- 非数值类型：返回 `NULL`（非 error，保持兼容）
- 并行模式下：当前实现可能返回非确定性结果（仅警告，不 panic）
