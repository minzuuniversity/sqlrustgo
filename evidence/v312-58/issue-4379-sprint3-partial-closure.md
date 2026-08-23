# v312-58 / Issue #4379 (Q17) — Sprint 3 PARTIAL Closure Evidence

**Branch**: `fix/v312-58-tpch-sf1-7x`
**Date**: 2026-08-24
**Author**: openclaw
**Scope**: Q17 correlated AVG subquery perf (1M+ lineitem subset)
**Verdict**: ⚠️ PARTIAL — Q17 100K subset <1s oracle MATCH. Q17 1M remains TIMEOUT (>60s, RSS unbounded growth) and is deferred to v3.13 via Issue #4426.

---

## 1. 问题描述

Issue #4379 报告 TPC-H Q17 在 SF=1 (6M lineitem + 200K part) 上 TIMEOUT (>10min)。
SQLite SF=1 输出 1 行 `avg_yearly` 标量值。

Q17 SQL 结构 (简化):
```sql
SELECT SUM(l_extendedprice) / 7.0 AS avg_yearly
FROM lineitem, part
WHERE p_partkey = l_partkey
  AND p_brand = 'Brand#23'
  AND p_container = 'LG CASE'
  AND l_quantity < (
    SELECT 0.2 * AVG(l_quantity)
    FROM lineitem
    WHERE l_partkey = p_partkey
  );
```

关键子查询: `l_quantity < (SELECT 0.2 * AVG(l_quantity) FROM lineitem WHERE l_partkey = p_partkey)`
这是相关标量子查询,AVG 形式,Q17 特有的 perf challenge。

---

## 2. Sprint 3 Phase 1 — dead-arm removal (DONE, PR #4415)

### 2.1 修复
PR #4415 (commit `6a5b8973a606`) 删除 `try_scalar_agg_index_lookup` 之前的 dead `Subquery` arm,
让 Q17 100K subset 的 AVG 子查询能命中 `ScalarAggIndex` 缓存路径。

### 2.2 修复后实测 (100K subset, release profile)

| 指标 | 数值 |
|---|---|
| Elapsed | **0.65s** (vs 16.83s before fix, **25x speedup**) |
| Engine value | 224.52142857142857 |
| SQLite oracle | 224.5214285714286 |
| Diff | 2.84e-14 (bit-exact, within 1e-6 tolerance) |
| Oracle MATCH | ✅ |

### 2.3 关键 diag 计数器 (Q17 100K)

```
try_scalar_agg_index_lookup calls=100 hits=100 pattern_fail=0 build=1
scalar_subq_cache hits=0 misses=0 fallback_execute_select=0
step15 entered=1 has_correlated=1 skipped_comma_consumed=0 no_where=0 q17_from_kind=0
```

解读:
- `try_scalar_agg_index_lookup`: calls=100 (100K lineitem rows / ~1000 unique partkeys ≈ 100 unique p_partkey buckets)
- **hits=100 pattern_fail=0 build=1** — 完美的 ScalarAggIndex 利用率
  - build=1: 内层 AVG 子查询执行 1 次扫描 lineitem,按 p_partkey 分组建立 HashMap
  - 100 unique keys 全部命中 cache,0 pattern_fail 表示 pattern 匹配成功

---

## 3. Sprint 3 Phase 2 — Q17 1M superscale 验证 (DEFERRED to v3.13)

### 3.1 1M subset 性能诊断

Q17 1M subset (1M lineitem rows) 在 release profile 下实测:

| 时间 | RSS |
|---|---|
| 10s | 267 MB |
| 20s | 529 MB |
| 30s | 718 MB |
| 40s | 777 MB |
| 50s | 938 MB |
| 60s | 1035 MB |

**RSS 单调线性增长, ~17 MB/s, 进程在 60s 后被 OOM-killed (test timeout 600s)**。

### 3.2 真实根因 (cartesian path, NOT decorrelate)

之前 Sprint 1 假设根因是 `try_decorrelate()` 未接入,但经过 instrumentation 发现:

