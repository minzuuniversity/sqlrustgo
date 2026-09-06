## ADDED Requirements

### Requirement: CREATE VIEW storage-layer implementation

`sqlrustgo` MUST accept `CREATE VIEW view_name [ ( col_alias [, ...] ) ]
AS query` and persist the view definition in the catalog. The view MUST
be queryable via `SELECT ... FROM view_name` (and optionally with an
alias), and the query MUST be re-executed each time the view is
referenced (lazy / non-materialized).

#### Scenario: Create view and query it

- GIVEN an empty database
- WHEN the user executes:
  ```
  CREATE TABLE t (id INT, val INT);
  INSERT INTO t VALUES (1, 10), (2, 20), (3, 30);
  CREATE VIEW v AS SELECT id, val * 2 AS doubled FROM t;
  SELECT * FROM v ORDER BY id;
  ```
- THEN the final SELECT returns three rows: `(1, 20)`, `(2, 40)`,
  `(3, 60)`.

#### Scenario: View with explicit column aliases

- WHEN the user creates
  `CREATE VIEW v(id, doubled) AS SELECT id, val*2 FROM t`
- THEN `SELECT * FROM v` returns columns named `id` and `doubled`.

### Requirement: DROP VIEW execution

`sqlrustgo` MUST accept `DROP VIEW [IF EXISTS] view_name`. If `IF EXISTS`
is not specified and the view does not exist, an error MUST be raised.

#### Scenario: Drop existing view

- GIVEN a view `v` exists
- WHEN `DROP VIEW v` is executed
- THEN the view is removed and subsequent `SELECT * FROM v` errors.

#### Scenario: Drop with IF EXISTS

- GIVEN view `v` does not exist
- WHEN `DROP VIEW IF EXISTS v` is executed
- THEN no error is raised (warning is acceptable).

### Requirement: SELECT FROM view

`sqlrustgo` MUST resolve view names in `FROM` clauses by re-executing the
view's underlying query. Nested view references MUST be supported up to
a depth of 16; deeper nesting MUST raise a clear error.

#### Scenario: Nested views within depth limit

- GIVEN view `a` is defined as `SELECT * FROM t`
- AND view `b` is defined as `SELECT * FROM a`
- AND view `c` is defined as `SELECT * FROM b`
- WHEN the user executes `SELECT * FROM c`
- THEN the rows of `t` are returned.

#### Scenario: Nested views exceed depth limit

- GIVEN views `v1` through `v17` where `v_{n+1}` references `v_n`
- WHEN the user executes `SELECT * FROM v17`
- THEN an error is raised: "view nesting depth exceeds 16".

### Requirement: View metadata persistence (FileStorage only)

`FileStorage` MUST persist view definitions to disk so that views
survive process restarts. Views MUST be loaded into the in-memory catalog
before the first SELECT is served.

#### Scenario: View persists across restart

- GIVEN a view `v` is created and committed
- WHEN the database process is restarted
- THEN `SELECT * FROM v` succeeds without re-creating the view.
