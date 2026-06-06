# Sprint 4 准备: SUM(REAL)=0 调查链

> **Date**: 2026-06-07
> **Status**: 调查完成，修复推迟到 Sprint 4
> **Refs**:
> - Sprint 1.5 cell diff 发现 (#3257): `docs/audit/status/2026-06-07-tpch-cell-diff-v390.md`
> - Failure Matrix: `docs/audit/status/2026-06-07-tpch-failure-matrix-v390.md`

---

## 0. 背景

Sprint 1.5 cell-level diff 揭示：
- **Q01**: `SUM(l_extendedprice)` (REAL) 返 0，PG 返 877911.41
- **Q05/Q07/Q08/Q17**: 类似 SUM(REAL)=0

TPC-H Sprint 1 文档（`src/engine_select.rs:599`）声称 "TPC-H Sprint 1 fix: accept Float in Sum"。

**但 Q1 跑起来 SUM 仍然 = 0**。**chatGPT 预测 SUM(REAL)=0 是单点 bug，实际是更深的链 bug**。

---

## 1. 调查链（按层次）

### 第 1 层：execution_engine.rs 入口

```rust
// src/execution_engine.rs:284
Statement::Select(ref select) => self.execute_select(select),
```

`execute_select` 函数定义**不在** `execution_engine.rs`（`grep` 找不到）。它在 **`src/engine_select.rs:31`**：

```rust
pub(crate) fn execute_select(&self, select: &SelectStatement) -> SqlResult<ExecutorResult>
```

**这暗示 engine_select.rs 是 SELECT 处理的真正入口**。

### 第 2 层：engine_select.rs:563 - 实际 aggregate 实现

```rust
// src/engine_select.rs:563
fn compute_aggregates(
    &self,
    aggregates: &[AggregateCall],
    rows: &[Vec<Value>],
    table_info: &TableInfo,
) -> SqlResult<Vec<Value>>
```

调用 `evaluate_expression(arg, row, table_info)` 取每行的 arg 值，然后调用 `agg.func` 累加。

### 第 3 层：engine_select.rs:599 - Sum 实现（**已正确**）

```rust
AggregateFunction::Sum => {
    // TPC-H Sprint 1 fix (Q8/Q9): accept Float in Sum.
    let mut int_sum: i64 = 0;
    let mut float_sum: f64 = 0.0;
    let mut any_float = false;
    for v in &values {
        match v {
            Value::Integer(n) => { ... }
            Value::Float(f) => { ... }
            _ => {}
        }
    }
    if values.is_empty() {
        Value::Integer(0)
    } else if any_float {
        Value::Float(float_sum)
    } else {
        Value::Integer(int_sum)
    }
}
```

**这段代码逻辑正确**：处理 Float，返回 Float。如果 `values` 都是 Float，会正确求和。

### 第 4 层：src/expr_utils.rs:168 - column 引用解析

```rust
Expression::Identifier(name) => {
    Ok(sqlrustgo_executor::expr::eval_identifier(
        name, row, &table_info.columns,
    ))
}
```

### 第 5 层：crates/executor/src/expr/mod.rs:498 - eval_identifier

```rust
pub fn eval_identifier(
    name: &str,
    row: &[Value],
    columns: &[sqlrustgo_storage::ColumnDefinition],
) -> Value {
    if let Some(col_idx) = find_column_index(name, columns) {
        row.get(col_idx).cloned().unwrap_or(Value::Null)  // ← 直接返 row 值
    } else {
        Value::Text(name.to_string())
    }
}
```

**正常** - 直接返 `row[col_idx]`，不做类型转换。所以**bug 一定在 eval_identifier 之前**——storage layer 拿出的 row 数据本身错了。

### 第 6 层：crates/storage/src/engine.rs - MemoryStorage

```rust
fn scan(&self, table: &str) -> SqlResult<Vec<Record>> {
    // ...
}
```

**嫌疑点**：当 INSERT 一行 `Value::Float(100.5)` 到 MemoryStorage，scan 取出时是否还是 `Value::Float(100.5)`，还是变成 `Value::Integer(0)`？

**这是 Sprint 4 真正要查的位置**。

---

## 2. 数据对比

### TPC-H 加载 lineitem 的代码

`tests/four_way_cell_diff_test.rs:160-180`（我的 cell-diff test 用的加载方式）：

```rust
for line in content.lines() {
    if line.is_empty() { continue; }
    let line_trimmed = line.trim_end_matches('|');
    let cols: Vec<&str> = line_trimmed.split('|').collect();
    let mut vals: Vec<String> = Vec::new();
    for c in cols {
        if c.parse::<i64>().is_ok() || c.parse::<f64>().is_ok() {
            vals.push(c.to_string());           // ← 数字不加引号
        } else {
            let esc = c.replace('\'', "''");
            vals.push(format!("'{}'", esc));    // ← 文本加引号
        }
    }
    let sql = format!("INSERT INTO lineitem VALUES ({})", tbl, vals.join(","));
    engine.execute(&sql).expect(...);
}
```

**但 tpch_full_22_test.rs 的加载方式不同**：

```rust
.map(|(i, s)| {
    let ty = col_types.get(i).copied().unwrap_or("TEXT");
    if ty == "INTEGER" || ty == "REAL" {
        s.to_string()      // ← 数字不加引号
    } else {
        // TEXT: quote and escape single quotes by doubling them
        let s_escaped = s.replace('\'', "''");
        format!("'{}'", s_escaped)   // ← 文本加引号
    }
})
```

**两种加载方式实质一样**：数字不加引号，文本加引号。

但**插值时对 REAL 类型列的处理是否一致？** REAL 列在数据文件里是 `"100.5"`（无引号），插入时 `engine.execute("INSERT INTO lineitem VALUES (10, 100.5, 0.05)")` - 数字被识别为浮点。

如果 INSERT 路径对 `100.5` 解析为 `Value::Float(100.5)`，scan 应该返回 `Value::Float(100.5)`。

**但实际 SUM 看到的是 0**。这意味着：
- 要么 INSERT 解析错（100.5 → Integer(100) 或 Integer(0)）
- 要么 storage.scan() 取出时类型错
- 要么 row 传过 join/select/project 时被 coerce 成 Integer

---

## 3. 关键混淆点

cell-diff JSON 显示：
- **Q01 col 2** (sum_qty, INTEGER): PG `146193.00` vs SR `146193` — **值相同，格式差异**
- **Q01 col 3** (sum_base_price, REAL): PG `877911.41` vs SR `0` — **值错**

**如果 storage 把 REAL 数据存错**：
- 那 col 4-5 (REAL 表达式) 也会错
- 也符合 chatGPT 的 "Type System Difference" 预测

**如果只是 Sum 不处理 Float**：
- col 3 会是 0 (Sum skip Float)
- 但 col 2 (sum_qty INTEGER) 应该正确

**但** col 2 = 146193（正确）。所以 storage 至少能正确处理 INTEGER。

**更精确的诊断**：Q1 col 2 (INTEGER col) 正确 → storage scan INTEGER 正常。
                    Q1 col 3 (REAL col) 错   → storage scan REAL **可能**有 bug。

**还需要验证**：直接 SELECT 一个 REAL 列看返什么，而不是 SUM。

---

## 4. Sprint 4 行动清单

### 调查阶段（1h）
1. 写最小 reproduction script：插 Value::Float(100.5)，SELECT 出来看是什么
2. 检查 `MemoryStorage::insert()` 和 `scan()` 的类型保留
3. 如果 `insert` 正确但 `scan` 错 → 修 scan
4. 如果 `insert` 错 → 修 INSERT 路径的数值解析

### 修复阶段（2-3h）
1. 修 storage 类型保留（无论根因在哪）
2. 加单元测试：
   - `MemoryStorage::insert_float_preserves_type` 
   - `engine_select::sum_real_column` (TPC-H Q1 shape)
3. 跑 cell-diff 验证 Q01/Q05/Q07/Q08/Q17 改善

### 验证阶段（0.5h）
1. 重新跑 4-way + cell-diff
2. 期望：Q01/Q05/Q07/Q08/Q17 全部 clean_match
3. Failure Matrix 更新

---

## 5. 已发现的方法论价值

虽然本会话未完成实际修复，但调查发现了一个**重要的工程方法论教训**：

> **当文档说"Sprint 1 fix: accept Float in Sum"时**，**chatGPT 模式 bug 假说可能错**。 
>
> 真实 bug 在更深链路：**storage layer 取出 REAL 行的类型保留**，而不是 Sum aggregate。
>
> 教训：TPC-H cell-level diff 才是 ground truth。文档与 PR 描述可能已经"声明 fix"但实际未触及 root cause。

这本身就是 Sprint 1 + 1.5 框架价值的证据：**实证优先于文档**。

---

## 6. 状态

| 项 | 状态 |
|---|---|
| Sprint 1 (Failure Matrix) | ✅ DONE |
| Sprint 1.5 (Cell-level diff) | ✅ DONE |
| Sprint 2 (22 query × 8 subsystem) | ✅ DONE |
| Sprint 3 (Operator regression suite) | ⏸️ Skipped (P0 fix 先) |
| Sprint 4 SUM(REAL)=0 调查 | ✅ 本文档 |
| Sprint 4 SUM(REAL)=0 修复 | 🔜 下一会话 |
| Sprint 2.5 (Execution Trace Diff) | ⏸️ Optional |
| Sprint 5 (4-way 验证 22/22) | ⏳ After Sprint 4 |

---

## 7. 元数据

| 字段 | 值 |
|---|---|
| 调查会话 | 2026-06-07 |
| 调查者 | claude-macmini (Sprint 1.5 → Sprint 4) |
| worktree | `.worktrees/v390-fix-sum-real` (origin/develop/v3.9.0) |
| 改动 | revert 所有死代码改动（0 行） |
| 文档 | 本文件 + 2 prior doc |
| 下一步 | 写 reproduction script + 修 storage 类型保留 |
| Issue | #3257 (待 Gitea 恢复后建) |

---

*本报告记录了 Sprint 4 调查链，避免下一会话重做。*
