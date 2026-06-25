# Wired Soak Test — 4 Critical Bugs Found

> **Status**: ✅ RESOLVED (2026-06-25)
> **Severity**: P0 — These bugs caused real failures in wired soak tests.
> **Related**: #3225 (wired 24h soak), #3229 (wired 168h soak), TPC-H wire test `#[ignore]`

---

## Overview

All 4 bugs have been resolved. The wired soak test infrastructure was upgraded to use real E2E (`sqlrustgo-mysql-server` + `sysbench`) instead of the in-process compressed-time simulation.

---

## Bug 1: INSERT Payload Truncation (COM_QUERY — VALUES content lost) — ✅ RESOLVED

### Fix

- **Commit**: `66c1d84ca8` — `fix(mysql-server): real multi-statement COM_QUERY execution + split_sql_statements`
- **PR**: #3592 (merged via feature/soak-bug3-create-database-fix)

### Resolution

The `split_sql_statements` function now correctly handles multi-statement SQL, preserving full payload content. The root cause was in how the server split multi-statement queries — content after the first statement was being dropped.

---

## Bug 2: sysbench prepare — bulk INSERT fails with Lost Connection — ✅ RESOLVED

### Fix

- **Commit**: `20810b9b89` — `fix(soak): G7 gate rewritten as wired E2E (sqlrustgo-mysql-server + sysbench)`
- **PR**: #3581 (merged via feature/wired-soak-4-bugs-analysis)

### Resolution

- `TlsStream::write` and `TlsStream::flush` now drain the **full** rustls cipher buffer in a loop with `WouldBlock` break (not a single `complete_io` per call). The single-shot pattern left partial TLS records in the writer buffer on multi-batch INSERTs, deadlocking the client.
- The G7 Soak Test gate was rewritten as a true wired E2E test using real `sqlrustgo-mysql-server` + `sysbench oltp_read_write` over the MySQL wire protocol.
- The in-process compressed-time simulation in `tests/soak_test.rs` was **decommissioned** (all 10 tests `#[ignore]`); the gate now validates real behavior.

### Known Follow-up (separate issue)

> Engine memory growth on bulk INSERT prepare (`table_size > ~25` OOMs the server). Separate bug in execution engine, tracked separately (not in scope for this issue).

---

## Bug 3: CREATE DATABASE not supported — ✅ RESOLVED

### Fix

- **Commit**: `8975b2a7a4` — `fix: add null-byte separator to MySQL error packets + CREATE/DROP DATABASE parsing`
- **Commit**: `345bc43629` — `fix(executor): add CREATE DATABASE / DROP DATABASE support (Bug 3, issue #3577)`
- **PR**: #3592 (merged via feature/soak-bug3-create-database-fix)

### Resolution

- Added `Token::Database` variant to the lexer
- Added `CreateDatabase` and `DropDatabase` variants to the `Statement` enum
- Added grammar rules for `CREATE DATABASE` and `DROP DATABASE`
- Added execution logic for both statements

---

## Bug 4: Error packet format bug — missing null-byte after SQL state — ✅ RESOLVED

### Fix

- **Commit**: `8975b2a7a4` — `fix: add null-byte separator to MySQL error packets + CREATE/DROP DATABASE parsing`
- **PR**: #3592 (same commit as Bug 3)

### Resolution

Added `p.push(0x00);` after the SQL state in `make_err_packet` in `crates/mysql-server/src/lib.rs`. The error packet now correctly uses format: `0xFF + error_code (u16) + 0x23 + SQL_STATE (5 bytes) + 0x00 + ERROR_MESSAGE`.

---

## Summary

| Bug | Status | Commit | PR |
|-----|--------|--------|-----|
| Bug 1 (INSERT truncation) | ✅ Resolved | `66c1d84ca8` | #3592 |
| Bug 2 (sysbench connection drop) | ✅ Resolved | `20810b9b89` | #3581 |
| Bug 3 (CREATE DATABASE) | ✅ Resolved | `345bc43629` | #3592 |
| Bug 4 (error packet null-byte) | ✅ Resolved | `8975b2a7a4` | #3592 |

All 4 bugs are resolved as of commits above (2026-06-21). This document is kept for historical record.
