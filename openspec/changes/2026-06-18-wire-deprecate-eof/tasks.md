# Tasks — wire-deprecate-eof-compat

## 1. Code change in `crates/mysql-server/src/lib.rs`

- [ ] Update `send_result_set` so the inter-record separator is
  gated on `cap & capability::DEPRECATE_EOF == 0`. When the bit is
  set, the separator is omitted entirely.
- [ ] Confirm the trailing terminator logic is gated on the same
  capability and emits `make_eof_packet` for classic,
  `make_ok_packet` for deprecated-EOF, and that the status flags
  in both branches include `SERVER_STATUS_AUTOCOMMIT` (0x0002).
- [ ] Verify the sequence numbers are contiguous and start at the
  value passed in (no off-by-one in either branch).

## 2. Code change in `tests/common/mod.rs`

- [ ] Add `pub fn connect_with_caps(addr: (&str, u16), user: &str,
  password: &str, caps: u32) -> wire_err::Result<Self>` to
  `MySqlTestClient`. Reuse the existing handshake/auth helper
  functions; only the capabilities field changes.
- [ ] Add `pub fn client_capabilities(&self) -> u32` (or store the
  capabilities on the struct) so `query_rows` can decide whether
  to read an inter-record separator.
- [ ] Update `query_rows` to skip the inter-record-EOF `read_packet`
  call when `DEPRECATE_EOF` is set in the client capabilities.
- [ ] Verify the existing `connect_default`, `connect_handle`,
  `connect_at`, `connect_with_config` constructors all still pass
  `capability::DEPRECATE_EOF = 0` (the test default).

## 3. New test surface

- [ ] Add `tests/wire_deprecate_eof_test.rs` with the two scenarios
  from the spec (`classic_caps_round_trip`,
  `deprecated_eof_caps_round_trip`).
- [ ] Add a `mod common;` declaration matching the other
  integration tests under `tests/`.

## 4. New manual gate

- [ ] Add `scripts/wire_smoke_mysql_cli.sh` that implements the
  smoke scenario in the spec.

## 5. Verification

- [ ] `cargo build --release -p sqlrustgo-mysql-server` exits 0.
- [ ] `cargo test --test wire_deprecate_eof_test -- --nocapture`
  passes both tests.
- [ ] `cargo test --test tpch_sf01_22_vs_3engines_test` (and the
  wider wire-protocol regression set cited in the spec) stays
  green.
- [ ] Manual: start `target/release/sqlrustgo-mysql-server serve
  --port 3457 --data-dir <tmp>` and run
  `mysql -h 127.0.0.1 -P 3457 -u root --ssl-mode=DISABLED
  -e "SELECT 1"`. Output: `1` (or `col_1 / 1`), no
  `ER_MALFORMED_PACKET`, no hang, exit code 0.
- [ ] `scripts/wire_smoke_mysql_cli.sh` exits 0.

## 6. Documentation

- [ ] Update the comment block at line 925 of
  `crates/mysql-server/src/lib.rs` so it no longer says "the new
  protocol path can be re-introduced once the client has caught up"
  (the path is now in place; point to the openspec change).
- [ ] Note in the spec's `MODIFIED Requirements` section that
  `wire-protocol-execution` is now updated to cover both
  branches, and reference `wire-deprecate-eof-compat` for the
  byte-level details.
