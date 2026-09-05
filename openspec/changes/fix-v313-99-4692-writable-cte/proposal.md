## Why

Issue #4692 (sub-problem 1 and 2): two related parser gaps.

1. **Writable CTE** (PostgreSQL 11+, SQLite 3.33+). The CTE body was
   only allowed to be a `SELECT` statement (or a nested `WITH` clause
   or `VALUES` clause). The body dispatched in
   `parse_with_clause`'s subquery arm went through
   `parse_select_or_union()`, which expected a leading `SELECT` token
   and reported `Parse error: Expected Select, got Delete` for
   `WITH cte AS (DELETE FROM t WHERE val < 25 RETURNING *) SELECT * FROM cte`.
   TPC-H Q15 and Q22 are the canonical users of this shape.

2. **`CREATE MATERIALIZED VIEW`**. The CREATE dispatcher in
   `parse_create` only recognised `Token::View`. PostgreSQL's
   `CREATE MATERIALIZED VIEW name AS SELECT ...` shipped the
   `MATERIALIZED` token as a plain identifier, so the dispatcher
   fell through to the catch-all `Err("Expected TABLE, INDEX, ...
   after CREATE, got MATERIALIZED")`.

## Fix

Three surgical edits in `crates/parser/src/parser.rs`:

1. `parse_with_clause`'s subquery dispatch (line 4390) now matches
   the leading token of the body against the same DML set that
   `parse_with_select` already supports (`Insert`, `Replace`,
   `Update`, `Delete`, `Values`), with a `With` arm preserved for
   nested CTE. Anything else falls through to
   `parse_select_or_union()`.

2. `parse_delete` now consumes an optional `RETURNING <expr-list>`
   clause after `WHERE`. This is the PostgreSQL/SQLite 3.33+ form
   `DELETE FROM t WHERE ... RETURNING *` that writable CTEs require.
   The column list is parsed and discarded — engine-side
   `RETURNING` is out of scope for this PR.

3. `parse_update` mirrors the same `RETURNING` consumption as
   `parse_delete`.

4. `parse_create` now accepts `MATERIALIZED` (case-insensitive
   identifier) when the following token is `View`, dispatching to
   `parse_create_view`. Bare `CREATE MATERIALIZED` without `VIEW`
   still falls through to the error arm, so we don't accidentally
   swallow typos.

## Capabilities

### New Capabilities

- `parser-writable-cte`: `WITH cte AS (DELETE|UPDATE|INSERT ...)`
  parses successfully. The CTE body remains a full
  `DeleteStatement` / `UpdateStatement` / `InsertStatement`; the
  outer `WITH` is recorded on a new `WithDml` wrapper.
- `parser-materialized-view`: `CREATE MATERIALIZED VIEW name AS
  SELECT ...` parses successfully. The `MATERIALIZED` qualifier is
  ignored downstream (the existing view materialisation path is
  unchanged).

### Modified Capabilities

- `parser-delete-statement`: now accepts an optional trailing
  `RETURNING <expr-list>` clause.
- `parser-update-statement`: now accepts an optional trailing
  `RETURNING <expr-list>` clause.

## Out of Scope

- Engine-side execution of writable CTEs (the existing DML path
  on `WithDml` does not currently evaluate the inner DELETE /
  UPDATE / INSERT against the catalog — that's a separate executor
  change).
- Engine-side materialisation of MATERIALIZED VIEWs (still
  parsed-as-view).
- Engine-side `RETURNING` projection (the columns are parsed
  and discarded; no values flow back to the caller).

## Verification

- 6 new tests in
  `tests/integration/sql/repro_v313_99_4692_writable_cte.rs`:
  `cte_with_delete_body_parses`,
  `cte_with_update_body_parses`,
  `cte_with_insert_body_parses`,
  `create_materialized_view_parses`,
  `create_regular_view_still_parses` (regression),
  `create_materialized_alone_does_not_eat_view` (negative test).
- No regression: `cte_materialization_test` (9/9),
  `parser_e2e_test` (249/249), `rollup_cube_test` (14/14),
  `repro_v312_93_cte_values_anchor` (4/4),
  `repro_v313_96_4717_insert_cte_subquery` (3/3),
  `repro_v313_97_4679_grouping_sets` (3/3),
  `repro_v313_98_4701_multi_select` (3/3).
- `cargo check --workspace --all-features`: clean.
- `openspec validate fix-v313-99-4692-writable-cte --strict`: valid.