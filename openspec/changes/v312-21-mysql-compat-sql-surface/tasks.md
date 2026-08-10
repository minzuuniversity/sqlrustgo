## V312-21 MySQL Compatibility SQL Surface

### 1. Documentation
- [ ] 1.1 Create `docs/releases/v3.12.0/MYSQL_COMPAT_STATUS.md` skeleton with unsupported items

### 2. SHOW TABLES / Metadata
- [ ] 2.1 Create `crates/sqlrustgo/tests/mysql_compat_show_tables.rs`
- [ ] 2.2 Test `SHOW TABLES` returns correct table list
- [ ] 2.3 Test `SHOW TABLES LIKE 'pattern'` filtering
- [ ] 2.4 Test `SHOW CREATE TABLE t`
- [ ] 2.5 Test `SHOW COLUMNS FROM t` / `DESCRIBE t`
- [ ] 2.6 Test `information_schema.tables` and `information_schema.columns`

### 3. Empty-Password Auth Edge
- [ ] 3.1 Test user with empty password can authenticate
- [ ] 3.2 Test wrong password fails authentication

### 4. Prepared Statements
- [x] 4.1 Test PREPARE / EXECUTE / DEALLOCATE cycle with INT params (wire_smoke_stmt_prepare_execute_int)
- [x] 4.2 Test PREPARE / EXECUTE with VARCHAR params (wire_smoke_stmt_prepare_execute_varchar)
- [x] 4.3 Test NULL parameter handling (wire_smoke_stmt_prepare_null)
- [x] 4.4 Test DEALLOCATE of non-existent prepared statement → error (wire_smoke_stmt_close_nonexistent)

### 5. ALTER TABLE
- [ ] 5.1 Create `crates/sqlrustgo/tests/mysql_compat_alter_table.rs`
- [ ] 5.2 Test `ALTER TABLE t RENAME TO t2`
- [ ] 5.3 Test `ALTER TABLE t ADD COLUMN c INT`
- [ ] 5.4 Test `ALTER TABLE t MODIFY COLUMN c VARCHAR(100)`
- [ ] 5.5 Test `ALTER TABLE t DROP COLUMN c`
- [ ] 5.6 Test `ALTER TABLE t ADD INDEX idx(c)`

### 6. TIMESTAMP Boundary
- [ ] 6.1 Create `crates/sqlrustgo/tests/mysql_compat_timestamp.rs`
- [ ] 6.2 Test TIMESTAMP DEFAULT CURRENT_TIMESTAMP auto-population
- [ ] 6.3 Test TIMESTAMP zero-value behavior
- [ ] 6.4 Document boundary conditions in MYSQL_COMPAT_STATUS.md

### 7. Unsupported Items
- [ ] 7.1 Document ROLLUP/CUBE as `deferred`
- [ ] 7.2 Document REPLACE INTO as `deferred`
- [ ] 7.3 Document RANK()/DENSE_RANK() as `deferred`
- [ ] 7.4 Document stored procedure tokens as `deferred`
- [ ] 7.5 Document column-level permissions as `deferred`
- [ ] 7.6 Document connection pool as `deferred`
- [ ] 7.7 Finalize `docs/releases/v3.12.0/MYSQL_COMPAT_STATUS.md`
