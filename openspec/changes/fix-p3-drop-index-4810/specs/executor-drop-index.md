## ADDED Requirements

### Requirement: DROP INDEX removes existing index

`sqlrustgo` MUST accept `DROP INDEX index_name` and remove the named
index from the catalog when it exists.

#### Scenario: Basic drop

- GIVEN `CREATE INDEX idx_a ON t(col)` has been executed
- WHEN `DROP INDEX idx_a` is invoked
- THEN the index is removed.
- AND a subsequent query that uses `t(col)` falls back to a sequential
  scan (no longer uses the dropped index).

#### Scenario: Drop already-dropped index errors

- GIVEN `idx_a` has been dropped already
- WHEN `DROP INDEX idx_a` is invoked without `IF EXISTS`
- THEN the statement fails with `IndexNotFound`.

### Requirement: DROP INDEX IF EXISTS is idempotent

`DROP INDEX IF EXISTS index_name` MUST NOT raise an error if the index
does not exist. (Optional warning is acceptable.)

#### Scenario: IF EXISTS with nonexistent index

- WHEN `DROP INDEX IF EXISTS no_such_index` is invoked
- THEN the statement succeeds (no error).

### Requirement: DROP TABLE cascades indexes

`DROP TABLE table_name` MUST also remove all indexes defined on that
table.

#### Scenario: Index removed with table

- GIVEN table `t` with index `idx_t` on `t(col)`
- WHEN `DROP TABLE t` is invoked
- THEN `idx_t` is removed from the catalog.
- AND a subsequent `DROP INDEX idx_t` fails with `IndexNotFound`.

### Requirement: DROP INDEX does not affect other tables

Dropping an index on `t` MUST NOT affect any index on other tables.

#### Scenario: Independent indexes

- GIVEN tables `t` and `u` with indexes `idx_t` and `idx_u`
- WHEN `DROP INDEX idx_t` is invoked
- THEN `idx_u` still exists and is queryable via indexed scan on `u`.