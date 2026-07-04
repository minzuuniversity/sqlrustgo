## Context

`sqlrustgo-mysql-server` is a SQL-92-subset RDBMS implementing the MySQL wire
protocol. sysbench 1.0.20 (using MariaDB Connector/C) is the canonical SOAK
benchmark for the G12 Gate (`scripts/gate/check_g12_sysbench.sh`).

The full COM_STMT_PREPARE → COM_STMT_EXECUTE binary path is implemented in
`crates/mysql-server/src/lib.rs`. Both `parse_stmt_execute_params` and
`replace_placeholders` exist; the in-tree unit tests for them (in
`prepared_stmt_params_test.rs`) pass. However the end-to-end binary
protocol round-trip (`tests/stmt_execute_repro.rs`) **fails today**:
`SELECT v FROM repro_t WHERE id = ?` with `id=3` returns 0 rows instead of 1.

sysbench's actual failure mode (`mysql_stmt_prepare() failed: MySQL error: 2000
"Unknown or undefined error code"`) appears to be a downstream consequence of the
same root issue, possibly compounded by missing trailing-OK packets after column
definitions when `DEPRECATE_EOF` capability is not negotiated.

## Goals / Non-Goals

**Goals:**
- Make `cargo test --test stmt_execute_repro` green
- Make `cargo test --test sysbench_protocol_parity_test` (new) green
- Make `bash scripts/sysbench/oltp_point_select.sh 4 5` produce transactions
- Make `bash scripts/gate/check_g12_sysbench.sh` 5/5 PASS
- Preserve the existing 141 mysql-server unit tests green

**Non-Goals:**
- Implement persistent statement caching across connections (already per-connection
  via `PreparedStatementManager`)
- Switch to plaintext `PREPARE … FROM` syntax (that's `parse_prepare` in the
  SQL parser, separate code path; the binary protocol is what sysbench uses)
- Replace rustls with `native-tls` (TLS issue is orthogonal; the failing test
  doesn't even involve TLS)

## Decisions

### D1. Diagnose via raw-protocol probe, not tcpdump

`tests/common::MySqlTestClient` already exposes
`stmt_prepare_raw` / `stmt_execute_raw` / `raw_stream`. The fastest way to
isolate the bug is to extend `tests/stmt_execute_repro.rs` with a
`println!` of every byte received on the wire after PREPARE — no
`tcpdump`, no `cargo` rebuild of `mysql-server`, just `cargo test --nocapture`.

**Alternatives considered:** packet capture with `tcpdump` (rejected — requires
`sudo`, host BPF device isn't always available, and the in-tree helper is faster).
Adding `tracing::trace!` calls in `crates/mysql-server/src/lib.rs` (rejected — adds
noise that obscures every other test run; better to do it once in the repro).

### D2. Fix `replace_placeholders` / `parse_stmt_execute_params` to handle the
int-bound case for `WHERE id = ?`

The repro binds the parameter as `type = LONG (0x03)` with `new_params_bound_flag = 1`.
The unit-test round-trip in `prepared_stmt_params_test.rs::parses_single_long_param`
passes — so the bug must be in **how the bound value is forwarded into
`replace_placeholders`**, not in the byte parser itself. Most likely the value
is serialized as `"3"` (decimal ASCII) but the SQL parser's `where_clause`
matches `id = ?` as a `Value::Null` placeholder rather than recognizing the
replacement result, OR the `is_numeric` flag is set incorrectly so the value
gets quoted as `'3'` (string), making the WHERE clause `id = '3'` — which
falls through the integer-comparison index lookup and returns 0 rows.

The fix is to ensure `parse_stmt_execute_params` sets `is_numeric = true` for
all numeric type codes (`LONG_TINY/SHORT/LONG/LONGLONG/FLOAT/DOUBLE/INT24/YEAR`),
and that `replace_placeholders` then splices the raw decimal bytes WITHOUT
single quotes.

### D3. Cap-cased trailing terminator in COM_STMT_PREPARE response

The current `send_binary_result_set` already switches on
`cap & capability::DEPRECATE_EOF`. The corresponding helper for
`COM_STMT_PREPARE`'s column/param definitions (see
`crates/mysql-server/src/lib.rs:2883-2892` for params and `:2941-2949`
for columns) **only sends the deprecate-EOF OK packet when the flag is set**.
sysbench's `0x00bfaa8d` capability set does **not** include `0x01000000`
(DEPRECATE_EOF), so the server falls into the classic-EOF branch and sends
`0xFE` — but with a SEQUENCE number that the client expected to be an OK
packet. The sysbench trace shows 4 packets successfully sent with `wants_write=false`
after each, so this is unlikely to be the cause but is worth verifying.

**If D2 alone fixes the repro test but sysbench still fails**, the next step is
to add a conditional in `make_eof_packet` to log the exact bytes sent on the wire
for sysbench's specific capability set, and ensure the EOF packet's 5-byte
payload (`0xFE + warnings(2 LE) + status_flags(2 LE)`) is byte-for-byte
correct — i.e. `0xFE 0x00 0x00 0x02 0x00`, not a 7-byte OK-shaped payload.

### D4. New test file `crates/mysql-server/tests/sysbench_protocol_parity_test.rs`

The test:
1. Boots ephemeral server via `EphemeralConfig::default()`
2. Creates `sbtest1` schema (int PK, char(120), char(60)) per sysbench's
   default schema
3. Inserts 50 rows (`id` in `1..=50`)
4. For each `id` in `1..=5`:
   - `COM_STMT_PREPARE: SELECT c FROM sbtest1 WHERE id = ?`
   - Drains OK + 1 param def + 1 EOF/OK + 1 column def + 1 EOF/OK
   - `COM_STMT_EXECUTE` with `type=LONG, new_params_bound_flag=1, value=<id>`
   - Drains column count + 1 column def + 1 EOF/OK + 1 row + 1 EOF/OK
   - Asserts the row's first column equals the expected `c` value
5. Asserts no `read timeout` / `connection reset` over the wire

This test does not need the `sysbench` binary and runs in CI in seconds.

### D5. G12 Gate extension

Insert a new check (5b) into `scripts/gate/check_g12_sysbench.sh` that runs the
new test binary directly:

```bash
cargo test --test sysbench_protocol_parity_test --quiet
```

If this fails, the gate fails — independent of whether `sysbench` is installed.

## Risks / Trade-offs

- **R1.** Changing `parse_stmt_execute_params` to mark numeric type codes as
  `is_numeric = true` could affect other tests in `prepared_stmt_params_test.rs`
  that build payloads with non-numeric types. Mitigation: run the full mysql-server
  test suite after the change; existing tests already validate the round-trip.
- **R2.** The new spec file `binary-prepared-statement-roundtrip/spec.md` adds a
  new top-level capability to the spec tree. This is intentional — the bug
  class (incomplete binary protocol) isn't currently expressed in any spec.
- **R3.** sysbench's actual 2000 error might be a real MariaDB Connector/C
  client-side bug — but working with both our repro test AND sysbench passing
  in CI is the reliable success signal; chasing MariaDB Connector/C source is
  out of scope.
- **R4.** The `make_eof_packet` change (D3, if needed) is wire-format-visible;
  rolling back later requires bumping the spec version.

## Open Questions

- Will D2 alone turn sysbench green, or will D3 also be required?
  (decided: ship D2 + D4, gate-check, then add D3 only if needed)
