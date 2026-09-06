## Context

Issue #4701 sub-problem 1: `CREATE INDEX ON t(CASE WHEN ... END)` fails
with `No columns in index`. The parser's column list only accepts
identifiers and silently breaks out of the loop on any other token,
so the executor sees an empty column list.

## Approach

Introduce a typed `IndexColumnSpec` value that captures either a
column name or an expression. The parser recognises both forms in
the column list. Storage `IndexInfo` and catalog `IndexInfo` mirror
the same shape, with a serde representation that stays compatible
with previously-persisted JSON (the new `expression` field is
`#[serde(default)]` so old rows still load, and `name` carries the
original string content).

## Files Changed

- `crates/parser/src/parser.rs` — new `IndexColumnSpec` struct,
  `parse_index_column_list()` helper, `parse_create_index` uses
  it, `CreateIndexStatement.columns` type changes.
- `crates/storage/src/engine.rs` — `IndexInfo.columns` type changes
  to `Vec<IndexColumnSpec>` (with serde default on `expression`).
- `crates/catalog/src/index.rs` — same change for the catalog's
  copy of `IndexInfo`.
- 4-5 sites that construct `IndexInfo` — `execution_engine.rs`,
  `crates/server/src/openclaw_endpoints.rs`,
  `crates/storage/src/{append_only_storage,table_level_storage}.rs`:
  pass a `Vec<IndexColumnSpec>` instead of `Vec<String>`. For
  identifier-only callers the conversion is mechanical
  (`vec![IndexColumnSpec::column("name")]`).
- `tests/integration/sql/repro_v313_100_4701_expression_index.rs`
  — 3 new tests.
- `Cargo.toml` — register the new test target.
- `openspec/changes/fix-v313-100-4701-expression-index/` — proposal,
  design, tasks, spec.

## Verification

| Test | Result |
|---|---|
| `repro_v313_100_4701_expression_index` | 3/3 PASS |
| `cte_materialization_test` | 9/9 PASS (no regression) |
| `parser_e2e_test` | 249/249 PASS (no regression) |
| `rollup_cube_test` | 14/14 PASS (no regression) |
| previous issue repro suites (4704-1, 4697, 4717, 4679, 4701-sub2, 4692) | all pass (no regression) |
| `cargo check --workspace --all-features` | clean |
| `openspec validate --strict` | valid |

## Non-Goals

- Storage-side expression evaluation (per-row, on INSERT/UPDATE).
- Index selection during query planning.
- Insert-time validation that the expression references only
  columns in the target table.