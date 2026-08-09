# V312-21 Design

## Test File Locations

- `crates/sqlrustgo/tests/mysql_compat_show_tables.rs`
- `crates/sqlrustgo/tests/mysql_compat_auth_edge.rs`
- `crates/sqlrustgo/tests/mysql_compat_alter_table.rs`
- `crates/sqlrustgo/tests/mysql_compat_timestamp.rs`

## Approach

### SHOW TABLES
- Use `LocalExecutor` directly (no wire protocol needed for unit-level verification)
- Execute `SHOW TABLES`, verify returns list
- Execute `SHOW TABLES LIKE 't%'`, verify filtered result
- Execute `SHOW CREATE TABLE t`, verify DDL string
- Execute `SHOW COLUMNS FROM t`, verify column metadata

### Empty-Password Auth Edge
- Use `mysql::远方::Client` or raw TCP handshake
- Create user with empty password via `CREATE USER ''@'%'`
- Attempt authentication with empty password → should succeed
- Attempt authentication with wrong password → should fail

### Prepared Statements
- Use `mysql::远方::prepared` or raw `COM_STMT_PREPARE` packets
- Test INT, VARCHAR, NULL parameter types
- Test `DEALLOCATE PREPARE` on non-existent statement → error

### ALTER TABLE
- Execute each DDL via `LocalExecutor::execute`
- Verify table metadata before/after via `SHOW CREATE TABLE` or schema query
- All 5 operations: RENAME, ADD, MODIFY, DROP, ADD INDEX

### TIMESTAMP
- Create table with TIMESTAMP column
- Insert row without value, verify auto-population
- Test zero-value handling
- Note: SQLRustGo uses Unix epoch internally; document the boundary

## Documentation

- Create `docs/releases/v3.12.0/MYSQL_COMPAT_STATUS.md` with:
  - Supported features with PASS evidence
  - Explicitly unsupported features with deferred/retired decision
  - No unbounded claims
