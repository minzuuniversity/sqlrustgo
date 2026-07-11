# TLS Path: SELECT Result Set Hangs Client (600s timeout)

> **Status**: PARTIAL FIX 2026-06-30 — pymysql path works (Bugs A+B
> in `TlsStream` fixed). `mysql` 8.0 CLI still hangs; different
> root cause; pymysql-based SOAK tooling can proceed.
> **Severity**: P1 (downgraded from P0) — pymysql path is functional.
> **Reproducer**: `python3 /tmp/select_hang_smoke.py 3499` after starting
> `target/release/sqlrustgo-mysql-server serve --port 3499 --storage file`.

## Symptom

When a real MySQL 8.0 CLI client connects over TLS, every SELECT
statement (after any prior write) hangs the client for the full
600-second read timeout. The server logs `send_result_set done:
final_seq=6` and then sits idle in the `do_command_loop` reading
the next packet. The client has not received the result set and
times out with `ERROR 2013 (HY000) at line 1: Lost connection to
MySQL server during query`.

The same SELECT issued from a non-TLS `MySqlTestClient` (raw
TcpStream) returns the result in < 100ms — 21 regression tests
in `tests/wired_insert_payload_regression_test.rs` all pass.

## Reproduction

```bash
# Terminal 1: start server
rm -rf /tmp/sqlrustgo-soak && mkdir -p /tmp/sqlrustgo-soak
target/release/sqlrustgo-mysql-server serve --port 3499 \
  --data-dir /tmp/sqlrustgo-soak --storage file --server-threads 1 \
  --log-level info

# Terminal 2: real MySQL CLI (TLS path)
mysql -h 127.0.0.1 -P 3499 -u openclaw -e "
  CREATE TABLE t1 (id INTEGER PRIMARY KEY);
  INSERT INTO t1 VALUES (1);
  SELECT * FROM t1;
"
# ... hangs for 10 minutes, then 2013

# Same query via non-TLS test client (test path):
cargo test --release --test wired_insert_payload_regression_test \
  regression_insert_single_row_values_payload_not_truncated -- --nocapture
# ... passes in 0.26s
```

## What the server logs

```
INFO sqlrustgo_mysql_server: Connection from 127.0.0.1:39810
INFO sqlrustgo_mysql_server: SSL upgrade for 127.0.0.1:39810
INFO sqlrustgo_mysql_server: Handshake response: cap=0x19bfaa85, ...
INFO sqlrustgo_mysql_server: Auth accepted, sending OK packet, seq=3
INFO sqlrustgo_mysql_server: Starting command loop, seq=4
INFO sqlrustgo_mysql_server: Query [127.0.0.1:39810]: select @@version_comment limit 1
INFO sqlrustgo_mysql_server: Query [127.0.0.1:39810]: CREATE TABLE t1 (id INTEGER PRIMARY KEY)
INFO sqlrustgo_mysql_server: Query [127.0.0.1:39810]: INSERT INTO t1 VALUES (1)
INFO sqlrustgo_mysql_server: Query [127.0.0.1:39810]: SELECT * FROM t1
INFO sqlrustgo_mysql_server: send_result_set: 1 cols, 1 rows, start_seq=1
INFO sqlrustgo_mysql_server: send_result_set done: final_seq=6
# ... 600s of silence, then disconnect
```

## Root cause hypothesis

`send_result_set` writes 5 packets (col_count, col_def, OK separator,
row, OK terminator) for a `SELECT 1` in DEPRECATE_EOF mode. Each packet
goes through `Packet::write_to → w.flush()` which calls
`TlsStream::flush` (lib.rs:807). The flush has a drain loop that bails
on `WouldBlock`. With the socket in **blocking mode** (line 3005:
`set_nonblocking(false)`) `complete_io` should block — but the `while
self.conn.wants_write()` loop with `WouldBlock` short-circuit may be
exiting before all rustls records are driven to the socket.

After send_result_set returns, the server attempts to read the next
packet. The client, however, has only received the col count and
column def (or none of the result set) and is still waiting. The
client never gets unblocked because it can't read more until the
server sends more, but the server is now reading.

