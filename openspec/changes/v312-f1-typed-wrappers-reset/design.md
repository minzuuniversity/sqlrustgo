# V312-F-1 Design: COM_RESET_CONNECTION Handler

## Problem

The MySQL `COM_RESET_CONNECTION` command (0x1F) is supposed to reset the
connection state without re-authenticating. After receiving the OK response,
the client expects subsequent packets to start from sequence=0 (server's
perspective) and the next SELECT should succeed.

Current behavior: server's COM_RESET_CONNECTION returns an OK packet but
fails to reset its internal packet sequence counter. The next query packet
arrives with seq=1 (client's view), server expects seq=0 (after reset),
but the server is still at seq=N. Result: protocol mismatch, error
"unexpected response (seq=1, first=0x01)".

## Fix

In `crates/mysql-server/src/lib.rs`, the COM_RESET_CONNECTION branch must:

1. **Reset packet sequence**: ensure next inbound packet is parsed with seq=0
2. **Reset session state**: charset, status flags, prepared statements, session vars
3. **Send OK with seq=1** (server-side seq, which is incremented on send)

The handler likely currently does the body of (3) but misses (1). Adding
`self.packet_seq = 0;` (or equivalent reset) before constructing the OK
packet resolves the protocol mismatch.

## Verification

Run `cargo test --test v312_13_typed_wrappers_test -- --test-threads=1`.
Before fix: 21 passed, 1 failed (v312_13_reset_connection_ok).
After fix: 22 passed, 0 failed.