## MODIFIED Requirements

### Requirement: TLS handshake and compression

The `wire-protocol-execution` spec MUST be extended so that `MySqlTestClient`
exposes `force_tls` and `force_compress` helpers. Both helpers MUST fail
(not silently fall back) if the server did not advertise the requested
capability.

#### Scenario: TLS handshake and authenticated query

- **GIVEN** a server started with `start_ephemeral` and the TLS feature enabled
- **WHEN** `MySqlTestClient::force_tls` negotiates SSLRequest after the
  HandshakeV10
- **THEN** the negotiated cipher SHALL appear in the test log
- **AND** a subsequent `SELECT 1` over the encrypted channel SHALL return `1`

#### Scenario: Compression shrinks payload

- **GIVEN** a server with zlib compression capability advertised
- **WHEN** `MySqlTestClient::force_compress` sends a `COM_QUERY` whose result
  set is > 1 KiB
- **THEN** the wire bytes observed by the server SHALL be smaller than the
  uncompressed payload (`compressed_len < raw_len`)
- **AND** the decompressed result SHALL match the uncompressed result

### Requirement: COM_RESET_CONNECTION

`MySqlTestClient::reset_connection` MUST send `COM_RESET_CONNECTION` (0x1F)
and the server SHALL respond with an OK packet. The server MUST revert
session-level state (`@@autocommit`, prepared statements, session variables)
to defaults.

#### Scenario: Reset reverts autocommit to server default

- **GIVEN** a connected client
- **WHEN** the client issues `SET @@autocommit = 0`, then `COM_RESET_CONNECTION`,
  then `SELECT @@autocommit`
- **THEN** the result SHALL equal the server's default autocommit value (1)
- **AND** any prepared statement id from before the reset SHALL be unknown
  to the server (a follow-up `COM_STMT_EXECUTE` returns `ERR` with
  sqlstate `HY000` and "Unknown statement id")
