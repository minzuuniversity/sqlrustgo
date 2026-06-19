## ADDED Requirements

### Requirement: Parser correctly parses SELECT statements
The system SHALL parse valid SELECT statements and produce correct AST.

#### Scenario: Simple SELECT
- **WHEN** `SELECT id, name FROM users` is parsed
- **THEN** parser SHALL produce Select stmt with projection [id, name] from users

#### Scenario: SELECT with WHERE clause
- **WHEN** `SELECT * FROM users WHERE age > 21 AND active = true` is parsed
- **THEN** parser SHALL produce Select stmt with Filter(And(age > 21, active = true))

#### Scenario: SELECT with JOIN
- **WHEN** `SELECT * FROM orders JOIN customers ON orders.customer_id = customers.id WHERE orders.amount > 100` is parsed
- **THEN** parser SHALL produce Select stmt with Join on customer_id and filter on amount

#### Scenario: SELECT with GROUP BY and HAVING
- **WHEN** `SELECT customer_id, SUM(amount) FROM orders GROUP BY customer_id HAVING SUM(amount) > 1000` is parsed
- **THEN** parser SHALL produce Select stmt with GroupBy aggregate and Having filter

### Requirement: Parser correctly parses DML statements
The system SHALL parse INSERT, UPDATE, DELETE statements.

#### Scenario: INSERT with values
- **WHEN** `INSERT INTO users (name, email) VALUES ('alice', 'alice@example.com')` is parsed
- **THEN** parser SHALL produce Insert stmt with columns [name, email] and values

#### Scenario: UPDATE with WHERE
- **WHEN** `UPDATE users SET active = false WHERE last_login < '2024-01-01'` is parsed
- **THEN** parser SHALL produce Update stmt setting active=false with WHERE filter

#### Scenario: DELETE with WHERE
- **WHEN** `DELETE FROM users WHERE id = 5 AND soft_deleted = true` is parsed
- **THEN** parser SHALL produce Delete stmt with composite WHERE filter

### Requirement: Parser correctly parses DDL statements
The system SHALL parse CREATE TABLE, ALTER TABLE, DROP TABLE statements.

#### Scenario: CREATE TABLE with types
- **WHEN** `CREATE TABLE users (id INT PRIMARY KEY, name VARCHAR(100), age INT DEFAULT 0)` is parsed
- **THEN** parser SHALL produce CreateTable stmt with correct column definitions and types

#### Scenario: ALTER TABLE ADD COLUMN
- **WHEN** `ALTER TABLE users ADD COLUMN email VARCHAR(255) NOT NULL` is parsed
- **THEN** parser SHALL produce AlterTable stmt with AddColumn action
