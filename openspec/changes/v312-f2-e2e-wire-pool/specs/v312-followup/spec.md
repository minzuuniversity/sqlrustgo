# V312-F-2 Spec: e2e_wire_protocol SERVER_POOL Isolation

## MODIFIED Requirements

### Requirement: e2e_wire_protocol tests MUST be process-isolated

Each test in `crates/mysql-server/tests/e2e_wire_protocol.rs` MUST see
a fresh server state, regardless of test execution order.

#### Scenario: Tests in any order see clean state
- **WHEN** 46 e2e_wire_protocol tests run in any order
- **THEN** each test's row counts match its own setup
- **AND** no test reads rows from a previous test's tables

### Requirement: SERVER_POOL MUST support per-test isolation

The test harness's `SERVER_POOL` MUST provide a way to obtain a
fresh, isolated engine per test, OR drop all tables between tests.

#### Scenario: Per-test isolation via cleanup
- **WHEN** a test begins
- **THEN** all tables from prior tests are absent
- **AND** the test can CREATE/INSERT/DROP its own tables without interference

### Requirement: Gate check_v312_13_wire_load_data.sh step 05 MUST PASS

The wire+load data gate step 05 (e2e_wire_protocol) MUST report status=pass.

#### Scenario: gate step 05 status=pass
- **WHEN** `bash scripts/gate/check_v312_13_wire_load_data.sh` runs
- **THEN** step 05 reports status=pass
- **AND** all 46 e2e_wire_protocol tests pass