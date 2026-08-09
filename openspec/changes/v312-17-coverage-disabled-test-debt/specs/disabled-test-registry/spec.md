# disabled-test-registry

## ADDED Requirements

### Requirement: Disabled test registry
The system SHALL maintain a registry of all disabled tests with disposition decisions.

#### Scenario: Registry completeness
- **WHEN** a test has `#[ignore]` marker
- **THEN** registry entry exists with: test name, file:line, decision, owner, expiry, evidence hash
- **AND** decision is one of: RESTORE, REWRITE, QUARANTINE, RETIRE

### Requirement: Parser disabled test disposition
The system SHALL document disposition for `test_parse_statements_multiple`.

#### Scenario: test_parse_statements_multiple decision
- **WHEN** evaluation of `test_parse_statements_multiple` (parser.rs:13242)
- **THEN** decision is RESTORE or QUARANTINE with owner and expiry
- **AND** if QUARANTINE: linked issue exists with fix plan

### Requirement: Parser disabled test disposition (no trailing)
The system SHALL document disposition for `test_parse_statements_no_trailing`.

#### Scenario: test_parse_statements_no_trailing decision
- **WHEN** evaluation of `test_parse_statements_no_trailing` (parser.rs:13249)
- **THEN** decision is RESTORE or QUARANTINE with owner and expiry
- **AND** if QUARANTINE: linked issue exists with fix plan

### Requirement: Quarantine expiration
The system SHALL NOT allow quarantined tests to remain past their expiry date.

#### Scenario: Expired quarantine
- **WHEN** quarantine expiry date passes
- **THEN** test appears in gate failure report
- **AND** owner is notified for action
