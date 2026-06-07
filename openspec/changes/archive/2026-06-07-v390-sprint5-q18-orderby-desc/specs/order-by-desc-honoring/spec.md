# Capability: order-by-desc-honoring

## ADDED Requirements

### Requirement: ORDER BY direction keyword is preserved in the AST

The SQL parser MUST capture the `ASC` or `DESC` keyword that follows each
`ORDER BY` expression and expose it as a structured field on the AST node
(`OrderByExpr.direction`). The default direction when neither keyword is
present MUST be `Asc` (SQL standard).

#### Scenario: ORDER BY with explicit DESC parses to direction=Desc

- **WHEN** the parser sees `SELECT id FROM t ORDER BY id DESC`
- **THEN** the resulting `OrderByExpr` for `id` has `direction = SortDirection::Desc`

#### Scenario: ORDER BY with explicit ASC parses to direction=Asc

- **WHEN** the parser sees `SELECT id FROM t ORDER BY id ASC`
- **THEN** the resulting `OrderByExpr` for `id` has `direction = SortDirection::Asc`

#### Scenario: ORDER BY with no direction defaults to ASC

- **WHEN** the parser sees `SELECT id FROM t ORDER BY id`
- **THEN** the resulting `OrderByExpr` for `id` has `direction = SortDirection::Asc`

### Requirement: Executor honors ORDER BY direction at sort time

The SELECT executor MUST use the parsed direction when sorting result rows.
A query with `ORDER BY col DESC` MUST produce rows in strictly descending
order of `col` (ties broken by SQL-standard stable sort). A query with
`ORDER BY col ASC` (or no direction) MUST produce rows in ascending order.

#### Scenario: ORDER BY col DESC returns rows in descending order

- **WHEN** the executor runs `SELECT col FROM t ORDER BY col DESC` against a
  table with at least 3 rows holding values {1, 5, 3}
- **THEN** the result rows are ordered [5, 3, 1]

#### Scenario: ORDER BY col ASC returns rows in ascending order

- **WHEN** the executor runs `SELECT col FROM t ORDER BY col ASC` against a
  table with at least 3 rows holding values {1, 5, 3}
- **THEN** the result rows are ordered [1, 3, 5]

#### Scenario: TPC-H Q18 returns top customer by total price

- **WHEN** the executor runs the TPC-H Q18 simplified form
  `SELECT l_orderkey, o_orderdate, o_totalprice FROM customer, orders, lineitem WHERE c_custkey = o_custkey AND o_orderkey = l_orderkey GROUP BY l_orderkey ORDER BY o_totalprice DESC LIMIT 1`
  against the SF=0.001 fixture
- **THEN** the top-1 row's `o_totalprice` is the maximum `o_totalprice` in
  the fixture (NOT the minimum) and matches PostgreSQL's top-1 result
