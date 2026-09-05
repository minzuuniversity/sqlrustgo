# parser-writable-cte Specification

## ADDED Requirements

### Requirement: WITH cte AS (DML) accepts DELETE, UPDATE, INSERT

The SQL parser SHALL accept a CTE body that is a DML statement
(DELETE, UPDATE, or INSERT), in addition to the existing SELECT /
WITH / VALUES forms. This is the writable-CTE shape required by
TPC-H Q15 / Q22 and by PostgreSQL 11+ and SQLite 3.33+.

#### Scenario: WITH cte AS (DELETE ... RETURNING *)

Given `WITH cte AS (DELETE FROM t WHERE val < 25 RETURNING *)
SELECT * FROM cte`, the parser succeeds and the resulting
`WithClause` carries one `CommonTableExpression` whose `subquery`
is a `DeleteStatement` wrapped in the appropriate `WithDml` /
`WithSelect` envelope.

#### Scenario: WITH cte AS (UPDATE ... SET ...)

Given `WITH cte AS (UPDATE t SET val = val * 2 WHERE id = 1
RETURNING *) SELECT * FROM cte`, the parser succeeds.

#### Scenario: WITH cte AS (INSERT INTO ... SELECT ...)

Given `WITH moved AS (INSERT INTO archive SELECT * FROM t WHERE
val < 25 RETURNING *) SELECT count(*) FROM moved`, the parser
succeeds.

### Requirement: DELETE / UPDATE accept trailing RETURNING

The SQL parser SHALL accept an optional `RETURNING <expr-list>`
clause at the end of `DELETE FROM t WHERE ...` and `UPDATE t SET
... WHERE ...`. The column list is parsed and discarded; engine
materialisation of the returning values is out of scope.

#### Scenario: DELETE ... RETURNING *

Given `DELETE FROM t WHERE val < 25 RETURNING *`, the parser
succeeds.

#### Scenario: UPDATE ... SET ... RETURNING <column list>

Given `UPDATE t SET val = val * 2 WHERE id = 1 RETURNING id, val`,
the parser succeeds.

### Requirement: CREATE MATERIALIZED VIEW parses

The SQL parser SHALL accept `CREATE MATERIALIZED VIEW name AS
SELECT ...`. The `MATERIALIZED` qualifier arrives as a plain
identifier in the lexer; the CREATE dispatcher recognises it and
hands off to the regular `parse_create_view` path when the
following token is `View`.

#### Scenario: CREATE MATERIALIZED VIEW parses

Given `CREATE MATERIALIZED VIEW mv AS SELECT * FROM t`, the parser
succeeds.

#### Scenario: bare CREATE MATERIALIZED (no VIEW) errors

Given `CREATE MATERIALIZED x`, the parser rejects the statement
with a clean error. The `MATERIALIZED` identifier is only
accepted when followed by `View`.

## Out of Scope

- Engine-side execution of writable CTEs.
- Engine-side materialisation of MATERIALIZED VIEWs.
- Engine-side `RETURNING` projection (the columns are parsed
  and discarded).