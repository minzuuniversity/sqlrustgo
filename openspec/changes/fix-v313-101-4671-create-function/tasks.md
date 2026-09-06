## 1. Code change

- [x] 1.1 In `crates/parser/src/parser.rs`, `parse_create_function`:
      drop the `AS` gate before the function body so the `BEGIN ...
      END` and single-`RETURN expr` forms are both accepted with or
      without the prefix.
- [x] 1.2 In `parse_create_function`, after `expect(Returns)` peek
      for `Token::Table`; if present, parse a parenthesised column
      list into `CreateFunctionStatement.return_columns`. Otherwise
      fall through to the historical scalar-type match.
- [x] 1.3 Run `openspec validate fix-v313-101-4671-create-function`.

## 2. Test

- [x] 2.1 New file
      `tests/integration/sql/repro_v313_101_4671_create_function.rs`
      with five tests: `parse_create_function_with_begin_end_no_as`
      (issue body f2), `parse_create_function_with_begin_end_with_as`
      (AS prefix regression),
      `parse_create_function_single_expr_still_works` (simple RETURN
      regression), `parse_create_function_with_returns_table` (issue
      body f3),
      `parse_create_function_with_returns_table_and_begin_end` (table
      return + multi-statement body combined).
- [x] 2.2 Register the new test target in `Cargo.toml`.
- [x] 2.3 Run
      `cargo test --test repro_v313_101_4671_create_function` — 5/5
      pass.
- [x] 2.4 Run cte_materialization + parser_e2e + rollup_cube +
      repro_v312_93 + repro_v313_96 + repro_v313_97 + repro_v313_98
      + repro_v313_99 + repro_v313_100 — all pass, no regression.

## 3. Documentation

- [x] 3.1 `CHANGELOG.md` entry under v3.13.0 follow-up (deferred per
       task convention).

## 4. Commit

- [x] 4.1 Commit message:
      `fix(v313-101 / #4671): parser accepts BEGIN/END body and RETURNS TABLE for CREATE FUNCTION`
- [ ] 4.2 Run `openspec archive
      fix-v313-101-4671-create-function` after merge.