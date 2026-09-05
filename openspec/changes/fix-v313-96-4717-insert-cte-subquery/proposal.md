## Why

Issue #4717: `INSERT INTO t SELECT ... FROM (WITH RECURSIVE ...)` fails with
`Parse error: Expected Select, got With`.

The FROM-clause dispatch already accepts `With` as a possible subquery opener
(via the `Select|With|Values` arm at `parse_table_ref` from-clause) and
delegates to `parse_select_statement`. But `parse_select_statement` itself
starts with `expect(Token::Select)`, which rejects `Token::With` and reports
the misleading "Expected Select, got With" error.

This is the FROM-subquery sibling of #4704-1. The CTE anchor grammar (the
`AS (` inside `WITH cte AS (...)`) already accepts VALUES; the FROM subquery
grammar now needs to accept the full `WITH [RECURSIVE] cte AS (...) SELECT`
prefix. Together these two changes unblock TPC-H Q15 (which uses an
INSERT … SELECT … FROM (WITH RECURSIVE …) pattern).

## Fix

Extend `parse_select_statement` in `crates/parser/src/parser.rs` so that when
the current token is `Token::With`, it delegates to `parse_with_select` and
unwraps the resulting `Statement::WithSelect { select, .. }` to a plain
`SelectStatement`. Any other `Statement` variant from `parse_with_select`
(WithDml) in subquery position is rejected with a clear error message.

This is parser-only. Executor semantics for nested CTE projections are
already covered by the existing CTE-materialization work; no engine change
is needed.

## Capabilities

### New Capabilities

- `parser-cte-in-subquery`: `SELECT ... FROM (WITH [RECURSIVE] cte AS (...)
  SELECT ...)` parses successfully. The WITH clause is bound to the
  derived table only; sibling FROM siblings cannot see it.

### Modified Capabilities

- `parser-select-statement`: the entry point now accepts a leading `WITH`
  token, delegates to `parse_with_select`, and unwraps the result. This
  is a strict superset of the previous behaviour — every SQL statement
  that previously parsed still parses.

## Out of Scope

- Cross-CTE visibility between the derived table and the outer query
  (SQL semantics: derived table is opaque to the outer scope).
- Recursive CTE in any other subquery position (e.g. `WHERE (WITH ...)`).
  Same engine-side semantics apply; only the FROM-derived-table path is
  common in practice (TPC-H Q15).
- `INSERT INTO ... WITH cte AS (...) ...` (without subquery in FROM) —
  already covered by PR #4757 / V312-89.

## Verification

- 3 new tests in
  `tests/integration/sql/repro_v313_96_4717_insert_cte_subquery.rs`:
  `insert_select_from_with_recursive_parses` (issue body example, with
  alias),
  `insert_select_from_with_non_recursive_parses`,
  `select_from_with_recursive_subquery_parses`. All 3/3 pass.
- No regression: `cte_materialization_test` (9/9),
  `parser_e2e_test` (249/249), `repro_v312_93_cte_values_anchor` (4/4).
- `cargo fmt --check -- crates/parser/src/parser.rs Cargo.toml`: PASS.
- `cargo check --workspace --all-features`: clean.
- `openspec validate fix-v313-96-4717-insert-cte-subquery --strict`:
  valid.