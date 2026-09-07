## ADDED Requirements

### Requirement: INDEXED BY forces named index

`SELECT ... FROM table_name INDEXED BY index_name WHERE ...` MUST
direct the executor to use `index_name` for the scan of `table_name`.

#### Scenario: Hint applied

- GIVEN:
  ```
  CREATE TABLE t (id INT PRIMARY KEY, val INT);
  CREATE INDEX idx_val ON t(val);
  INSERT INTO t VALUES (1, 10), (2, 20), (3, 30);
  ```
- WHEN `SELECT * FROM t INDEXED BY idx_val WHERE val = 20`
- THEN the result row is `(2, 20)`.
- AND the executor uses `idx_val` for the scan (verifiable via
  instrumentation log or query plan output).

### Requirement: INDEXED BY errors on missing index

#### Scenario: Nonexistent index name

- WHEN `SELECT * FROM t INDEXED BY no_such_index WHERE id = 1`
- THEN the statement fails with `IndexNotFound: no_such_index`.

### Requirement: INDEXED BY errors on table mismatch

#### Scenario: Index belongs to a different table

- GIVEN `CREATE TABLE u (...); CREATE INDEX idx_u ON u(col);`
  AND table `t` exists separately
- WHEN `SELECT * FROM t INDEXED BY idx_u WHERE ...`
- THEN the statement fails with `IndexNotOwnedByTable: idx_u (table=u)
  cannot be used for table=t`.

### Requirement: NOT INDEXED forces sequential scan

`SELECT ... FROM table_name NOT INDEXED` MUST force the executor to
use a sequential scan of `table_name` even if other indexes are
defined.

#### Scenario: NOT INDEXED skips index

- GIVEN table `t` with index on `val`
- WHEN `SELECT * FROM t NOT INDEXED WHERE val = 20`
- THEN the row is returned correctly.
- AND the executor uses sequential scan (verifiable via
  instrumentation).

### Requirement: INDEXED BY does not change query results

`INDEXED BY` and the unhinted version MUST return identical result
sets when both succeed. The hint only changes the execution plan, not
the result.

#### Scenario: Same result with and without hint

- GIVEN table `t` with index on `val` and rows (1, 10), (2, 20), (3, 30)
- WHEN:
  ```
  SELECT * FROM t WHERE val >= 20;       -- (1)
  SELECT * FROM t INDEXED BY idx_val WHERE val >= 20;  -- (2)
  ```
- THEN (1) and (2) return the same rows: `(2, 20), (3, 30)`.

### Requirement: INDEXED BY in JOIN

`INDEXED BY` MUST apply only to the immediately preceding table in a
JOIN chain, not to other tables in the FROM clause.

#### Scenario: Hint applies to one table only

- GIVEN tables `t` and `u`, with index `idx_t` on `t(val)`
- WHEN:
  ```
  SELECT * FROM t INDEXED BY idx_t JOIN u ON t.id = u.id WHERE t.val = 20
  ```
- THEN the scan of `t` uses `idx_t`.
- AND the scan of `u` uses whatever plan the executor chooses (or
  another hint if specified).