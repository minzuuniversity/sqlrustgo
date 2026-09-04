## ADDED Requirements

### Requirement: Executor MUST evaluate POSITION(substr IN str) with 1-based indexing

`POSITION` follows the SQL standard: returns the 1-based byte position of `substr` within `str`, or 0 if not found. Comparison is case-sensitive.

#### Scenario: `POSITION('bc' IN 'abc')` returns 2

- **WHEN** the executor runs `SELECT POSITION('bc' IN 'abc')`
- **THEN** the result MUST be `2`

#### Scenario: `POSITION('bc' IN 'a%bc')` returns 3

- **WHEN** the executor runs `SELECT POSITION('bc' IN 'a%bc')`
- **THEN** the result MUST be `3`

#### Scenario: `POSITION('xyz' IN 'abc')` returns 0 (not found)

- **WHEN** the executor runs `SELECT POSITION('xyz' IN 'abc')`
- **THEN** the result MUST be `0`

#### Scenario: `POSITION` is case-sensitive

- **WHEN** the executor runs `SELECT POSITION('BC' IN 'abc')`
- **THEN** the result MUST be `0`

#### Scenario: NULL input returns NULL

- **WHEN** the executor runs `SELECT POSITION(NULL IN 'abc')`, `SELECT POSITION('bc' IN NULL)`
- **THEN** each result MUST be NULL

### Requirement: Executor MUST evaluate LOCATE(substr, str[, pos]) per MySQL

`LOCATE` follows MySQL semantics: returns the 1-based position of `substr` in `str`, or 0 if not found. Optional `pos` starts search from that 1-based position.

#### Scenario: `LOCATE('bc', 'abc')` returns 2

- **WHEN** the executor runs `SELECT LOCATE('bc', 'abc')`
- **THEN** the result MUST be `2`

#### Scenario: `LOCATE('xyz', 'abc')` returns 0 (not found)

- **WHEN** the executor runs `SELECT LOCATE('xyz', 'abc')`
- **THEN** the result MUST be `0`

#### Scenario: `LOCATE('bc', 'a%bc%bc', 3)` starts from position 3

- **WHEN** the executor runs `SELECT LOCATE('bc', 'a%bc%bc', 3)`
- **THEN** the result MUST be `4` (second occurrence starting from position 3)

#### Scenario: `LOCATE` is case-sensitive

- **WHEN** the executor runs `SELECT LOCATE('BC', 'abc')`
- **THEN** the result MUST be `0`

#### Scenario: NULL input returns NULL

- **WHEN** the executor runs `SELECT LOCATE(NULL, 'abc')`, `SELECT LOCATE('bc', NULL)`, `SELECT LOCATE('bc', 'abc', NULL)`
- **THEN** each result MUST be NULL
