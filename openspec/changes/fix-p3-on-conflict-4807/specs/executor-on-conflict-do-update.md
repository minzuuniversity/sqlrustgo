## ADDED Requirements

### Requirement: ON CONFLICT DO UPDATE actually executes

When `INSERT INTO t ... ON CONFLICT (conflict_target) DO UPDATE SET
assignment [, ...] [WHERE condition]` is executed and a row conflicts
with an existing row in `t`, the existing row MUST be updated to
reflect the assignments. The new column values MUST be sourced from
the incoming INSERT row (referred to as `excluded.col`).

#### Scenario: Basic UPDATE with excluded.col

- GIVEN a table `t (id INT PRIMARY KEY, name TEXT, cnt INT DEFAULT 0)`
  with row `(1, 'a', 0)`
- WHEN the user executes:
  ```
  INSERT INTO t (id, name, cnt) VALUES (1, 'a', 5)
  ON CONFLICT (id) DO UPDATE SET cnt = excluded.cnt
  ```
- THEN `SELECT cnt FROM t WHERE id = 1` returns `5` (not `0`).
- AND the update row count is `1` (not `0`).

#### Scenario: Multi-row UPSERT

- GIVEN a table with rows `(1, 'a', 0), (2, 'b', 0), (3, 'c', 0)`
- WHEN the user executes:
  ```
  INSERT INTO t (id, name, cnt) VALUES (1, 'a', 5), (2, 'b', 6), (3, 'c', 7)
  ON CONFLICT (id) DO UPDATE SET cnt = excluded.cnt
  ```
- THEN all three rows are updated: `(1, 'a', 5), (2, 'b', 6), (3, 'c', 7)`.

### Requirement: excluded.col refers to incoming row

In an `ON CONFLICT ... DO UPDATE SET` clause, the qualifier `excluded`
MUST refer to the column values from the row being inserted, not the
existing row in the table. A bare column name (without the `excluded.`
qualifier) refers to the existing row's column value (per PostgreSQL
semantics).

#### Scenario: excluded.col vs bare col

- GIVEN a table with row `(1, 'a', 10)`
- WHEN:
  ```
  INSERT INTO t (id, name, cnt) VALUES (1, 'b', 5)
  ON CONFLICT (id) DO UPDATE SET
    cnt = excluded.cnt,        -- 5 (incoming)
    name = name                -- 'a' (existing, unchanged)
  ```
- THEN the row becomes `(1, 'a', 5)` (name stays 'a', cnt becomes 5).

### Requirement: WHERE clause on UPDATE

`ON CONFLICT ... DO UPDATE SET ... WHERE excluded.col > N` MUST filter
which conflict rows are actually updated.

#### Scenario: UPDATE WHERE filters rows

- GIVEN rows `(1, 10), (2, 20), (3, 30)`
- WHEN:
  ```
  INSERT INTO t VALUES (1, 100), (2, 200), (3, 300)
  ON CONFLICT (id) DO UPDATE SET val = excluded.val
  WHERE excluded.val > 50
  ```
- THEN only rows with id=1 and id=3 are updated (vals 100 and 300);
  row id=2 keeps val=20.
