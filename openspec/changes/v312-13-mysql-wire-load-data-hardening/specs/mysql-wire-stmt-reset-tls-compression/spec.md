## ADDED Requirements

### Requirement: COM_STMT_PREPARE / EXECUTE / CLOSE on MySqlTestClient

`tests/common/mod.rs::MySqlTestClient` MUST expose three new methods
that the existing spec does not yet cover. Each method MUST round-trip
through the raw wire protocol; the methods MUST NOT depend on the `mysql`
crate (the `sha1` crate for `mysql_native_password` math is already in use).

- `prepare(&mut self, sql: &str) -> Result<u32, MysqlError>` MUST send
  `COM_STMT_PREPARE` (0x16), parse the OK packet, and return the statement id.
- `execute(&mut self, stmt_id: u32, params: &[Value]) -> Result<Vec<Vec<String>>, MysqlError>`
  MUST send `COM_STMT_EXECUTE` (0x17) with binary-encoded params and parse
  the result-set or OK packet.
- `close_stmt(&mut self, stmt_id: u32) -> Result<(), MysqlError>` MUST send
  `COM_STMT_CLOSE` (0x19); the method MUST succeed without a server
  response packet.

#### Scenario: Round-trip GREEN

- **GIVEN** an ephemeral server
- **WHEN** a test calls `prepare("SELECT ?, ?")`, `execute(1, &[Int(1), Str("a")])`,
  `close_stmt(1)`
- **THEN** `execute` returns `[["1", "a"]]` and the test passes

### Requirement: COM_RESET_CONNECTION on MySqlTestClient

`MySqlTestClient::reset_connection(&mut self) -> Result<(), MysqlError>` MUST
send `COM_RESET_CONNECTION` (0x1F), parse the OK packet, and assert the
client can immediately issue a new `SELECT @@autocommit`.

#### Scenario: Reset clears session state

- **WHEN** a test sets `@@autocommit = 0`, calls `reset_connection`, then
  reads `@@autocommit`
- **THEN** the result equals the server default (1)

### Requirement: TLS and compression helpers on MySqlTestClient

`MySqlTestClient::force_tls` and `force_compress` MUST each return
`Result<(), MysqlError>` and MUST fail (not silently fall back) if the
server did not advertise the requested capability.

#### Scenario: TLS handshake

- **GIVEN** a server with TLS feature
- **WHEN** `force_tls` negotiates
- **THEN** the negotiated cipher is recorded in the test log
- **AND** `SELECT 1` over the encrypted channel returns `1`

#### Scenario: Compression

- **GIVEN** a server with zlib compression capability
- **WHEN** `force_compress` sends a > 1 KiB result-set query
- **THEN** `compressed_len < raw_len` for the wire payload
- **AND** the decompressed result matches the uncompressed query result
