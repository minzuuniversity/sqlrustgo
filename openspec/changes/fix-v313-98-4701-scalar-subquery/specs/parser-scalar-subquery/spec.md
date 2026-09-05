# parser-scalar-subquery Specification

## ADDED Requirements

### Requirement: FROM-projectable scalar subquery in projection

The SQL parser SHALL accept a leading `(` in the column-list position
of a `SELECT` (or `INSERT ... SELECT ...`) projection. When the
parenthesised content is a `SELECT` or `WITH` query, the result is a
scalar subquery expression. Otherwise the result is a parenthesised
expression such as `(n + 1)`. The column may optionally be followed by
an `AS alias` clause; the loop then re-enters on the next column
comma or on the trailing `FROM` / `ORDER BY` / `LIMIT` / `OFFSET`
without the parser falling out of the projection loop.

#### Scenario: scalar subquery with AS alias in plain SELECT

Given `SELECT (SELECT count(*) FROM b WHERE b.id = a.id) AS matched
FROM a`, the parser succeeds and the resulting projection carries
the scalar subquery as its expression and `Some("matched")` as its
alias.

#### Scenario: scalar subquery without alias

Given `SELECT (SELECT 1) FROM a`, the parser succeeds and the column's
alias is `None`.

#### Scenario: INSERT ... SELECT with subquery, ORDER BY, LIMIT, OFFSET

Given the issue #4701 example
`INSERT INTO t SELECT a.id, a.val, (SELECT count(*) FROM b WHERE
b.id = a.id) AS matched FROM a ORDER BY a.val DESC LIMIT 2 OFFSET 1`,
the parser succeeds. The first two columns (`a.id`, `a.val`) parse
as plain identifier projections, the third parses as a scalar
subquery with `AS matched`, the outer `FROM a` parses, and the
trailing `ORDER BY a.val DESC LIMIT 2 OFFSET 1` parse.

### Requirement: Keyword-form identifiers accepted as column alias

The SQL parser SHALL accept keyword-form tokens as column aliases.
Currently this covers `Token::Matched` (the keyword reserved for the
`WHEN MATCHED` arm of a `MERGE` statement); the rule is intended to
extend to any future lexer-keyword token that the user wants to
employ as a plain identifier in a column projection.

#### Scenario: `Matched` as alias

Given `SELECT (SELECT count(*) FROM b WHERE b.id = a.id) AS matched
FROM a`, the alias is captured as `Some("matched")` even though
`Matched` is a lexer keyword.

#### Scenario: bare keyword identifier as alias

Given `SELECT (SELECT 1) AS matched FROM a`, the alias is captured
as `Some("matched")`.

## Out of Scope

- Engine decorrelation of correlated scalar subqueries.
- Plan-level `EXPLAIN QUERY PLAN` rendering of scalar subqueries.
- Constant folding of trivial subqueries such as `(SELECT 1)`.