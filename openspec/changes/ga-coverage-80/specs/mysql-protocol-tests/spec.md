## ADDED Requirements

### Requirement: MySQL server completes handshake correctly
The system SHALL complete MySQL protocol handshake and establish connection.

#### Scenario: Successful authentication
- **WHEN** client sends correct username and password hash
- **THEN** server SHALL send OK packet and accept subsequent commands

#### Scenario: Failed authentication
- **WHEN** client sends incorrect password
- **THEN** server SHALL send Error packet with ER_ACCESS_DENIED_ERROR

### Requirement: MySQL server handles COM_QUERY
The system SHALL parse and execute MySQL COM_QUERY packets.

#### Scenario: Simple SELECT query
- **WHEN** client sends COM_QUERY with "SELECT 1 AS result"
- **THEN** server SHALL return ResultSet with one row containing column "result" = 1

#### Scenario: INSERT statement
- **WHEN** client sends COM_QUERY with "INSERT INTO t VALUES (1)"
- **THEN** server SHALL execute insert and return Ok packet with affected_rows = 1

#### Scenario: Multi-statement query
- **WHEN** client sends "SELECT 1; SELECT 2" as multi-statement
- **THEN** server SHALL return two ResultSets sequentially

### Requirement: MySQL server handles COM_STMT_PREPARE
The system SHALL parse and prepare statement without executing it.

#### Scenario: Prepare SELECT
- **WHEN** client sends COM_STMT_PREPARE with "SELECT * FROM users WHERE id = ?"
- **THEN** server SHALL return StatementWithParameterCount with param_count = 1, no rows

#### Scenario: Execute prepared statement with params
- **WHEN** client sends COM_STMT_EXECUTE with param values for prepared "SELECT * FROM users WHERE id = ?"
- **THEN** server SHALL return ResultSet with rows matching parameter value
