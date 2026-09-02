# parser-timestampdiff-unit-keyword

## Purpose

`TIMESTAMPDIFF(<unit>, <expr1>, <expr2>)` 是 MySQL 5.7 标准函数，第一个参数 `<unit>` 必须是 unit 关键字（MINUTE/HOUR/DAY/SECOND/MONTH/YEAR/MICROSECOND/QUARTER/WEEK），不应被 binder 当作列名查找。

## ADDED Requirements

### Requirement: TIMESTAMPDIFF unit is parsed as a keyword, not an identifier

The parser SHALL accept `TIMESTAMPDIFF(MINUTE, ts1, ts2)` (or any of `MICROSECOND`/`SECOND`/`MINUTE`/`HOUR`/`DAY`/`WEEK`/`MONTH`/`QUARTER`/`YEAR`) where the unit token is recognized as a unit keyword, not bound to a column lookup.

#### Scenario: MINUTE unit evaluates to minute difference

- GIVEN a table `t(ts TIMESTAMP)` with row `('2026-09-01 12:34:56')`
- WHEN `SELECT TIMESTAMPDIFF(MINUTE, ts, '2026-09-01 13:00:00') FROM t`
- THEN output is `25`

#### Scenario: HOUR unit

- GIVEN same setup with diff = 2 hours
- WHEN `SELECT TIMESTAMPDIFF(HOUR, ts1, ts2)`
- THEN output is `2`

#### Scenario: identifier fallback for back-compat

- WHEN `TIMESTAMPDIFF('MINUTE', ts1, ts2)` (unit as string literal)
- THEN parses successfully and returns same result

### Requirement: short INSERT statement pads missing columns with NULL

The binder SHALL accept `INSERT INTO t VALUES (v1)` when `t` has more columns than values, padding missing columns with `NULL`.

#### Scenario: short INSERT into 2-column table

- GIVEN table `b(id INT, val INT)` with no rows
- WHEN `INSERT INTO b VALUES (10)`
- THEN row inserted is `(10, NULL)`

#### Scenario: short INSERT with extra columns still errors

- GIVEN table `b(id INT, val INT)`
- WHEN `INSERT INTO b VALUES (1, 2, 3)`
- THEN binder returns error `table b has 2 columns but 3 values were supplied`

### Requirement: simplified CASE val WHEN NULL is accepted

The parser SHALL accept `CASE <expr> WHEN NULL THEN ... END` where NULL is recognized as a literal value, not a parse error.

#### Scenario: CASE WHEN NULL matches NULL row

- GIVEN table `t(v INT)` with rows (10), (NULL), (30)
- WHEN `SELECT CASE v WHEN NULL THEN 'null' WHEN > 20 THEN 'big' ELSE 'small' END FROM t`
- THEN output is `('small'), ('null'), ('big')`

### Requirement: UPSERT executes INSERT-or-UPDATE semantics

The engine SHALL execute `ON CONFLICT (col) DO UPDATE SET ...` and `ON DUPLICATE KEY UPDATE ...` so that on conflict the existing row is updated per the assignments rather than the INSERT failing.

#### Scenario: ON DUPLICATE KEY UPDATE increments

- GIVEN table `kv(k INT PRIMARY KEY, v INT)` with row `(1, 100)`
- WHEN `INSERT INTO kv VALUES (1, 1) ON DUPLICATE KEY UPDATE v = v + 1`
- THEN resulting row is `(1, 101)`

#### Scenario: ON CONFLICT DO UPDATE sets value

- GIVEN table `kv(k INT PRIMARY KEY, v INT)` with row `(1, 100)`
- WHEN `INSERT INTO kv VALUES (1, 999) ON CONFLICT (k) DO UPDATE SET v = 999`
- THEN resulting row is `(1, 999)`

#### Scenario: ON CONFLICT DO NOTHING skips

- GIVEN table `kv(k INT PRIMARY KEY, v INT)` with row `(1, 100)`
- WHEN `INSERT INTO kv VALUES (1, 999) ON CONFLICT (k) DO NOTHING`
- THEN row remains `(1, 100)`