# wire-deprecate-eof-compat

## Purpose

The MySQL wire-protocol result-set packet sequence MUST be a function
of the client's `DEPRECATE_EOF` capability. The server SHALL emit the
classic sequence (inter-record EOF, trailing EOF) when the client
declares `DEPRECATE_EOF=0`, and the deprecated-EOF sequence (no
inter-record separator, trailing OK packet) when the client declares
`DEPRECATE_EOF=1`. This is the behavior implemented in MySQL 5.7+
servers, and it is the behavior that mysql-client 8.0+ and all
modern libmysqlclient/mariadb-client/pymysql builds expect.

The current implementation in `crates/mysql-server/src/lib.rs::send_result_set`
emits the classic sequence unconditionally, which causes mysql-client
8.0+ to either drop the result set entirely (silently return zero
rows) or report `ER_MALFORMED_PACKET` (2027) once the server also
tries to switch the trailing terminator without first removing the
inter-record EOF.

## ADDED Requirements

### Requirement: Inter-record separator is capability-controlled

The inter-record packet between the column-definition stream and the
row stream in `send_result_set` SHALL be sent if and only if the
client's `DEPRECATE_EOF` capability is unset. When the capability is
set, the server SHALL skip the inter-record separator entirely. The
capability bit is `0x01000000`.

#### Scenario: classic client receives inter-record EOF

- GIVEN a client that does NOT advertise `DEPRECATE_EOF`
  (e.g. `MySqlTestClient` with the legacy capabilities set
  `0x00008201` = `LONG_PASSWORD | PROTOCOL_41 | SECURE_CONNECTION`)
- WHEN the client runs `SELECT 1`
- THEN the server SHALL emit, in order:
  1. column count packet
  2. one column-definition packet
  3. one classic inter-record EOF packet (0xFE + u16 0 + u16 0x0002, 5 bytes)
  4. one row packet
  5. one classic trailing EOF packet (same layout as step 3)

#### Scenario: deprecated-EOF client receives no inter-record separator

- GIVEN a client that DOES advertise `DEPRECATE_EOF`
  (e.g. mysql-client 8.0.46 with capabilities `0x19bfa28d`)
- WHEN the client runs `SELECT 1`
- THEN the server SHALL emit, in order:
  1. column count packet
  2. one column-definition packet
  3. (no inter-record separator)
  4. one row packet
  5. one trailing OK packet (0x00 + lenenc 0 + lenenc 0 + u16 0x0002 + u16 0, 7 bytes)

### Requirement: Trailing terminator byte layout is fixed

The trailing terminator emitted by `send_result_set` SHALL be:

- For clients with `DEPRECATE_EOF=0`: a classic EOF packet, produced
  by `make_eof_packet(seq, 0x0002)`. The packet payload is the
  5-byte sequence `0xFE`, `0x00 0x00` (warnings), `0x02 0x00`
  (status flags, little-endian, with `SERVER_STATUS_AUTOCOMMIT` set).

- For clients with `DEPRECATE_EOF=1`: an OK packet, produced by
  `make_ok_packet(seq, 0, 0, 0x0002, 0)`. The packet payload is the
  7-byte sequence `0x00`, `0x00` (lenenc affected_rows = 0),
  `0x00` (lenenc last_insert_id = 0), `0x02 0x00` (status flags),
  `0x00 0x00` (warnings).

In both branches the status flags MUST include
`SERVER_STATUS_AUTOCOMMIT` (0x0002) so the client observes the same
autocommit state in both protocol variants.

#### Scenario: status flags include AUTOCOMMIT in both branches

- GIVEN either a classic or a deprecated-EOF client
- WHEN the client runs any `SELECT` statement
- THEN the trailing terminator's status-flags field SHALL have
  `0x0002` (`SERVER_STATUS_AUTOCOMMIT`) set, and no other status
  flags SHALL be set unless the engine explicitly changed the
  transaction state.

### Requirement: Packet sequence numbers are contiguous

Within a single `send_result_set` invocation, the sequence numbers on
the four packet types — column count, column defs, inter-record
separator (classic only), row, trailing terminator — SHALL be a
contiguous strictly-increasing sequence starting at the sequence
number passed in. Each `write_to` of a packet SHALL use the
contemporary sequence number; the sequence number SHALL be
incremented by 1 after each packet write.

#### Scenario: sequence numbers match MySQL protocol spec

