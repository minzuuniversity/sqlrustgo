# v313-10 — SQL Logic Test: ORDER BY / LIMIT Fix

**owner:** openclaw | **expiry:** v3.13.0 GA

## 1. Pre-flight

- [ ] `git branch` — confirm on `v313` release branch
- [ ] `git status` — working tree clean, no uncommitted changes
- [ ] `cargo build --all-features` — project compiles without errors

## 2. Analysis

- [ ] Read the `.test` logic test file in this directory and identify every failing query
- [ ] For each failure: trace back to the root cause
  - Is it incorrect sort key evaluation in ORDER BY?
  - Is it an off-by-one error in LIMIT/OFFSET?
  - Is it a NULL sort ordering issue (NULLS FIRST/LAST)?
  - Is it a type mismatch between ORDER BY expression and selected columns?
- [ ] List the specific Rust functions/structures implicated

## 3. Implementation

- [ ] Fix the identified bug(s) in `src/executor/` sort/limit execution
- [ ] Add or update unit tests for the corrected path
- [ ] Ensure no other existing tests regress (`cargo test --all-features`)

## 4. Validation

- [ ] Run the logic test file: `cargo run --bin sqlrustgo -- <path/to/v313-10.test>` or the harness command
- [ ] Confirm all queries in the `.test` file now return the expected results
- [ ] Run `cargo clippy --all-features -- -D warnings` — no new lints

## 5. Commit

- [ ] `git add -A`
- [ ] `git commit -m "fix(v313-10): resolve ORDER BY/LIMIT logic test failures

See openspec/changes/v313-10-sql-logictest-order-limit-fix/"`
- [ ] `git push`
