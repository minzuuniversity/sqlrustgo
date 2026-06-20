---
name: deprecate-eof-fix
description: Fix MySQL error 2027 "Malformed packet" caused by incorrect DEPRECATE_EOF OK packet marker (0xFE vs 0x00)
source: auto-skill
extracted_at: '2026-06-20T05:30:00.000Z'
---

## MySQL Wire Protocol: DEPRECATE_EOF OK Packet Marker Fix

When sysbench (mysql-client 8.0.46 + libmysqlclient 8.0.46) fails with error 2027 "Malformed packet" during the DEPRECATE_EOF path, the server is likely sending the wrong packet marker.

### Root Cause

The DEPRECATE_EOF capability (0x01000000) changes how the server terminates result sets:

| Capability | Result-set terminator marker |
|---|---|
| DEPRECATE_EOF = 0 (classic) | EOF packet with 0xFE marker |
| DEPRECATE_EOF = 1 (MySQL 8.0+) | OK packet with 0x00 marker |

**Bug**: `make_deprecate_eof_ok_packet` was incorrectly using `p.push(0xfe)` — the EOF marker — instead of `p.push(0x00)` — the OK marker.

### Fix

**File**: `crates/mysql-server/src/lib.rs`

```rust
// WRONG (classic EOF marker):
p.push(0xfe);

// CORRECT (DEPRECATE_EOF OK packet marker):
p.push(0x00);
```

### Inter-record Separator Gap

`send_binary_result_set` had a similar gap: the inter-record separator between column definitions and the row stream only sent EOF, missing the DEPRECATE_EOF branch entirely. This also caused "Malformed packet" errors on prepared statements (COM_STMT_EXECUTE).

```rust
// Before: only EOF, missing DEPRECATE_EOF branch
if cap & capability::DEPRECATE_EOF == 0 {
    make_eof_packet(seq, 0x0002).write_to(w)?;
    seq = seq.wrapping_add(1);
}
// MISSING: else { make_deprecate_eof_ok_packet(...) }

// After: both branches present
if cap & capability::DEPRECATE_EOF == 0 {
    make_eof_packet(seq, 0x0002).write_to(w)?;
    seq = seq.wrapping_add(1);
} else {
    make_deprecate_eof_ok_packet(seq, 0, 0, 0x0002, 0).write_to(w)?;
    seq = seq.wrapping_add(1);
}
```

### Verification

Run the wire deprecate EOF tests:

```bash
cargo test --test wire_deprecate_eof_test -- --nocapture
```

Both `classic_caps_round_trip` and `deprecated_eof_caps_round_trip` must pass.

### Debug Checklist for MySQL Error 2027

1. **Check packet marker**: In the DEPRECATE_EOF path, the result-set terminator must use 0x00 (OK packet), not 0xFE (EOF packet)
2. **Check all DEPRECATE_EOF branches**: Ensure every place that conditionally sends EOF also has a corresponding DEPRECATE_EOF OK packet branch
3. **Check inter-record separator**: Between column definitions and row stream — must also honor DEPRECATE_EOF
4. **Check trailing terminator**: After the row stream — must also honor DEPRECATE_EOF
5. **Verify with test client**: Use `MySqlTestClient::connect_with_caps` with `CAP_DEPRECATE_EOF` to test the deprecated-EOF path

### Relevant Files

- `crates/mysql-server/src/lib.rs` — `make_deprecate_eof_ok_packet()`, `send_binary_result_set()`, `do_command_loop()`
- `tests/wire_deprecate_eof_test.rs` — End-to-end wire protocol tests
- `tests/common/mod.rs` — `CLIENT_CAPABILITIES` (default test client should NOT advertise DEPRECATE_EOF)
