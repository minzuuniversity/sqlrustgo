## ADDED Requirements

### Requirement: ROLLBACK Undoes In-Memory DML

The `ExecutionEngine<MemoryStorage>` backend MUST revert all DML operations (INSERT, UPDATE, DELETE) performed inside a `BEGIN ... ROLLBACK` block, restoring the table state to what it was at the moment of `BEGIN`. The reverted state MUST be observable by subsequent `SELECT` queries issued after the `ROLLBACK`.

#### Scenario: ROLLBACK undoes inserted rows
- **WHEN** a table `t` contains one row `(v=1)` before `BEGIN`
- **AND** `BEGIN; INSERT INTO t VALUES (2),(3); ROLLBACK;` is executed
- **THEN** `SELECT COUNT(*) FROM t` returns `1`
- **AND** `SELECT v FROM t` returns `[1]`

#### Scenario: ROLLBACK undoes mixed UPDATE + DELETE
- **WHEN** table `t(id, v)` contains rows `(1,10),(2,20)` before `BEGIN`
- **AND** `BEGIN; UPDATE t SET v=999 WHERE id=1; DELETE FROM t WHERE id=2; ROLLBACK;` is executed
- **THEN** `SELECT id, v FROM t ORDER BY id` returns `(1,10)` then `(2,20)`
- **AND** the original row values are preserved with no partial application

#### Scenario: COMMIT persists DML across rollback boundary
- **WHEN** `BEGIN; INSERT INTO t VALUES (1),(2),(3); COMMIT;` is executed
- **THEN** subsequent queries see all 3 rows
- **AND** no `ROLLBACK` issued after `COMMIT` affects them

### Requirement: MemoryStorage Transaction Boundaries Are Bracketed

`BEGIN` and `COMMIT` / `ROLLBACK` MUST form exclusive transactional brackets on the in-memory backend. Nested `BEGIN` within an active transaction MUST error. A `COMMIT` or `ROLLBACK` without a preceding `BEGIN` MUST error. After `COMMIT` or `ROLLBACK`, the storage MUST be in a clean state ready to accept another `BEGIN`.

#### Scenario: BEGIN without matching COMMIT/ROLLBACK does not leak
- **WHEN** `BEGIN; INSERT INTO t VALUES (1);` is followed by a new `BEGIN` without an intervening `COMMIT` or `ROLLBACK`
- **THEN** the second `BEGIN` returns an error
- **AND** no partial transaction state is observable from a fresh `SELECT` query (since no commit/rollback was issued, the inserted row is held in the transaction log only)

#### Scenario: COMMIT then BEGIN starts a fresh transaction
- **WHEN** `BEGIN; INSERT INTO t VALUES (1); COMMIT;` has run
- **AND** `BEGIN; INSERT INTO t VALUES (2); ROLLBACK;` follows
- **THEN** only the row from the first transaction (value `1`) is visible after both blocks

### Requirement: Test Harness Compatible With parking_lot RwLock

The DML integration test harness (`tests/dml_integration_test.rs`) MUST construct `ExecutionEngine<MemoryStorage>` using the same lock type that `ExecutionEngine::new` accepts — currently `parking_lot::RwLock`. The test file MUST compile cleanly and run all its tests with no `RwLock` type-mismatch errors.

#### Scenario: cargo test dml_integration_test compiles
- **WHEN** `cargo test --test dml_integration_test --no-run` is run on a clean checkout
- **THEN** the test binary builds with zero compile errors
- **AND** no `E0308` mismatched-types errors mention `RwLock`

#### Scenario: All dml_integration_test cases pass
- **WHEN** `cargo test --test dml_integration_test` is run after compilation succeeds
- **THEN** all 18 tests in the file pass (or, at minimum, the 5 named in issue #3724 pass with no others regressing)

### Requirement: Audit Script Reproduces Test Run

A shell script at `audit/check_c3_rollback.sh` MUST run the 5 named C-3a/C-3b tests and append their output to `audit/c3-acid-rollback.log`. The script MUST exit non-zero if any test fails.

#### Scenario: check_c3_rollback.sh passes on a healthy checkout
- **WHEN** the script is run after the test-file fix is applied
- **THEN** it exits with status 0
- **AND** `audit/c3-acid-rollback.log` contains one line per test with its pass/fail status

#### Scenario: check_c3_rollback.sh fails loud on a regression
- **WHEN** the test file is broken (e.g., a future change reverts to `std::sync::RwLock`)
- **THEN** the script exits non-zero
- **AND** the log records the failing test name and stderr