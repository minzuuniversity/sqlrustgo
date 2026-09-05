## 1. Code change

- [x] 1.1 In `crates/parser/src/parser.rs` `parse_with_select`, add
      `Some(Token::Values) =>` arm to the body `match` that:
        - Calls a new helper `parse_values_as_select()` (or inlines the
          logic) to consume the `VALUES (...), (...)` rows
        - Wraps the resulting `Vec<Vec<Expression>>` in a
          `SelectStatement { columns: vec![SelectColumn { name: "*",
          ...}], from_values: ..., ... }` (matching the
          FROM-(VALUES)-as-table pattern already used by
          `parse_table_ref`)
        - Falls through to the existing INSERT/UPDATE/DELETE branch
          via the same `return Ok(Statement::WithSelect(WithSelect { ... }))`
          for the SELECT path.

- [x] 1.2 Run `openspec validate fix-v312-93-4704-cte-values-anchor`.

## 2. Test

- [x] 2.1 New test file
      `tests/integration/sql/repro_v312_93_cte_values_anchor.rs`:
      - `cte_recursive_with_values_anchor`:
        `WITH RECURSIVE walk(n) AS (VALUES (1) UNION ALL SELECT n+1
        FROM walk WHERE n < 5) SELECT * FROM walk;` — expect rows
        1..=5.
      - `cte_non_recursive_with_values`: `WITH t AS (VALUES (1, 'a'),
        (2, 'b')) SELECT * FROM t;` — expect 2 rows.
      - `cte_with_columns_and_values`: `WITH t(id, name) AS (VALUES
        (1, 'a')) SELECT * FROM t;` — expect 1 row with id=1, name='a'.
      - `cte_with_recursive_values_anchor_compound`: 2-column
        recursive CTE with VALUES anchor and compound SELECT body.

- [x] 2.2 Run `cargo test --release -p sqlrustgo --test
       repro_v312_93_cte_values_anchor` — 4/4 pass.

- [x] 2.3 Run `cargo test -p sqlrustgo --test
       cte_materialization_test` and
       `parser_e2e_test` — no regression (9/9, 249/249 pass).

## 3. Documentation

- [x] 3.1 `CHANGELOG.md` entry under v3.13.0 follow-up (deferred until
       v3.13 release — this is parser-only, no behavior change for
       v3.12.0 CLI users). Note: deferred per task spec; CHANGELOG
       is updated by v3.13 release workflow, not per PR.

## 4. Commit

- [x] 4.1 Commit message:
      `fix(v312-93 / #4704-1): CTE anchor accepts VALUES clause`
- [ ] 4.2 Run `openspec archive fix-v312-93-4704-cte-values-anchor`
       after merge.
