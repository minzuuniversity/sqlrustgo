## MODIFIED Requirements

### Requirement: COM_STMT_PREPARE / EXECUTE / CLOSE lifecycle

The `binary-prepared-statement-roundtrip` spec MUST cover an explicit `COM_STMT_CLOSE` round-trip and an error-packet assertion for bad parameter types and parameter count mismatch. The existing prepare + execute + log behavior with substituted SQL is preserved.

#### Scenario: Prepare / execute / close produces no leak

- **GIVEN** an ephemeral server started via `start_ephemeral`
- **WHEN** a test issues `COM_STMT_PREPARE` for `SELECT ?, ?`, then
  `COM_STMT_EXECUTE` with `(1, 'a')`, then `COM_STMT_CLOSE` with the returned
  statement id
- **THEN** the server SHALL respond `OK` to PREPARE, return one row `(1, 'a')`
  on EXECUTE, and SHALL NOT respond to CLOSE (no packet at all)
- **AND** a follow-up EXECUTE with the same statement id SHALL receive an
  `ERR` packet with sqlstate `HY000` and message containing "Unknown statement id"

#### Scenario: Bad parameter count returns ERR

- **WHEN** a test issues `COM_STMT_EXECUTE` with 1 parameter against a
  prepared `SELECT ?, ?`
- **THEN** the server SHALL respond with an `ERR` packet (0xFF) carrying
  sqlstate `HY000` and a message containing "parameter count mismatch"

#### Scenario: Parameter type mismatch returns ERR

- **WHEN** a test issues `COM_STMT_EXECUTE` with a string parameter against
  a prepared `SELECT CAST(? AS INT)`
- **THEN** the server SHALL respond with an `ERR` packet (0xFF) carrying
  sqlstate `HY000` and a message containing "invalid parameter type"
