# coverage-report-per-crate

## ADDED Requirements

### Requirement: Per-crate coverage tracking
The system SHALL track and report coverage independently for each critical crate.

#### Scenario: Parser crate coverage
- **WHEN** coverage measurement runs on workspace
- **THEN** parser crate coverage appears in report with percentage
- **AND** report location: `docs/releases/v3.12.0/coverage-baseline/sqlrustgo-parser-lib.json`

#### Scenario: MySQL server crate coverage
- **WHEN** coverage measurement runs on workspace
- **THEN** mysql-server crate coverage appears in report with percentage
- **AND** report location: `docs/releases/v3.12.0/coverage-baseline/sqlrustgo-mysql-server-lib.json`

#### Scenario: MySQL client crate coverage
- **WHEN** coverage measurement runs on workspace
- **THEN** mysql-client crate coverage appears in report with percentage
- **AND** report location: `docs/releases/v3.12.0/coverage-baseline/sqlrustgo-mysql-client-lib.json`

### Requirement: Coverage gap identification
The system SHALL identify specific files/functions with low coverage.

#### Scenario: Low coverage file identification
- **WHEN** coverage report is generated
- **THEN** report lists files with < 50% line coverage
- **AND** each entry includes file path and uncovered line ranges
