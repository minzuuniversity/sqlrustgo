## Context

Issue #4807 — `INSERT ... ON CONFLICT (col) DO UPDATE SET col =
excluded.col` 在 executor 层静默失效。Parser 已经接受 SQL,AST 中
`OnConflictClause.action` 是 `OnConflictAction::DoUpdate { assignments:
Vec<(String, Expr)>, where_clause: Option<Expr> }`。

`assignments` 中的 `Expr::QualifiedColumn("excluded", "col")` 是
SQL 标准 `excluded.col` 引用的 AST 形式,executor 在 evaluate 时
必须 lookup **当前 INSERT 的新 row** 的 col 值。

当前 bug 推测路径:
1. INSERT 时 conflict 检测返回 `existing_row` (原表 row)。
2. Executor 在 UPDATE SET 时,把 `excluded.col` 错误地求值为
   `existing_row[col]`,即原表的列值。
3. 由于 `existing_row[col] = some value`,UPDATE SET 写回原值,行没变化。
4. 或者更糟: conflict 检测没触发,INSERT 走 no-op 路径,新行根本没
   写入。

无论哪种 bug,根本修复都是:**executor 必须在 ON CONFLICT UPDATE 分支
中重新 evaluate `excluded.col` 为新 INSERT 行的列值,并真正执行 UPDATE
写入 storage。**

## Approach

### A1. 修复 `excluded.col` 解析

在 `src/executor/dml.rs` 的 `execute_insert` 函数中,ON CONFLICT 分支:

```rust
// 当前疑似 buggy 代码 (简化):
for conflict_row in conflicts {
    let mut new_row = conflict_row.existing.clone();
    for (col, expr) in &on_conflict.assignments {
        if let Expr::QualifiedColumn("excluded", c) = expr {
            // BUG: 可能用了 conflict_row.existing[c] 而不是新行的值
            let new_val = conflict_row.incoming[c].clone();  // 应该是这个
            new_row[c] = new_val;
        }
    }
    // 写入 storage
    storage.update_row(table_name, &conflict_row.key, &new_row)?;
}
```

修复后: `conflict_row.incoming` 是 INSERT 的新行(包含 conflict 的列 +
其他列的新值),`conflict_row.existing` 是原表行。

### A2. UPDATE actually writes to storage

验证 `storage.update_row` / `storage.write_row` 在 conflict 行上真的
执行了 UPDATE,而不是 silently 返回 success 没改数据。

加 debug log 或 return value 验证:UPDATE 后 `get_row(key)` 应该返回
`new_row`,不是 `existing_row`。

### A3. Multi-row UPSERT 处理

```sql
INSERT INTO t (id, name, cnt) VALUES (1,'a',5), (2,'b',6), (3,'c',7)
ON CONFLICT (id) DO UPDATE SET cnt = excluded.cnt;
```

每行的 `excluded.cnt` 是该行自己的新值:
- 行 1: `excluded.cnt = 5`
- 行 2: `excluded.cnt = 6`
- 行 3: `excluded.cnt = 7`

不能跨行共享同一个 `excluded.cnt` 值。

### A4. WHERE clause on UPDATE

PostgreSQL 扩展:`ON CONFLICT (col) DO UPDATE SET col = excluded.col
WHERE excluded.col > 0`。`excluded.col` 在 WHERE 中同样指新行的值。

实现:`WHERE excluded.cnt > 0` 的 evaluate 在每行 UPSERT 时独立 evaluate。

### A5. 完整的 SET clause 表达式支持

- `SET col = excluded.col` — basic form
- `SET col = excluded.col + 1` — 表达式
- `SET col = 100` — 常量
- `SET col = col + 1` — 引用原表的 col (PostgreSQL: `col` 指 existing,
  `excluded.col` 指 incoming)

## Files Changed

| File | Lines | Purpose |
|------|-------|---------|
| `src/executor/dml.rs` | +60 | 修复 excluded.col evaluate + 实际 UPDATE 写入 |
| `src/execution_engine.rs` | +30 | ON CONFLICT 分支整体修正,确保 UPDATE 路径触发 |
| `src/engine_dml.rs` | +40 | ON CONFLICT DO UPDATE 分支 state machine |
| `tests/integration/sql/p3_on_conflict_4807_test.rs` | +200 | 8 个集成测试 |
| `Cargo.toml` | +5 | 注册新 test target |

Total: ~335 lines, ~5 files touched.

## Verification

| Test | Expected |
|------|----------|
| `cargo build --all-features` | clean |
| `p3_on_conflict_4807_test` | 8/8 PASS |
| `repro_v313_99_4692_writable_cte` | 6/6 PASS (no regression) |
| `parser_e2e_test` | 249/249 PASS |
| `cte_materialization_test` | 9/9 PASS |
| `cargo test --all-features --lib` | no regression |

## Non-Goals

- RETURNING with ON CONFLICT (deferred to v3.14)
- Cross-table UPSERT
- ON CONSTRAINT name form
- DO UPDATE column reference order (strict declaration order, no out-of-order)

## Difficulty Tier

**🔴 HARD** — 多步 DML state machine + `excluded.col` 语义正确性 +
实际 UPDATE 写入路径,涉及 INSERT → conflict 检测 → UPDATE 三阶段
orchestration,bug 隐蔽 (silent fail)。
