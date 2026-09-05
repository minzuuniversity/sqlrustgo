# cte-recursive-anchor Specification

## Purpose
TBD - created by archiving change fix-v312-93-4704-cte-values-anchor. Update Purpose after archive.
## Requirements
### Requirement: `WITH [RECURSIVE] name AS (VALUES (...) UNION [ALL] SELECT ...)` parses

The system MUST accept a `VALUES` clause as the body of a CTE definition
anchor, in both recursive and non-recursive forms. The VALUES rows become
the first row(s) of the CTE before any recursive iteration.

#### Scenario: Recursive CTE with VALUES anchor

- GIVEN no prior tables
- WHEN executing `WITH RECURSIVE walk(n) AS (VALUES (1) UNION ALL SELECT n+1 FROM walk WHERE n < 5) SELECT * FROM walk;`
- THEN the result contains 5 rows: `n=1, 2, 3, 4, 5`
- AND no parse error is returned

#### Scenario: Non-recursive CTE with VALUES

- GIVEN no prior tables
- WHEN executing `WITH t AS (VALUES (1, 'a'), (2, 'b')) SELECT * FROM t;`
- THEN the result contains 2 rows: `1,'a'` and `2,'b'`

#### Scenario: CTE with explicit column list and VALUES

- GIVEN no prior tables
- WHEN executing `WITH t(id, name) AS (VALUES (1, 'x')) SELECT * FROM t;`
- THEN the result contains 1 row with `id=1, name='x'`

#### Scenario: Existing SELECT anchor still works (no regression)

- GIVEN table `t(id)` with rows `(1),(2),(3)`
- WHEN executing `WITH walk AS (SELECT id FROM t WHERE id=1 UNION ALL SELECT id+1 FROM walk WHERE id<3) SELECT * FROM walk;`
- THEN the result contains rows `1, 2, 3`
- AND no parse error is returned

