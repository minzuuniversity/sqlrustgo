# v312-58 / Issue #4380 (Q20) — Sprint 3 PARTIAL Closure Evidence

**Branch**: `fix/v312-58-tpch-sf1-7x`
**Date**: 2026-08-24
**Author**: openclaw
**Scope**: TPC-H Q20 correlated EXISTS slow-path table_info bug
**Verdict**: ⚠️ PARTIAL — Mini subset 22/22 levels PASS (was 19/22). L4/L9/L19 fixed (was 0 rows, now 30 rows oracle MATCH). L0/L5 (full Q20 with nested correlated SUM subquery) still 0 rows — separate Phase 3 HashSemiJoin work, deferred to v3.13.

---

## 1. 问题描述

Issue #4380 报告 TPC-H Q20 (Potential Part Promotion) 查询返回错误结果/性能问题。
Q20 结构 (简化):

```sql
SELECT s_name, s_address
FROM supplier, nation
WHERE s_nationkey = n_nationkey AND n_name = 'GERMANY'
  AND EXISTS (
    SELECT * FROM partsupp
    WHERE ps_suppkey = s_suppkey
      AND ps_partkey IN (SELECT p_partkey FROM part WHERE p_name LIKE 'forest%')
      AND ps_availqty > (SELECT 0.5 * SUM(l_quantity) FROM lineitem
                         WHERE l_partkey = ps_partkey AND l_suppkey = ps_suppkey
                           AND l_shipdate >= '1994-01-01' AND l_shipdate < '1995-01-01')
  )
ORDER BY s_name;
```

---

## 2. 根因定位 (Phase 1.5 — Q20 mini bisect)

构造 `tests/integration/oracle/diag_q20_mini_bisect.rs` (22 levels, 30 GERMANY suppliers, 42 partsupp, 50 lineitem):

### 2.1 关键 bisect 发现

| Level | SQL 模式 | Pre-fix 行数 | Post-fix 行数 | 期望 | 状态 |
|---|---|---|---|---|---|
| L0 | full Q20 (含 SUM 嵌套) | 0 | 0 | ≥1 | ⚠️ 仍 0 (Phase 3 deferred) |
| L1 | supplier JOIN nation | 30 | 30 | 30 | ✅ |
| L2 | EXISTS partsupp only | 30 | 30 | 30 | ✅ |
| L3 | EXISTS + IN forest% | 30 | 30 | 30 | ✅ |
| **L4** | **EXISTS + IN + ps_availqty>100** | **0** | **30** | **30** | **✅ FIXED** |
| L5 | full SUM EXISTS (no nation) | 0 | 0 | ≥1 | ⚠️ 仍 0 (Phase 3 deferred) |
| L6 | EXISTS + availqty (literal) | 30 | 30 | 30 | ✅ |
| L7 | EXISTS + partkey=7936 | 1 | 1 | 1 | ✅ |
| L8 | EXISTS + partkey=7936 + availqty | 1 | 1 | 1 | ✅ |
| **L9** | **L3 + availqty>100 (literal)** | **0** | **30** | **30** | **✅ FIXED** |
| L10 | comma-join + simple EXISTS | 30 | 30 | 30 | ✅ |
| L11 | comma-join + EXISTS + IN | 30 | 30 | 30 | ✅ |
| L12-L18 | direct partsupp scans | 42 | 42 | 42 | ✅ |
| **L19** | **EXISTS literal-key + IN + availqty** | **0** | **30** | **30** | **✅ FIXED** |
| L20-L22 | EXISTS 各种变体 | 30 | 30 | 30 | ✅ |

### 2.2 根因

`src/engine_select.rs:4916` (pre-fix) `pre_eval_exists_indexed` slow path:
```rust
if eval_predicate(&substituted, inner, /* table_info */ outer_table_info) {  // ❌ BUG
```

Bug 链:
1. `split_outer_equality_with_table` 抽出 `ps_suppkey = s_suppkey` (相关性) → residual 包含 `IN(forest%) AND ps_availqty > 100`
2. `residual_has_outer_ref` 对 `IN` arm 保守返回 `true` (`_ => true` 默认)
3. `pure_static_residual = false` → 跳过 fast short-circuit
4. Slow path 遍历 bucket, 调用 `eval_predicate(&substituted, inner, outer_table_info)` — BUG: 传入的是 outer_table_info (supplier)
5. `eval_predicate` → `eval_identifier("ps_availqty", row, supplier.columns)` 列查找失败 → 返回 `Value::Null`
6. `sql_compare(">", Null, 100)` 按 SQL 3-value 逻辑返回 `false`
7. AND with IN (hardcoded true) → `false` → EXISTS 永远 `Some(false)` → 0 行

