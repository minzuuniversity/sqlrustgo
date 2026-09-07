## Context

Issue #4816 — `NTILE(N) OVER (ORDER BY col)` 返回错误的桶分配。当前
bug 推测:NTILE 的实现用了 `bucket = (row_index % n_buckets) + 1`,这
给出 (1,2,3,4,1,2,3,4,...) 的 round-robin 分布,不是 balanced。

正确算法 (PostgreSQL doc):

```
n = partition row count
k = n_buckets (NTILE arg)
rows_per_bucket = n / k         // floor
remainder = n % k
for i in 0..n:
    bucket = i / rows_per_bucket   // (0-based bucket index)
    if bucket >= k: bucket = k - 1  // overflow safety
    return bucket + 1              // 1-based
```

但 PostgreSQL 的"balanced"行为是: 前 `remainder` 个桶各 +1 行,大桶
在前。即:

```
n = 10, k = 4
rows_per_bucket = 10 / 4 = 2
remainder = 10 % 4 = 2
大桶数 = 2 → bucket 1, 2 各 3 行
小桶数 = 4 - 2 = 2 → bucket 3, 4 各 2 行
顺序: 1,1,1, 2,2,2, 3,3, 4,4 ✓
```

这与 `bucket = i / rows_per_bucket + 1` 在 remainder = 0 时一致,但
remainder > 0 时前者需要额外 offset:`if i < remainder * (rows_per_bucket + 1) { bucket = i / (rows_per_bucket + 1) } else { bucket = remainder + (i - remainder*(rows_per_bucket+1)) / rows_per_bucket }`。

## Approach

### A1. 实现 balanced NTILE

在 `src/executor/window.rs` 的 `evaluate_window_function` 中,NTILE 分支:

```rust
WindowFunction::Ntile(n_buckets) => {
    if n_buckets <= 0 {
        return Err(SqlError::InvalidArgument(
            format!("NTILE requires positive bucket count, got {}", n_buckets)
        ));
    }
    let n = partition_rows.len();
    if n == 0 {
        return Ok(vec![]);  // empty partition → empty result
    }
    
    let rows_per_big_bucket = (n + n_buckets - 1) / n_buckets;  // ceil
    let big_bucket_count = n % n_buckets;  // 前 big_bucket_count 个大桶
    
    let mut bucket_assignments = vec![0; n];
    let mut idx = 0;
    for big_bucket in 0..n_buckets {
        let bucket_size = if big_bucket < big_bucket_count {
            rows_per_big_bucket
        } else {
            rows_per_big_bucket - 1  // if remainder > 0
        };
        // 防 overflow:最后一桶可能较小
        let actual_size = (n - idx).min(bucket_size);
        for _ in 0..actual_size {
            bucket_assignments[idx] = big_bucket + 1;  // 1-based
            idx += 1;
        }
        if idx >= n { break; }
    }
    
    Ok(bucket_assignments)
}
```

### A2. 集成到 Window Executor

`evaluate_window_function` 已经处理 PARTITION BY + ORDER BY。NTILE
需要在排序后的 partition 上 evaluate,把 bucket assignment 作为新列
添加到 projection result。

### A3. NULLS FIRST/LAST interaction

如果 NTILE(4) OVER (ORDER BY col NULLS LAST),NULL 排在 partition 末
尾。NTILE 应该把 NULL 行的 bucket 编号正确计算 (因为 partition 内
idx 包含 NULL)。

测试:10 行有 2 NULL,8 非 NULL → NTILE(4) 仍把 10 行分到 4 桶,桶 1-2
含非 NULL,桶 3-4 含 NULL (因为 idx = 8,9 在 bucket 3, 4 范围内)。

### A4. 多 partition

```sql
SELECT cat, NTILE(2) OVER (PARTITION BY cat ORDER BY id) FROM t;
```

每个 partition 独立 count + bucket 分配。当前 PARTITION BY 路径已经
保证 partition boundary,所以 NTILE 内部循环只在当前 partition 上。

## Files Changed

| File | Lines | Purpose |
|------|-------|---------|
| `src/executor/window.rs` | +50 | NTILE 算法实现 + 大桶前小桶后分布 |
| `src/executor/window_functions.rs` | +30 | NTILE function 注册 + arg validation |
| `src/error.rs` | +10 | InvalidArgument for NTILE(<=0) |
| `tests/integration/sql/p3_ntile_4816_test.rs` | +200 | 8 tests |

Total: ~290 lines, ~4 files touched.

## Verification

| Test | Expected |
|------|----------|
| `cargo build --all-features` | clean |
| `p3_ntile_4816_test` | 8/8 PASS |
| `window_function_test` | no regression |
| `partition_test` | no regression |
| `parser_e2e_test` | 249/249 PASS |

## Non-Goals

- WIDTH_BUCKET (Oracle-style numeric bucketing)
- NTILE with custom frames
- NTILE inside aggregate (NTILE 不能在 aggregate 内)

## Difficulty Tier

**🟡 MEDIUM** — 算术正确性 + edge case 全覆盖(N=0, k>N, k=1, k<=0),
与现有 window executor 集成。算法本身 10 行,但测试矩阵需要 8 个
case 防 regression。