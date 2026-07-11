## ADDED Requirements

### Requirement: MemoryStorage rollback restores prior state
The system SHALL make `BEGIN; …; ROLLBACK;` on a `MemoryStorage`
engine leave the table state identical to its pre-`BEGIN` snapshot.
This applies to `INSERT`, `UPDATE`, and `DELETE` issued inside the
transaction.

#### Scenario: INSERT inside transaction is rolled back
- **WHEN** a session runs `CREATE TABLE t (v INTEGER); BEGIN; INSERT INTO t VALUES (1),(2); ROLLBACK;`
- **THEN** `SELECT COUNT(*) FROM t` returns 0 and the table is byte-for-byte identical to its post-`CREATE TABLE` state

#### Scenario: UPDATE inside transaction is rolled back
- **WHEN** a session runs `CREATE TABLE t (id INTEGER, v INTEGER); INSERT INTO t VALUES (1,10),(2,20); BEGIN; UPDATE t SET v = 999 WHERE id = 1; UPDATE t SET v = 888 WHERE id = 2; ROLLBACK;`
- **THEN** `SELECT v FROM t ORDER BY id` returns `(10, 20)` — neither UPDATE is visible

#### Scenario: DELETE inside transaction is rolled back
- **WHEN** a session runs `CREATE TABLE t (v INTEGER); INSERT INTO t VALUES (1),(2),(3),(4); BEGIN; DELETE FROM t WHERE v > 2; ROLLBACK;`
- **THEN** `SELECT COUNT(*) FROM t` returns 4 and `SELECT v FROM t ORDER BY v` returns `(1, 2, 3, 4)`

### Requirement: MemoryStorage commit makes writes visible
The system SHALL make `BEGIN; …; COMMIT;` on a `MemoryStorage`
engine persist the buffered DML exactly as if the statements had run
outside a transaction.

#### Scenario: INSERT inside transaction is committed
- **WHEN** a session runs `CREATE TABLE t (v INTEGER); BEGIN; INSERT INTO t VALUES (1),(2); COMMIT;`
- **THEN** `SELECT COUNT(*) FROM t` returns 2 and `SELECT v FROM t ORDER BY v` returns `(1, 2)`

### Requirement: MemoryStorage transactions do not nest
The system SHALL reject a second `BEGIN` while a transaction is
already active on the same engine, returning an error rather than
silently starting a savepoint.

#### Scenario: Nested BEGIN errors
- **WHEN** a session runs `BEGIN; BEGIN;`
- **THEN** the second `BEGIN` returns an error and the original transaction is left intact (a subsequent `ROLLBACK` still reverts the writes)