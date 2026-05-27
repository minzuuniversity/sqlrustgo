## ADDED Requirements

### Requirement: Real-time compliance judgment
The system SHALL evaluate operations against GMP compliance rules and return a judgment result within 500ms.

#### Scenario: Fast judgment request
- **WHEN** a compliance judgment request is received
- **THEN** the system SHALL complete the LLM inference and return a judgment within 500ms

#### Scenario: Timeout handling
- **WHEN** a compliance judgment request exceeds 500ms timeout
- **THEN** the system SHALL return a "TIMEOUT" status with partial results if available

### Requirement: Judgment result structure
The system SHALL return a structured judgment result containing: status, reason, confidence_score, and evidence.

#### Scenario: Compliant operation
- **WHEN** an operation is judged as compliant
- **THEN** the result SHALL contain status="COMPLIANT", confidence_score (0.0-1.0), and supporting evidence

#### Scenario: Non-compliant operation
- **WHEN** an operation is judged as non-compliant
- **THEN** the result SHALL contain status="NON_COMPLIANT", confidence_score, and violation details

#### Scenario: Uncertain judgment
- **WHEN** the LLM cannot make a definitive judgment
- **THEN** the result SHALL contain status="NEEDS_REVIEW" with confidence_score below threshold

### Requirement: Multi-language rule support
The system SHALL support compliance rule evaluation for multiple languages (Chinese, English, Japanese, etc.).

#### Scenario: Chinese rule evaluation
- **WHEN** a compliance rule is written in Chinese
- **THEN** the system SHALL correctly interpret and evaluate against Chinese GMP standards

#### Scenario: English rule evaluation
- **WHEN** a compliance rule is written in English
- **THEN** the system SHALL correctly interpret and evaluate against international GMP standards
