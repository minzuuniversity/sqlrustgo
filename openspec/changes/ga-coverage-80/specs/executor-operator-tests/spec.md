## ADDED Requirements

### Requirement: Trigger execution fires on DML events
The system SHALL fire trigger actions when DML events (INSERT/UPDATE/DELETE) occur on watched tables.

#### Scenario: BEFORE INSERT trigger fires
- **WHEN** BEFORE INSERT trigger is defined on `audit_log` and `INSERT INTO orders VALUES (...)` is executed
- **THEN** trigger SHALL execute before row is inserted and can modify the row being inserted

#### Scenario: AFTER DELETE trigger fires
- **WHEN** AFTER DELETE trigger is defined on `orders` and a row is deleted
- **THEN** trigger SHALL execute after deletion and can write to audit table

#### Scenario: INSTEAD OF trigger fires on view
- **WHEN** INSTEAD OF trigger is defined on view `order_view` and user deletes from view
- **THEN** trigger SHALL execute instead of actual delete and can route to underlying tables

### Requirement: Aggregate operators compute correct aggregates
The system SHALL correctly compute COUNT, SUM, AVG, MIN, MAX aggregates over groups.

#### Scenario: COUNT with group by
- **WHEN** `SELECT customer_id, COUNT(*) FROM orders GROUP BY customer_id` is executed
- **THEN** system SHALL return correct count per customer_id

#### Scenario: SUM with null handling
- **WHEN** `SELECT SUM(amount) FROM orders WHERE amount IS NOT NULL` is executed
- **THEN** system SHALL ignore null values in sum

#### Scenario: AVG with multiple groups
- **WHEN** `SELECT department, AVG(salary) FROM employees GROUP BY department HAVING AVG(salary) > 50000` is executed
- **THEN** system SHALL compute average per department and filter by having clause

### Requirement: Join operators produce correct results
The system SHALL correctly join tables using hash join, nested loop join, or sort-merge join.

#### Scenario: INNER JOIN with hash join
- **WHEN** `SELECT * FROM orders JOIN customers ON orders.customer_id = customers.id` is executed
- **THEN** system SHALL return only rows with matching customer_id

#### Scenario: LEFT JOIN preserves left rows
- **WHEN** `SELECT * FROM orders LEFT JOIN customers ON orders.customer_id = customers.id` is executed
- **THEN** system SHALL return all orders rows, with NULL for unmatched customer fields

#### Scenario: Multi-column JOIN
- **WHEN** `SELECT * FROM a JOIN b ON a.x = b.x AND a.y = b.y` is executed
- **THEN** system SHALL match rows where both x and y columns match
