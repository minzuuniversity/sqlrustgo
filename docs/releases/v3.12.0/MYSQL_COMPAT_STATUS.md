# MySQL Compatibility Status — v3.12.0

**Branch:** `develop/v3.12.0`
**Date:** 2026-08-10
**Commit:** `941a63dbd` (PR #3948, #3955, #3956 merged)

---

## Supported

| Feature | Status | Evidence |
|---------|--------|----------|
| `SELECT`, `INSERT`, `UPDATE`, `DELETE` | ✅ Supported | Full SQL-92 surface |
| `CREATE TABLE`, `DROP TABLE` | ✅ Supported | Basic DDL |
| `CREATE INDEX`, `DROP INDEX` | ✅ Supported | Via `CREATE INDEX` / `DROP INDEX` |
| Prepared statements (binary protocol) | ✅ Supported | `COM_STMT_PREPARE`, `COM_STMT_EXECUTE`, `COM_STMT_CLOSE` |
| Binary result set encoding (INT/VARCHAR/NULL) | ✅ Supported | Fixed in PR #3948 |
| `SHOW TABLES` | ✅ Supported | Via `execute()` path |
| `SHOW TABLES LIKE 'pattern'` | ✅ Supported | Parser supports `ShowStatement::TablesLike` |
| `DESCRIBE` / `SHOW COLUMNS` | ✅ Supported | Parser supports `Statement::Describe` |
| Error packets | ✅ Supported | Well-formed MySQL error packets |
| VARCHAR space-padding | ✅ Handled | `trim_end()` strips MySQL-padded spaces |
| Wire smoke tests | ✅ Supported | `wire_smoke_mysql_cli.rs` 11/11 PASS |

---

## Deferred Items (with tracking)

### V312-13 Scope: LOAD DATA + TLS

| Feature | Decision | Owner | Expiry | Tracking |
|---------|----------|-------|--------|----------|
| LOAD DATA INFILE parser | 🔜 Deferred | V312-24 (Test Infrastructure) | 2026-09-30 | New issue needed |
| LOAD DATA SF=1 full execution (row count + hash) | 🔜 Deferred | V312-24 | 2026-09-30 | New issue needed |
| LOAD DATA SF=10 | 🔜 Deferred | V312-24 | 2026-09-30 | New issue needed |
| TLS handshake server-side | 🔜 Deferred | V312-24 | 2026-09-30 | New issue needed |
| zlib compression | 🔜 Deferred | V312-24 | 2026-09-30 | New issue needed |
| Parameterized query binary result set | 🔜 Deferred | V312-24 | 2026-09-30 | New issue needed |
| COM_RESET_CONNECTION server-side | 🔜 Deferred | V312-24 | 2026-09-30 | New issue needed |

### V312-21 Scope: MySQL Compatibility Surface

| Feature | Decision | Owner | Expiry | Tracking |
|---------|----------|-------|--------|----------|
| `ALTER TABLE` (RENAME/MODIFY/ADD/DROP) | 🔜 Deferred | V312-21 backlog | 2026-09-30 | `v312-21-mysql-compat-sql-surface-backlog` |
| `information_schema` tables | 🔜 Deferred | V312-21 backlog | 2026-09-30 | `v312-21-mysql-compat-sql-surface-backlog` |
| `TIMESTAMP DEFAULT CURRENT_TIMESTAMP` | 🔜 Deferred | V312-21 backlog | 2026-09-30 | `v312-21-mysql-compat-sql-surface-backlog` |
| Empty-password authentication | 🔜 Deferred | V312-21 backlog | 2026-09-30 | `v312-21-mysql-compat-sql-surface-backlog` |
| `SHOW CREATE TABLE` | 🔜 Deferred | V312-21 backlog | 2026-09-30 | `v312-21-mysql-compat-sql-surface-backlog` |

### Known Unsupported (No Implementation Planned)

| Feature | Decision | Notes |
|---------|----------|-------|
| Stored procedures | ❌ Not supported | PL/SQL not supported |
| Window functions (`RANK`, `DENSE_RANK`) | ❌ Not supported | Window functions not implemented |
| Column-level permissions | ❌ Not supported | Auth system out of scope |
| Connection pooling | ❌ Not supported | Server is single-threaded per connection |
| `LOAD DATA LOCAL INFILE` | ❌ Not supported | Client-side file reading not implemented |
| `ROLLUP` / `CUBE` | ❌ Not supported | Not in SQL-92 |
| `REPLACE INTO` | ❌ Not supported | Not implemented |

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
| `wire_smoke_mysql_cli.rs` | 11/11 PASS (PR #3948) |
| Architecture invariants (C-ARCH-01~05) | 5/5 PASS |
| `check_load_data_infile.sh` | 4/4 PASS |
| `check_anti_fabrication.sh` | ERRORS=0, WARNINGS=3, PASS |

---

## V312-13 Closure Scope

V312-13 (#3900) closes with the following deliverables:

**Completed:**
- ✅ Binary row parsing fix (INT/VARCHAR/NULL type matching)
- ✅ Wire smoke tests (11/11 PASS)
- ✅ TPC-H SF=1 fixture generated (102.80 MB)
- ✅ Architecture invariants gate (5/5 PASS)
- ✅ Anti-fabrication gate (0 ERRORS)

**Deferred to V312-24:**
- LOAD DATA parser, SF=1/SF=10 execution
- TLS/compression
- Parameterized query binary result set
- COM_RESET_CONNECTION server-side
