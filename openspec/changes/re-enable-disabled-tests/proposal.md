## Why

v3.10.0 GA disabled 22 integration tests due to API drift from refactoring. These tests cover critical paths (storage, executor, WAL, parser, optimizer, transaction) and cannot be re-enabled without updating them to match the current API surface. Without these tests, the main crate coverage sits at 14.71% — far below the v3.11.0 target of 80%. Re-enabling these tests is the single highest-leverage action for closing the coverage gap.

## What Changes

- Rewrite 22 disabled test files to use current APIs:
  - `execute(Statement)` → `execute(&str)` pattern
  - `WalManager::new(...)` → `LegacyWalManager::new(...)` pattern
  - `WalWriter::log_begin/log_insert/log_commit` → `WalWriter::append(&WalEntry)` pattern
  - `storage.write().map_err(...)` → `storage.write()` (parking_lot)
  - `insert.on_duplicate` → `insert.on_duplicate_key_update`
  - `insert.replace` → `insert.is_replace`
  - `Value::Date`/`Value::Timestamp` → replacement value representation
  - `IndexScanExec::new(...)` → updated constructor signature
  - Removed APIs → equivalent alternatives or `#![cfg(any())]` removal
- Remove `#![cfg(any())]` guard after each file is fixed
- Verify each test compiles and passes individually
- Run full `cargo test -p sqlrustgo --all-features` to confirm no regressions
- Run `cargo llvm-cov -p sqlrustgo --lib --tests` to measure coverage impact

## Capabilities

### New Capabilities
- `re-enabled-test-files`: Track which test files are re-enabled and their fix patterns

### Modified Capabilities
- *(none — no spec-level behavior changes, only test code updates)*

## Impact

- **22 test files** in `tests/` directory tree (anomaly, integration, e2e, stress)
- **Cargo.toml**: no changes (tests already defined, just need to compile)
- **No production code changes**: test-only API migration
- **No new dependencies**: all APIs already exist in current codebase
- **Coverage**: expected to significantly improve the main crate's 14.71% line coverage
