# parser-create-function-extensions Specification

## ADDED Requirements

### Requirement: CREATE FUNCTION accepts BEGIN...END body without AS

The SQL parser SHALL accept a multi-statement function body of the
form `BEGIN ... END` both with and without an `AS` keyword. The
historical single-`RETURN expr` form is also accepted in both
prefixes.

#### Scenario: issue body f2 — BEGIN/END without AS

Given `CREATE FUNCTION f2(x INT) RETURNS INT BEGIN DECLARE r INT; SET
r = x + 100; RETURN r; END`, the parser succeeds. The resulting
`CreateFunctionStatement.body_block` is `Some(...)` carrying the
captured token stream between `BEGIN` and `END`; `body_expr` is
empty.

#### Scenario: BEGIN/END with AS prefix

Given `CREATE FUNCTION f2(x INT) RETURNS INT AS BEGIN DECLARE r INT;
SET r = x + 100; RETURN r; END`, the parser succeeds (AS prefix
regression).

#### Scenario: single-expression RETURN body

Given `CREATE FUNCTION f1(x INT) RETURNS INT RETURN x * 2`, the
parser succeeds. `body_expr` carries the expression; `body_block` is
`None`.

### Requirement: RETURNS TABLE records a column list

The SQL parser SHALL accept a `RETURNS TABLE(col1 type1, col2 type2,
...)` clause on a `CREATE FUNCTION` and record the columns in
`CreateFunctionStatement.return_columns`. The column type names are
accepted as plain identifiers (`INT`, `TEXT`, `FLOAT`, `BOOLEAN`)
or the corresponding keyword tokens.

#### Scenario: issue body f3 — RETURNS TABLE with a single RETURN body

Given `CREATE FUNCTION f3() RETURNS TABLE(id INT, name TEXT) RETURN
SELECT x FROM t`, the parser succeeds. `return_columns` contains two
`UdfParam` entries (`(id, INT)`, `(name, TEXT)`); `return_type` is
the placeholder `"TABLE"`.

#### Scenario: RETURNS TABLE with a multi-statement body

Given `CREATE FUNCTION f4() RETURNS TABLE(id INT) BEGIN RETURN
SELECT x FROM t; END`, the parser succeeds. `return_columns`
contains the `id` column spec; `body_block` is `Some(...)`.

## Out of Scope

- Executor support for table-valued UDFs (the AST records the column
  list, but the runtime does not yet produce rows from a `RETURNS
  TABLE(...)` function).
- `DECLARE` statement parsing inside the multi-statement body.
- `OUT` / `INOUT` UDF parameters.