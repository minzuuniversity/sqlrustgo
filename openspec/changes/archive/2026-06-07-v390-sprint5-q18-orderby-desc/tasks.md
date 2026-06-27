## 1. Set up Sprint 5 worktree

- [ ] 1.1 Create worktree from develop/v3.9.0 head (`3c2c8136a541` post-#3303 merge). Use tarball API to avoid smart-HTTP intermittency.
- [ ] 1.2 Branch: `fix/3282-q18-orderby-desc`. Init git + fetch + reset --hard to FETCH_HEAD.

## 2. Write 3 reproduction tests (must FAIL pre-fix)

- [ ] 2.1 Add `tests/repro_3282_orderby_desc_test.rs` with 3 tests:
  - [ ] 2.1.1 `repro_3282_parser_preserves_desc_direction` — parses `ORDER BY x DESC` and asserts `OrderByExpr.direction == SortDirection::Desc`.
  - [ ] 2.1.2 `repro_3282_engine_sorts_desc_when_desc` — executes on 3-row table, asserts row order is [5, 3, 1] (descending).
  - [ ] 2.1.3 `repro_3282_tpch_q18_top_customer` — loads SF=0.001 fixture, runs Q18, asserts top-1 `o_totalprice` is the max in fixture (not min).
- [ ] 2.2 Register in `Cargo.toml` as `[[test]]` entry.
- [ ] 2.3 Run `cargo test --test repro_3282_orderby_desc` — confirm **at least one fails** pre-fix (RED state).

## 3. Locate the bug in parser + engine

- [ ] 3.1 Read `crates/parser/src/ast.rs` — find `OrderByExpr` struct, check if `direction` field exists.
- [ ] 3.2 Read `crates/parser/src/parser.rs` — find `ORDER BY` parse code, see if `DESC` keyword is consumed.
- [ ] 3.3 Read `src/engine_select.rs` sort code — see if it reads direction or always sorts ASC.
- [ ] 3.4 Note exact lines + commit to patch in task 4.

## 4. Apply the fix (minimum change)

- [ ] 4.1 Patch parser AST to add `direction: SortDirection` field if missing.
- [ ] 4.2 Patch parser to bind `direction` from `DESC`/`ASC` token (default Asc).
- [ ] 4.3 Patch `engine_select.rs` sort comparator to read `direction` and use `<` or `>` accordingly.
- [ ] 4.4 Keep change as small as possible: target ~5-15 lines total.

## 5. Verify GREEN

- [ ] 5.1 Run `cargo test --test repro_3282_orderby_desc` — **3/3 PASS**.
- [ ] 5.2 Run `cargo test --test repro_3285_real_preservation --test tpch_bug_regression_test` — confirm no regression (**9/9 PASS**).
- [ ] 5.3 Run `cargo test --test tpch_value_test_v2` (with `TPCH_DATA_DIR=tests/data/tpch-sf001`) — measure Q18 specifically: did it move from FAIL to PASS, or row 0 col 0 = `Customer#000000009`?

## 6. PR + merge

- [ ] 6.1 Commit with message referencing #3282 + 3 PASS + no regression.
- [ ] 6.2 Push branch to Gitea (use embedded credential URL).
- [ ] 6.3 Create PR #N via Gitea API (`head=fix/3282-q18-orderby-desc`, `base=develop/v3.9.0`).
- [ ] 6.4 Merge with `force_merge=true`, `{"do":"merge"}`.
- [ ] 6.5 Verify `develop/v3.9.0` head advances and #3282 auto-closes via "Closes #3282".

## 7. Archive OpenSpec

- [ ] 7.1 Run `openspec validate v390-sprint5-q18-orderby-desc` — must be PASS.
- [ ] 7.2 Run `openspec archive v390-sprint5-q18-orderby-desc -y` — moves change to archive/, adds new spec to `openspec/specs/`.
- [ ] 7.3 Verify `openspec/specs/order-by-desc-honoring/spec.md` now exists.
- [ ] 7.4 Optional: git commit the OpenSpec changes (OpenSpec archive does NOT auto-commit).
