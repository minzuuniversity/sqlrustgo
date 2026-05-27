## ADDED Requirements

### Requirement: Rule parsing and normalization
The system SHALL parse compliance rules from structured format (YAML/JSON) and normalize them for LLM consumption.

#### Scenario: Parse valid rule
- **WHEN** a valid compliance rule in YAML format is provided
- **THEN** the system SHALL parse it into an internal Rule struct with fields: id, text, severity, language, category

#### Scenario: Invalid rule format
- **WHEN** an invalid compliance rule format is provided
- **THEN** the system SHALL return a ParseError with details about the malformed section

### Requirement: Rule validation
The system SHALL validate compliance rules for completeness and consistency before use.

#### Scenario: Valid rule passes validation
- **WHEN** a complete and consistent rule is validated
- **THEN** the system SHALL return ValidationResult { valid: true }

#### Scenario: Incomplete rule fails validation
- **WHEN** a rule missing required fields is validated
- **THEN** the system SHALL return ValidationResult { valid: false, errors: [...] }
