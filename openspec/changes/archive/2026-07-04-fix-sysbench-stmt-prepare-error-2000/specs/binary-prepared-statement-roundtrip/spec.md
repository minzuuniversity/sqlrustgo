# binary-prepared-statement-roundtrip Specification

## Purpose

TBD - created by archiving change fix-sysbench-stmt-prepare-error-2000. Update Purpose after archive.

## Requirements

### Requirement: COM_STMT_PREPARE response is wire-compatible with the MySQL binary protocol spec

The MySQL wire-protocol `COM_STMT_PREPARE` handler in `sqlrustgo-mysql-server`
SHALL emit a response sequence that conforms to the MySQL binary-protocol
specification. The response sequence MUST consist of:

1. One OK-shaped packet carrying:
   - `0x00` status byte
   - `stmt_id` (u32 LE)
   - `column_count` (u16 LE)
   - `param_count` (u16 LE)
   - reserved filler (1 byte)
   - `warnings` (u16 LE)
2. For each of `param_count` placeholders, one ColumnDefinition packet
   carrying the parameter type the server inferred.
3. For each of `column_count` projected columns, one ColumnDefinition packet
   carrying the column metadata.
4. After (2) and (3), a terminating packet:
   - `0xFE + u16_LE warnings + u16_LE status_flags` (5 bytes) when the client
     does NOT advertise `CLIENT_DEPRECATE_EOF` (0x01000000)
   - 7-byte OK-shaped packet when `CLIENT_DEPRECATE_EOF` is set

#### Scenario: PREPARE for SELECT with one parameter emits the four expected packets

- **GIVEN** an ephemeral server started via `start_ephemeral`
- **WHEN** the client sends `COM_STMT_PREPARE` for `SELECT c FROM sbtest1 WHERE id = ?`
- **THEN** the response MUST consist of exactly 5 packets in order:
  1. STMT_PREPARE_OK with `stmt_id=1`, `cols=1`, `params=1`
  2. One parameter ColumnDefinition for `?`
  3. EOF terminator packet (5 bytes, first byte 0xFE) for params
  4. One column ColumnDefinition for `c`
  5. EOF terminator packet (5 bytes, first byte 0xFE) for columns

#### Scenario: DEPRECATE_EOF OFF client receives 0xFE terminators

- **GIVEN** a client that does NOT advertise `CLIENT_DEPRECATE_EOF`
- **WHEN** the server sends a terminator after a column/param group
- **THEN** the terminator packet payload SHALL start with byte `0xFE`
  (EOF marker, MySQL classic protocol)
- **AND** the payload SHALL be exactly 5 bytes

### Requirement: COM_STMT_EXECUTE row framing conforms to the binary-protocol spec

The MySQL wire-protocol `COM_STMT_EXECUTE` response MUST use the spec-correct
binary row format:

- Each row packet payload starts with a single `0x00` header byte
- Followed by **exactly `(cols + 7) / 8` bytes** of null bitmap (NOT +9)
- Followed by each column value, length-encoded per the column's MySQL type

#### Scenario: 1-column row has exactly 1 byte of null bitmap

- **WHEN** a SELECT result set has 1 column and any number of rows
- **THEN** each row packet payload SHALL be `1 + 1 + N_value_bytes` long,
  where `1` is the `0x00` header byte and the second `1` is the null bitmap
- **AND NOT** `1 + 2 + N_value_bytes - 1` (the off-by-one bug the previous
  code produced)

#### Scenario: Bind numeric param to WHERE id = ? returns matching row

- **GIVEN** a client sends `COM_STMT_PREPARE` for `SELECT v FROM t WHERE id = ?`
  followed by `COM_STMT_EXECUTE` with `param_count=1`,
  `null_bitmap=[0x00]`, `new_params_bound_flag=1`, `param_type=LONG(0x03)`,
  `value=3i32_le`
- **THEN** the spliced SQL SHALT be `SELECT v FROM t WHERE id = 3` (numeric,
  no quotes — verified by `tracing::info!` log of `final_sql`)
- **AND** the response SHALL contain at least one row whose first column
  equals the `v` of the row where `id = 3`

### Requirement: REGRESSION TEST for binary protocol round-trip

The repo MUST contain `tests/stmt_execute_repro.rs` with the test
`repro_stmt_execute_returns_malformed_packet`. This test MUST walk the full
`COM_STMT_PREPARE → COM_STMT_EXECUTE → row drain` cycle using the raw-protocol
test client `MySqlTestClient` (no TLS, no MariaDB Connector/C dependency).

#### Scenario: Test passes after the fix

- **WHEN** `cargo test --test stmt_execute_repro` is run after the fix is applied
- **THEN** it MUST report `test repro_stmt_execute_returns_malformed_packet ... ok`
- **AND** `cargo test -p sqlrustgo-mysql-server` MUST report all 141+ tests passing
- **AND** `cargo test --test mysql_wire_protocol_test` MUST report all 28 tests passing

#### Scenario: Test client correctly distinguishes row packets from OK terminators

The test client parser SHALL use a 4-phase state machine to handle the
COM_STMT_EXECUTE response:
1. Phase 0 (column count): read one packet and extract the column-count lenenc integer
2. Phase 1 (column definitions): read exactly `column_count` packets
3. Phase 2 (separator): read ONE EOF or OK packet that is NOT the final terminator
4. Phase 3 (rows): read row packets until the final terminator

The final terminator SHALL be detected as either:
- EOF: `payload[0] == 0xFE && payload.len() == 5`
- OK: `payload[0] == 0x00 && payload.len() == 7 && bytes[1..2] == 0x00 0x00 && bytes[5..6] == 0x00 0x00`

## ADDED Requirements

### Requirement: Wire-format trace logging for COM_STMT_EXECUTE

`crates/mysql-server/src/lib.rs::do_command_loop` MUST log, when
`RUST_LOG=sqlrustgo_mysql_server=info` is set, the SQL after parameter
substitution for each `COM_STMT_EXECUTE` execution so operators can correlate
client-bound parameters to the executed query.

#### Scenario: Trace log shows spliced SQL

- **WHEN** a client sends `COM_STMT_PREPARE` + `COM_STMT_EXECUTE` for
  `SELECT v FROM t WHERE id = ?` bound to `3`
- **THEN** the server `info` log line for that execution MUST contain the
  string `WHERE id = 3` (decimal, no quotes)
