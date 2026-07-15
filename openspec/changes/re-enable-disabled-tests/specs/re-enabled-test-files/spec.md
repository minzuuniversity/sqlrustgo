## ADDED Requirements

### Requirement: Disabled tests shall compile with current API

Each previously disabled test file SHALL compile and pass after updating its API calls to match the current codebase. The test SHALL verify the same contracts as before — only the call syntax changes.

#### Scenario: Test compiles after API migration
- **WHEN** the `#![cfg(any())]` guard is removed
- **THEN** `cargo test --test <test_name> --no-run` SHALL succeed

#### Scenario: Test passes at runtime
- **WHEN** `cargo test --test <test_name>` is run
- **THEN** all test functions in the file SHALL pass (not just compile)

### Requirement: Fix patterns shall follow established conventions

API migration SHALL use the same fix patterns already validated in the v3.10.0 test compilation fixes. Each pattern type SHALL be applied consistently across all affected test files.

#### Scenario: execute changes use &str
- **WHEN** a test calls `execute(parse("...").unwrap())`
- **THEN** it SHALL be replaced with `execute("...")`

#### Scenario: WalManager changes use LegacyWalManager
- **WHEN** a test calls `WalManager::new(...)`
- **THEN** it SHALL use `LegacyWalManager::new(...)` with `use crate::wal::LegacyWalManager`

#### Scenario: WalWriter changes use append
- **WHEN** a test calls `log_begin()`/`log_insert()`/`log_commit()`
- **THEN** it SHALL use `WalWriter::append(&WalEntry)` with appropriate WalEntry structs

#### Scenario: parking_lot RwLock changes
- **WHEN** a test calls `.write().map_err(...)` or `.read().map_err(...)`
- **THEN** the `.map_err(...)` SHALL be removed (parking_lot::RwLock doesn't poison)
