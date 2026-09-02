# parser-maintenance-commands Specification

## Purpose
TBD - created by archiving change v312-64a-parser-batch. Update Purpose after archive.
## Requirements
### Requirement: VACUUM with no table name

The system MUST accept `VACUUM;` and return success with no output.

#### Scenario: VACUUM;

GIVEN sql `VACUUM;`
WHEN parsed and executed
THEN ExecutorResult::Empty is returned with exit code 0; no Parse or runtime error.

### Requirement: REINDEX with no table name

The system MUST accept `REINDEX;` and return success with no output.

#### Scenario: REINDEX;

GIVEN sql `REINDEX;`
WHEN parsed and executed
THEN ExecutorResult::Empty is returned with exit code 0; no Parse error.

### Requirement: ANALYZE with no table name collects all tables

When ANALYZE has no table name, the system MUST collect stats for every user table and return success.

#### Scenario: ANALYZE; over multiple tables

GIVEN sql `ANALYZE;` after creating tables t1/t2/t3
WHEN parsed and executed
THEN `collect_table_stats` runs for t1, t2, t3 and writes to the stats map; ExecutorResult::Empty is returned; no "table name is required" error.

### Requirement: ANALYZE with table name

When ANALYZE names a table, the system MUST collect stats for that single table.

#### Scenario: ANALYZE t;

GIVEN sql `ANALYZE t;`
WHEN parsed and executed
THEN `collect_table_stats` runs for t; ExecutorResult::Empty is returned.

