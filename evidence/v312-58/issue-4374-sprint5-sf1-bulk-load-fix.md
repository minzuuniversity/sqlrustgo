# v312-58 / Sprint 5 — bulk_load O(N²) → O(N) fix + SF=1 perf results

**Branch**: `fix/v312-58-q20-sprint4-step15-16` + local `bulk_load_fix_sprint5` (uncommitted)
**Date**: 2026-08-25
**Author**: openclaw (claude-code minimax-m3 session)
**Verdict**: ✅ **Q17 SF=1 PASS (68.38s)** + **Q22 SF=1 PASS (3.97s)** + ⚠️ **Q20 SF=1 still TIMEOUT** (per upstream commit `0d31cc892` Q20 deferred to v3.13 Phase 3 HashSemiJoin)

---

## 1. 根因 — MemoryStorage::insert() 的 O(N²) 内存 churn

### 1.1 文件位置

`crates/storage/src/engine.rs:1473-1538` (insert 函数)

### 1.2 原始代码 (有 bug)

```rust
fn insert(&mut self, table: &str, records: Vec<Record>) -> SqlResult<()> {
    let table_key = table.to_lowercase();
    let padded: Vec<Record> = if let Some(info) = self.table_infos.get(&table_key).cloned() {
        let ncols = info.columns.len();
        let auto_inc_cols: Vec<usize> = info.columns.iter().enumerate()
            .filter_map(|(idx, c)| if c.auto_increment { Some(idx) } else { None })
            .collect();
        // ❌ Bug: 不管有没有 auto_inc 都 clone 全部现有行
        let existing_rows = self.tables.get(&table_key).cloned().unwrap_or_default();
        let mut next_auto: i64 = auto_inc_cols.iter()
            .filter_map(|&idx| {
                existing_rows.iter()
                    .filter_map(|row| row.get(idx).and_then(|v| match v {
                        Value::Integer(n) => Some(*n),
                        _ => None,
                    }))
                    .max()
            })
            .max()
            .map(|m| m + 1)
            .unwrap_or(1);
        records.into_iter().map(|mut row| { ... }).collect()
    } else { records };
    // ...
}
```

### 1.3 影响

`bulk_load_tbl_file` 把每 1024 行作为一个 batch 调用 `insert()`。lineitem/part/customer/nation/orders/supplier/partsupp 都**没有 auto_increment 列**,但每个 batch 都 clone 了当时的全部现有行,只为计算一个从来不会被读到的 `next_auto`。

**复杂度**:N 行 K batch → 总 clone 数 = Σ_{i=0}^{K-1} (i × batch_size) = O(K² × batch_size) = O(N² / batch_size)

| 规模 | batch 数 | 现有行 (clone count per batch) | 总 clone 行数 | 等效字节 (640B/row) |
|------|---------|---------------|-----------------|---------------------|
| 100K | 98 | 0, 1024, 2048, …, 98K | 4.8M | 3.1 GB |
| 1M | 977 | 0 … 977K | **488M** | **312 GB** |
| 6M (SF=1) | 5856 | 0 … 5856K | **17.2B** | **11 TB** |

11 TB 的内存 churn 直接把 SF=1 bulk_load 推到 hours 级(实际表现为挂死)。

---

## 2. 修复 (Sprint 5)

### 2.1 代码 diff 概要

`crates/storage/src/engine.rs:1483-1523` — 分支拆分:

```rust
// V312-58 Sprint 5 (Issue #4374 SF=1 wall-clock): without this
// guard, `bulk_load_tbl_file` for tables WITHOUT auto_increment
// (e.g. lineitem, part, customer) used to clone the existing
// row set on EVERY batch just to feed the auto_inc scan that
// never fires. SF=1 lineitem is ~6M rows in ~5856 batches of
// 1024 → ~17B row clones → multi-TB memory churn → bulk_load
// hangs. Splitting the branches keeps the auto_inc path
// unchanged and turns the non-auto_inc path from O(N^2) into
// O(N).
if auto_inc_cols.is_empty() {
    records.into_iter().map(|mut row| {
        while row.len() < ncols {
            let default = info.columns.get(row.len())
                .and_then(|c| c.default_value.as_deref())
                .map(parse_default_literal)
                .unwrap_or(Value::Null);
            row.push(default);
        }
        row
    }).collect()
} else {
    // 原 auto_inc 路径,行为不变
    let existing_rows = self.tables.get(&table_key).cloned().unwrap_or_default();
    let mut next_auto: i64 = /* ... */;
    records.into_iter().map(|mut row| { /* ... */ }).collect()
}
```

### 2.2 正确性论证

- 无 auto_increment 表的语义:**完全未改变**。只移除了无用的 `existing_rows.cloned()` 步骤,该步骤的结果只用于 `next_auto`,而 `next_auto` 只在有 auto_inc 列时被消费。
- 有 auto_increment 表(例如测试用例里使用 auto_increment 的表):走 else 分支,与原代码一致。
- padding 循环(为短行补默认值)只读 `info.columns`,不读 `existing_rows`,所以两个分支都正确。