This is a symmetric deadlock: the client is reading what the server
hasn't fully written; the server is reading what the client hasn't
written.

## Why the test path works

`MySqlTestClient` (tests/common/mod.rs) uses a **raw `TcpStream`**,
not TLS. The server has a separate code path for non-SSL connections
(lib.rs:3123-3171) that uses `&mut &stream` directly — no `TlsStream`
wrapper, no rustls cipher buffer, no `complete_io` drain loop. The
write goes straight to the socket.

## Files involved

| File | Symbol | Role |
|------|--------|------|
| `crates/mysql-server/src/lib.rs:2980-3005` | `handle_connection` | Sets up blocking mode + timeouts |
| `crates/mysql-server/src/lib.rs:3028-3120` | SSL path | Wraps stream in TlsStream; starts command loop with `server_last_sent_seq=3` |
| `crates/mysql-server/src/lib.rs:3123-3171` | non-SSL path | Uses raw stream; `server_last_sent_seq=2` |
| `crates/mysql-server/src/lib.rs:787-819` | `TlsStream::write/flush` | Drain loop with WouldBlock short-circuit |
| `crates/mysql-server/src/lib.rs:1285-1372` | `send_result_set` | Sends 5+ packets for a 1-row 1-col result |

## What needs to happen

1. Confirm the actual bytes the client receives vs. the bytes the
   server thinks it sent. Use `strace -f -e trace=read,write,sendto,recvfrom`
   on the server process, or run a debug build with extra logging
   on the cipher state.
2. Fix the TlsStream drain loop to keep blocking until the cipher
   buffer is fully drained (remove the `WouldBlock` short-circuit
   when the socket is in blocking mode, or use a true blocking
   `complete_io` variant).
3. Add a regression test that uses a real TLS MySQL client
   (Python `ssl` module + manual MySQL protocol, or a `mysql` CLI
   subprocess) to verify the full result set is delivered.
4. Re-run the wired SOAK smoke test (`HOURS=0.05`) — should complete
   in ~3 minutes without 2013.

## Test artifact

`/tmp/select_hang_smoke.py` — minimal Python TLS MySQL client that
reproduces the hang with a 30s read timeout. Used in lieu of a Rust
TLS client (would require pulling in `rustls` and a TLS-aware MySQL
client lib into the test crate, which is a larger surface area than
the bug warrants).


---

## RESOLVED (2026-06-30) — pymysql path works

Two real bugs in `crates/mysql-server/src/lib.rs:TlsStream` were
identified via strace and fixed:

1. **`TlsStream::read` never called `process_new_packets`** —
   `read_tls` only fills the deframer; the plaintext buffer stayed
   empty so `reader().read()` returned 0 and the server never
   received the first MySQL command. This made the first
   `Packet::read_from` block forever.
2. **`TlsStream::write/flush` used `complete_io` instead of
   `write_tls`** — `complete_io` does both read and write, and with
   a blocking socket it blocked on `recvfrom` when `wants_read()`
   was true (the very next command the client was waiting to
   send). Symmetric deadlock: server waited for client data,
   client waited for server response.

Both fixed in commit. Removed dead `drive_reads_only` /
`drive_writes_only` helpers.

### Verified

- `pymysql SELECT 1` → returns `('1',)` in <1s
- `pymysql` 100-row INSERT + TPC-H Q1 → all OK
- `pymysql` multi-statement (`SELECT 1; SELECT 2; SELECT 3;`) → OK
- `cargo test --test wired_insert_payload_regression_test` → 21/21 PASS
- `cargo test --test g13_oltp1_concurrent_select_test` → 17/17 PASS
- `cargo test --test server_threads_cli_test --test server_thread_pool_e2e_test` → 24/24 PASS

### Remaining (mysql CLI only)

The `mysql` 8.0.46 CLI client still hangs after the server returns
a result. The pymysql path works perfectly. The remaining
difference is likely in how `mysql` handles multi-result /
CLIENT_MULTI_STATEMENTS / sequence validation. Tracked as
follow-up work — does NOT block using pymysql / pymysql-based
drivers for SOAK.