## 1. Fix Test Compilation

- [x] 1.1 In `tests/dml_integration_test.rs`, replace `use std::sync::{Arc, RwLock};` with `use std::sync::Arc;` and add `use parking_lot::RwLock;`
- [x] 1.2 Verify `cargo test --test dml_integration_test --no-run` exits 0 with no E0308 errors

## 2. Run Target Tests for C-3a/C-3b

- [x] 2.1 Run `cargo test --test dml_integration_test transaction_rollback_undoes_dml` and confirm PASS
- [x] 2.2 Run `cargo test --test dml_integration_test transaction_update_then_rollback` and confirm PASS
- [x] 2.3 Run `cargo test --test dml_integration_test transaction_commit_persists_dml` (regression guard) and confirm PASS
- [x] 2.4 Run `cargo test --test dml_integration_test transaction_delete_then_commit` (regression guard) and confirm PASS
- [x] 2.5 Run `cargo test --test dml_integration_test failed_insert_does_not_corrupt_table` (regression guard) and confirm PASS
- [x] 2.6 Run the full test file `cargo test --test dml_integration_test` and verify no other test regressed (24/24 PASS)

## 3. Audit Script and Gate Evidence

- [x] 3.1 Create `audit/check_c3_rollback.sh` that runs the 5 named tests and appends results to `audit/c3-acid-rollback.log`
- [x] 3.2 Make the script executable (`chmod +x`)
- [x] 3.3 Run the script and confirm `audit/c3-acid-rollback.log` contains 5 PASS lines (RESULT: PASS 5/5)

## 4. Hygiene Gates

- [x] 4.1 Run `cargo fmt --check --all` and confirm clean (after `cargo fmt --all` reordered imports)
- [x] 4.2 Run `cargo clippy --lib -p sqlrustgo --all-features -- -D warnings` — clean for changed files. Note: 2 pre-existing clippy errors in `crates/mysql-server/src/lib.rs` (UserStore private interface, manual_div_ceil) are unrelated to this change and exist on `develop/v3.10.0` HEAD.

## 5. Documentation and Issue Closure

- [x] 5.1 Commit the change with message referencing #3724 and the openspec change name
- [x] 5.2 Post a comment on issue #3724 linking to the commit and the audit log
- [x] 5.3 Verify follow-up issue #3738 (C-3c trigger-in-tx) is documented as a separate work item (created as Gitea issue #3738)