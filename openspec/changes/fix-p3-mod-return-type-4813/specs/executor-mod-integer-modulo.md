## ADDED Requirements

### Requirement: MOD on integers returns integer remainder

`MOD(a, b)` where both `a` and `b` are integers MUST return the
truncated integer remainder `a % b` (PostgreSQL/SQLite semantics).

#### Scenario: Basic integer MOD

- GIVEN the expression `MOD(10, 3)`
- THEN the result is `Value::Integer(1)`.
- AND the column type of the result is `Integer` (not `Float`).

#### Scenario: Exact division

- GIVEN `MOD(9, 3)`
- THEN the result is `Value::Integer(0)`.

#### Scenario: Dividend zero

- GIVEN `MOD(0, 5)`
- THEN the result is `Value::Integer(0)`.

### Requirement: MOD with negative operands uses truncated division

`MOD(a, b)` for negative operands MUST follow truncated division
semantics (the sign of the result matches the dividend `a`).

#### Scenario: Negative dividend

- GIVEN `MOD(-7, 3)`
- THEN the result is `Value::Integer(-1)` because `-7 = 3 * (-2) + (-1)`.

#### Scenario: Negative divisor

- GIVEN `MOD(7, -3)`
- THEN the result is `Value::Integer(1)` because
  `7 = (-3) * (-2) + 1`.

### Requirement: MOD with zero divisor returns NULL

`MOD(a, 0)` MUST return `NULL` (SQLite-style) rather than raising a
division-by-zero error (PostgreSQL-style).

#### Scenario: Zero divisor

- GIVEN `MOD(10, 0)`
- THEN the result is `Value::Null`.

### Requirement: MOD with NULL operands returns NULL

`MOD(NULL, b)` and `MOD(a, NULL)` MUST both return `NULL` (standard SQL
NULL propagation).

#### Scenario: NULL operand

- GIVEN `MOD(NULL, 3)`
- THEN the result is `Value::Null`.
- GIVEN `MOD(10, NULL)`
- THEN the result is `Value::Null`.

### Requirement: MOD on floats returns float

`MOD(a, b)` where at least one operand is a float MUST return a float
remainder.

#### Scenario: Float MOD

- GIVEN `MOD(10.0, 3.0)`
- THEN the result is `Value::Float(1.0)` (or close to it due to
  floating-point representation).