## Context

Issue #4671: `CREATE FUNCTION` is currently restricted to a single
`RETURN expr` body. The MySQL/SQLite multi-statement `BEGIN ... END`
form is rejected (and the existing `BEGIN ... END` path is gated
behind a mandatory `AS` keyword). `RETURNS TABLE(col type, ...)` is
also rejected.

## Approach

Two minimal parser edits. The `read_until_end_block` helper for
`BEGIN ... END` already exists; we remove the `AS` gate and add a
`RETURNS TABLE` peek. No executor changes (the AST captures the
table-return column list, but the runtime does not yet produce rows
from a table-valued UDF).

## Files Changed

- `crates/parser/src/parser.rs` — `parse_create_function`: drop the
  `AS` gate before the function body; add a `Token::Table` peek
  after `RETURNS` that consumes a column list.
- `tests/integration/sql/repro_v313_101_4671_create_function.rs` —
  5 new tests.
- `Cargo.toml` — register the new test target.
- `openspec/changes/fix-v313-101-4671-create-function/` — proposal,
  design, tasks, spec.

## Verification

| Test | Result |
|---|---|
| `repro_v313_101_4671_create_function` | 5/5 PASS |
| `cte_materialization_test` | 9/9 PASS (no regression) |
| `parser_e2e_test` | 249/249 PASS (no regression) |
| `rollup_cube_test` | 14/14 PASS (no regression) |
| `repro_v312_93_cte_values_anchor` | 4/4 PASS (no regression) |
| `repro_v313_96_4717_insert_cte_subquery` | 3/3 PASS (no regression) |
| `repro_v313_97_4679_grouping_sets` | 3/3 PASS (no regression) |
| `repro_v313_98_4701_multi_select` | 3/3 PASS (no regression) |
| `repro_v313_99_4692_writable_cte` | 6/6 PASS (no regression) |
| `repro_v313_100_4701_expression_index` | 3/3 PASS (no regression) |
| `cargo check --workspace --all-features` | clean |
| `openspec validate --strict` | valid |

## Non-Goals

- Table-valued UDF execution.
- `DECLARE` statement parsing inside the body.
- `OUT` / `INOUT` UDF parameters.