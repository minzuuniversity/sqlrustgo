# v313-12 — SQL Logic Test: Constraint Semantics

**owner:** openclaw | **expiry:** v3.13.0 GA

## 1. Pre-flight

- [ ] `git branch` — confirm on `v313` release branch
- [ ] `git status` — working tree clean, no uncommitted changes
- [ ] `cargo build --all-features` — project compiles without errors

## 2. Analysis

- [ ] Read the `.test` logic test file in this directory and identify every failing query
- [ ] For each failure: trace back to the root cause
  - Is it incorrect NOT NULL constraint enforcement?
  - Is it an incorrect UNIQUE constraint check?
  - Is it an incorrect CHECK constraint evaluation?
  - Is it an incorrect PRIMARY KEY / FOREIGN KEY implementation?
  - Is it a constraint violation error message or error code mismatch?
- [ ] List the specific Rust functions/structures implicated

## 3. Implementation

- [ ] Fix the identified bug(s) in the semantic/catalog constraint validation layer
- [ ] Add or update unit tests for the corrected path
- [ ] Ensure no other existing tests regress (`cargo test --all-features`)

## 4. Validation

- [ ] Run the logic test file: `cargo run --bin sqlrustgo -- <path/to/v313-12.test>` or the harness command
- [ ] Confirm all queries in the `.test` file now return the expected results
- [ ] Run `cargo clippy --all-features -- -D warnings` — no new lints

## 5. Commit

- [ ] `git add -A`
- [ ] `git commit -m "fix(v313-12): resolve constraint semantics logic test failures

See openspec/changes/v313-12-sql-logictest-constraint-semantics/"`
- [ ] `git push`
