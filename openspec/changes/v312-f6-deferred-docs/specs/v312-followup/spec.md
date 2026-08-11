# V312-F-6 Spec: DEFERRED Items Documentation Update

## ADDED Requirements

### Requirement: V312-13-REPORT.md MUST declare DONE/DEFERRED boundary

The V312-13 evidence report MUST have an explicit section listing
which steps are DONE in V312-13 vs DEFERRED to V312-24 (#3959).

#### Scenario: report has boundary section
- **WHEN** the report is read
- **THEN** it contains a `## Done/Deferred Boundary` section
- **AND** every step in the report is marked DONE or DEFERRED

### Requirement: MYSQL_COMPAT_STATUS.md MUST cross-reference #3959

The MySQL compatibility status doc MUST link to ISSUE #3959 for
items deferred from V312-13 to V312-24.

#### Scenario: cross-reference present
- **WHEN** the doc is read
- **THEN** it contains a reference to `openclaw/sqlrustgo#3959`

### Requirement: load-data-report.md MUST note server-side deferred status

The LOAD DATA report MUST clearly state that server-side execution
is deferred, with cross-reference to #3959.

#### Scenario: deferred status noted
- **WHEN** the report is read
- **THEN** it contains a section titled "Server-Side Execution: DEFERRED"
- **AND** it cross-references #3959