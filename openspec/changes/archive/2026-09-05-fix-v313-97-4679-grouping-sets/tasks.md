## 1. Code change

- [x] 1.1 In `crates/parser/src/parser.rs`, add
      `SelectStatement.grouping_sets: Vec<Vec<Expression>>` field.
- [x] 1.2 In the GROUP BY branch of `parse_select_statement`, detect
      `GROUPING SETS ((...), (...), ...)`. The set columns are unioned
      (deduped) into the `group_by` field so the main aggregation
      pass groups by every column that appears in any set.
- [x] 1.3 Patch all `SelectStatement { ... }` construction sites in
      parser.rs to include `grouping_sets: vec![]` (or its real value
      when propagated). Five sites use `..Default::default()` to inherit
      the rest; the rest use explicit fields including `grouping_sets`.
- [x] 1.4 In `src/execution_engine.rs`, add `grouping_sets:
      select.grouping_sets.clone()` in the SelectStatement construction.
- [x] 1.5 Run `openspec validate fix-v313-97-4679-grouping-sets`.

## 2. Test

- [x] 2.1 New file
      `tests/integration/sql/repro_v313_97_4679_grouping_sets.rs` with
      three tests: `grouping_sets_with_grand_total`,
      `grouping_sets_empty_alone`,
      `grouping_sets_single_column_is_subset`.
- [x] 2.2 Register the new test target in `Cargo.toml`.
- [x] 2.3 Run
      `cargo test --test repro_v313_97_4679_grouping_sets` — 3/3 pass.
- [x] 2.4 Run rollup/cube + cte_materialization + parser_e2e +
      repro_v312_93 + repro_v313_96 — 14/14, 9/9, 249/249, 4/4,
      3/3 pass (no regression).

## 3. Documentation

- [x] 3.1 `CHANGELOG.md` entry under v3.13.0 follow-up (deferred per
       task convention; added to v3.13 release workflow).

## 4. Commit

- [x] 4.1 Commit message:
      `fix(v313-97 / #4679): GROUP BY GROUPING SETS((...),(...),...) support`
- [ ] 4.2 Run `openspec archive fix-v313-97-4679-grouping-sets`
      after merge.