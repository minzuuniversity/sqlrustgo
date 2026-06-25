---
name: mysql-server-wire-protocol-fix
description: Fix MySQL wire protocol issues in mysql-server crate: column definition packet byte ordering, COM_QUERY SELECT routing, and protocol compatibility with MySQL clients
source: auto-skill
extracted_at: '2026-06-21T10:30:00.000Z'
---

## MySQL Wire Protocol Fixes in mysql-server Crate

When MySQL clients (mysql-client, sysbench, etc.) fail with protocol errors while connecting to sqlrustgo, the issues are typically in one of these areas: column definition packet byte ordering, COM_QUERY result-set routing, or EOF/OK packet markers.

### Fix 1: Column Definition Packet Byte Ordering

When MySQL clients fail to parse column definitions (e.g., "Malformed packet"), the column definition packet may have incorrect byte ordering for `charset_collation` and `flags` fields.

**Root cause**: The column definition packet uses Little-Endian byte ordering, and the `charset_collation` (2 bytes) and `flags` (2 bytes) fields must be written in the correct byte order.

**File**: `crates/mysql-server/src/lib.rs`

```rust
// WRONG (big-endian or swapped):
// charset_collation: 0x3000 (utf8_general_ci written backwards)
// flags: 0x0002 (written with wrong byte order)

// CORRECT (little-endian):
// charset_collation: 0x0030 (utf8_general_ci in little-endian)
// flags: 0x0000 (no flags set)
```

**Key values**:
- `charset_collation`: `0x0030` = utf8_general_ci (little-endian)
- `flags`: `0x0000` = no flags set (must be zero, not 0x0002 which was the previous incorrect value)

**Debug checklist**:
1. Compare column definition packet against MySQL's actual binary protocol (Wireshark capture of MySQL server)
2. Verify `charset_collation` is in little-endian byte order (0x0030, not 0x3000)
3. Verify `flags` is 0x0000 (no flags set for standard utf8_general_ci columns)
4. Test with both mysql-client and sysbench

### Fix 2: COM_QUERY SELECT Result-Set Routing

When `COM_QUERY` is sent to the server, the response must differ based on whether the query is a SELECT or a non-SELECT (INSERT/UPDATE/DELETE/DDL):

| Query type | Response |
|---|---|
| SELECT | Result-set packets: column count → column definitions → EOF → rows |
| Non-SELECT (INSERT/UPDATE/DELETE/DDL) | OK packet (0x00) |

**Root cause**: Without proper routing, both SELECT and non-SELECT queries may send the same response type, causing the client to fail.

**File**: `crates/mysql-server/src/lib.rs`

```rust
// Before: no routing, both types sent as OK packet
let mut p = make_ok_packet(seq, affected_rows, 0, 0, 0);

// After: route based on query type
let is_select = query.trim_start().to_ascii_uppercase().starts_with("SELECT");
if is_select {
    // Send result-set packet: column count → column definitions → EOF → rows
} else {
    // Send OK packet for non-SELECT
}
```

**Debug checklist**:
1. Test SELECT with `mysql-client`: verify result-set packets are received correctly
2. Test INSERT/UPDATE/DELETE with `mysql-client`: verify OK packet is received
3. Test DDL (CREATE TABLE) with `mysql-client`: verify OK packet is received
4. Test with sysbench: verify no protocol errors during DML operations
5. Test prepared statements (COM_STMT_PREPARE + COM_STMT_EXECUTE): verify result-set routing

### Fix 3: EOF vs OK Packet Markers (DEPRECATE_EOF)

This is covered by the `auto-skill-deprecate-eof-fix` skill. When a client fails with error 2027 "Malformed packet" on the DEPRECATE_EOF path, check that OK packets use 0x00 marker, not 0xFE.

### Fix 4: Make helper functions pub for testing

When writing wire protocol tests, the helper functions `read_lenenc_int` and `read_lenenc_str` need to be `pub` for test access.

**File**: `tests/common/mod.rs`

```rust
// Before:
fn read_lenenc_int(data: &[u8]) -> Result<u64> { ... }
fn read_lenenc_str(data: &[u8]) -> Result<String> { ... }

// After:
pub fn read_lenenc_int(data: &[u8]) -> Result<u64> { ... }
pub fn read_lenenc_str(data: &[u8]) -> Result<String> { ... }
```

### Verification

Run the wire protocol tests:

```bash
# Run wire deprecate EOF tests
cargo test --test wire_deprecate_eof_test -- --nocapture

# Run multi-statement tests
cargo test --test multi_statement_test -- --nocapture

# Run all mysql-server tests
cargo test --test mysql_server_test -- --nocapture
```

### Relevant Files

- `crates/mysql-server/src/lib.rs` — Column definition packets, COM_QUERY routing, OK/EOF packets
- `tests/common/mod.rs` — Helper functions for wire protocol testing
- `tests/multi_statement_test.rs` — Multi-statement wire protocol tests
- `tests/wire_deprecate_eof_test.rs` — DEPRECATE_EOF wire protocol tests