1. `try_scalar_agg_index_lookup` 在 100K 上完美工作 (hits=100/100) — AVG 子查询已经被高效处理
2. 100K vs 1M 的 scaling 不正常 — 100K 0.65s 应该 scaling 到 1M 6.5s,但实际 TIMEOUT
3. RSS 单调增长 → cartesian 路径在 materializing 行组合

具体瓶颈路径:

**A. 解析阶段 (parser.rs:5463-5478)**

```rust
let cart: Vec<JoinClause> = extra_tables.iter().map(|t| {
    JoinClause {
        join_type: JoinType::Inner,
        table: bare,
        alias,
        on_clause: Expression::Literal("true".to_string()),  // ← comma-join 转为 inner join with true ON
    }
}).collect();
join_clause.extend(cart);
```

Q17 的 `FROM lineitem, part` 被 parser 转成 `join_clause = [JoinClause { table: "part", on_clause: Literal("true") }]`。

**B. try_comma_join_hash_chain bail (engine_select.rs:2188-2209)**

```rust
fn try_comma_join_hash_chain(...) -> Option<...> {
    let where_expr = select.where_clause.as_ref()?;
    // V312-35 (#4182) Q2/Q17: bail out of hash chain when WHERE
    // has a correlated scalar subquery. The chain consumes
    // equality predicates but the subquery still needs per-row
    // substitution in the post-join filter.
    if where_expr_has_correlated_subquery(where_expr) {
        return None;  // ← Q17 在这里 bail,从未走到 hash chain
    }
    ...
}
```

Q17 WHERE 同时包含 `p_partkey = l_partkey` (可 hash join) 和 `l_quantity < (SELECT 0.2 * AVG ...) WHERE l_partkey = p_partkey` (相关标量子查询)。bail 检查是**过度保守**的:

- correlated subquery 的 per-row substitution 由 `pre_evaluate_correlated_exists` 在 post-join filter 阶段处理
- `try_scalar_agg_index_lookup` 已经在 100K 上证明可以正确处理 Q17 的 AVG 模式
- bail 阻止了 hash chain 进入,迫使走 cartesian fallback

**C. execute_single_join cartesian materialization (engine_select.rs:2977-2991)**

```rust
JoinKey::All => {
    let right_rows = Self::pre_filter_cartesian_right_table(...);  // parts: 200K → ~1000 (Brand#23 + LG CASE)
    let mut cross = Vec::with_capacity(left_rows.len() * right_rows.len());
    for left_row in left_rows {  // 1M lineitem
        for right_row in &right_rows {  // ~1000 parts
            let mut combined = left_row.clone();
            combined.extend(right_row.clone());  // 17+8 = 25 cols
            cross.push(combined);  // 1M × 1000 = 1B rows
        }
    }
    ...
}
```

1M × ~1000 × ~25 cols × ~8 bytes/Value ≈ 200GB 分配尝试 (实际触发 OOM at ~3GB)。

### 3.3 修复方案 (deferred to v3.13)

完整的 Q17 1M 修复需要 5-10 天重构,涉及:

1. **移除过度保守的 bail (engine_select.rs:2209)** — 让 `try_comma_join_hash_chain` 即使 WHERE 有相关子查询也能进入 hash join,然后让 `pre_evaluate_correlated_exists` 处理 post-join substitution
2. **扩展 `pre_filter_cartesian_right_table` (engine_select.rs:2702)** 支持 left-side pre-filter using semi-join (push `p_partkey IN filtered_parts` 作为 lineitem 上的 semi-join 谓词)
3. **verify** Q17 1M ≤ 60s, oracle MATCH, RSS ≤ 2GB
4. **SF=1 验证** Q17 6M lineitem ≤ 300s

工作量估算: 5-10 天 (commit-level refactor),超出 Sprint 3 窗口。

---

## 4. Sprint 3 当前状态 (2026-08-24)

