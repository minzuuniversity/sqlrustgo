## ADDED Requirements

### Requirement: builtin function coverage for common SQL scalar functions
The SQL scalar function registry (`eval_fn` in `crates/executor/src/expr/mod.rs`) MUST register and correctly evaluate the following functions when called as scalar expressions:

- `NOW()` — returns current timestamp as `Value::Text("YYYY-MM-DD HH:MM:SS")`
- `CURDATE()` / `CURRENT_DATE` — returns current date as `Value::Text("YYYY-MM-DD")`
- `CURTIME()` / `CURRENT_TIME` — returns current time as `Value::Text("HH:MM:SS")`
- `YEAR(date)` — extracts 4-digit year from a date/datetime string; returns `Value::Integer`
- `MONTH(date)` — extracts month (1-12) from a date/datetime string; returns `Value::Integer`
- `DAY(date)` — extracts day of month (1-31) from a date/datetime string; returns `Value::Integer`
- `DATEDIFF(date1, date2)` — returns `Value::Integer` days between two date strings (date1 − date2)
- `ROUND(number, decimals)` — rounds to `decimals` decimal places; returns `Value::Float`
- `RAND()` — returns a pseudo-random `Value::Float` in [0.0, 1.0)
- `LENGTH(str)` — returns UTF-8 character count of string; returns `Value::Integer`

Functions MUST be case-insensitive (matching by upper-cased name).

#### Scenario: now() returns a timestamp
- **WHEN** executing `SELECT NOW()` (or `now()`, `Now()`)
- **THEN** the result row contains a single `Text` value matching regex `^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$`

#### Scenario: round(x, n) returns rounded value
- **WHEN** executing `SELECT ROUND(3.14159, 2)`
- **THEN** result is approximately `3.14`

#### Scenario: year/month/day extract integers
- **WHEN** executing `SELECT YEAR('2019-05-15'), MONTH('2019-05-15'), DAY('2019-05-15')`
- **THEN** result is three rows: `2019`, `5`, `15`

#### Scenario: datediff counts days
- **WHEN** executing `SELECT DATEDIFF('2019-01-01', '2018-01-01')`
- **THEN** result is `365`

#### Scenario: rand() returns a value in [0, 1)
- **WHEN** executing `SELECT RAND()`
- **THEN** result is a `Float` ≥ 0.0 and < 1.0

#### Scenario: length returns char count
- **WHEN** executing `SELECT LENGTH('alice')`
- **THEN** result is `5`

### Requirement: unknown function default behavior preserved
`eval_fn` MUST continue to return `Value::Null` for unregistered function names (current behavior preserved).

#### Scenario: unknown function returns Null
- **WHEN** executing `SELECT FOO_BAR_BAZ()`
- **THEN** result is `Null` (no panic, no error)