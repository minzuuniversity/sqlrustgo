# V312-F-1 Spec: COM_RESET_CONNECTION Packet Sequence Reset

## MODIFIED Requirements

### Requirement: COM_RESET_CONNECTION MUST reset packet sequence

After the server handles `COM_RESET_CONNECTION` (0x1F), it MUST reset
its packet sequence counter so that the next inbound packet from the
client has sequence=0 (server's expectation).

#### Scenario: Reset followed by SELECT returns OK
- **WHEN** a client sends `COM_RESET_CONNECTION` then immediately `COM_QUERY "SELECT 1"`
- **THEN** the server returns an OK packet for the reset
- **AND** returns a ResultSet packet for the SELECT
- **AND** no protocol sequence error is raised

### Requirement: typed-wrappers integration test gate MUST yield 22 passed

The typed-wrappers integration test `cargo test --test v312_13_typed_wrappers_test -- --test-threads=1` MUST pass 22/22 tests including `v312_13_reset_connection_ok`.

#### Scenario: typed-wrappers test fully green
- **WHEN** the test runs after the fix is applied
- **THEN** all 22 tests pass with 0 failures

### Requirement: Gate check_v312_13_wire_load_data.sh step 02 MUST PASS

The wire+load data gate step 02 (typed-wrappers) MUST report status=pass.

#### Scenario: gate step 02 status=pass
- **WHEN** `bash scripts/gate/check_v312_13_wire_load_data.sh` runs
- **THEN** step 02 reports status=pass