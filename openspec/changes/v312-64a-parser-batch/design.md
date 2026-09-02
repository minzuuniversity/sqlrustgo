# v312-64a Parser Batch — Design

## Scope

5 parser-only (or trivial no-op executor) issues from V312-63+ follow-up sweep:

| Issue | Surface | Type |
|-------|---------|------|
| #4651 | `CAST(... AS DATE/DATETIME/TIMESTAMP)` | Parser |
| #4662 | `CREATE TRIGGER ... INSTEAD OF ...` | Parser + lex keyword |
| #4663 | `VACUUM` / `REINDEX` / `ANALYZE` (no-table) | Lex + parser + no-op exec |
| #4655 | `DATE_TRUNC('unit', dt)` function | Executor function registry |
| #4665 | `OVER (ORDER BY id ROWS BETWEEN ...)` window frame | Parser + executor moving window |

## Architecture

### #4651 — CAST AS DATE

`crates/parser/src/parser.rs:8380` CAST AS match has 4 keyword arms (Integer, Text, Float, Boolean) + Identifier fallback. Issue: `Token::Date` is a keyword token (set in `crates/parser/src/lexer.rs:527`) but not in the CAST match — falls through to `_` arm and errors.

Fix: add `Some(Token::Date)` arm pushing `Literal("DATE")`. DATETIME/TIMESTAMP/TIME/YEAR are NOT in lexer's keyword map, so they fall through to Identifier — already handled by the existing `Some(Token::Identifier(ty))` arm.

### #4662 — INSTEAD OF trigger

`parse_create_trigger` at `crates/parser/src/parser.rs:3469` only matches `Token::Before | Token::After`. Add:
1. `Token::Instead` to enum + lex keyword map
2. New arm before the BEFORE/AFTER block: `Some(Token::Instead) => { self.next(); self.expect(Token::Of)?; "INSTEAD OF" }`
3. Expose as timing string "INSTEAD OF" (existing catalog already keyed on timing string, no schema change)

### #4663 — VACUUM / REINDEX / ANALYZE no-table

Two new tokens (Vacuum, Reindex), two new Statement variants (Vacuum, Reindex — mirror AnalyzeStatement shape with optional table_name), two new dispatch arms in `parse_statement`, two new no-op executor arms in `execution_engine.rs::execute_statement`, two new arms in `distributed/read_write_splitter.rs::classify_statement`.

For ANALYZE without table_name: existing parser returns `None` (line 10497 accepts Semicolon|None). Current executor (line 750) errors. Fix: when `table_name` is `None`, iterate `self.storage.list_tables()`, run `collect_table_stats` per table, write to stats map, return success — same effect as full sweep.

### #4655 — DATE_TRUNC

`crates/executor/src/expr/mod.rs` has a function registry keyed by name. Add `DATE_TRUNC` with 2 args:
- arg[0]: string unit ("year" | "quarter" | "month" | "day" | "hour" | "minute" | "second")
- arg[1]: date/datetime value (string `'YYYY-MM-DD'` or `'YYYY-MM-DD HH:MM:SS'`)

Implementation: parse arg[1] to components, truncate per unit, format back. Return string.

Edge cases:
- Quarter truncation: M → ((M-1)/3)*3 + 1
- Day truncation: h=m=s=0
- Hour truncation: m=s=0 (output `YYYY-MM-DD HH:00:00`)

### #4665 — Window frame

`WindowSpecification` (currently holds only `partition_by`, `order_by`) gets a new `frame: Option<WindowFrame>` field. `WindowFrame { kind: Rows|Range, start: FrameBound, end: Option<FrameBound> }`. `FrameBound { kind: BoundKind, value: Option<i64> }`. `BoundKind = UnboundedPreceding | Preceding(n) | CurrentRow | Following(n) | UnboundedFollowing`.

Parser: in `parse_over_clause` after ORDER BY, peek for `Rows|Range` — if found, consume `Rows|RANGE`, optionally `BETWEEN`, parse first bound (UnboundedPreceding, n Preceding, Current Row, n Following), if `AND` was seen parse second bound.

Executor: `crates/executor/src/expr/window_call.rs` (or `expr/mod.rs`) implements window aggregation. Current default is `RANGE UNBOUNDED PRECEDING` (running total). Add ROWS BETWEEN path:
- For each output row, scan partition rows, include those at indices `[row_idx - preceding, row_idx + following]` (clamped to partition bounds), aggregate via SUM/COUNT/AVG.
- Minimum: support `SUM/COUNT/AVG ... ROWS BETWEEN a PRECEDING AND b FOLLOWING` and unbounded variants. Other frame kinds (RANGE) — implement using RANK-based comparison (defer if too complex).

## Test Surface

`tests/integration/sql/repro_v312_64a.rs` (10-12 tests):
- 2 cast_as_date / cast_as_datetime
- 1 trigger_instead_of_view
- 3 vacuum / reindex / analyze_no_table
- 2 date_trunc_month / date_trunc_year
- 2 window_rows_between_sum / window_rows_unbounded_preceding
- 1 cli smoke (CSV batch mode covers all 5 issues end-to-end)

## Pre-existing failures (NOT mine)

- `parse_one_table_constraint` clippy dead-code (commit 3b44106dc1)
- `test_execute_with_params` / `test_parse_alter_table_modify_column` / `test_insert_ignore_parsing`