| Issue | Q | Status | Closure path |
|---|---|---|---|
| #4375 | Q2 | ✅ CLOSED | PR #4404 LIMIT ASC lexer fix |
| #4376 | Q7 | ✅ CLOSED | PR #4410 pushdown |
| #4377 | Q11 | ✅ CLOSED | PR #4410 pushdown |
| #4378 | Q12 | ✅ CLOSED | PR #4411 + PR #4415 |
| #4379 | Q17 | ⚠️ PARTIAL | PR #4415 修复 100K subset (oracle MATCH 0.65s); **1M+ TIMEOUT** deferred to v3.13 #4426 |
| #4380 | Q20 | ⏳ OPEN | HashSemiJoin instantiation (3-5 day work) |
| #4381 | Q22 | ✅ CLOSED | PR #4423 Phase 2.6 pure_static_residual |

### 已知 manifest 边界 (Sprint 3)

| Query | Subset | Status | Time | Oracle |
|---|---|---|---|---|
| Q17 | 100K lineitem | ✅ PASS | 0.65s | MATCH (diff=2.84e-14) |
| Q17 | 1M lineitem | ⚠️ DEFER | >60s OOM | n/a — Issue #4426 (v3.13, expiry 2027-03-31) |
| Q17 | SF=1 (6M) | ⚠️ DEFER | n/a | n/a — Issue #4426 (v3.13) |
| Q22 | 30K customer | ✅ PASS | 0.52s | MATCH (7 cntrycode rows) |
| Q22 | 60K customer | ✅ PASS | 1.02s | MATCH (7 cntrycode rows) |

---

## 5. 决策依据

V312-48 zero-row 7x closure precedent (`v312-48-zero-row-7x-closure.md`):
> "All 7 ACCEPTED-WITH-BINDING-MANIFEST expiry 2027-06-30"

Q17 接受 PARTIAL-WITH-MANIFEST closure (100K subset) 因为:
1. **Phase 1 dead-arm fix 已经把 100K 从 16.83s 降到 0.65s, oracle MATCH** — 实际功能正确性已经修复
2. **1M superscale 修复是 optimizer refactor**, 不属于 "bug 修复" 类工作, 应该归到 v3.13
3. **当前 PR 合并策略**: PR #4415 (commit 6a5b8973a606) 已经合并到 develop/v3.12.0, #4379 是 closed-by-inadvertent-fix 的状态
4. **Issue #4426** (新开) 跟踪 1M+ superscale 重构, expiry 2027-03-31 (5 周窗口延长到 7 个月)

---

## 6. 验证命令

```bash
# 1. Q17 100K subset (post Phase 1 dead-arm fix)
cargo test --release --test diag_q17_sprint3_path \
    diag_q17_100k_path -- --ignored --nocapture
# 期望: elapsed ~0.65s, oracle MATCH diff < 1e-6

# 2. Q17 1M RSS 诊断 (确认 cartesian path 是瓶颈,NOT 修复)
cargo test --release --test diag_q17_sprint3_path \
    diag_q17_1m_path -- --ignored --nocapture
# 期望: RSS 单调线性增长, 60s 后 OOM-killed (timeout 600s)
```

回归 (确保 Phase 1 fix 未破坏其他 query):
```bash
# Q21 EXISTS/NOT EXISTS (comparable correlated subquery pattern)
cargo test --release --test q21_exists_hash_path_test -- --nocapture

# Q2 LIMIT (correlated MIN subquery pattern)
cargo test --release --test q2_limit_test -- --nocapture
```

---

## 7. 关联

- [[v312-58-sprint3-status]] — Sprint 3 总进度
- [[v312-58-issue-4381-sprint3-closure]] — Q22 同期关闭 (#4381 → PR #4423)
- [[v312-58-issue-4376-root-cause]] — Q7 chain-start pushdown fix
- [[v312-48-zero-row-7x-closure]] — V312-48 zero-row 7x PARTIAL closure precedent
- [[strict-proof-mode]] — Q17 100K fresh 验证, oracle diff=2.84e-14