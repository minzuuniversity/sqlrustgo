# parser-cast-date-types

## Purpose

`CAST(expr AS TYPE)` MUST accept DATE / DATETIME / TIMESTAMP / TIME / YEAR as target types without raising Parse error.

## ADDED Requirements

### Requirement: cast string to DATE type

The parser MUST accept `CAST(expr AS DATE)` and propagate "DATE" as the target type to the executor.

#### Scenario: DATE keyword token

GIVEN sql `SELECT CAST('2026-09-01' AS DATE)`
WHEN parser parses the CAST clause
THEN args list receives Literal("DATE"), the closing RParen is consumed, no Parse error is raised.

### Requirement: cast string to DATETIME/TIMESTAMP/TIME/YEAR

The parser MUST accept `CAST(expr AS DATETIME|TIMESTAMP|TIME|YEAR)` because those identifiers already route through the Identifier arm.

#### Scenario: DATETIME type

GIVEN sql `SELECT CAST('2026-09-01 12:00' AS DATETIME)`
WHEN parser parses
THEN args receives Literal("DATETIME"), no Parse error.

### Requirement: unknown types must error

The parser MUST reject unknown target type names.

#### Scenario: BOGUS type

GIVEN sql `SELECT CAST(1 AS BOGUS)`
WHEN parser parses
THEN a Parse error "Expected type name after CAST AS" is raised.

## Acceptance Criteria

- parser unit test `parse_cast_as_date_type` PASS
- parser unit test `parse_cast_as_datetime_timestamp_time_year` PASS
- integration test `cast_as_date_returns_string_value` PASS
- `cargo test --all-features` introduces no new failures