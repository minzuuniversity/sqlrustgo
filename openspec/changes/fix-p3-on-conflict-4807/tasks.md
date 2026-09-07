# Tasks — Issue #4807: ON CONFLICT DO UPDATE SET excluded.col 真正执行

## 1. Bug Localization

- [ ] 1.1 在 `src/execution_engine.rs` 找到 ON CONFLICT dispatch 表,
      记录当前 ON CONFLICT 路径走的是哪个 execute_* 方法。
- [ ] 1.2 在 `src/engine_dml.rs` 的 `execute_insert` 中找到 ON CONFLICT
      分支,记录当前 `for conflict_row in conflicts` 循环的实现。
- [ ] 1.3 用 debug println 验证三个候选 bug 路径:
      - excluded.col 是否 evaluate 为 incoming row 的值?
      - conflict 检测是否触发?
      - UPDATE 是否真的写入 storage (返回前 get_row 验证)?
- [ ] 1.4 把 issue anchor 测试 (`INSERT INTO t VALUES (1,'a',5) ON
      CONFLICT (id) DO UPDATE SET cnt = excluded.cnt`) 先写出来,确认
      在当前代码下 FAILED。

## 2. AST + Parser 验证

- [ ] 2.1 验证 `OnConflictClause::DoUpdate { assignments }` 中
      `excluded.col` 解析为 `Expr::QualifiedColumn("excluded", "col")`。
- [ ] 2.2 验证 `Expr::QualifiedColumn` 在 `evaluate_expression` 中
      已支持作为 excluded reference 解析(查 expr eval 现有代码)。
- [ ] 2.3 如果 AST 缺失字段 (例如 excluded schema 不暴露),新增
      `OnConflictClause::DoUpdate` 的 schema 字段 `incoming_rows:
      Vec<Vec<Value>>`(传入每行的 incoming 值)。

## 3. Executor 修复

- [ ] 3.1 在 `execute_insert` 的 ON CONFLICT 分支,确认每条 conflict
      行都执行了以下步骤:
      ```rust
      for incoming_row in incoming_rows {
          if let Some(existing) = storage.get_row_by_key(table, &key_of(&incoming_row)) {
              // conflict — DO UPDATE branch
              let mut updated_row = existing.clone();
              for (col, expr) in &on_conflict.assignments {
                  let new_val = evaluate_upsert_expr(expr, &incoming_row, &existing);
                  updated_row[column_index(col)] = new_val;
              }
              // WHERE filter
              if let Some(where_expr) = &on_conflict.where_clause {
                  if !eval_where(where_expr, &updated_row) { continue; }
              }
              storage.update_row(table, &key, &updated_row)?;
              updated_count += 1;
          } else {
              // no conflict — INSERT normally
              storage.insert_row(table, &incoming_row)?;
              inserted_count += 1;
          }
      }
      ```
- [ ] 3.2 修复 `evaluate_upsert_expr` 函数,正确处理:
      - `excluded.col` → `incoming_row[col]`
      - `col` (bare) → `existing_row[col]` (PostgreSQL: bare col 指原表)
      - 其他表达式 → 用 `incoming_row` 作为 evaluation context
        (新行的值,但不写入)
- [ ] 3.3 验证 `storage.update_row` 真正执行 UPDATE (返回 success 表示
      行已更新,not silently no-op)。必要时加 instrumentation。
- [ ] 3.4 确保 multi-row UPSERT 中每行的 incoming 是独立的(不能用
      last incoming row)。

## 4. Tests (8 tests)

- [ ] 4.1 `on_conflict_do_update_actually_updates` — issue anchor:
      `INSERT INTO t VALUES (1,'a',5) ON CONFLICT (id) DO UPDATE SET
      cnt = excluded.cnt` 后 `SELECT cnt` 返回 5 (was 0)。
- [ ] 4.2 `on_conflict_do_update_with_excluded_default_zero` — 验证
      excluded.col 不是默认 0 (anchor 测试已覆盖)。
- [ ] 4.3 `on_conflict_do_update_multi_row` — `INSERT VALUES
      (1,'a',5),(2,'b',6),(3,'c',7) ON CONFLICT (id) DO UPDATE SET
      cnt = excluded.cnt`,3 行都更新。
- [ ] 4.4 `on_conflict_do_update_with_where_clause` — `WHERE
      excluded.cnt > 0` 过滤。
- [ ] 4.5 `on_conflict_do_nothing_still_works` — 回归测试:DONOTHING 仍
      不改行。
- [ ] 4.6 `on_conflict_do_update_set_constant` — `SET cnt = 100` (无
      excluded 引用)正确。
- [ ] 4.7 `on_conflict_do_update_multiple_columns` — `SET cnt =
      excluded.cnt, name = excluded.name` 同时更新多列。
- [ ] 4.8 `on_conflict_do_update_with_expression` — `SET cnt =
      excluded.cnt + 1` 表达式求值正确。

## 5. Verification

- [ ] 5.1 `cargo build --all-features` clean。
- [ ] 5.2 `cargo test --test p3_on_conflict_4807_test` 8/8 PASS。
- [ ] 5.3 `cargo test --all-features --lib` no regression。
- [ ] 5.4 `cargo test --all-features --test parser_e2e_test --test
      cte_materialization_test --test repro_v313_99_4692_writable_cte`
      全绿。
- [ ] 5.5 `openspec validate fix-p3-on-conflict-4807 --strict` valid。

## 6. Commit + Memory

- [ ] 6.1 Commit message:
      `fix(P3 / #4807): ON CONFLICT DO UPDATE actually executes SET excluded.col`
- [ ] 6.2 在 `memory/` 下新增
      `p3-on-conflict-4807-actually-updates.md` 记录 excluded.col
      evaluate context 与 PostgreSQL 语义对照。
- [ ] 6.3 `openspec archive fix-p3-on-conflict-4807` after merge。

## 7. Out of Scope (deferred)

- [ ] 7.1 RETURNING with ON CONFLICT — v3.14。
- [ ] 7.2 ON CONSTRAINT name form — v3.14。
- [ ] 7.3 Cross-table UPSERT — v3.15。
- [ ] 7.4 SET col reference order — v3.14 (PostgreSQL strict order)。
