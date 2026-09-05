## 1. Code change

- [x] 1.1 In `crates/parser/src/parser.rs` add a `GeneratedColumn`
      struct (`expression: String`, `stored: bool`).
- [x] 1.2 Add `generated: Option<GeneratedColumn>` field to
      `ColumnDefinition` with `#[serde(default)]`. Add `Default` to
      the existing derive list.
- [x] 1.3 In `parse_column_definition`, declare
      `let mut generated: Option<GeneratedColumn> = None;` next to the
      other locals. Add a `match` arm consuming
      `GENERATED ALWAYS AS (<expr>) [STORED|VIRTUAL]`. Include
      `generated` in the returned struct literal.
- [x] 1.4 Run `openspec validate fix-v313-95-4697-generated-columns`.

## 2. Test

- [x] 2.1 Tighten existing `test_parse_generated_column` and
      `test_parse_generated_column_stored` with `assert!(result.is_ok())`.
- [x] 2.2 Add `test_parse_generated_column_virtual` for the
      `... GENERATED ALWAYS AS (a + b) VIRTUAL` case.
- [x] 2.3 Add `test_parse_generated_column_complex_expr` matching the
      issue body (`CREATE TABLE gc(a int, b int, c int GENERATED ALWAYS
      AS (a + b) STORED)`).
- [x] 2.4 Run `cargo test -p sqlrustgo-parser --test parser_coverage_tests
      test_parse_generated_column` — all 4 pass.
- [x] 2.5 Run `cargo test --test cte_materialization_test --test
      parser_e2e_test` — no regression (9/9, 249/249).

## 3. Documentation

- [x] 3.1 `CHANGELOG.md` entry under v3.13.0 follow-up (deferred to v3.13
      release workflow, not per-PR).

## 4. Commit

- [x] 4.1 Commit message:
      `fix(v313-95 / #4697): CREATE TABLE accepts GENERATED ALWAYS AS clause`
- [ ] 4.2 Run `openspec archive fix-v313-95-4697-generated-columns`
      after merge.