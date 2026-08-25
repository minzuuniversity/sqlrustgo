# v312-58 Phase 3 实施计划 — Q20 L0/L5 SUM 子查询 Composite Key 修复

**Date**: 2026-08-24
**Branch**: `fix/v312-58-tpch-sf1-7x`
**Goal**: 修复 Q20 L0/L5 (full SUM subquery) 仍返回 0 行的问题
**预估**: 5-7 工作日
**Target**: v3.13 Sprint 4 完成 #4380 完整 closure + #4374 父 issue 关闭

---

## 1. 现状与目标

### 1.1 现状

| L | 模式 | 状态 |
|---|---|---|
| L0 | 完整 Q20 (含 SUM 嵌套) | ❌ 0 行 |
| L5 | Q20 减去 nation filter (含 SUM 嵌套) | ❌ 0 行 |
| L4/L9/L19 | EXISTS + IN + availqty (literal) | ✅ 30 行 (Sprint 3 PARTIAL fix) |

### 1.2 根因

`try_scalar_agg_index_lookup` (engine_select.rs:4607) 仅支持单 equality key。

Q20 L0/L5 SUM 子查询:
```sql
(SELECT 0.5 * SUM(l_quantity) FROM lineitem
 WHERE l_partkey = ps_partkey              -- key 1
   AND l_suppkey = ps_suppkey              -- key 2 (composite!)
   AND l_shipdate >= '1994-01-01'          -- range predicate
   AND l_shipdate < '1995-01-01')          -- range predicate
```

- `find_equality_inner_outer` 返回 `Option<(String, usize)>` — 只取**第一个**匹配等值,Q20 需要返回**两个**
- `build_scalar_agg_index` 用单 `key_col_idx` 构建 `HashMap<Value, agg_result>` — Q20 需要 `HashMap<(Value, Value), agg_result>`
- 残余 range predicates (`l_shipdate >= AND <`) 未参与 index 构建 — 应作为 build-time static_predicate 提前过滤

### 1.3 目标

- L0/L5 mini subset 返回 oracle 行数(预测: 30 行的子集)
- 不破坏 L4/L9/L19 (composite 路径应向后兼容单 key)
- Q17 / Q22 回归 bit-exact
- clippy + fmt PASS

---

## 2. 实施步骤

### 步骤 2.1 — 扩展 `find_equality_inner_outer` (0.5 天)

**当前签名** (engine_select.rs:5621):
```rust
fn find_equality_inner_outer(
    where_expr: &Expression,
    agg_arg_expr: &Expression,
    inner_table_name: &str,
    outer_table_info: &TableInfo,
) -> Option<(String, usize)>
```

**新签名**:
```rust
fn find_composite_equality_inner_outer(
    where_expr: &Expression,
    inner_table_name: &str,
    outer_table_info: &TableInfo,
) -> (Vec<(String, usize)>, Option<Box<Expression>>) {
    // 返回: (equality_pairs, residual_static_predicate)
}
```

**关键改动**:
- 遍历 AND 树所有叶子,收集所有 `<inner_col> = <outer_ref>` 对
- 非等值叶子(`>=`, `<`, `LIKE`, etc)累积为 residual
- 验证 inner cols 都属于 inner_table,outer cols 都属于 outer_table_info
- 至少返回 1 个 pair;空 → 返回 `None` 模式

**向后兼容**:
- 保留 `find_equality_inner_outer` 单元素版(给 Q17 等单 key 调用点用),或
- 让 Q17 调用点用 `find_composite_equality` 单元素版本(vec.len() == 1)

### 步骤 2.2 — 扩展 `build_scalar_agg_index` (1 天)

**当前签名** (engine_select.rs:5733):
```rust
fn build_scalar_agg_index(
    rows: &[Vec<Value>],
    _table_info: &TableInfo,
    key_col_idx: usize,
    agg_col_idx: Option<usize>,
    agg_func: AggregateFunction,
    op_factor: f64,
) -> ScalarAggIndexMap  // HashMap<Value, Value>
```

**新签名**:
```rust
fn build_composite_scalar_agg_index(
    rows: &[Vec<Value>],
    table_info: &TableInfo,
    key_col_idxs: &[usize],
    agg_col_idx: Option<usize>,
    agg_func: AggregateFunction,
    op_factor: f64,
    static_predicate: Option<&Expression>,
) -> ScalarAggIndexMap
```

**关键改动**:
- Composite key 用 `Vec<Value>` (按 key_col_idxs 顺序拼)
- Static predicate 在 build 时过滤掉不满足 range 的行(节省内存 + 提升正确性)
- 单 key 路径继续走 `ScalarAggIndexMap` 类型 (Vec<Value> 单元素)

**向后兼容**:
- 保留 `build_scalar_agg_index` 单 key 版,或
- 统一调用 `build_composite_scalar_agg_index` 传 `&[key_col_idx]` (单元素时退化为单 key)

### 步骤 2.3 — 扩展 `try_scalar_agg_index_lookup` (1-2 天)

**当前签名** (engine_select.rs:4698):
```rust
fn try_scalar_agg_index_lookup(
    &self,
    subq: &SelectStatement,
    outer_row: &[Value],
    outer_table_info: &TableInfo,
) -> Option<Value>
```

**关键改动**:
1. 调用 `find_composite_equality_inner_outer` 替代 `find_equality_inner_outer`
2. Cache key 包含 composite key cols (`format!("{}|{:?}|...", real_table, key_col_idxs, ...)`)
3. Static predicate 在 build 时应用 (见步骤 2.2)
4. Lookup 时组装 composite key 从 outer_row 的对应 cols
5. DIAG counter `DIAG_TRY_SCALAR_AGG_COMPOSITE_HITS` 用于验证

