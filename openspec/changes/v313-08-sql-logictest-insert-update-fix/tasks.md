# v313-08 — SQL Logic Test: INSERT/UPDATE Fix

**owner:** openclaw | **expiry:** v3.13.0 GA

## 1. Pre-flight

- [ ] `git branch` — confirm on `v313` release branch
- [ ] `git status` — working tree clean, no uncommitted changes
- [ ] `cargo build --all-features` — project compiles without errors

## 2. Analysis

- [ ] Read the `.test` logic test file in this directory and identify every failing query
- [ ] For each failure: trace back to the root cause in the executor or semantic layer
  - Is it a NULL value handling bug in INSERT?
  - Is it an incorrect column binding in UPDATE?
  - Is it a type mismatch in the expression evaluation?
- [ ] List the specific Rust functions/structures implicated

## 3. Implementation

- [ ] Fix the identified bug(s) in `src/executor/` or `src/parser/` / `src/semantic/`
- [ ] Add or update unit tests for the corrected path
- [ ] Ensure no other existing tests regress (`cargo test --all-features`)

## 4. Validation

- [ ] Run the logic test file: `cargo run --bin sqlrustgo -- <path/to/v313-08.test>` or the harness command
- [ ] Confirm all queries in the `.test` file now return the expected results
- [ ] Run `cargo clippy --all-features -- -D warnings` — no new lints

## 5. Commit

- [ ] `git add -A`
- [ ] `git commit -m "fix(v313-08): resolve INSERT/UPDATE logic test failures

See openspec/changes/v313-08-sql-logictest-insert-update-fix/"`
- [ ] `git push`
