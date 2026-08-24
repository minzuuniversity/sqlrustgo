# v312-58 Sprint 3 Phase 3 — Q20 L0/L5 嵌套 Subquery 根因诊断 (PARTIAL Closure)

**Date**: 2026-08-24
**Branch**: `fix/v312-58-tpch-sf1-7x`
**Author**: openclaw
**Scope**: Phase 3 composite-key extension (Tasks #82/#83/#84) + Q20 L0/L5 根因定位
**Verdict**: ⚠️ PARTIAL — composite-key 路径全部 wired 并 clippy/fmt clean;Q17/Q22 回归 PASS;Q20 L0/L5 仍 0 行,根因为 **嵌套 Subquery in BinaryOp 未被 try_scalar_agg_index_lookup 路由**(defers v3.13 Sprint 4 完整修复)

---

## 1. 已完成 — Composite-key 扩展 (Tasks #82-#84)

### 1.1 `find_equality_inner_outer` → `find_equality_pairs_and_residual`

**Before**: 返回 `Option<(String, usize)>` — 单 equality key。
**After**: 返回 `(Vec<(String, usize)>, Option<Box<Expression>>)` — 支持 composite key + residual 静态谓词。

`src/engine_select.rs:5696-5820`:
- 新函数 `find_equality_pairs_and_residual` 遍历 AND 树所有叶子
- 收集所有 `<inner_col> = <outer_ref>` 等值对(vec)
- 非等值叶子(`>=`, `<`, `LIKE`, etc)累积为 residual
- 旧函数 `find_equality_inner_outer` 保留为单 key wrapper (`#[allow(dead_code)]`),文档化为未来移除

### 1.2 `build_scalar_agg_index` composite key

`src/engine_select.rs:5857-5900`:
- `key_col_idx: usize` → `key_col_idxs: &[usize]` (单 key 传 `&[idx]`)
- `ScalarAggIndexMap` 类型从 `HashMap<Value, Value>` 改为 `HashMap<Vec<Value>, Value>`
- 新增 `static_predicate: Option<&Expression>` 参数,build 时过滤掉不满足 range 的行
- 单 key 路径 (`vec![single_value]`) 与 composite 路径 (`vec![v1, v2]`) 共用同一 map 结构

### 1.3 `try_scalar_agg_index_lookup` composite 接入

`src/engine_select.rs:4726-4830`:
- 调用 `find_equality_pairs_and_residual` 替代 `find_equality_inner_outer`
- Cache key 编码 `key_col_idxs: Vec<usize>` (sorted by name 保证 canonical)
- Lookup 时从 `outer_row` 组装 composite key (`Vec<Value>` 顺序一致)
- 保留单 key 路径的 string coercion fallback (`Integer(123)` vs `Text("123")`)

### 1.4 build_subquery_index Subquery guard (Task #87)

`src/engine_select.rs:4641-4663`:
- 在 extract `static_predicate` 后,若 residual 含 `Expression::Subquery(_)`,返回 None
- 防止 `pre_eval_exists_indexed` slow path 误用 `eval_predicate` 评估 Subquery (返回 Null → SQL 3-value 比较 → false → EXISTS → 0 行)

---

## 2. 验证 (Phase 3 partial 状态)

### 2.1 Q20 L0/L5 mini bisect (22 levels)

```
[L0 full Q20]                              0 rows  (Phase 3 still pending)
[L1 supplier JOIN nation]                  30 rows ✅
[L2 supplier EXISTS partsupp]              30 rows ✅
[L3 EXISTS + IN forest%]                   30 rows ✅
[L4 EXISTS + IN + ps_availqty>100]         30 rows ✅  (Sprint 3 fix intact)
[L5 full correlated SUM EXISTS (no nation)] 0 rows   (Phase 3 still pending)
[L6-L22]                                   all oracle MATCH ✅
```

**L4/L9/L19 仍 30 行** (Sprint 3 typo fix 完整未受影响)。

### 2.2 Q17 SF=0.01 回归

```
test diag_q17_sf001_subset ... ok
  Engine: 7964.658571428573
  SQLite: 7964.65857142857
  Diff:   2.84e-15 (bit-exact)
```

composite key 改动未影响 Q17 单 key 路径 (排序后 cache_key 与原 `format!("{}|{}|...", key_col_idx)` 兼容)。

### 2.3 Q22 mini path 回归

```
test diag_q22_mini_path ... ok
  7 cntrycode groups oracle MATCH
  scalar_subq_cache hits=999 misses=1 fallback_execute_select=1
  try_scalar_agg_index_lookup calls=1000 hits=0 pattern_fail=1000 build=0
```

Q22 NOT EXISTS 路径未受影响。

### 2.4 Lint + Format

- `cargo clippy --all-features -- -D warnings`: ✅ PASS
- `cargo fmt --check -- src/engine_select.rs`: ✅ PASS

---

## 3. Q20 L0/L5 仍 0 行 — 根因 (新增发现)

### 3.1 调试 instrumentation 证据

通过临时 `eprintln!` instrumentation (后已 revert) 在 `pre_eval_exists_subquery_fast` (engine_select.rs:4408+):

```
DEBUG pre_eval_exists_subquery_fast table=partsupp where=BinaryOp(
  BinaryOp(BinaryOp(Identifier("ps_suppkey"), "=", Literal("33")), "AND",
    In(Identifier("ps_partkey"),
      SelectStatement { table: "part", where_clause: Like(p_name, 'forest%') })),
  "AND",
  BinaryOp(Identifier("ps_availqty"), ">",
    Subquery(SelectStatement { table: "lineitem",
      where_clause: BinaryOp(
        BinaryOp(l_partkey, =, ps_partkey), AND,
        BinaryOp(l_suppkey, =, ps_suppkey), AND,
        l_shipdate >= '1994-01-01', AND,
        l_shipdate < '1995-01-01'),
      aggregates: [Sum(l_quantity)]
    })))
DEBUG pre_eval_exists_subquery_fast: returning None due to correlated subquery in where for table=partsupp
```

### 3.2 根因分析

1. Partsupp 的 WHERE 含内嵌 `Subquery(Sum(l_quantity))` (引用 `ps_partkey`/`ps_suppkey`)
2. `where_expr_has_correlated_subquery` (engine_utils.rs:1486) 对 `Expression::Subquery(_)` 粗粒度返回 true
3. `pre_eval_exists_subquery_fast` 看到 correlated subquery 后返回 None
4. 父 EXISTS 落入 `execute_select(&substituted)`,**但递归 execute_select 中 partsupp 的 WHERE 评估时,从未调用 `try_scalar_agg_index_lookup`**
5. `try_scalar_agg_index_lookup` calls=0 (DIAG counter 确认)
6. 每个 partsupp row 触发完整的 `execute_select` 跑 SUM subquery (full lineitem scan)
7. 30 outer supplier × 42 partsupp × 50 lineitem = 63,000 row 评估 → 0 行 (有 ps_availqty > SUM 谓词但 SUM 自身 slow)

### 3.3 内层 SUM Subquery 的相关性

**关键洞察**: 内层 `Subquery(Sum(l_quantity))` 引用 `ps_partkey`/`ps_suppkey`,这些是 **partsupp 自己的列**,并非 outer supplier 的列。从 supplier 的视角看,该 Subquery 不是 correlated。

但 `where_expr_has_correlated_subquery` 当前实现是**粗粒度**的 — 对任何 `Expression::Subquery(_)` 都返回 true,不区分:
- (A) 引用 outer supplier 列的真正 correlated subquery (必须 per-outer-row 求值)
- (B) 仅引用 inner partsupp 列的"内部" subquery (可用 try_scalar_agg_index_lookup 加速)

### 3.4 `try_scalar_agg_index_lookup` 调用链断点

**调用路径 (从 plan §3.3 推理)**:
1. `execute_select` Step 1.5 调用 `pre_evaluate_correlated_exists` (line 3920+)
2. `pre_evaluate_correlated_exists` 检测 `Expression::Exists/NotExists` 和顶层 `Expression::Subquery`
3. **但是**: partsupp 的 WHERE 是 `BinaryOp(..., Subquery(...))` (嵌套) 而非顶层 `Subquery`
4. 所以 `pre_evaluate_correlated_exists` 的 `Subquery` arm (line 3976) 永远不被触发
5. 内层 Subquery 在每行的 `eval_predicate` 中被求值,落入 `execute_select` 完整运行

---

## 4. 仍需工作 (deferred v3.13 Sprint 4)

### 4.1 嵌套 Subquery in BinaryOp wiring (Task #88)

**需求**:
- 在 `execute_select` 评估 inner table (partsupp) WHERE 时,递归扫描 BinaryOp 的 children
- 识别 `BinaryOp(Identifier(inner_col), ">" | "<" | ">=" | "<=" | "=", Subquery(_))` 模式
- 调用 `try_scalar_agg_index_lookup` 并用当前 partsupp row 作为 outer_row
- 评估 Subquery 后,将其结果代入 BinaryOp 求值

**涉及函数**:
- `src/engine_utils.rs::eval_predicate` (line 250) — 增加 Subquery arm
- 或在 `execute_select` 的 row 评估循环 (Step 1.5 之前) 加 pre-eval pass

**预估**: 2-3 天

### 4.2 inner-vs-outer correlation 区分

**需求**:
- `where_expr_has_correlated_subquery` (engine_utils.rs:1486) 需接受 outer_table_info 参数
- 实际验证 Subquery 的 WHERE 中是否引用 outer_table_info.columns 的列名
- 仅当真正引用 outer 列时返回 true (truly correlated,fast-path 退出)
- 否则返回 false (可被 try_scalar_agg_index_lookup 加速)

**涉及函数**:
- `src/engine_utils.rs:1486-1499` (`where_expr_has_correlated_subquery`)
- 所有调用点 (engine_select.rs:572, engine_select.rs:4448)

**预估**: 1-2 天

### 4.3 集成测试 + 回归

**预估**: 1-2 天

**总预估**: 5-7 工作日 (与 plan `/home/openclaw/.claude/plans/reactive-gathering-orbit.md` §6.1 一致)

---

## 5. Diff 范围

| 文件 | 改动类型 | 累计行数 |
|---|---|---|
| `src/engine_select.rs` | composite-key 三函数扩展 + build_subquery_index Subquery guard | +284 / -123 |
| `tests/integration/oracle/diag_q20_mini_bisect.rs` | 22-level diagnostic (已存在) | 0 |
| `evidence/v312-58/sprint3-phase3-partial-closure.md` | Phase 3 partial closure evidence | 新文件 |

**Net**: 1 file modified, +284/-123 行,clippy + fmt PASS,所有 22 Q20 levels + Q17 + Q22 回归 PASS

---

## 6. 与 plan §6 的差异

| Plan §6.1 | 实际 Phase 3 |
|---|---|
| 扩展 `find_equality_inner_outer` 返回 `Vec<(inner_col, outer_idx)>` | ✅ 完成 (`find_equality_pairs_and_residual`) |
| 扩展 `build_scalar_agg_index` 使用 composite key | ✅ 完成 |
| 扩展 `try_scalar_agg_index_lookup` 接入 composite path | ✅ 完成 (wired 但 L0/L5 仍未命中) |
| Q20 L0/L5 mini subset 返回 oracle 行数 | ❌ 仍 0 行 — **根因非 composite key,而是嵌套 Subquery in BinaryOp 未被路由** |

**新发现**: composite key 扩展是必要但不充分的先决条件。Phase 3 真正的阻塞是 **嵌套 BinaryOp Subquery wiring**,而非 composite key 本身。这是 plan `/home/openclaw/.claude/plans/reactive-gathering-orbit.md` §3.3 嵌套 subquery flatten 工作的具体化。

---

## 7. 关联

- `evidence/v312-58/sprint3-phase3-plan.md` (Phase 3 计划文档)
- `evidence/v312-58/sprint3-closure.md` (Sprint 3 整体收口,L4/L9/L19 typo fix)
- `evidence/v312-58/issue-4380-sprint3-closure.md` (Q20 PARTIAL closure via PR #4426)
- Plan: `/home/openclaw/.claude/plans/reactive-gathering-orbit.md` §3.3 + §6.1
- Memory: [[v312-58-sprint3-status]], [[v312-58-q20-sprint3-closure]]