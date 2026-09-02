# executor-varchar-length-validation Specification

## Purpose
TBD - created by archiving change v312-63-executor-batch-2. Update Purpose after archive.
## Requirements
### Requirement: VARCHAR(N) length validation on INSERT
INSERT statements MUST reject any row whose string value exceeds the declared `VARCHAR(N)` or `CHAR(N)` length of the target column.

#### Scenario: INSERT into VARCHAR(5) with 6-char string returns error
- GIVEN a table `t(c5 VARCHAR(5))`
- WHEN `INSERT INTO t VALUES ('abcdef')` runs
- THEN the call returns `Err(SqlError::ExecutionError)` with a message containing the column name and the actual-vs-declared length

#### Scenario: INSERT into VARCHAR(5) with 3-char string succeeds
- GIVEN a table `t(c5 VARCHAR(5))`
- WHEN `INSERT INTO t VALUES ('abc')` runs
- THEN the call returns `Ok` and the row is stored as `'abc'`

#### Scenario: CHAR(5) accepts short strings via padding but rejects overlong
- GIVEN a table `t(c CHAR(5))`
- WHEN `INSERT INTO t VALUES ('abc')` runs THEN `'abc  '` is stored (padded)
- AND when `INSERT INTO t VALUES ('abcdef')` runs THEN the call returns `Err`

### Requirement: VARCHAR(N) length validation on UPDATE
UPDATE statements MUST reject any assignment whose string value exceeds the declared `VARCHAR(N)` / `CHAR(N)` length of the target column.

#### Scenario: UPDATE setting VARCHAR(5) to 6-char string returns error
- GIVEN a table `t(id INT, c5 VARCHAR(5))` with row `(1, 'abc')`
- WHEN `UPDATE t SET c5 = 'abcdef' WHERE id = 1` runs
- THEN the call returns `Err(SqlError::ExecutionError)`

### Requirement: ODKU length validation
`ON DUPLICATE KEY UPDATE` SET expressions MUST also be validated against the declared VARCHAR/CHAR length, mirroring the regular UPDATE rule.

#### Scenario: ODKU assigning overlong string returns error
- GIVEN a table `t(id INT PRIMARY KEY, c5 VARCHAR(5))` with row `(1, 'abc')`
- WHEN `INSERT INTO t VALUES (1, 'x') ON DUPLICATE KEY UPDATE c5 = 'abcdef'` runs
- THEN the call returns `Err(SqlError::ExecutionError)`

