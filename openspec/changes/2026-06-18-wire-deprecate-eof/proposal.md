# Proposal: wire-protocol-DEPRECATE_EOF-compat

## Why

`sqlrustgo-mysql-server` violates the MySQL wire protocol when the
client advertises the `DEPRECATE_EOF` capability (which all current
mysql-client 8.0+ libraries do by default). The result is that the
canonical CLI tool — `mysql` 8.0.46, on the development host — returns
either no rows at all (the original symptom on this codebase) or
`ER_MALFORMED_PACKET` (2027) on a partial result set, depending on
which sub-branch of the trailing-terminator logic is in effect.

This is a v3.9.0 P0 blocker for any cross-engine baseline that drives
sqlrustgo through an external mysql client (notably the TPC-H SF=1.0
cross-engine baseline requested in issue #3423, and any future GA soak
that uses pymysql/mysqlclient/mariadb-client against sqlrustgo).

The existing spec `wire-protocol-execution` (openspec/changes/archive/2026-06-03-mysql-server-canonical-entry/specs/wire-protocol-execution/spec.md)
defines the result-set packet sequence as "column count, column
definitions, **single EOF separator**, row packets, terminal EOF / OK",
which describes the **classic** (DEPRECATE_EOF=0) flow only. The
DEPRECATE_EOF=1 flow is currently unspecified and the implementation
sends an unconditional inter-record EOF regardless of the client's
capabilities. The comment in
`crates/mysql-server/src/lib.rs::send_result_set` (line 925) admits
the gap: "the new protocol path can be re-introduced once the client
has caught up". It has caught up — mysql 8.0 ships the
DEPRECATE_EOF-capable libmysqlclient 8.0.46 — and the gap is now
producing silent data loss / malformed-packet errors.

## What Changes

- `send_result_set` in `crates/mysql-server/src/lib.rs` honors the
  client's `DEPRECATE_EOF` capability for the inter-record separator
  between column defs and the row stream, instead of always emitting
  the classic EOF.
- The trailing terminator (EOF vs. OK packet) is already conditional
  in code; the spec now pins down the byte layout for both branches
  so future refactors don't drift.
- `MySqlTestClient` (the canonical test-side raw-protocol client in
  `tests/common/mod.rs`) is extended to negotiate a known
  capabilities set and to read the result-set terminator that matches
  the server's behavior for that capabilities set. This removes the
  implicit assumption that the server always sends classic EOF.
- One new integration test (`tests/wire_deprecate_eof_test.rs`) drives
  the same SQL through a `MySqlTestClient` configured with both
  capabilities variants and asserts that both paths produce the same
  row data. This guards against regressions in either branch.
- A new external-cli smoke script (`scripts/wire_smoke_mysql_cli.sh`)
  validates that `mysql` 8.0+ can connect to the running server and
  `SELECT 1` returns a single row with no `ER_MALFORMED_PACKET`. This
  is the acceptance check tied to this change.

## Capabilities

### New Capabilities

- `wire-deprecate-eof-compat`: the wire stack supports both the
  classic (DEPRECATE_EOF=0) and the deprecated-EOF (DEPRECATE_EOF=1)
  result-set packet sequences; the choice is driven by the client's
  capabilities. The trailing terminator is `make_eof_packet` (0xFE
  + u16 warnings + u16 status_flags, 5 bytes) for classic, and
  `make_ok_packet` (0x00 + lenenc affected_rows + lenenc last_insert_id
  + u16 status_flags + u16 warnings, 7 bytes) for deprecated-EOF.
  The inter-record separator between column defs and the row stream
  is `make_eof_packet` (5 bytes) for classic and is omitted entirely
  for deprecated-EOF.

### Modified Capabilities

- `wire-protocol-execution`: the result-set packet sequence is
  updated to specify both the classic and the deprecated-EOF flows,
  with the trailing terminator byte layout fixed for each. The
  "single EOF separator" wording in the prior spec is preserved for
  the classic path and is paired with the new "no inter-record
  separator" rule for the deprecated-EOF path.

## Non-Goals

- This change does **not** alter the LOAD DATA LOCAL INFILE packet
  sequence. That flow is unaffected by DEPRECATE_EOF and is governed
  by the existing spec.
- This change does **not** add support for the MySQL X Protocol
  (port 33060), prepared-statement binary protocol beyond the current
  scope, or multi-result-set. Those are separate work items.
- This change does **not** change `MySqlTestClient`'s default
  capabilities. The test client continues to advertise the minimal
  set (PROTOCOL_41 | SECURE_CONNECTION | LONG_PASSWORD) and is
  extended to expose a `connect_with_caps` builder so individual
  tests can opt into the DEPRECATE_EOF bit.
- This change does **not** modify `tpch_data_gen`, the SF=1.0 fixture
  loader, or any of the in-process test harnesses. Those are governed
  by their own specs.

## Acceptance

1. With the fix in place and `mysql 8.0.46` (or any
   DEPRECATE_EOF-capable client) connected, `SELECT 1` returns one
   row of `1` with no `ER_MALFORMED_PACKET` and no hang.
2. With the fix in place and a DEPRECATE_EOF-incapable client
   connected (e.g. `MySqlTestClient` with the legacy caps),
   `SELECT 1` returns the same row via the classic EOF path.
3. All existing wire-protocol tests (the 10 tests in
   `tests/tpch_sf01_22_vs_3engines_test.rs` and the wider 38-test
   regression set cited in the existing spec) continue to pass.
4. The new `tests/wire_deprecate_eof_test.rs` test asserts the row
   content parity described in point 2.
5. The new `scripts/wire_smoke_mysql_cli.sh` script exits 0 when run
   against a `sqlrustgo-mysql-server serve` subprocess.
