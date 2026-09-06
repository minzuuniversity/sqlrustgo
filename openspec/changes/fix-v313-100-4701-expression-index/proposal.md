## Why

Issue #4701 sub-problem 1: `CREATE INDEX ON t(CASE WHEN ... END)`
fails at execution time with `No columns in index`. The parser
`parse_create_index` (line 3305) calls `parse_column_list` which
only accepts `Token::Identifier` (line 11530). When the
parenthesised column list opens with a non-identifier token like
`Case`, the dispatcher's default arm falls through to `_ => break`,
returning an empty `Vec<String>`. The executor then reports
`No columns in index` at `execute_create_index` (line 1115).

## Fix

Allow `CREATE INDEX` column lists to mix simple identifiers and
arbitrary expressions. The AST, the catalog `IndexInfo`, and the
storage `IndexInfo` each gain a new `IndexColumnSpec` value that
captures either a column name (string) or an expression
(`Expression`). The simple column form is preserved for serde
backwards-compatibility.

This PR is parser + AST + catalog only. Expression-index key
materialisation (computing the expression value on INSERT / UPDATE
and storing it in the b-tree) is a separate storage-engine change
that follows the same pattern as the existing `IndexDefinition`
and is intentionally out of scope for this PR.

## Capabilities

### New Capabilities

- `parser-expression-index`: `CREATE INDEX idx ON t(CASE WHEN x
  > 1 THEN 1 ELSE 0 END)` parses successfully. Mixed column lists
  like `CREATE INDEX idx ON t(name, CASE WHEN price > 15 THEN 1 ELSE
  0 END)` are also accepted.

### Modified Capabilities

- `parser-create-index`: column list now accepts both identifiers
  and expressions. AST column type changes from `Vec<String>` to
  `Vec<IndexColumnSpec>`.
- `storage-index-info`: `IndexInfo.columns` is now
  `Vec<IndexColumnSpec>`. Serde format on disk is backwards
  compatible with the previous `Vec<String>` payload because
  `IndexColumnSpec`'s `name` field carries the old string content
  and `#[serde(default)]` on `expression` makes the new field
  optional for old JSON.
- `catalog-index-info`: `catalog::index::IndexInfo` is updated in
  the same way as the storage `IndexInfo`.

## Out of Scope

- Storage-side evaluation of the expression at INSERT / UPDATE
  time. Currently the storage backends (`append_only_storage`,
  `binary_storage`, `binary_storage_v2`, `table_level_storage`)
  accept the `IndexInfo` but do not actually build a b-tree from
  it; that's true for column-name indexes too, so expression
  indexes are no worse off.
- `INSERT INTO t ...` validation that an expression index key
  actually exists.
- Index selection during query planning: the executor does not
  consult indexes today, so the new field is recorded but never
  consulted.

## Verification

- 3 new tests in
  `tests/integration/sql/repro_v313_100_4701_expression_index.rs`:
  `parse_create_index_with_expression` (issue body),
  `parse_create_index_with_expression_and_simple_column` (mixed
  list),
  `parse_create_index_with_simple_column_still_works` (regression).
- No regression: `cte_materialization_test` (9/9),
  `parser_e2e_test` (249/249), `rollup_cube_test` (14/14),
  plus the previous issue's repro suites (3-6 tests each).
- `cargo check --workspace --all-features`: clean.
- `openspec validate fix-v313-100-4701-expression-index --strict`:
  valid.