## ADDED Requirements

### Requirement: Executor MUST apply USING-merge semantics — match on `t1.col = t2.col` and project each USING column once

The executor MUST recognize `JoinClause.using_columns` and apply USING-merge semantics:

1. The join predicate is the conjunction `t1.col_i = t2.col_i` for each column in the USING list.
2. Each USING column appears exactly once in the output rows and the output schema (the duplicate right-side column is projected away).
3. LEFT JOIN ... USING MUST preserve all left rows, padding unmatched right-side USING columns with NULL.
4. INNER JOIN ... USING MUST return only the matched rows.

#### Scenario: INNER JOIN USING single col — match only

- **WHEN** the executor runs:
  1. `CREATE TABLE t1 (id INT, a TEXT)` with rows `(1, 'x'), (2, 'y'), (3, 'z')`
  2. `CREATE TABLE t2 (id INT, b TEXT)` with rows `(2, 'B'), (3, 'C'), (4, 'D')`
  3. `SELECT t1.id, t1.a, t2.b FROM t1 INNER JOIN t2 USING (id)`
- **THEN** the result MUST contain 2 rows: `(2, 'y', 'B')` and `(3, 'z', 'C')`

#### Scenario: INNER JOIN USING with no overlap — zero rows (regression)

- **WHEN** the executor runs:
  1. `CREATE TABLE t1 (id INT)` with rows `(1), (2), (3)`
  2. `CREATE TABLE t2 (id INT)` with rows `(10), (20)`
  3. `SELECT COUNT(*) FROM t1 INNER JOIN t2 USING (id)`
- **THEN** the result MUST be 1 row with COUNT = 0 (not the pre-fix 6-row Cartesian product)

#### Scenario: LEFT JOIN USING preserves unmatched left rows with NULL padding

- **WHEN** the executor runs:
  1. `CREATE TABLE t1 (id INT, a TEXT)` with rows `(1, 'x'), (2, 'y'), (3, 'z')`
  2. `CREATE TABLE t2 (id INT, b TEXT)` with row `(2, 'B')`
  3. `SELECT t1.id, t1.a, t2.b FROM t1 LEFT JOIN t2 USING (id)`
- **THEN** the result MUST contain 3 rows:
  - `(1, 'x', NULL)` — left row, no match
  - `(2, 'y', 'B')` — matched
  - `(3, 'z', NULL)` — left row, no match
- **AND MUST NOT** return the pre-fix 3×1 = 3-row Cartesian product (or any N_left × N_right multiplication).

#### Scenario: SELECT \* with USING projects one column per USING col

- **WHEN** the executor runs:
  1. `CREATE TABLE t1 (id INT, a INT)` with row `(1, 10)`
  2. `CREATE TABLE t2 (id INT, b INT)` with row `(1, 20)`
  3. `SELECT * FROM t1 INNER JOIN t2 USING (id)`
- **THEN** the result MUST contain 1 row with 3 columns: `(1, 10, 20)` — the duplicate `t2.id` is projected away.

#### Scenario: USING with multiple columns

- **WHEN** the executor runs:
  1. `CREATE TABLE t1 (id INT, code INT, a TEXT)` with rows `(1, 100, 'x1'), (1, 200, 'x2')`
  2. `CREATE TABLE t2 (id INT, code INT, b TEXT)` with rows `(1, 100, 'B1'), (1, 300, 'B2')`
  3. `SELECT * FROM t1 INNER JOIN t2 USING (id, code)`
- **THEN** the result MUST contain 1 row with 4 columns: `(1, 100, 'x1', 'B1')` — both right-side USING cols are projected away.