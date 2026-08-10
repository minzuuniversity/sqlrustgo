# v313-15 — SQL Logic Test: DuckDB Harness Compatibility Fix

**owner:** openclaw | **expiry:** v3.13.0 GA

## 1. Pre-flight

- [ ] `git branch` — confirm on `v313` release branch
- [ ] `git status` — working tree clean, no uncommitted changes
- [ ] `cargo build --all-features` — project compiles without errors

## 2. Analysis

- [ ] Read the `.test` logic test file in this directory and identify every failing query
- [ ] For each failure: trace back to the root cause
  - Is it a DuckDB-specific SQL dialect feature not supported by the parser?
  - Is it an expected result format mismatch between DuckDB and sqlrustgo?
  - Is it a supported feature producing subtly different output (e.g., float precision, date format)?
  - Is it a missing SQL function that DuckDB provides?
  - Is it a DuckDB-specific hint or directive in the `.test` file not handled by the harness?
- [ ] List the specific Rust functions/structures implicated

## 3. Implementation

- [ ] Fix the identified bug(s) — may span parser, executor, type system, or harness layer
- [ ] If DuckDB produces correct output and sqlrustgo is wrong: fix sqlrustgo
- [ ] If the test file uses unsupported DuckDB syntax: add a valid sqlrustgo-compatible variant
- [ ] Add or update unit tests for the corrected path
- [ ] Ensure no other existing tests regress (`cargo test --all-features`)

## 4. Validation

- [ ] Run the logic test file: `cargo run --bin sqlrustgo -- <path/to/v313-15.test>` or the harness command
- [ ] Confirm all queries in the `.test` file now return the expected results
- [ ] Run `cargo clippy --all-features -- -D warnings` — no new lints

## 5. Commit

- [ ] `git add -A`
- [ ] `git commit -m "fix(v313-15): resolve DuckDB harness compatibility failures

See openspec/changes/v313-15-sql-logictest-duckdb-harness/"`
- [ ] `git push`
