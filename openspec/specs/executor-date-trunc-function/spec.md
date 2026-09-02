# executor-date-trunc-function Specification

## Purpose
TBD - created by archiving change v312-64a-parser-batch. Update Purpose after archive.
## Requirements
### Requirement: DATE_TRUNC must support common time units

`DATE_TRUNC('unit', date)` MUST truncate the date to the start of the given unit and return the result as a string.

#### Scenario: month truncation

GIVEN table t with dt = '2026-09-15'
WHEN `SELECT DATE_TRUNC('month', dt) FROM t`
THEN output is '2026-09-01'.

#### Scenario: year truncation

GIVEN table t with dt = '2026-09-15'
WHEN `SELECT DATE_TRUNC('year', dt) FROM t`
THEN output is '2026-01-01'.

#### Scenario: quarter truncation (Q3 → July 1)

GIVEN table t with dt = '2026-09-15' (Q3)
WHEN `SELECT DATE_TRUNC('quarter', dt) FROM t`
THEN output is '2026-07-01'.

#### Scenario: day truncation

GIVEN table t with dt = '2026-09-15 12:34:56'
WHEN `SELECT DATE_TRUNC('day', dt)`
THEN output is '2026-09-15 00:00:00'.

#### Scenario: hour truncation

GIVEN table t with dt = '2026-09-15 12:34:56'
WHEN `SELECT DATE_TRUNC('hour', dt)`
THEN output is '2026-09-15 12:00:00'.

### Requirement: unknown unit must error

`DATE_TRUNC` MUST reject unsupported unit names with a SqlError.

#### Scenario: decade not recognised

GIVEN `SELECT DATE_TRUNC('decade', dt)`
WHEN executed
THEN a SqlError containing "DATE_TRUNC unit must be one of" is raised.

