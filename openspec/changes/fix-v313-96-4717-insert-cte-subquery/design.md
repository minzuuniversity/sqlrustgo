## Context

Issue #4717: the `FROM (subquery)` derived-table path accepts `WITH` at
the dispatch level but routes to `parse_select_statement`, which only
accepts a literal `SELECT`. The error message "Expected Select, got With"
leaves users (and TPC-H Q15) stranded.

## Approach

Two-line edit in `crates/parser/src/parser.rs`:

1. `parse_select_statement` checks `Token::With` at entry; if present,
   calls `parse_with_select` and unwraps the resulting `WithSelect.select`
   as the return value.
2. Any non-`WithSelect` return (i.e. `WithDml`) is rejected with an
   explicit error rather than silently mishandled.

The CTE scope semantics are unchanged: the WITH clause binds to the
derived table only.

## Files Changed

- `crates/parser/src/parser.rs` — one entry-point branch in
  `parse_select_statement`.
- `tests/integration/sql/repro_v313_96_4717_insert_cte_subquery.rs` — new
  test file with 3 cases.
- `Cargo.toml` — register the new test target.
- `openspec/changes/fix-v313-96-4717-insert-cte-subquery/` — proposal,
  design, tasks, spec.

## Verification

| Test | Result |
|---|---|
| `insert_select_from_with_recursive_parses` | PASS |
| `insert_select_from_with_non_recursive_parses` | PASS |
| `select_from_with_recursive_subquery_parses` | PASS |
| `cte_materialization_test` | 9/9 PASS |
| `parser_e2e_test` | 249/249 PASS |
| `repro_v312_93_cte_values_anchor` | 4/4 PASS |
| `cargo check --workspace --all-features` | clean |
| `openspec validate --strict` | valid |

## Non-Goals

- Visibility of derived-table CTEs to the outer scope.
- WITH in non-FROM subquery positions.
- Engine-side changes (no engine change needed; CTE materialization
  already handles nested projections).