## 1. Code change

- [x] 1.1 In `crates/parser/src/parser.rs`, add
      `pub struct IndexColumnSpec { pub name: Option<String>,
      pub expression: Option<Expression> }` with helper constructors.
- [x] 1.2 Change `CreateIndexStatement.columns: Vec<String>` to
      `CreateIndexStatement.columns: Vec<IndexColumnSpec>`.
- [x] 1.3 In `parse_create_index`, replace
      `self.parse_column_list()?` with
      `self.parse_index_column_list()?`.
- [x] 1.4 Add `fn parse_index_column_list() ->
      Result<Vec<IndexColumnSpec>, String>` that accepts either an
      `Identifier` (mapped to `IndexColumnSpec::column(...)`) or any
      other expression-starting token (parsed via `parse_expression`,
      mapped to `IndexColumnSpec::expression(...)`).
- [x] 1.5 In `crates/storage/src/engine.rs` and
      `crates/catalog/src/index.rs`, change `IndexInfo.columns` to
      `Vec<IndexColumnSpec>`. Mark the new `expression` field
      `#[serde(default)]` for backward compatibility.
- [x] 1.6 Update `IndexInfo` construction sites
      (`src/execution_engine.rs`,
      `crates/server/src/openclaw_endpoints.rs`,
      `crates/storage/src/{append_only_storage,table_level_storage}.rs`)
      to pass a `Vec<IndexColumnSpec>`. Pure-column callers wrap each
      name as `IndexColumnSpec::column(name)`.
- [x] 1.7 Run `openspec validate
      fix-v313-100-4701-expression-index`.

## 2. Test

- [x] 2.1 New file
      `tests/integration/sql/repro_v313_100_4701_expression_index.rs`
      with three tests: `parse_create_index_with_expression` (issue
      body), `parse_create_index_with_expression_and_simple_column`
      (mixed list),
      `parse_create_index_with_simple_column_still_works`
      (regression).
- [x] 2.2 Register the new test target in `Cargo.toml`.
- [x] 2.3 Run
      `cargo test --test repro_v313_100_4701_expression_index` — 3/3
      pass.
- [x] 2.4 Run rollup/cube + cte_materialization + parser_e2e +
      previous issue repro suites — no regression.

## 3. Documentation

- [x] 3.1 `CHANGELOG.md` entry under v3.13.0 follow-up (deferred
      per task convention).

## 4. Commit

- [x] 4.1 Commit message:
      `fix(v313-100 / #4701): parser accepts expression index columns`
- [ ] 4.2 Run `openspec archive
      fix-v313-100-4701-expression-index` after merge.