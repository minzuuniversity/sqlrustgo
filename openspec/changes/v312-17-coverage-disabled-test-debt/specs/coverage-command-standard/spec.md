# coverage-command-standard

## ADDED Requirements

### Requirement: Canonical coverage command
The system SHALL provide a canonical, reproducible coverage measurement command for RC gate execution.

#### Scenario: Coverage command execution
- **WHEN** operator runs `cargo llvm-cov --workspace --tests --all-features`
- **THEN** coverage report generates without error
- **AND** report contains per-crate line coverage percentages

#### Scenario: Coverage command with HTML output
- **WHEN** operator runs `cargo llvm-cov --workspace --tests --all-features --open`
- **THEN** HTML report opens in default browser
- **AND** report is viewable offline at `llvm_cov/`

### Requirement: Coverage threshold enforcement
The system SHALL fail RC gate if per-crate coverage falls below documented threshold.

#### Scenario: Coverage below threshold
- **WHEN** coverage measurement returns < 80% for any tracked crate
- **THEN** gate script exits with non-zero status
- **AND** failure report identifies deficient crates

### Requirement: Coverage report format
Coverage reports SHALL use machine-readable JSON format for automated gate processing.

#### Scenario: JSON report generation
- **WHEN** coverage measurement completes
- **THEN** JSON report writes to `docs/releases/v3.12.0/coverage-baseline/<crate>-lib.json`
- **AND** each JSON contains `data[0].summary.percent_covered` field
