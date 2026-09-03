## ADDED Requirements

### Requirement: Parser MUST accept `INSERT ... RETURNING col_list`

The parser MUST accept `INSERT INTO ... RETURNING col1, col2, ...` (PostgreSQL/MySQL 8.0+ syntax) and store the column list in `InsertStatement.returning`. The executor MUST then return one row per inserted row, projected to the requested columns. An empty row set (zero columns in the projection) is not allowed.

#### Scenario: basic RETURNING with explicit columns

- **WHEN** the parser is given `INSERT INTO t VALUES (1, 100) RETURNING id, val`
- **THEN** `parse(...)` returns `Statement::Insert(InsertStatement { returning: Some(["id", "val"]), .. })`

#### Scenario: RETURNING *

- **WHEN** the parser is given `INSERT INTO t VALUES (1, 100) RETURNING *`
- **THEN** `parse(...)` returns `Statement::Insert(InsertStatement { returning: Some(["*"]), .. })`

#### Scenario: RETURNING with ON CONFLICT

- **WHEN** the parser is given `INSERT INTO t (id) VALUES (1) ON CONFLICT (id) DO UPDATE SET val = 99 RETURNING id, val`
- **THEN** `parse(...)` returns `Ok(...)` and `returning` is `Some(["id", "val"])`

### Requirement: Executor MUST project inserted rows for RETURNING

When `InsertStatement.returning` is `Some(cols)`, the executor MUST return an `ExecutorResult` with one row per inserted row, projected to the requested columns. For `cols = ["*"]`, project every column of the table. For non-existent columns, use `Value::Null`.

#### Scenario: RETURNING with explicit columns

- **WHEN** the executor runs:
  1. `CREATE TABLE t(id INT, val INT)`
  2. `INSERT INTO t VALUES (1, 100) RETURNING id, val`
- **THEN** the result MUST contain exactly one row with `id = 1`, `val = 100`

#### Scenario: RETURNING with multi-row insert

- **WHEN** the executor runs:
  1. `CREATE TABLE t(id INT, val INT)`
  2. `INSERT INTO t VALUES (1, 100), (2, 200) RETURNING id`
- **THEN** the result MUST contain two rows with `id = 1` and `id = 2`

#### Scenario: RETURNING * projects all columns

- **WHEN** the executor runs:
  1. `CREATE TABLE t(id INT, val INT)`
  2. `INSERT INTO t VALUES (1, 100) RETURNING *`
- **THEN** the result MUST contain one row with two columns `id = 1`, `val = 100`

#### Scenario: INSERT without RETURNING returns empty (regression check)

- **WHEN** the executor runs:
  1. `CREATE TABLE t(id INT, val INT)`
  2. `INSERT INTO t VALUES (1, 100)`
- **THEN** the result MUST have zero rows (existing behaviour preserved)