- GIVEN any client and any `SELECT` statement
- WHEN the server emits the result-set packets
- THEN the four packet sequence numbers SHALL be
  `{n, n+1, ..., n+k-1}` where `k` is the number of packets in the
  branch (4 for classic with one column, 4 for deprecated-EOF with
  one column; the count grows by one for each additional column).

### Requirement: `MySqlTestClient` exposes a capabilities builder

`MySqlTestClient` SHALL expose a `connect_with_caps(addr, user,
password, caps)` method (in addition to the existing
`connect_default`, `connect_with_config`, `connect_at`,
`connect_handle`) that lets a test set the capabilities advertised in
the HandshakeResponse41 packet. The existing constructors SHALL
preserve their current default capabilities
(`LONG_PASSWORD | PROTOCOL_41 | SECURE_CONNECTION`) so no test
regresses.

`MySqlTestClient::query_rows` SHALL use the same `cap` value the
client advertised in the handshake when deciding whether to read an
inter-record separator. With the default caps, the client reads one
inter-record EOF; with `DEPRECATE_EOF` set, the client reads none.

#### Scenario: deprecated-EOF test client reads the right shape

- GIVEN a `MySqlTestClient` connected via `connect_with_caps` with
  `DEPRECATE_EOF` set in the capabilities
- WHEN the test calls `query_rows("SELECT 1")`
- THEN the client SHALL successfully parse the 4-packet result set
  (column count, column def, row, OK) and return
  `vec![vec!["1".to_string()]]`

## MODIFIED Requirements

### wire-protocol-execution

The result-set packet sequence under
`wire-protocol-execution::ADDED Requirements` is updated to read:

> Sends COM_QUERY (0x03) packets and parses the result set:
> column count, column definitions, **then either a single EOF
> separator (when the client's `DEPRECATE_EOF` capability is unset)
> or no inter-record separator (when it is set)**, row packets,
> **and a trailing terminator which is a classic EOF packet in the
> classic branch and an OK packet in the deprecated-EOF branch**.

This rewording preserves the original "single EOF separator" rule
for the classic path and adds the new "no inter-record separator"
rule for the deprecated-EOF path. The byte layout of the trailing
terminator is now specified in `wire-deprecate-eof-compat` and is
not duplicated here.

## ADDED Test Surface

### Requirement: wire_deprecate_eof_test covers both branches

`tests/wire_deprecate_eof_test.rs` SHALL contain at least two
integration tests:

- `classic_caps_round_trip`: starts an ephemeral server via
  `start_ephemeral`, connects with `MySqlTestClient` using the
  default (classic) capabilities, runs `SELECT 1` and asserts the
  result is `vec![vec!["1"]]`.
- `deprecated_eof_caps_round_trip`: same setup, but the client is
  built with `connect_with_caps` and `DEPRECATE_EOF` set in the
  capabilities. Asserts the same row data is returned.

Both tests SHALL run as part of `cargo test --tests` and SHALL pass.

#### Scenario: regression guard

- GIVEN the implementation in this change is in place
- WHEN a developer runs `cargo test --test wire_deprecate_eof_test`
- THEN both tests SHALL pass and the assertion message SHALL
  identify which capabilities variant was under test.

### Requirement: scripts/wire_smoke_mysql_cli.sh is the manual gate

`scripts/wire_smoke_mysql_cli.sh` SHALL be a POSIX shell script
that:

1. Starts a `sqlrustgo-mysql-server serve` subprocess on an
   OS-assigned port in the background, with `--data-dir` pointing
   at a fresh temp directory.
2. Waits up to 5 seconds for the listener to come up.
3. Runs `mysql --ssl-mode=DISABLED -h 127.0.0.1 -P <port> -u root
   -e "SELECT 1"` and captures the exit code and the first 5 lines
   of stdout / stderr.
4. Asserts that the exit code is 0, that stdout contains a row
   showing `1`, and that stdout does NOT contain the substring
   `ER_MALFORMED_PACKET`.
5. Tears down the server subprocess (and the temp directory) on
   exit, regardless of pass / fail.

The script SHALL exit non-zero on any assertion failure.

#### Scenario: smoke script is GREEN against the patched server

- GIVEN a freshly-built `target/release/sqlrustgo-mysql-server`
  binary and a `mysql` 8.0+ client on the host PATH
- WHEN a developer runs `scripts/wire_smoke_mysql_cli.sh`
- THEN the script SHALL exit 0 within 10 seconds and SHALL print
  a `PASS` line summarizing the captured row and the absence of
  any `ER_MALFORMED_PACKET` substring.
