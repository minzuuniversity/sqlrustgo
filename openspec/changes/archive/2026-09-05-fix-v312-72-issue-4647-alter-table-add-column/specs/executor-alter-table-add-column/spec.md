## ADDED Requirements

### Requirement: Executor MUST actually add a column AND backfill existing rows

`ALTER TABLE t ADD COLUMN <name> <type> [DEFAULT <value>]` MUST extend the table's schema (add the column to `TableInfo.columns`) AND backfill every existing row with the column's DEFAULT (or `Value::Null` when no default is specified). Subsequent `SELECT * FROM t` MUST return the new column for all existing rows. The persisted on-disk representation (for FileStorage) MUST reflect the new schema and backfilled rows.

#### Scenario: ADD COLUMN with DEFAULT backfills existing rows

- **WHEN** the executor runs:
  1. `CREATE TABLE t (id INT)`
  2. `INSERT INTO t VALUES (1), (2)`
  3. `ALTER TABLE t ADD COLUMN new_col INT DEFAULT 99`
  4. `SELECT * FROM t`
- **THEN** the result MUST contain 2 rows, each with 2 columns: `id = 1, new_col = 99` and `id = 2, new_col = 99`

#### Scenario: ADD COLUMN without DEFAULT backfills with NULL

- **WHEN** the executor runs:
  1. `CREATE TABLE t (id INT)`
  2. `INSERT INTO t VALUES (1)`
  3. `ALTER TABLE t ADD COLUMN new_col INT`
  4. `SELECT * FROM t`
- **THEN** the result MUST contain 1 row with 2 columns: `id = 1, new_col = NULL`

#### Scenario: ADD COLUMN is reflected in the persisted catalog

- **WHEN** the executor runs (via FileStorage):
  1. `CREATE TABLE t (id INT)` — persisted to `t.json`
  2. `INSERT INTO t VALUES (1)`
  3. `ALTER TABLE t ADD COLUMN new_col INT DEFAULT 5`
- **THEN** the on-disk `t.json` MUST contain both `id` and `new_col` columns in `columns`, and `rows[0]` MUST have 2 elements (`1` and `5`)

#### Scenario: SELECT * after ADD COLUMN shows new column (regression check)

- **WHEN** the executor runs the same commands as the first scenario
- **THEN** `SELECT id, new_col FROM t` MUST return 2 rows with `new_col = 99` each (proves the column is queryable, not just visible in `SELECT *`)
