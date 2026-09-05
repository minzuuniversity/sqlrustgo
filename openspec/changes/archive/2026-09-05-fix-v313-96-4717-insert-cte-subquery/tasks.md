## 1. Code change

- [x] 1.1 In `crates/parser/src/parser.rs`, extend `parse_select_statement`
      so a leading `Token::With` delegates to `parse_with_select` and
      returns the unwrapped `WithSelect.select`.
- [x] 1.2 Reject any non-`WithSelect` return (`WithDml`) from
      `parse_with_select` with an explicit error.
- [x] 1.3 Run `openspec validate fix-v313-96-4717-insert-cte-subquery`.

## 2. Test

- [x] 2.1 New file
      `tests/integration/sql/repro_v313_96_4717_insert_cte_subquery.rs`
      with three tests: recursive INSERT FROM WITH, non-recursive INSERT
      FROM WITH, and SELECT FROM WITH RECURSIVE.
- [x] 2.2 Register the new test target in `Cargo.toml`.
- [x] 2.3 Run
      `cargo test --test repro_v313_96_4717_insert_cte_subquery` — 3/3
      pass.
- [x] 2.4 Run
      `cargo test --test cte_materialization_test --test parser_e2e_test
      --test repro_v312_93_cte_values_anchor` — 9/9, 249/249, 4/4 pass
      (no regression).

## 3. Documentation

- [x] 3.1 `CHANGELOG.md` entry under v3.13.0 follow-up (deferred per
       task convention; added to v3.13 release workflow).

## 4. Commit

- [x] 4.1 Commit message:
      `fix(v313-96 / #4717): FROM subquery accepts WITH [RECURSIVE] prefix`
- [ ] 4.2 Run `openspec archive fix-v313-96-4717-insert-cte-subquery`
      after merge.