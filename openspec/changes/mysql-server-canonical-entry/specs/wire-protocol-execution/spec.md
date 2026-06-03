# wire-protocol-execution

## Purpose

Every end-to-end and performance test SHALL drive SQL through
the MySQL wire protocol (HandshakeV10 → HandshakeResponse41 →
COM_QUERY → result set) against an `start_ephemeral` server or
the canonical `sqlrustgo-mysql-server serve` subprocess. The
`tests/common/mod.rs::MySqlTestClient` raw-protocol client is
the canonical test-side wire-protocol surface.

## ADDED Requirements

### Requirement: Raw-protocol test client

`MySqlTestClient` SHALL be a raw-protocol MySQL client in
`tests/common/mod.rs` that:

- Reads HandshakeV10 from the server and extracts the 20-byte
  scramble.
- Computes a `mysql_native_password` auth response from the
  scramble and the configured password.
- Sends HandshakeResponse41 with `PROTOCOL_41 | SECURE_CONNECTION
  | LONG_PASSWORD` capabilities and reads the server's OK /
  ERR response.
- Sends COM_QUERY (0x03) packets and parses the result set
  (column count, column definitions, single EOF separator, row
  packets, terminal EOF / OK).
- Exposes `exec(&mut self, sql)`, `query_rows(&mut self, sql) ->
  Vec<Vec<String>>`, and `query_one_i64(&mut self, sql) -> i64`
  for the test surface.
- Exposes `connect_handle(handle)` for in-process ephemerals and
  `connect_at(addr, user, password)` for external (subprocess)
  servers.

The client SHALL NOT depend on the `mysql` crate; the
`mysql_native_password` math is implemented inline with the
`sha1` crate.

#### Scenario: Wire-protocol round-trip is GREEN

- GIVEN an ephemeral server started via `start_ephemeral`
- WHEN the test connects with `MySqlTestClient::connect_default`
  and runs CREATE + INSERT + SELECT
- THEN the test MUST observe the inserted rows via
  `query_rows` and `query_one_i64`

### Requirement: EphemeralConfig flags

`EphemeralConfig` SHALL expose two opt-in flags:

- `bootstrap_tables: bool` (default `true`) — pre-create the
  internal catalog tables `content`, `vectors`, `documents`
- `bootstrap_users: bool` (default `true`) — pre-create a
  `tester` user with password `tester`

A test that asserts on a clean catalog SHALL set
`bootstrap_tables: false` and keep `bootstrap_users: true` so
authentication still works.

#### Scenario: Clean catalog for SHOW TABLES

- GIVEN a test starts an ephemeral with
  `EphemeralConfig { bootstrap_tables: false, .. }`
- WHEN it runs `SHOW TABLES` against the empty database
- THEN the result set SHALL have zero rows
