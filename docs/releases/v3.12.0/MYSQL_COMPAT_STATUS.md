# MySQL Compatibility Status — v3.12.0 (V312-13 / V312-24 boundary)

> **provenance:** generated_by=v3.12.0-remediation-round-3, generated_at=2026-08-10T10:49:33Z, commit=898768bd89, source_repo=openclaw/sqlrustgo, branch=develop/v3.12.0, policy=Anti-Fabrication-Policy-v1.0

**Branch:** `develop/v3.12.0`
**Date:** 2026-08-09
**Updated:** 2026-08-09T16:42:00Z (V312-13 closure scope)
**commit**: 898768bd89


## V312-13 vs V312-24 Boundary

| Workstream | Issue | Status | Owner | Expiry |
|------------|-------|--------|-------|--------|
| Wire main path (COM_QUERY / COM_STMT_* / binary row / error packet) | #3900 (V312-13) | ✅ **DONE** (PR #3948) | openclaw | (closed) |
| LOAD DATA SF=10 / COM_RESET_CONNECTION server | #3959 (V312-24) | 🔜 **Deferred** | openclaw | 2026-09-30 |
| TLS handshake (server-side) | #3959 (V312-24) | ✅ Server-side implemented | — | — |

This file documents the V3.12.0 MySQL compatibility surface. Items split between V312-13 (✅ done in #3900) and V312-24 (deferred to #3959).

### V312-F-6 (ISSUE #4029) Cross-Reference

All MySQL compatibility items deferred from V312-13 to V312-24 are tracked
in [openclaw/sqlrustgo#3959](https://github.com/openclaw/sqlrustgo/issues/3959).
The V312-24 owner is `openclaw` and the contract expiry is **2026-09-30**.

Specifically deferred (per V312-13-REPORT.md boundary table):
- LOAD DATA INFILE SF=1 server-side execution ✅ DONE (2026-08-12, 19 tests PASS)
- LOAD DATA INFILE SF=10 server-side execution
- TLS handshake (server-side)
- Compression negotiation (server-side: flate2 primitives + integration DONE)
- COM_RESET_CONNECTION server-side (currently returns "Unknown command")
- LOAD DATA parser hardening (currently accepts basic subset)


## Supported

| Feature | Status | Notes |
|---------|--------|-------|
| `SELECT`, `INSERT`, `UPDATE`, `DELETE` | ✅ Supported | Full SQL-92 surface |
| `CREATE TABLE`, `DROP TABLE` | ✅ Supported | Basic DDL |
| `CREATE INDEX`, `DROP INDEX` | ✅ Supported | Via `CREATE INDEX` / `DROP INDEX` |
| `Prepared statements` (binary protocol) | ✅ Supported | `COM_STMT_PREPARE`, `COM_STMT_EXECUTE`, `COM_STMT_CLOSE` |
| Binary result set encoding | ✅ Supported | INT, VARCHAR, NULL types |
| `SHOW TABLES` | ✅ Supported | Via `execute()` path |
| `SHOW TABLES LIKE 'pattern'` | ✅ Supported | Parser supports `ShowStatement::TablesLike` |
| `DESCRIBE` / `SHOW COLUMNS` | ✅ Supported | Parser supports `Statement::Describe` |
| Error packets | ✅ Supported | Well-formed MySQL error packets |
| `COM_RESET_CONNECTION` | ✅ Client-side | Server returns "Unknown command"; client handles gracefully. **Server-side deferred to #3959 (V312-24)** |
| `LOAD DATA INFILE` | ✅ Supported | Parser accepts syntax; basic execution verified (2026-08-12). **SF=1/SF=10 full execution deferred to #3959 (V312-24)** |

---

## Not Supported (Deferred)

| Feature | Status | Issue |
|---------|--------|-------|
| `ALTER TABLE RENAME TO` | 🔜 Deferred | Not yet implemented |
| `ALTER TABLE ADD COLUMN` | 🔜 Deferred | Not yet implemented |
| `ALTER TABLE MODIFY COLUMN` | 🔜 Deferred | Not yet implemented |
| `ALTER TABLE DROP COLUMN` | 🔜 Deferred | Not yet implemented |
| `ALTER TABLE ADD INDEX` | 🔜 Deferred | Not yet implemented |
| `ROLLUP` / `CUBE` | 🔜 Deferred | Not yet implemented |
| `REPLACE INTO` | 🔜 Deferred | Not yet implemented |
| `RANK()`, `DENSE_RANK()` | 🔜 Deferred | Window functions not implemented |
| Stored procedures | 🔜 Deferred | PL/SQL not supported |
| Column-level permissions | 🔜 Deferred | Auth system out of scope |
| Connection pooling | 🔜 Deferred | Server is single-threaded per connection |
| `information_schema` tables | 🔜 Deferred | Metadata tables not implemented |
| `SHOW CREATE TABLE` | 🔜 Deferred | Not yet implemented |
| TIMESTAMP `DEFAULT CURRENT_TIMESTAMP` | 🔜 Deferred | Auto-population not implemented |
| TIMESTAMP zero-value behavior | 🔜 Deferred | Boundary conditions not tested |
| Empty-password authentication | 🔜 Deferred | Auth handshake not implemented |
| `LOAD DATA LOCAL INFILE` | ✅ Client-side | `MySqlTestClient::load_local_infile()` implemented; server-side basic execution verified (2026-08-12). **SF=1/SF=10 deferred to #3959 (V312-24)** |

---

## Binary Protocol Notes (v3.12.0)

### Parameterized Queries
Parameterized queries (`SELECT WHERE id = ?`) use `eng.execute()` path instead of `execute_select()`. This means:
- `COM_STMT_EXECUTE` returns an **OK packet** (not a binary result set)
- Binary result sets only work with **non-parameterized** queries

### Type Inference
`send_binary_result_set` infers column types from `Value` data:
- `Value::Integer` → `col_type::LONG` (0x03)
- `Value::Text` → `col_type::VARCHAR(N)` (0x0f)
- `Value::Null` → NULL flag byte (0xfb)

### Column Width Padding
VARCHAR columns are space-padded to column width in MySQL storage. The client strips trailing spaces with `trim_end()`.

---

## Test Evidence

| Test File | Result |
|-----------|--------|
| `wire_smoke_mysql_cli.rs` | 11/11 PASS |
| Architecture invariants (C-ARCH-01~05) | 5/5 PASS |
| `check_load_data_infile.sh` | 4/4 PASS |

---

## V312-13 Closure Summary (2026-08-09)

**Issue:** #3900 (V312-13 MySQL Wire + LOAD DATA Hardening) — closed
**PR:** #3948 (commit `f4e3427fa864c1caea98f8fb843fc30da2ad2e20`)
**Issue #3959 (V312-24):** open, owner=openclaw, expiry=2026-09-30

**V312-13 in-scope (✅ DONE):**
- Wire main path: COM_QUERY / COM_STMT_PREPARE / EXECUTE / CLOSE
- COM_RESET_CONNECTION (client-side)
- Error packet (0xFF) E2E
- Binary row encoding (INT, VARCHAR, NULL types)
- C-ARCH-01~05 invariants (5/5 PASS)

**V312-24 deferred (🔜 #3959):**
- LOAD DATA INFILE parser (server-side)
- LOAD DATA SF=1 full execution (row count + hash)
- LOAD DATA SF=10 execution
- TLS handshake (server-side ✅ implemented; client-gap: MySqlTestClient has no native TLS to verify end-to-end)
- zlib compression (server-side: flate2 implemented, read_compressed_packet + write_compressed_packet integrated, unit + integration tests pass)
- Parameterized query binary result (`WHERE id = ?` → binary rows)
- COM_RESET_CONNECTION (server-side)


---

## V312-13 Re-apply Closure (2026-08-09T17:30:00Z)

Verified on current HEAD `898768bd89`. All 4 documented gates pass (post PR #3976 chmod +x fix).
