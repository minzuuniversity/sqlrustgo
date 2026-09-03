## ADDED Requirements

### Requirement: Executor MUST return empty string for `SUBSTRING(s, 0)`

The executor MUST return an empty string when the start position of `SUBSTRING` / `SUBSTR` is 0 (or any non-positive value). Position 1 is the first character. Positions greater than the string length return an empty string.

#### Scenario: `SUBSTRING('hello', 0)` returns empty

- **WHEN** the executor runs `SELECT SUBSTRING('hello', 0)`
- **THEN** the result MUST be the empty string `''`

#### Scenario: `SUBSTRING('hello', 1)` returns the whole string

- **WHEN** the executor runs `SELECT SUBSTRING('hello', 1)`
- **THEN** the result MUST be `'hello'`

#### Scenario: `SUBSTRING('hello', 5)` returns the last char

- **WHEN** the executor runs `SELECT SUBSTRING('hello', 5)`
- **THEN** the result MUST be `'o'`

#### Scenario: `SUBSTRING('hello', 0, 3)` returns empty (zero start, any length)

- **WHEN** the executor runs `SELECT SUBSTRING('hello', 0, 3)`
- **THEN** the result MUST be `''`

#### Scenario: `SUBSTRING('hello', 1, 3)` returns first 3 chars

- **WHEN** the executor runs `SELECT SUBSTRING('hello', 1, 3)`
- **THEN** the result MUST be `'hel'`

#### Scenario: `SUBSTRING('hello', 100)` returns empty (out of range)

- **WHEN** the executor runs `SELECT SUBSTRING('hello', 100)`
- **THEN** the result MUST be `''`

#### Scenario: `SUBSTR` (alias) behaves identically

- **WHEN** the executor runs `SELECT SUBSTR('hello', 0), SUBSTR('hello', 1), SUBSTR('hello', 5)`
- **THEN** the result MUST be `('', 'hello', 'o')`
