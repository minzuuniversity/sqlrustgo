## 1. Code change

- [x] 1.1 In `crates/parser/src/parser.rs` `parse_select_statement`,
      inside the existing `Some(Token::LParen)` projection arm
      (no-binary-op branch), add an alias consumer that:
        - matches `Token::As` and consumes it;
        - matches either `Token::Identifier(name)` or
          `Token::Matched` and captures the alias;
        - if an alias was captured, advances the cursor past it.
- [x] 1.2 Run `openspec validate fix-v313-98-4701-scalar-subquery`.

## 2. Test

- [x] 2.1 New file
      `tests/integration/sql/repro_v313_98_4701_multi_select.rs` with
      three tests: `insert_select_subquery_with_limit_offset_parses`
      (issue body example),
      `select_subquery_with_alias_parses`,
      `select_subquery_no_alias_parses`.
- [x] 2.2 Register the new test target in `Cargo.toml`.
- [x] 2.3 Run
      `cargo test --test repro_v313_98_4701_multi_select` — 3/3 pass.
- [x] 2.4 Run rollup/cube + cte_materialization + parser_e2e +
      repro_v312_93 + repro_v313_96 + repro_v313_97 — 14/14, 9/9,
      249/249, 4/4, 3/3, 3/3 pass (no regression).

## 3. Documentation

- [x] 3.1 `CHANGELOG.md` entry under v3.13.0 follow-up (deferred per
       task convention; added to v3.13 release workflow).

## 4. Commit

- [x] 4.1 Commit message:
      `fix(v313-98 / #4701): parser accepts scalar subquery + keyword alias in projection`
- [ ] 4.2 Run `openspec archive fix-v313-98-4701-scalar-subquery`
      after merge.