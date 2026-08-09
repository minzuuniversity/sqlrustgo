# MySQL Compatibility Status — v3.12.0

**Branch:** `swe/_fix/binary-row-parsing`
**Date:** 2026-08-09

---

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
| `COM_RESET_CONNECTION` | ⚠️ Partial | Server returns "Unknown command"; client handles gracefully |
| `LOAD DATA INFILE` | ✅ Documented | Gate script passes |
| VARCHAR space-padding | ✅ Handled | `trim_end()` strips MySQL-padded spaces |

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
| `LOAD DATA LOCAL INFILE` | 🔜 Deferred | Client-side file reading not implemented |

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
