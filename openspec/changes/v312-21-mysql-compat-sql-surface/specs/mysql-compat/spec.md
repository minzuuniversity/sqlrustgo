# V312-21 MySQL Compatibility SQL Surface Spec

## SHOW TABLES / Metadata

- `SHOW TABLES` returns table list from current database
- `SHOW TABLES LIKE 'pattern'` supports LIKE pattern
- `SHOW CREATE TABLE t` returns DDL
- `SHOW COLUMNS FROM t` / `DESCRIBE t` returns column metadata
- `information_schema.tables` / `information_schema.columns` queryable

## Empty-Password Auth Edge

- User with empty password can authenticate and connect
- User with non-empty password cannot connect with empty password
- Auth failure returns proper MySQL error

## Prepared Statements

- `PREPARE` / `EXECUTE` / `DEALLOCATE PREPARE` cycle works
- Placeholder substitution for INT, VARCHAR, NULL
- Prepared statement persists across sessions (different connection)
- Error on DEALLOCATE non-existent prepared statement

## ALTER TABLE

- `ALTER TABLE t RENAME TO t2` — rename table
- `ALTER TABLE t ADD COLUMN c INT` — add column
- `ALTER TABLE t MODIFY COLUMN c VARCHAR(100)` — modify column type
- `ALTER TABLE t DROP COLUMN c` — drop column
- `ALTER TABLE t ADD INDEX idx(c)` — add index
- All operations verified with SELECT before/after

## TIMESTAMP

- `CREATE TABLE t (ts TIMESTAMP DEFAULT CURRENT_TIMESTAMP)`
- Insert without value → auto-populated
- `TIMESTAMP` vs `DATETIME` behavior difference on zero value
- Range: '1970-01-01 00:00:01' UTC to '2038-01-19 03:14:07' UTC

## Explicitly Unsupported (Documented in MYSQL_COMPAT_STATUS.md)

- ROLLUP/CUBE — deferred
- REPLACE INTO — deferred
- RANK()/DENSE_RANK() — deferred
- stored procedure tokens — deferred
- column-level permissions — deferred
- connection pool — deferred

## Acceptance Criteria

- GMP/production path subset has fixture PASS evidence
- Non-target items have explicit `unsupported` or `deferred` decision in `docs/releases/v3.12.0/MYSQL_COMPAT_STATUS.md`
- No unbounded claims in release notes
