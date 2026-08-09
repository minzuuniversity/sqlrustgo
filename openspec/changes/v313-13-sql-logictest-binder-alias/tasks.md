# v313-13 — SQL Logic Test: Binder / Alias Resolution Fix

**owner:** openclaw | **expiry:** v3.13.0 GA

## 1. Pre-flight

- [ ] `git branch` — confirm on `v313` release branch
- [ ] `git status` — working tree clean, no uncommitted changes
- [ ] `cargo build --all-features` — project compiles without errors

## 2. Analysis

- [ ] Read the `.test` logic test file in this directory and identify every failing query
- [ ] For each failure: trace back to the root cause
  - Is it an incorrect column alias resolution in SELECT?
  - Is it a table alias not being resolved in JOINs?
  - Is it an ORDER BY referencing a column number or alias incorrectly?
  - Is it a name collision or shadowing issue?
  - Is it a missing fallback to column position when name lookup fails?
- [ ] List the specific Rust functions/structures implicated

## 3. Implementation

- [ ] Fix the identified bug(s) in `src/binder/` or `src/semantic/` alias resolution
- [ ] Add or update unit tests for the corrected path
- [ ] Ensure no other existing tests regress (`cargo test --all-features`)

## 4. Validation

- [ ] Run the logic test file: `cargo run --bin sqlrustgo -- <path/to/v313-13.test>` or the harness command
- [ ] Confirm all queries in the `.test` file now return the expected results
- [ ] Run `cargo clippy --all-features -- -D warnings` — no new lints

## 5. Commit

- [ ] `git add -A`
- [ ] `git commit -m "fix(v313-13): resolve binder/alias resolution logic test failures

See openspec/changes/v313-13-sql-logictest-binder-alias/"`
- [ ] `git push`
