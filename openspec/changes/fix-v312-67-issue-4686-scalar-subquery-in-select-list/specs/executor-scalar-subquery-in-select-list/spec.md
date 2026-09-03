## ADDED Requirements

### Requirement: Executor MUST evaluate scalar subqueries in the SELECT list

When a SELECT list contains a scalar subquery as a column expression (e.g. `SELECT (subq) AS alias` or `SELECT (subq)`), the executor MUST execute the subquery, take the first row's first column, and use that as the cell value. An empty subquery result yields `NULL`. A subquery execution error MUST surface as an `SqlError::ExecutionError` with a descriptive message; it MUST NOT be silently dropped.

#### Scenario: literal scalar subquery

- **WHEN** the executor runs `SELECT (SELECT 1) AS x`
- **THEN** the result MUST contain exactly one row with column `x = 1`

#### Scenario: aggregate scalar subquery from a real table

- **WHEN** the executor runs:
  1. `CREATE TABLE t (a INT)` and `INSERT INTO t VALUES (10), (20), (30)`
  2. `SELECT (SELECT MAX(a) FROM t) AS m`
- **THEN** the result MUST contain exactly one row with column `m = 30`

#### Scenario: multiple scalar subqueries in the SELECT list

- **WHEN** the executor runs `SELECT (SELECT 1) AS a, (SELECT 2) AS b`
- **THEN** the result MUST contain exactly one row with columns `a = 1`, `b = 2`

#### Scenario: empty scalar subquery yields NULL

- **WHEN** the executor runs `SELECT (SELECT * FROM t WHERE 1=0) AS empty` against a populated table
- **THEN** the result MUST contain exactly one row with column `empty = NULL`

### Requirement: No regression on existing scalar-subquery usage

The fix MUST NOT regress the existing `WHERE val = (subq)` (#4629) and `WHERE val > ANY (subq)` (#4641) paths.

#### Scenario: scalar subquery in WHERE equality

- **WHEN** the executor runs:
  1. `CREATE TABLE u (b INT)` and `INSERT INTO u VALUES (5), (15), (25)`
  2. `CREATE TABLE t (a INT)` and `INSERT INTO t VALUES (10), (20), (30)`
  3. `SELECT * FROM t WHERE a = (SELECT MAX(b) FROM u)`
- **THEN** the result MUST contain only the row with `a = 25`

#### Scenario: quantified subquery in WHERE

- **WHEN** the executor runs:
  1. `CREATE TABLE a (val INT)` with `(10), (20), (30)`
  2. `CREATE TABLE b (val INT)` with `(10), (25), (35)`
  3. `SELECT * FROM a WHERE val > ANY (SELECT val FROM b)`
- **THEN** the result MUST contain rows `(20)` and `(30)`
