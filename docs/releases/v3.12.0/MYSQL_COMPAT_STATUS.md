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
| `SHOW FULL TABLES` | ✅ Supported | Returns `Name` + `Type` (`BASE TABLE` / `VIEW`) columns; supports `FROM db`, `LIKE 'pattern'`, `WHERE expr` filters. V312-59-A / #4384 (56A-R3 anti-deferral). |
| `SHOW TABLE STATUS` | ✅ Supported (controlled subset) | Returns MySQL 18 columns (Name, Engine, Version, Row_format, Rows, Avg_row_length, Data_length, Max_data_length, Index_length, Data_free, Auto_increment, Create_time, Update_time, Check_time, Collation, Checksum, Create_options, Comment); supports `FROM db`, `LIKE 'pattern'`, `WHERE expr`. `Rows` is the real scanned row count; Engine=InnoDB, Version=10, Row_format=Dynamic, Collation=utf8mb4_general_ci. V312-59-A / #4384 (56A-R3 anti-deferral). |
| `DESCRIBE` / `SHOW COLUMNS` | ✅ Supported | Both `DESCRIBE <table>` and `SHOW COLUMNS FROM <table> [LIKE 'pattern']` return Field/Type/Null/Key/Default/Extra rows. V312-56A / #4251. |
| `SHOW INDEX FROM <table>` | ✅ Supported (controlled subset) | Returns MySQL 5.7 SHOW INDEX columns. Catalog-driven when wired up; falls back to PK from `ColumnDefinition.primary_key` for storage-only engines. V312-56A / #4251. |
| `SHOW CREATE TABLE <table>` | ✅ Supported (controlled subset) | Reconstructs `CREATE TABLE <name> (col TYPE [NOT NULL] [DEFAULT ...], ..., PRIMARY KEY (...))` from the live schema. DEFAULT depends on `ColumnDefinition.default_value` being populated (#4154 follow-up for parser-level DEFAULT extraction). V312-56A / #4251. |
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
| Stored procedures / triggers | ⚠️ Partial / P0 remediation | Parser、catalog、`CALL`、`CREATE TRIGGER` 和 DML hook 局部存在；v3.12 V312-55 要求补齐基础受控子集和 `check_v312_procedure_trigger_gate.sh`，未通过前不得宣称 MySQL-style 生产兼容 |
| Column-level permissions | 🔜 Deferred | Auth system out of scope |
| Connection pooling | 🔜 Deferred | Server is single-threaded per connection |
| `information_schema` SQL path (`SELECT FROM information_schema.tables/columns/indexes`) | ⚠️ Explicit-unsupported | Parser rejects the dot-qualified schema prefix; executor has no virtual catalog scan. The `crates/information-schema/` library exposes `InformationSchema::get_tables/columns/indexes` directly. V312-56A / #4251 acceptance: explicit-unsupported branch satisfied. |
| `SHOW CREATE TABLE` | ✅ Moved to Supported | Moved to "Supported" section above (V312-56A / #4251). |
| TIMESTAMP `DEFAULT CURRENT_TIMESTAMP` | 🔜 Deferred | Auto-population not implemented |
| TIMESTAMP zero-value behavior | 🔜 Deferred | Boundary conditions not tested |
| Empty-password authentication | 🔜 Deferred | Auth handshake not implemented |
| `LOAD DATA LOCAL INFILE` | ✅ Client-side | `MySqlTestClient::load_local_infile()` implemented; server-side basic execution verified (2026-08-12). **SF=1/SF=10 deferred to #3959 (V312-24)** |

---

## V312-56F / V312-56G Disposition (2026-08-17)

| Feature | Status | Notes |
|---------|--------|-------|
| `CREATE VIEW` (definition storage) | ✅ Supported (definition-only) | Parser + storage layer. View expansion (SELECT * from view) not implemented. V312-56F / #4256. |
| `CREATE FULLTEXT INDEX` | ✅ Supported (parser + storage) | Parser + `FullTextIndex` storage layer. SQL `MATCH/AGAINST` **NOT** wired — GMP uses BM25-like via `crates/vector_retrieval`. V312-56G / #4257. |
| `TABLE PARTITION BY` (RANGE/LIST/HASH) | 🔜 Deferred | Not implemented in v3.12. GMP uses `HashPartitioner` for vector sharding (separate abstraction). V312-56G / #4257. |
| `WITH RECURSIVE` (CTE materialization) | 🔜 Deferred → v3.13 | CTE materialization supported for non-recursive; recursive CTE returns explicit error. V312-56F / #4256. |
| `MERGE` statement | 🔜 Deferred → v3.13 | Returns "MERGE not yet supported via execute()"; `LocalExecutorDml` path may support in v3.13. V312-56F / #4256. |

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
