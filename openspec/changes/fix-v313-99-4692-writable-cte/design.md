## Context

Issue #4692 has two sub-problems, both parser-level:

1. `WITH cte AS (DELETE FROM t WHERE val < 25 RETURNING *) SELECT * FROM cte`
   fails with `Parse error: Expected Select, got Delete`. The CTE
   body dispatch in `parse_with_clause` only matched `With` or
   anything (delegated to `parse_select_or_union`).
2. `CREATE MATERIALIZED VIEW name AS SELECT * FROM t` fails with
   `Parse error: Expected TABLE/INDEX/... after CREATE, got
   MATERIALIZED`. The CREATE dispatcher only matched `Token::View`.

## Approach

Three minimal edits in `crates/parser/src/parser.rs`:

1. Extend `parse_with_clause`'s subquery arm to match the DML tokens
   that `parse_with_select` already supports (`Insert`, `Replace`,
   `Update`, `Delete`, `Values`). Anything else falls through to
   `parse_select_or_union`. Each DML arm calls the corresponding
   `parse_*` method; the resulting `Statement` is stored as the
   CTE's `subquery` field (a `Box<Statement>`).

2. `parse_delete` and `parse_update` now consume an optional
   `RETURNING <expr-list>` clause at the end. The column list is
   parsed and discarded.

3. `parse_create` now accepts `MATERIALIZED` (case-insensitive
   identifier) when the following token is `View`, dispatching
   to `parse_create_view`.

## Files Changed

- `crates/parser/src/parser.rs` — three edits.
- `tests/integration/sql/repro_v313_99_4692_writable_cte.rs` —
  6 new tests.
- `Cargo.toml` — register the new test target.
- `openspec/changes/fix-v313-99-4692-writable-cte/` — proposal,
  design, tasks, spec.

## Verification

| Test | Result |
|---|---|
| `repro_v313_99_4692_writable_cte` | 6/6 PASS |
| `cte_materialization_test` | 9/9 PASS (no regression) |
| `parser_e2e_test` | 249/249 PASS (no regression) |
| `rollup_cube_test` | 14/14 PASS (no regression) |
| `repro_v312_93_cte_values_anchor` | 4/4 PASS (no regression) |
| `repro_v313_96_4717_insert_cte_subquery` | 3/3 PASS (no regression) |
| `repro_v313_97_4679_grouping_sets` | 3/3 PASS (no regression) |
| `repro_v313_98_4701_multi_select` | 3/3 PASS (no regression) |
| `cargo check --workspace --all-features` | clean |
| `openspec validate --strict` | valid |

## Non-Goals

- Engine-side execution of writable CTEs.
- Engine-side materialisation of MATERIALIZED VIEWs.
- Engine-side `RETURNING` projection.