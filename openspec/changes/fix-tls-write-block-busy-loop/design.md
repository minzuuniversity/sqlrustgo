# Design: Fix TlsStream WouldBlock busy-loop

## Problem

The `TlsStream` write path (Issue #3694 fix) tight-spins on `WouldBlock`:

```
TlsStream::write():
  loop complete_io(sock):
    WouldBlock → continue  # tight loop, never yields
    Ok          → continue
    Error       → return

TlsStream::flush():
  loop complete_io(sock):
    WouldBlock → continue
    Ok          → continue
    Error       → return

drive_writes_only() / force_drain():
  loop complete_io(sock):
    WouldBlock → continue
    Ok          → continue
    Error       → return
```

When the kernel socket buffer is full on a non-blocking socket, `complete_io` returns `WouldBlock` immediately on every iteration — the CPU spins at 100% with zero I/O progress. The TLS cipher records never reach the client.

## Prior art (working paths)

Already break on `WouldBlock`:
- `Read::read()` (line 766): `Err(ref e) if e.kind() == WouldBlock => break`
- `flush_pending()` (line 749): `Err(ref e) if e.kind() == WouldBlock => break`
- `drive_reads_only()` (line 786): `Err(ref e) if e.kind() == WouldBlock => break`

## Fix

Change all three methods to break on `WouldBlock` instead of looping.

When `WouldBlock` occurs during `write()`:
1. Data is already accepted into the rustls internal buffer (`self.conn.writer().write(buf)` at line 796 always succeeds — it's a buffer-to-buffer copy)
2. Some cipher records may have been flushed, some remain buffered
3. Return `Ok(n)` (the number of app bytes accepted) — standard `Write` contract

When `WouldBlock` occurs during `flush()`:
1. Any remaining buffered cipher records stay in rustls
2. Return `Ok(())` — standard `flush` contract (best-effort)

On the NEXT `Packet::read_from()` → `TlsStream::read()` call, `complete_io` processes inbound ciphertext AND flushes pending outbound records (rustls `complete_io` does both). This ensures all buffered data eventually reaches the socket.

## Safety

This is the standard pattern for non-blocking TLS streams. The current code is a bug: it replaces `WouldBlock` (which means "try again later") with an infinite busy-loop.

The MariaDB Connector/C client holds the TCP connection open while waiting for the PREPARE response, so the read→write cycle of the next command loop iteration will flush remaining data.

## Key file

`crates/mysql-server/src/lib.rs`:
- Line 794–833: `TlsStream::write()`
- Line 834–857: `TlsStream::flush()`  
- Line 872–902: `drive_writes_only()` (called by `force_drain()`)
