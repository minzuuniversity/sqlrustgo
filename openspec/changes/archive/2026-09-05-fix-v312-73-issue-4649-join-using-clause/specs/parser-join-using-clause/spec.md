## ADDED Requirements

### Requirement: Parser MUST accept `JOIN ... USING (col_list)` and store the column list on the AST

The parser MUST accept `SELECT ... FROM t1 [LEFT|RIGHT|INNER|FULL|CROSS] JOIN t2 USING (col1, col2, ...)`. The `JoinClause.using_columns` field MUST be `Some(Vec<String>)` containing the column names when USING is present, and `None` for plain ON joins. USING is mutually exclusive with ON — when both are present, ON takes precedence (matching PostgreSQL/SQLite semantics).

#### Scenario: Single-column USING is parsed and recorded

- **WHEN** the parser parses `SELECT * FROM a JOIN b USING (id)`
- **THEN** the resulting `SelectStatement` has `join_clause[0].using_columns == Some(vec!["id"])`

#### Scenario: Multi-column USING is parsed and recorded

- **WHEN** the parser parses `SELECT * FROM t1 INNER JOIN t2 USING (id, name)`
- **THEN** the resulting `SelectStatement` has `join_clause[0].using_columns == Some(vec!["id", "name"])`

#### Scenario: LEFT JOIN USING preserves join type

- **WHEN** the parser parses `SELECT * FROM t1 LEFT JOIN t2 USING (id)`
- **THEN** the resulting `SelectStatement` has `join_clause[0].join_type == JoinType::Left` AND `join_clause[0].using_columns == Some(vec!["id"])`

#### Scenario: USING () is a syntax error

- **WHEN** the parser parses `SELECT * FROM a JOIN b USING ()`
- **THEN** the parser MUST return `Err(...)` with a message indicating that USING requires at least one column