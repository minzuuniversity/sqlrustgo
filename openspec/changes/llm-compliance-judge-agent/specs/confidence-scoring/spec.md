## ADDED Requirements

### Requirement: Confidence score calculation
The system SHALL calculate a confidence score (0.0-1.0) for each judgment based on LLM logprob and response characteristics.

#### Scenario: High confidence judgment
- **WHEN** LLM returns high logprob and unambiguous response
- **THEN** the confidence score SHALL be >= 0.9

#### Scenario: Low confidence judgment
- **WHEN** LLM returns low logprob or ambiguous response
- **THEN** the confidence score SHALL be < 0.7

### Requirement: Confidence threshold
The system SHALL flag judgments below confidence threshold (0.7) for human review.

#### Scenario: Below threshold
- **WHEN** confidence score is below 0.7
- **THEN** the judgment status SHALL be set to "NEEDS_REVIEW"

### Requirement: Judgment explanation
The system SHALL provide human-readable explanation for each judgment citing relevant rule sections.

#### Scenario: Provide explanation
- **WHEN** a judgment is generated
- **THEN** the result SHALL include an explanation field citing specific rule text that influenced the decision
