# multi-table-dml Specification

## Purpose
TBD - created by archiving change fix-dml-mvcc-and-multitable. Update Purpose after archive.
## Requirements
### Requirement: Multi-table UPDATE applies against the joined view
The system SHALL parse `UPDATE t1, t2 SET t1.col = t2.col WHERE …`
and apply the SET clauses across the cartesian product of the listed
tables, honouring the WHERE clause.

#### Scenario: Update multiple tables in a single statement
- **WHEN** a session runs `CREATE TABLE a (v INTEGER); CREATE TABLE b (v INTEGER); INSERT INTO a VALUES (1); INSERT INTO b VALUES (1); UPDATE a, b SET a.v = 10, b.v = 20;`
- **THEN** `SELECT v FROM a` returns `(10)` and `SELECT v FROM b` returns `(20)`

#### Scenario: WHERE clause restricts multi-table update
- **WHEN** a session runs `CREATE TABLE a (id INTEGER, v INTEGER); CREATE TABLE b (id INTEGER, v INTEGER); INSERT INTO a VALUES (1, 100), (2, 200); INSERT INTO b VALUES (1, 999), (2, 888); UPDATE a, b SET b.v = 0 WHERE a.id = b.id AND a.id = 1;`
- **THEN** `SELECT v FROM b WHERE id = 1` returns `(0)` and `SELECT v FROM b WHERE id = 2` returns `(888)` (the row matching the WHERE was updated, the rest untouched)

### Requirement: Multi-table DELETE applies against the joined view
The system SHALL parse `DELETE t1, t2 FROM t1, t2 WHERE …` and
remove rows from each listed table according to the joined-row
predicate.

#### Scenario: Delete rows from multiple tables
- **WHEN** a session runs `CREATE TABLE a (v INTEGER); CREATE TABLE b (v INTEGER); INSERT INTO a VALUES (1); INSERT INTO b VALUES (1); DELETE a, b FROM a, b;`
- **THEN** `SELECT COUNT(*) FROM a` returns 0 and `SELECT COUNT(*) FROM b` returns 0

#### Scenario: WHERE clause restricts multi-table delete
- **WHEN** a session runs `CREATE TABLE a (id INTEGER); CREATE TABLE b (id INTEGER); INSERT INTO a VALUES (1), (2); INSERT INTO b VALUES (1), (2); DELETE a, b FROM a, b WHERE a.id = 1;`
- **THEN** `SELECT id FROM a ORDER BY id` returns `(2)` and `SELECT id FROM b ORDER BY id` returns `(2)` — only the joined row with `id = 1` is removed

### Requirement: Single-table UPDATE/DELETE continues to work
The system SHALL keep accepting `UPDATE t SET …` and
`DELETE FROM t WHERE …` (single-table) and produce the same
behaviour as before this change.

#### Scenario: Single-table update still works
- **WHEN** a session runs `CREATE TABLE t (id INTEGER, v INTEGER); INSERT INTO t VALUES (1,10); UPDATE t SET v = 42 WHERE id = 1;`
- **THEN** `SELECT v FROM t WHERE id = 1` returns `(42)`

