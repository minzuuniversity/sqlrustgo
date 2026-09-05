# executor-sqlite-master-system-table Specification

## Purpose
TBD - created by archiving change fix-v312-71-issue-4664-sqlite-master-system-table. Update Purpose after archive.
## Requirements
### Requirement: Executor MUST synthesize `sqlite_master` view at query time

When the executor receives a `SELECT ... FROM sqlite_master` (or `sqlite_schema`) statement, it MUST return a synthesized result with one row per user table, projecting the requested columns. The synthesized columns are: `type` (always `'table'`), `name` (table name), `tbl_name` (same as `name`), `rootpage` (synthesized as 0), `sql` (a synthesized `CREATE TABLE` statement, or `NULL` if the executor cannot reconstruct it).

#### Scenario: SELECT * FROM sqlite_master returns one row per user table

- **WHEN** the executor runs:
  1. `CREATE TABLE a (x INT)`
  2. `CREATE TABLE b (y INT, z INT)`
  3. `SELECT * FROM sqlite_master`
- **THEN** the result MUST contain at least 2 rows, one for each user table. Each row has columns `type = 'table'`, `name = 'a' or 'b'`, `tbl_name = same as name`, `rootpage = 0`, `sql = a string starting with 'CREATE TABLE'`

#### Scenario: SELECT name FROM sqlite_master returns just the name column

- **WHEN** the executor runs:
  1. `CREATE TABLE t1 (a INT); CREATE TABLE t2 (b INT)`
  2. `SELECT name FROM sqlite_master`
- **THEN** the result MUST contain one Text cell per user table, each equal to the table name

#### Scenario: SELECT * FROM sqlite_schema (alias) works too

- **WHEN** the executor runs:
  1. `CREATE TABLE t (x INT)`
  2. `SELECT name FROM sqlite_schema`
- **THEN** the result MUST contain a Text cell with value `'t'`

#### Scenario: WHERE filters sqlite_master

- **WHEN** the executor runs:
  1. `CREATE TABLE foo (x INT); CREATE TABLE bar (y INT)`
  2. `SELECT name FROM sqlite_master WHERE name = 'foo'`
- **THEN** the result MUST contain exactly one Text cell with value `'foo'`

#### Scenario: Empty catalog returns empty result

- **WHEN** the executor runs `SELECT * FROM sqlite_master` on a database with no user tables
- **THEN** the result MUST have zero rows (not an error)

#### Scenario: Non-system tables still resolve normally (regression)

- **WHEN** the executor runs `SELECT * FROM sqlite_master` after creating a user table
- **THEN** `SELECT * FROM <user_table>` still works (no change to existing behaviour)

