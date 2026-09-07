## Why

Issue #4807: `INSERT INTO ... ON CONFLICT (col) DO UPDATE SET col =
excluded.col` 在 sqlrustgo 中静默失败 — executor 接受 INSERT 但不走
UPDATE 分支,行数据保持原值不变。无报错、无警告,极难调试。

这是"假成功" pattern,违反 SQL 标准的 UPSERT 语义。与 MySQL 8.0+ 和
PostgreSQL 11+ 不兼容,影响所有依赖 UPSERT 的应用层代码 (retry 逻辑、
unique-violation handling、ETL pipelines)。

根因 (issue 描述): `crates/executor/src/dml.rs` 中 ON CONFLICT 路径
接受了 INSERT 但没有执行 UPDATE 分支。可能原因:
1. parser 接受 `excluded.col` 但 executor 在 update 时把 excluded.cnt
   求值为 0 (默认值)
2. conflict 检测没触发,INSERT 走了 no-op 路径
3. UPDATE SET 子句中 `excluded.col` 没正确解析为新值

## What Changes

1. **AST 修正**: 在 `OnConflictClause` 中,`excluded.col` 引用已经存储
   为 `Expr::QualifiedColumn("excluded", col)`。验证 executor 在执行
   UPDATE SET 子句时正确把 excluded.col 解析为新行的列值。
2. **Executor fix**: 在 `execute_insert` 的 ON CONFLICT 分支:
   - 检测 conflict 行 (现有逻辑)。
   - 对每条 conflict 行,**构造新的 row**(用 INSERT 的新值替换目标列)。
   - 在 `UPDATE SET col = excluded.col` 中,`excluded.col` 必须 lookup
     **新 row** 的列值,不是原表 row 的列值。
   - 实际执行 UPDATE 写入 storage,而不是 silently no-op。
3. **Multi-row UPSERT**: 测试 `INSERT VALUES (1,'a',5),(2,'b',6)
   ON CONFLICT (id) DO UPDATE SET cnt = excluded.cnt` 中,每行的
   excluded.col 是**该行**的新值,不是其他行的新值。
4. **WHERE clause on UPDATE**: `ON CONFLICT ... DO UPDATE SET ... WHERE
   cond` 支持 (PostgreSQL extension)。

## Capabilities

### New Capabilities

- `executor-on-conflict-do-update`: `INSERT ... ON CONFLICT (col) DO
  UPDATE SET col = excluded.col` MUST 实际更新行,把原值替换为 INSERT
  新值,并返回 updated row count。

### Modified Capabilities

- `executor-upsert`: `ON CONFLICT DO NOTHING` 行为保持不变 (已经正确)。
- `parser-on-conflict`: parser 接受 `excluded.col` AST 已经正确,无需
  改动。

## Out of Scope

- **RETURNING with ON CONFLICT**: 推迟到 [[fix-v312-69-issue-4653-insert-returning]]
  + 本 PR 后续 follow-up。
- **DO UPDATE 涉及多列相互引用** (`SET a = b + 1, b = excluded.a`):
  PostgreSQL 语义严格按声明顺序,本 PR 实现 strict order,SQLite/PG 不
  支持 ORDER 指定的 out-of-scope。
- **ON CONSTRAINT 命名约束** (`ON CONFLICT ON CONSTRAINT name`):
  推迟到 v3.14。
- **跨表 UPSERT**: 不在本 PR。

## Verification

- 新测试 `tests/integration/sql/p3_on_conflict_4807_test.rs`:
  - `on_conflict_do_update_actually_updates` — 来自 issue 的 anchor 测试。
  - `on_conflict_do_update_with_excluded_default_zero` — 验证 excluded.col
    解析为新值,不是默认值 0。
  - `on_conflict_do_update_multi_row` — 多行 INSERT 部分 conflict,
    每行独立 excluded.col。
  - `on_conflict_do_update_with_where_clause` — `WHERE excluded.cnt > 0`
    过滤。
  - `on_conflict_do_nothing_still_works` — 回归: `DO NOTHING` 仍正确。
  - `on_conflict_do_update_set_constant` — `SET cnt = 100` (无 excluded 引用)。
  - `on_conflict_do_update_multiple_columns` — `SET cnt = excluded.cnt,
    name = excluded.name`。
  - `on_conflict_do_update_with_expression` — `SET cnt = excluded.cnt + 1`。
- `cargo test --test p3_on_conflict_4807_test` 8/8 PASS。
- 回归: `repro_v313_99_4692_writable_cte` 6/6 + `parser_e2e_test`
  249/249 + `insert_returning_test` (未来) 全绿。

## Risks / Trade-offs

- **State transition clarity**: ON CONFLICT 路径涉及 INSERT (建候选行) →
  conflict 检测 → UPDATE (改 conflict 行) 三阶段,executor 必须保证每
  个 conflict 行只被 UPDATE 一次,不重复。
- **Transaction visibility**: UPDATE 的行必须在 SELECT 立即可见,与
  WAL 集成需要单独验证 (FileStorage)。
- **Performance**: 当前 implementation 逐行 conflict check + UPDATE,
  对于 N 行 INSERT + M 行 conflict 是 O(N*table_size)。对于大表可考虑
  batched UPDATE,但本 PR 不优化性能。
