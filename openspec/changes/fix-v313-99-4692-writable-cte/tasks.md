## 1. Code change

- [x] 1.1 In `crates/parser/src/parser.rs` `parse_with_clause`, extend
      the subquery dispatch to match `With`, `Insert`, `Replace`,
      `Update`, `Delete`, `Values`. Anything else falls through to
      `parse_select_or_union`.
- [x] 1.2 In `parse_delete`, consume an optional `RETURNING
      <expr-list>` clause after `WHERE`.
- [x] 1.3 In `parse_update`, consume an optional `RETURNING
      <expr-list>` clause after `WHERE`.
- [x] 1.4 In `parse_create`, accept `MATERIALIZED` (case-insensitive
      identifier) when the following token is `View`, dispatching to
      `parse_create_view`.
- [x] 1.5 Run `openspec validate fix-v313-99-4692-writable-cte`.

## 2. Test

- [x] 2.1 New file
      `tests/integration/sql/repro_v313_99_4692_writable_cte.rs` with
      six tests: `cte_with_delete_body_parses`,
      `cte_with_update_body_parses`,
      `cte_with_insert_body_parses`,
      `create_materialized_view_parses`,
      `create_regular_view_still_parses`,
      `create_materialized_alone_does_not_eat_view`.
- [x] 2.2 Register the new test target in `Cargo.toml`.
- [x] 2.3 Run
      `cargo test --test repro_v313_99_4692_writable_cte` — 6/6 pass.
- [x] 2.4 Run cte_materialization + parser_e2e + rollup_cube +
      repro_v312_93 + repro_v313_96 + repro_v313_97 + repro_v313_98
      — 9/9, 249/249, 14/14, 4/4, 3/3, 3/3, 3/3 pass (no
      regression).

## 3. Documentation

- [x] 3.1 `CHANGELOG.md` entry under v3.13.0 follow-up (deferred per
       task convention; added to v3.13 release workflow).

## 4. Commit

- [x] 4.1 Commit message:
      `fix(v313-99 / #4692): parser accepts writable CTE and MATERIALIZED VIEW`
- [ ] 4.2 Run `openspec archive fix-v313-99-4692-writable-cte`
      after merge.