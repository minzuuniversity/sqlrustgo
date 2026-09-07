## ADDED Requirements

### Requirement: CHECK constraint validates on INSERT

`sqlrustgo` MUST evaluate every `CHECK (expr)` constraint defined on a
table when a row is being inserted. If the expression evaluates to
`false` (or NULL — per SQL standard, NULL is "unknown" and the row is
accepted), the INSERT MUST fail with `ConstraintViolation` error.

#### Scenario: Basic CHECK on column value

- GIVEN a table `t (id INT, age INT, CHECK (age >= 0))`
- WHEN the user executes `INSERT INTO t VALUES (1, -1)`
- THEN the statement fails with error:
  `CHECK constraint '<unnamed>' violated: row ...`
- AND the table is not modified (no row inserted).

#### Scenario: Valid value passes

- GIVEN the same table as above
- WHEN the user executes `INSERT INTO t VALUES (1, 25)`
- THEN the row is inserted successfully.

### Requirement: CHECK constraint validates on UPDATE

`sqlrustgo` MUST evaluate every `CHECK (expr)` constraint when a row is
being updated. Updating a column to a value that violates a constraint
MUST fail.

#### Scenario: UPDATE violating CHECK

- GIVEN `t (id INT, age INT, CHECK (age >= 0))` with row `(1, 25)`
- WHEN `UPDATE t SET age = -1 WHERE id = 1`
- THEN the statement fails and the row's age remains `25`.

### Requirement: Multiple CHECK constraints all evaluated

When a table has multiple `CHECK` constraints, ALL MUST be evaluated
for each row. A single failing constraint rejects the row.

#### Scenario: Two constraints, one fails

- GIVEN `t (id INT, age INT, salary INT, CHECK (age >= 0), CHECK
  (salary >= 0))`
- WHEN `INSERT INTO t VALUES (1, 25, -100)`
- THEN the insert fails (salary CHECK fails).

### Requirement: Named CHECK constraint for error message

`CONSTRAINT name CHECK (expr)` MUST preserve the constraint name in the
catalog and report it in error messages.

#### Scenario: Named constraint error

- GIVEN:
  ```
  CREATE TABLE t (id INT, age INT, CONSTRAINT age_check CHECK (age >= 0))
  ```
- WHEN `INSERT INTO t VALUES (1, -1)`
- THEN the error message includes `age_check`.

### Requirement: Multi-column CHECK

`CHECK (col1 > col2)` referencing multiple columns MUST be supported.

#### Scenario: Price greater than cost

- GIVEN `products (id INT, price INT, cost INT, CHECK (price >= cost))`
- WHEN `INSERT INTO products VALUES (1, 5, 10)` (price < cost)
- THEN the insert fails.
- AND `INSERT INTO products VALUES (1, 10, 5)` (price > cost) succeeds.