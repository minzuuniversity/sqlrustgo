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

## Bulk loader: LOAD DATA LOCAL INFILE

The wire stack supports the MySQL `LOAD DATA LOCAL INFILE` protocol
for bulk-loading TBL data without per-row INSERT round-trips.

### Packet sequence

1. Client → Server: `COM_QUERY` with SQL
   `LOAD DATA LOCAL INFILE '<path>' INTO TABLE <t>`
2. Server: validates `<path>` is inside the configured `data_dir`
   (canonicalize + `starts_with`); rejects with 1146 ERR otherwise.
3. Server → Client: 0xFB packet, payload = path.
4. Client → Server: stream of file-content packets (≤ 16 MB each),
   terminated by an empty packet.
5. Server: batches lines into multi-row INSERTs at the
   `bulk_insert_buffer_size` boundary (default 1 MB).
6. Server → Client: OK packet with `affected_rows = total rows loaded`.

### Configuration

- `EphemeralConfig.data_dir` (existing): the only directory the server
  will read from. Required for LOAD DATA LOCAL INFILE to work.
- `EphemeralConfig.bulk_insert_buffer_size` (new, default 1 MB):
  threshold for flushing the in-memory batch to disk.

### Test surface

- `tests/load_local_infile_test.rs` — 5 tests, all green.
- Regression: `cargo test --tests` keeps the existing 38/38 wire-
  protocol tests green.
