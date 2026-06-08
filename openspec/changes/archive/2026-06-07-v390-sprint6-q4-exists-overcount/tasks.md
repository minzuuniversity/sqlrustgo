## 1. Set up Sprint 6 worktree

- [ ] 1.1 Create worktree from develop/v3.9.0 head (`5576bb0f23c5` post-#3304 merge). Use tarball API.
- [ ] 1.2 Branch: `fix/3281-q4-exists-overcount`. Init git + fetch + reset --hard to FETCH_HEAD.

## 2. Write reproduction tests (RED expected pre-fix)

- [ ] 2.1 Add `tests/repro_3281_q4_exists_test.rs` with 3 tests:
  - [ ] 2.1.1 `repro_3281_minimal_exists_evaluates_per_outer_row` — outer table with 4 rows, inner table with 1 matching row. `EXISTS(SELECT 1 FROM inner WHERE inner.x = outer.x)` should return true for the matching outer row and false for the 3 non-matching. Total count should be 1, not 4.
  - [ ] 2.1.2 `repro_3281_tpch_q4_counts_match_postgres` — load SF=0.001 fixture, run Q4 simplified, assert 5 rows with counts matching PG `[27, 23, 29, 23, 16]`.
  - [ ] 2.1.3 `repro_3281_q13_q16_no_regression` — Q13 and Q16 cell values match existing 4-way JSON.
- [ ] 2.2 Register in `Cargo.toml`.
- [ ] 2.3 Run `cargo test --test repro_3281_q4_exists` — confirm **at least 1 fails** pre-fix (RED state).

## 3. Locate the EXISTS bug

- [ ] 3.1 Read `src/engine_utils.rs:215` — confirm `Exists(_) => true` is the conservative fallback.
- [ ] 3.2 Find `pre_eval_exists_subquery_fast` (commit f5072d99f) — see if it's wired into the Q4 evaluation path or bypassed.
- [ ] 3.3 Trace Q4 evaluation: WHERE clause → EXISTS expression → conservative `=> true` → what does the count actually count?
- [ ] 3.4 Determine the 4x factor: is it Cartesian product, or something else?

## 4. Apply the fix

- [ ] 4.1 Either:
  - (a) Route Q4's EXISTS to the fast path evaluator, OR
  - (b) Replace the conservative `=> true` with a real correlated-subquery execution that runs the inner SELECT with outer-row column values substituted.
- [ ] 4.2 If the fast path is N²: keep as-is for this PR (correctness first, perf later).
- [ ] 4.3 If a new function is needed, add it next to `pre_eval_exists_subquery_fast` with explicit documentation.

## 5. Verify GREEN

- [ ] 5.1 Run `cargo test --test repro_3281_q4_exists` — **3/3 PASS**.
- [ ] 5.2 Run `cargo test --test repro_3282_orderby_desc --test repro_3285_real_preservation --test tpch_bug_regression_test` — confirm **26/26 PASS** (no Sprint 3/4/5 regression).
- [ ] 5.3 Run `cargo test --test tpch_value_test_v2` (with `TPCH_DATA_DIR=tests/data/tpch-sf001`) — measure: did Q4 move from FAIL to PASS? Did Q13/Q16 stay PASS? Total pass rate target: 11/22 (up from 10/22).

## 6. PR + merge

- [ ] 6.1 Commit with message referencing #3281 + 3 PASS + no regression.
- [ ] 6.2 Push branch to Gitea (use embedded credential URL).
- [ ] 6.3 Create PR #N via Gitea API (`head=fix/3281-q4-exists-overcount`, `base=develop/v3.9.0`).
- [ ] 6.4 Merge with `force_merge=true`, `{"do":"merge"}`.
- [ ] 6.5 Verify `develop/v3.9.0` head advances and #3281 auto-closes via "Closes #3281".

## 7. Archive OpenSpec

- [ ] 7.1 Run `openspec validate v390-sprint6-q4-exists-overcount` — must be PASS.
- [ ] 7.2 Run `openspec archive v390-sprint6-q4-exists-overcount -y` — moves change to archive/, adds new spec to `openspec/specs/`.
- [ ] 7.3 Verify `openspec/specs/correlated-exists-subquery-evaluation/spec.md` now exists.
