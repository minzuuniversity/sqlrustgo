## ADDED Requirements

### Requirement: Executor MUST evaluate `MOD`, `POWER`, `LOG`, `EXP`, `SQRT` with f64 arithmetic

The executor MUST evaluate the five standard SQL math functions using IEEE 754 double-precision arithmetic. All functions return `Value::Float`. NULL input propagates NULL. Domain errors (sqrt of negative, log of non-positive) return `Value::Null` — matching SQLite behavior.

#### Scenario: `MOD(10, 3)` returns 1.0

- **WHEN** the executor runs `SELECT MOD(10, 3)`
- **THEN** the result MUST be `1.0`

#### Scenario: `MOD(4.5, 2.1)` returns floating-point remainder

- **WHEN** the executor runs `SELECT MOD(4.5, 2.1)`
- **THEN** the result MUST be approximately `0.3` (f64 IEEE 754)

#### Scenario: `POWER(2, 3)` returns 8.0

- **WHEN** the executor runs `SELECT POWER(2, 3)`
- **THEN** the result MUST be `8.0`

#### Scenario: `POWER(100, 0.5)` returns 10.0 (square root via POWER)

- **WHEN** the executor runs `SELECT POWER(100, 0.5)`
- **THEN** the result MUST be `10.0`

#### Scenario: `SQRT(16)` returns 4.0

- **WHEN** the executor runs `SELECT SQRT(16)`
- **THEN** the result MUST be `4.0`

#### Scenario: `SQRT(2)` returns approximately 1.414...

- **WHEN** the executor runs `SELECT SQRT(2)`
- **THEN** the result MUST be approximately `1.4142135623730951`

#### Scenario: `SQRT(-1)` returns NULL (domain error)

- **WHEN** the executor runs `SELECT SQRT(-1)`
- **THEN** the result MUST be NULL

#### Scenario: `LOG(2.718281828)` returns approximately 1.0 (natural log)

- **WHEN** the executor runs `SELECT LOG(2.718281828)`
- **THEN** the result MUST be approximately `1.0` (±0.0001)

#### Scenario: `LOG(-1)` returns NULL (domain error)

- **WHEN** the executor runs `SELECT LOG(-1)`
- **THEN** the result MUST be NULL

#### Scenario: `EXP(1)` returns approximately 2.718...

- **WHEN** the executor runs `SELECT EXP(1)`
- **THEN** the result MUST be approximately `2.718281828`

#### Scenario: `EXP(0)` returns 1.0

- **WHEN** the executor runs `SELECT EXP(0)`
- **THEN** the result MUST be `1.0`

#### Scenario: NULL input propagates NULL

- **WHEN** the executor runs `SELECT MOD(NULL, 3)`, `SELECT POWER(NULL, 2)`, `SELECT LOG(NULL)`, `SELECT EXP(NULL)`, `SELECT SQRT(NULL)`
- **THEN** each result MUST be NULL

#### Scenario: NULL second argument returns NULL

- **WHEN** the executor runs `SELECT MOD(10, NULL)`, `SELECT POWER(2, NULL)`
- **THEN** each result MUST be NULL

#### Scenario: Table column usage

- **WHEN** the executor runs `CREATE TABLE t(v REAL); INSERT INTO t VALUES (2), (4), (100); SELECT v, MOD(v, 3), POWER(v, 2), SQRT(v) FROM t;`
- **THEN** the results MUST be `(2, 2.0, 4.0, 1.414...)`, `(4, 1.0, 16.0, 2.0)`, `(100, 1.0, 10000.0, 10.0)`