### 步骤 2.4 — DIAG counter + 测试 (1 天)

**新增 counter**:
```rust
static DIAG_TRY_SCALAR_AGG_COMPOSITE_CALLS: AtomicU64 = AtomicU64::new(0);
static DIAG_TRY_SCALAR_AGG_COMPOSITE_HITS: AtomicU64 = AtomicU64::new(0);
static DIAG_TRY_SCALAR_AGG_COMPOSITE_BUILD: AtomicU64 = AtomicU64::new(0);
```

**测试**:
- 扩展 `diag_q20_mini_bisect.rs` 增加 oracle ground truth 比对
- 新增 `diag_q20_composite_key_path.rs` — 单独验证 composite 路径

### 步骤 2.5 — 验证 + 收口 (1 天)

- `cargo test --release --test diag_q20_mini_bisect -- --ignored --nocapture`
- `cargo test --release --test diag_q20_sprint3_path -- --ignored --nocapture`
- `cargo test --release --test diag_q17_* -- --ignored --nocapture` (回归)
- `cargo test --release --test diag_q22_* -- --ignored --nocapture` (回归)
- `cargo clippy --all-features -- -D warnings`
- `cargo fmt --check --all`

### 步骤 2.6 — 推送 + 关闭 issue (0.5 天)

- 创建 PR (commit composite-key extension)
- 通过 252 REST API rebase + merge
- 关闭 #4380 (FULL closure,不再 PARTIAL)
- 关闭 #4374 parent (7/7 子项全部 FULL closure)
- 更新 memory

---

## 3. 风险与缓解

| 风险 | 概率 | 影响 | 缓解 |
|---|---|---|---|
| Composite key path 性能不如预期 | 中 | Q20 L0/L5 oracle MATCH 但耗时 > 60s | 已有 `cache_key` HashMap 加速,composite key 仅增加 vec 分配开销 |
| Q17 回归 (单 key 走 composite path 出现 regression) | 中 | Q17 100K 从 16.83s 退化 | 严格保留单 key fast path;composite 路径对单 key 用 `vec![key]` 透明 |
| Static predicate 误过滤 (e.g. residual 引用 outer) | 中 | 错误结果 | static predicate 必须在 build 时为 inner 表自包含;若引用 outer,标 `None` 不应用 |
| DIAG counter 增加但实际未命中 | 低 | 误导 | composite path 必须有 eprintln 调试输出,需手动验证一行用例 |

---

## 4. 文件改动清单

| 文件 | 改动类型 | 累计行数估算 |
|---|---|---|
| `src/engine_select.rs` | find_equality_inner_outer 重构 + build_scalar_agg_index 重构 + try_scalar_agg_index_lookup composite path + DIAG counter | +150 / -50 |
| `tests/integration/oracle/diag_q20_mini_bisect.rs` | 增加 oracle ground truth 比对 + L0/L5 expected count | +30 / -10 |
| `tests/integration/oracle/diag_q20_composite_key_path.rs` (新) | 单独 composite key path 测试 | 新文件 +80 |
| `Cargo.toml` | 注册新 test target | +3 |
| `evidence/v312-58/sprint3-phase3-closure.md` (新) | Phase 3 完整 closure evidence | 新文件 +200 |

---

## 5. 时间表

| 日 | 任务 | 完成标准 |
|---|---|---|
| Day 1 (Mon) | 步骤 2.1 `find_composite_equality_inner_outer` | 单测通过,Q17 单 key 路径无回归 |
| Day 2-3 (Tue-Wed) | 步骤 2.2 `build_composite_scalar_agg_index` + 步骤 2.3 接入 | Q20 L5 mini subset 非 0 行 (oracle count 范围) |
| Day 4 (Thu) | 步骤 2.4 DIAG counter + 步骤 2.5 验证 | Q20 L0/L5 oracle MATCH, Q17/Q22 回归 PASS, clippy/fmt PASS |
| Day 5 (Fri) | 步骤 2.6 PR + merge + issue close | #4380 FULL closure + #4374 parent closed |

---

## 6. 与 plan `/home/openclaw/.claude/plans/reactive-gathering-orbit.md` 的差异

| 计划 Phase 3 | 实际 Phase 3 (本文) |
|---|---|
| 实例化 HashSemiJoin 用于 Q20 EXISTS/NOT EXISTS | 扩展 try_scalar_agg_index_lookup 支持 composite key |
| 重点:Q20 L0/L5 SUM 子查询无法被 try_scalar_agg 覆盖 | 改为:composite key 扩展让 try_scalar_agg 覆盖 Q20 L0/L5 |
| 5-7 天 | 5 天 (更聚焦,改动更小) |

**理由调整**: Q20 L0/L5 的 SUM 子查询**不是** EXISTS/NOT EXISTS 模式,而是 scalar aggregate 模式。`try_scalar_agg_index_lookup` 已经能正确识别 scalar aggregate pattern,只是缺 composite key 支持。直接扩展它比引入 HashSemiJoin (后者是为 Q20 L1-L4/L9/L19 设计的,但这些已被 Sprint 3 typo fix 解决) 改动更小。

---

## 7. 关联

- See also: `evidence/v312-58/issue-4380-sprint3-closure.md` (Sprint 3 PARTIAL closure)
- See also: `evidence/v312-58/sprint3-closure.md` (Sprint 3 整体收口)
- Plan: `/home/openclaw/.claude/plans/reactive-gathering-orbit.md`
- Code: `src/engine_select.rs:4607` (`try_scalar_agg_index_lookup`)
- Code: `src/engine_select.rs:5621` (`find_equality_inner_outer`)
- Code: `src/engine_select.rs:5733` (`build_scalar_agg_index`)
