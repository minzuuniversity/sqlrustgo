# Spec: parser-cte-in-subquery

## ADDED Requirements

### Requirement: FROM subquery accepts a leading WITH prefix

The SQL parser SHALL accept `SELECT ... FROM (WITH [RECURSIVE] cte AS
( ... ) SELECT ... ) AS alias` and similar shapes where the derived
table inside `FROM (...)` begins with a CTE clause. The CTE scope is
limited to the derived table (standard SQL semantics).

#### Scenario: WITH RECURSIVE inside INSERT ... FROM subquery

Given `INSERT INTO t SELECT x, x * 10 FROM (WITH RECURSIVE s(x) AS
(VALUES (1) UNION ALL SELECT x + 1 FROM s WHERE x < 3) SELECT * FROM s)
AS sub`, when the parser processes the statement, then it succeeds without
error and the resulting AST contains the CTE clause bound to the derived
table only.

#### Scenario: WITH (non-recursive) inside INSERT ... FROM subquery

Given `INSERT INTO t SELECT a, b FROM (WITH c AS (SELECT 1 AS a, 2 AS b)
SELECT * FROM c) AS sub`, the parser succeeds and the CTE `c` is bound
to the derived table only.

#### Scenario: WITH RECURSIVE inside SELECT ... FROM subquery

Given `SELECT * FROM (WITH RECURSIVE walk(n) AS (VALUES (1) UNION ALL
SELECT n + 1 FROM walk WHERE n < 5) SELECT * FROM walk) AS sub`, the
parser succeeds.

#### Scenario: WITH followed by DML inside subquery fails

Given `SELECT * FROM (WITH c AS (SELECT 1) INSERT INTO t SELECT * FROM c)
AS sub`, the parser rejects the statement with an explicit error message
mentioning that `WITH` in subquery position must be followed by `SELECT`.

### Requirement: parse_select_statement entry point accepts leading WITH

`parse_select_statement` SHALL accept a leading `Token::With` and
delegate to `parse_with_select`, returning the unwrapped
`WithSelect.select` as the function result. When the WITH is followed by
a DML statement (INSERT/UPDATE/DELETE) in subquery position, the parser
SHALL return an explicit error rather than silently returning a
non-SELECT value.

#### Scenario: SELECT statement still parses normally

Given any `SELECT ...` statement that previously parsed, the parser
still succeeds without error (this is a strict-superset change).

#### Scenario: Pre-existing CTE top-level still parses

Given `WITH cte AS (SELECT 1) SELECT * FROM cte`, the parser still
succeeds and produces the same AST as before this fix.