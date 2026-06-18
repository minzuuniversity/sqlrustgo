## ADDED Requirements

### Requirement: Engine select chooses correct physical plan
The system SHALL select the optimal physical execution plan for a given SQL query based on cost estimation.

#### Scenario: Simple SELECT with index
- **WHEN** `SELECT * FROM users WHERE id = 1` is executed
- **THEN** system SHALL use index scan instead of full table scan

#### Scenario: SELECT with join
- **WHEN** `SELECT * FROM orders JOIN customers ON orders.customer_id = customers.id` is executed
- **THEN** system SHALL use hash join or nested loop join based on table sizes

#### Scenario: SELECT with aggregate
- **WHEN** `SELECT COUNT(*) FROM orders GROUP BY customer_id` is executed
- **THEN** system SHALL use hash aggregate or sort aggregate based on cardinality

### Requirement: Execution engine executes query plan
The system SHALL execute a physical query plan and return correct results.

#### Scenario: Execute table scan
- **WHEN** `SELECT id, name FROM users` is executed with a table scan plan
- **THEN** system SHALL return all rows with correct column values

#### Scenario: Execute filter
- **WHEN** `SELECT * FROM users WHERE age > 21` is executed
- **THEN** system SHALL return only rows where age > 21

#### Scenario: Execute insert
- **WHEN** `INSERT INTO users (name, age) VALUES ('alice', 30)` is executed
- **THEN** system SHALL insert the row and return affected row count = 1

### Requirement: Engine builder constructs execution engine
The system SHALL correctly build an execution engine from catalog metadata.

#### Scenario: Build engine for in-memory catalog
- **WHEN** EngineBuilder builds an engine with in-memory catalog
- **THEN** engine SHALL be able to execute queries against registered tables

#### Scenario: Build engine with storage backend
- **WHEN** EngineBuilder builds an engine with storage backend
- **THEN** engine SHALL route read queries to storage and write queries through transaction layer
