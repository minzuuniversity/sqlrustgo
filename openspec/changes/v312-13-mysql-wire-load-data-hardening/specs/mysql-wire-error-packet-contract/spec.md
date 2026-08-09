## ADDED Requirements

### Requirement: ERR packet shape

The server MUST respond with an ERR packet (0xFF) whenever it cannot
complete a `COM_QUERY` or `COM_STMT_EXECUTE`. The packet MUST carry:

- 2-byte error code (little-endian)
- 1-byte `#` SQLSTATE marker
- 5-byte SQLSTATE (e.g. `42000` for syntax error, `23000` for integrity,
  `28000` for auth)
- human-readable message

#### Scenario: Syntax error

- **WHEN** a client sends `COM_QUERY` for `SELEC * FROM t`
- **THEN** the server SHALL return an ERR packet with sqlstate `42000`
  and a message containing "syntax error"

#### Scenario: Integrity violation

- **WHEN** a client inserts a duplicate primary key
- **THEN** the server SHALL return an ERR packet with sqlstate `23000`
  and a message containing "Duplicate entry"

#### Scenario: Auth failure

- **WHEN** a client connects with the wrong password
- **THEN** the server SHALL return an ERR packet with sqlstate `28000`
  and a message containing "Access denied"

### Requirement: expect_err parses the ERR packet

`MySqlTestClient::expect_err` MUST send SQL and parse the ERR packet; the returned tuple `(error_code, sqlstate, message)` SHALL be returned via `Result<(u16, String, String), MysqlError>`.

#### Scenario: expect_err returns tuple

- **WHEN** a test calls `expect_err("SELEC 1")`
- **THEN** the result is `Ok((<code>, "42000", "syntax error..."))`
- **AND** a test asserts `result.unwrap().1 == "42000"`
