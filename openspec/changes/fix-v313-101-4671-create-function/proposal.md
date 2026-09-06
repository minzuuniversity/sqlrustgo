## Why

Issue #4671: `CREATE FUNCTION` parsing is currently restricted to the
simplest single-`RETURN` form. Two related sub-problems:

1. The function body is not accepted as a multi-statement `BEGIN ...
   END` block unless preceded by the (SQL Server-style) `AS` keyword.
   The MySQL/SQLite-style `RETURNS int BEGIN ... END` (without `AS`)
   is rejected with `Parse error: Expected Return, got Begin`.
2. `RETURNS TABLE(col1 type1, col2 type2, ...)` (table-valued UDF
   return type) is rejected with `Parse error: Expected return type,
   got Table`.

## Fix

Two surgical edits in `crates/parser/src/parser.rs`:

1. `parse_create_function` no longer requires `AS` before the
   function body. Both forms are accepted:
   - `CREATE FUNCTION f() RETURNS int BEGIN ... END`
   - `CREATE FUNCTION f() RETURNS int AS BEGIN ... END`
   - `CREATE FUNCTION f() RETURNS int RETURN expr`
   - `CREATE FUNCTION f() RETURNS int AS RETURN expr`
   The existing `read_until_end_block` helper handles the `BEGIN ... END`
   form (it was already present but gated behind `AS`).
2. After `RETURNS`, the parser peeks for `Token::Table`; if present
   it consumes a parenthesised column list and stores the columns
   in `CreateFunctionStatement.return_columns`. Otherwise it falls
   through to the historical scalar-type match.

## Capabilities

### New Capabilities

- `parser-create-function-begin-end`: multi-statement function body
  with or without the `AS` keyword now parses successfully.
- `parser-create-function-table-return`: `RETURNS TABLE(col1 type1,
  ...)` records the column list in
  `CreateFunctionStatement.return_columns`.

### Modified Capabilities

- `parser-create-function`: the function-body parse path no longer
  requires `AS`; both `AS` and bare `BEGIN ... END` (and bare
  `RETURN expr`) are accepted.
- `CreateFunctionStatement` gains a new `return_columns: Vec<UdfParam>`
  field. The field is empty for scalar returns; the executor does
  not yet consume table-valued UDFs (deferred to v3.13).

## Out of Scope

- Executor support for table-valued UDFs (the AST records the column
  list, but the runtime does not produce rows from a `RETURNS
  TABLE(...)` function yet).
- `DECLARE` statement parsing inside the multi-statement body. The
  body is captured as a raw token stream via `read_until_end_block`,
  which is the same approach the existing `CREATE PROCEDURE` body
  parser uses; the runtime does not yet execute those statements.
- `OUT` / `INOUT` parameters in UDFs (UDFs implicitly have `IN`-only
  parameters today).

## Verification

- 5 new tests in
  `tests/integration/sql/repro_v313_101_4671_create_function.rs`:
  `parse_create_function_with_begin_end_no_as` (issue body f2),
  `parse_create_function_with_begin_end_with_as` (AS prefix
  regression),
  `parse_create_function_single_expr_still_works` (regression for
  simple `RETURN expr`),
  `parse_create_function_with_returns_table` (issue body f3),
  `parse_create_function_with_returns_table_and_begin_end` (table
  return + multi-statement body combined).
- No regression: all previous repro_v313_* and cte_materialization /
  parser_e2e / rollup_cube test suites.
- `cargo check --workspace --all-features`: clean.
- `openspec validate fix-v313-101-4671-create-function --strict`:
  valid.