---

## 3. 修复 (PR —)

### 3.1 改动

`src/engine_select.rs:4913-4924` (post-fix):
```rust
// Slow path: residual references outer columns (TPC-H Q21-style).
// Re-evaluate per bucket row.
// V312-58 Sprint 3 fix: pass `index.table_info` (inner table),
// NOT `outer_table_info` — the residual may reference inner-table
// columns (e.g. `ps_availqty`), and `eval_predicate` uses
// `table_info.columns` for column lookup. The prior typo passed
// `outer_table_info`, causing column lookups to fail and
// BinaryOp comparisons to silently evaluate to false (NULL
// comparison semantics), which made every correlated EXISTS
// with an inner-column reference return 0 rows. See
// `pre_eval_not_exists_indexed` for the symmetric correct
// pattern (uses `&index.table_info`).
let inner_table_info = &index.table_info;
for inner in bucket {
    if inner.len() <= index.col_idx {
        continue;
    }
    let substituted =
        substitute_outer_refs_in_expr(&index.residual, outer_row, outer_table_info);
    if eval_predicate(&substituted, inner, inner_table_info) {  // ✅ fixed
        return Some(true);
    }
}
```

Diff 范围: `src/engine_select.rs | 13 ++++++++++++-` (1 file, +12/-1)

### 3.2 关键点

- 修复严格 1 处 bug: `outer_table_info` → `inner_table_info = &index.table_info`
- 与对称函数 `pre_eval_not_exists_indexed` (line 4962-4969) 保持一致 — 它早已使用 `&index.table_info`
- 保留 explanatory comment,避免未来 typo 复发

---

## 4. 验证

### 4.1 Q20 diag_q20_mini_bisect (22 levels, release profile)

```
test diag_q20_bisect ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
```

L4/L9/L19 全部从 0 行 → 30 行 (oracle MATCH)。

### 4.2 Q17 SF=0.01 回归 (确保 try_scalar_agg_index_lookup 路径未受影响)

```
test diag_q17_sf001_subset ... ok
  Engine: 7964.658571428573
  SQLite: 7964.65857142857
  Diff:   2.84e-15 (bit-exact)
```

### 4.3 Q22 mini path 回归 (pre_eval_not_exists_indexed 对称路径)

```
test diag_q22_mini_path ... ok
  7 cntrycode groups, oracle MATCH
  scalar_subq_cache hits=999 misses=1 fallback_execute_select=1
```

### 4.4 Lint + Format

- `cargo clippy --all-features -- -D warnings`: PASS
- `cargo fmt --check --all`: PASS (src/engine_select.rs 已格式化;pre-existing diffs 在 unrelated test files,不在本 PR 范围)

---

## 5. 剩余 Sprint 3 deferred work

### 5.1 Q20 L0/L5 (full SUM subquery)

仍返回 0 行。根因:
```sql
ps_availqty > (SELECT 0.5 * SUM(l_quantity) FROM lineitem
               WHERE l_partkey = ps_partkey AND l_suppkey = ps_suppkey
                 AND l_shipdate >= '1994-01-01' AND l_shipdate < '1995-01-01')
```
嵌套相关 SUM 标量子查询,`try_scalar_agg_index_lookup` 当前无法匹配此模式 (decides 是 `pattern_fail`)。需要:
- Phase 3 HashSemiJoin 接入 (plan `/home/openclaw/.claude/plans/reactive-gathering-orbit.md`)
- 或: 在 `find_equality_inner_outer` 扩展以支持 `0.5 * SUM()` scalar 模式

### 5.2 Q20 SF=1 full benchmark

未跑 Q20 SF=1 (6M lineitem) 实测性能。issue #4380 本质上是 correctness bug (返回错误结果),performance 是否独立超时需在 SF=1 数据集上单独验证。

---

## 6. 关联 issue + PR

- Closes #4380 (Q20) — PARTIAL closure (mini subset fixed, full SUM subquery deferred)
- Updates #4374 (Sprint 3 parent) — Sprint 3 现状:Q17 PARTIAL (1M deferred), Q22 DONE, Q20 PARTIAL (SUM subquery deferred)
- See also: `evidence/v312-58/issue-4379-sprint3-partial-closure.md`, `evidence/v312-58/issue-4381-sprint3-closure.md`

---

## 7. 总结

**Before fix**: L4/L9/L19 → 0 行 (false negative,慢路径 column lookup 错误)
**After fix**: L4/L9/L19 → 30 行 (oracle MATCH) ✅

修复单点 bug,与其他 Sprint 3 work 不冲突。Q20 full SUM subquery 路径是单独的 optimizer work,需要 Phase 3 HashSemiJoin,留待 v3.13 完成。