---

## 3. 验证结果

### 3.1 单元测试 — `q17_1m_rss_regression` PASS

```text
=== Q17 1M RSS-bound regression (Issue #4374 Sprint 4) ===
Q17 1M: elapsed=10.86s, result: 1 rows × 1 cols, peak RSS=5693MB

V312-58 Sprint 3 diag:
try_scalar_agg_index_lookup calls=825 hits=825 pattern_fail=0 build=1
scalar_subq_cache hits=0 misses=0 fallback_execute_select=0
step15 entered=1 has_correlated=1 skipped_comma_consumed=0 no_where=0 q17_from_kind=0

Q17 1M engine value: 41743.15857142856
test q17_1m_rss_bounded ... ok

test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 20.31s
```

修复前:wall-clock 481s(bulk_load 占 470s)→ **修复后:20.31s total**,**24x 加速**。
正确性保持:`try_scalar_agg_index_lookup calls=825 hits=825 pattern_fail=0 build=1`。

### 3.2 SF=1 bulk_load bench PASS

```text
=== SF=1 bulk_load benchmark ===
part bulk_load START
part bulk_load DONE in 494.570368ms
lineitem bulk_load START (file size: 759863287 bytes)
lineitem bulk_load DONE in 21.801012108s
lineitem row count: 6001215
test sf1_bulk_load_bench ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 42.07s
```

**lineitem SF=1 bulk_load 从 hours(挂死)降到 21.8s**,6001215 行准确。

### 3.3 Q17 SF=1 perf — `q17_small_order_shortage_sf1` PASS

```text
=== Loading fixtures ===
part loaded: 1.438s
lineitem loaded: 29.179s
=== Executing Q17 ===
elapsed: 68.38432614s
row_count: 1
value: Float(249963.75857142854)
V312-58 Sprint 3 diag:
try_scalar_agg_index_lookup calls=4800 hits=4800 pattern_fail=0 build=1
scalar_subq_cache hits=0 misses=0 fallback_execute_select=0
step15 entered=1 has_correlated=1 skipped_comma_consumed=0 no_where=0 q17_from_kind=0
```

**Wall-clock 68.38s** (vs. budget 1800s per #4432 relaxation)。
**Oracle MATCH**:期望 `249963.75857142857` → 实际 `249963.75857142854`,diff ~3×10⁻¹¹ << 1e-3 tolerance。
**优化路径完全生效**:4800 hits / 4800 calls / build=1 / pattern_fail=0 / step15 entered=1 / has_correlated=1。

### 3.4 Q22 SF=1 perf — `q22_global_sales_opportunity_perf_sf1` PASS

```text
Q22 elapsed: 3.968603314s
test q22_global_sales_opportunity_sf1 ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 9.66s
```

**Wall-clock 3.97s** (vs. budget 300s per #4381 AC)。
**Oracle MATCH**:row_count=7(预期 SQLite 7 行)✓。

### 3.5 Q20 SF=1 perf — TIMEOUT

**结果**:kill -9 强制终止,总 CPU 时间 36:38, RSS 稳定 15GB。进程仍在跑 1800s+ 预算之后,说明 Q20 仍在 cartesian 路径循环,无 oracle 验证。

**已知原因**(per `evidence/v312-58/issue-4380-sprint3-closure.md`):
- Q20 L0/L5 (full SF=1 nested SUM subquery) 之前就识别为 v3.13 Phase 3 HashSemiJoin deferral
- 上游 commit `0d31cc892 fix(v312-58 / #4380): Q20 correlated EXISTS slow-path table_info bug` 已部分修复 mini subset
- V312-58 Sprint 5 不在 Q20 L0/L5 范围内 — 该 deferral 仍然生效

---

## 4. 总结

| 查询 | 预算 | 实测 wall-clock | oracle 验证 | 备注 |
|------|------|----------------|-----------|------|
| Q17 SF=1 | 1800s | **68.38s** ✅ | MATCH ✅ | Sprint 5 bulk_load fix 解除瓶颈;ScalarAggInWhere 优化生效 (4800/4800 hits) |
| Q20 SF=1 | 1800s | TIMEOUT ❌ | 未验证 | 已知 v3.13 Phase 3 HashSemiJoin deferral (commit 0d31cc892) |
| Q22 SF=1 | 300s | **3.97s** ✅ | MATCH ✅ | ScalarAggInWhere + NOT EXISTS 路径全开 |

**Sprint 5 主贡献**:`crates/storage/src/engine.rs:1483` 的 O(N²) → O(N) 修复,使得 6M 行 bulk_load 从 hours 降到 ~30s,直接解锁 Q17/Q22 SF=1 perf 测试。

**Q20 SF=1 仍 TIMEOUT**:不在 Sprint 5 范围,作为 v3.13 Phase 3 跟踪项继续 deferred(详见 issue #4429)。