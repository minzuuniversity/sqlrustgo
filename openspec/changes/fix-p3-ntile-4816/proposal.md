## Why

Issue #4816: `SELECT NTILE(4) OVER (ORDER BY id) FROM t` 在 sqlrustgo
中分桶错误 — NTILE 应该把 N 行平均分配到 4 个桶,每个桶相差不超过
1 行,但 sqlrustgo 返回 (1,1,1,1) 或 (1,2,3,4) 等错误的桶分配。

NTILE 是 SQL 标准窗口函数,PostgreSQL / MySQL 8.0+ / SQL Server /
Oracle 行为一致:
- 10 行分配到 4 桶 → 4,3,3 buckets = (1,1,1,2,2,2,3,3,3,4)
- 4 行分配到 4 桶 → (1,1,1,1)
- 5 行分配到 4 桶 → (1,2,3,4,4)
- 0 行或 n_buckets > rows → 全部为 1 (即每行一个 bucket)

错误分布会破坏:
- 分位分析(median = NTILE(2) = 2)
- 数据采样(SELECT WHERE NTILE(10) = 1 → top 10%)
- PageRank / quartile 计算

## What Changes

1. **AST 验证**: NTILE 已经存在,验证 NTILE arg (Int) 解析正确。
2. **Bucket 算法**: 在 `window.rs` 或 `window_functions.rs` 中:
   ```
   rows_per_bucket = ceil(N / n_buckets)   // 实际是 floor + 1 if remainder
   remainder = N mod n_buckets
   大桶数 = remainder, 每个大桶有 rows_per_bucket 行
   小桶数 = n_buckets - remainder, 每个小桶有 rows_per_bucket - 1 行
   ```
   例如 10 行 / 4 桶:
     - rows_per_bucket = ceil(10/4) = 3
     - remainder = 10 mod 4 = 2
     - 2 大桶各 3 行, 2 小桶各 2 行 → 2*3 + 2*2 = 10 ✓
     - 桶分布: 1,1,1,2,2,2,3,3,4,4 ✓
3. **Special cases**:
   - `n_buckets = 1` → 所有行 bucket = 1。
   - `n_buckets >= N` → 前 N 行 bucket = 1..N,后 (n - N) 个 bucket
     为空(实际 SQL 不返回空 bucket)。
   - `N = 0` → 0 行输出。
   - `n_buckets <= 0` → 错误。

## Capabilities

### New Capabilities

- `window-ntile-bucketing`: NTILE(N) MUST 按 "balanced buckets"
  算法分桶,前 `N mod n_buckets` 个桶各 +1 行。

## Out of Scope

- NTILE 在 `RANGE BETWEEN ...` frame 内的特殊行为:本 PR 不支持
  NTILE with frames,要求 NTILE 必须在整个 partition 内工作。
- WIDTH_BUCKET (Oracle 风格): 推迟到 v3.14。
- PERCENT_RANK / CUME_DIST (类似 rank 函数): 不在本 PR。

## Verification

- 新测试 `tests/integration/sql/p3_ntile_4816_test.rs`:
  - `ntile_4_of_10_rows` — issue anchor:10 行 / 4 桶 = (1,1,1,2,2,2,3,3,4,4)。
  - `ntile_exact_division` — 8 行 / 4 桶 = (1,1,2,2,3,3,4,4)。
  - `ntile_more_buckets_than_rows` — 3 行 / 5 桶 = (1,2,3)。
  - `ntile_single_bucket` — 5 行 / 1 桶 = (1,1,1,1,1)。
  - `ntile_with_partition` — NTILE(2) OVER (PARTITION BY cat ORDER BY id)。
  - `ntile_zero_buckets_error` — NTILE(0) 应该报错。
  - `ntile_negative_buckets_error` — NTILE(-1) 应该报错。
  - `ntile_large_n` — 100 行 / 7 桶,验证 remainder 分布。

Total: 8 tests。

- `cargo test --test p3_ntile_4816_test` 8/8 PASS。
- 回归: window_function_test, partition_test (确保 NTILE 不破坏其他
  window functions)。

## Risks / Trade-offs

- **ORDER BY NULL 行为**: NTILE(4) OVER (ORDER BY id) 中 NULL 值位置
  影响分桶。PostgreSQL 把 NULL 排在最后 (default NULLS LAST)。本 PR
  沿用默认 NULLS LAST 行为,与现有 window 排序一致。
- **Tie 行为**: ORDER BY 中 ties 的 row 如何分配桶?标准是:ties 必须在
  同一个 bucket。本 PR 使用 stable sort + bucket counter 保证 ties
  同桶。
- **Performance**: NTILE 需要 partition 内的 full row count 才能算
  bucket size,这意味着需要 2-pass (先 count, 再 assign)。当前 window
  executor 已经 2-pass (materialize + window 评估),新增成本可忽略。