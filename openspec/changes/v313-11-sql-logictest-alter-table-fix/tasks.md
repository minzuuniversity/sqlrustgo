# v313-11 — SQL Logic Test: ALTER TABLE Fix

**owner:** openclaw | **expiry:** v3.13.0 GA

## 1. Pre-flight

- [ ] `git branch` — confirm on `v313` release branch
- [ ] `git status` — working tree clean, no uncommitted changes
- [ ] `cargo build --all-features` — project compiles without errors

## 2. Analysis

- [ ] Read the `.test` logic test file in this directory and identify every failing query
- [ ] For each failure: trace back to the root cause
  - Is it an incorrect column add/drop/m rename implementation?
  - Is it a type or default value handling error?
  - Is it a catalog/metadata update failure?
  - Is it a case-sensitivity issue with column names?
- [ ] List the specific Rust functions/structures implicated

## 3. Implementation

- [ ] Fix the identified bug(s) in `src/executor/` or catalog layer for ALTER TABLE
- [ ] Add or update unit tests for the corrected path
- [ ] Ensure no other existing tests regress (`cargo test --all-features`)

## 4. Validation

- [ ] Run the logic test file: `cargo run --bin sqlrustgo -- <path/to/v313-11.test>` or the harness command
- [ ] Confirm all queries in the `.test` file now return the expected results
- [ ] Run `cargo clippy --all-features -- -D warnings` — no new lints

## 5. Commit

- [ ] `git add -A`
- [ ] `git commit -m "fix(v313-11): resolve ALTER TABLE logic test failures

See openspec/changes/v313-11-sql-logictest-alter-table-fix/"`
- [ ] `git push`
